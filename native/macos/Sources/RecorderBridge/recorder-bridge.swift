import CoreAudio

/// The object behind an opaque `recorder_handle`: one capture source at a time.
final class RecorderHandle {
    let microphone: MicrophoneCapture
    let system: SystemCapture
    private(set) var capture: CaptureSession
    private var applicationPIDs: [pid_t] = []

    init(calls: CoreAudioCalls, systemCalls: SystemAudioCalls) {
        microphone = MicrophoneCapture(calls: calls)
        system = SystemCapture(calls: systemCalls)
        capture = microphone
    }

    /// Switches to the requested source; fails while the current one is prepared or running,
    /// and for the application source until processes are set.
    func select(_ sourceKind: UInt32) -> BridgeResult {
        guard capture.isIdle else { return .invalidState }
        switch sourceKind {
        case sourceKindSystem:
            system.includedPIDs = nil
            capture = system
        case sourceKindApplication:
            guard !applicationPIDs.isEmpty else { return .invalidState }
            system.includedPIDs = applicationPIDs
            capture = system
        default:
            capture = microphone
        }
        return .ok
    }

    func setProcesses(_ pids: [pid_t]) -> BridgeResult {
        guard capture.isIdle else { return .invalidState }
        applicationPIDs = pids
        return .ok
    }
}

let sourceKindMicrophone: UInt32 = 1
let sourceKindSystem: UInt32 = 2
let sourceKindApplication: UInt32 = 3
let maxApplicationProcesses: UInt32 = 64
/// `recorder_process`: `int32_t pid`, `uint32_t flags`, `char bundle_id[256]`.
let processEntryBytes = 264
let processBundleIDBytes = 256
let processRunningOutput: UInt32 = 1 << 0
let processRunningInput: UInt32 = 1 << 1

// Each entry point validates its pointers and forwards to a testable function that takes
// the Core Audio calls explicitly. No Swift error crosses the C ABI.

@_cdecl("recorder_microphone_access")
public func recorderMicrophoneAccess(_ request: UInt32, _ access: UnsafeMutablePointer<UInt32>?) -> Int32 {
    queryMicrophoneAccess(request, access, calls: .live)
}

func queryMicrophoneAccess(_ request: UInt32, _ access: UnsafeMutablePointer<UInt32>?, calls: CoreAudioCalls)
    -> Int32
{
    guard let access else { return BridgeResult.invalidArgument.rawValue }
    access.pointee = microphoneAccess(calls: calls, request: request != 0).rawValue
    return BridgeResult.ok.rawValue
}

@_cdecl("recorder_list")
public func recorderList(_ count: UnsafeMutablePointer<UInt32>?) -> Int32 {
    listSources(count, calls: .live)
}

func listSources(_ count: UnsafeMutablePointer<UInt32>?, calls: CoreAudioCalls) -> Int32 {
    guard let count else { return BridgeResult.invalidArgument.rawValue }
    count.pointee = 0
    var device = AudioObjectID(kAudioObjectUnknown)
    guard calls.defaultInputDevice(&device) == noErr else { return BridgeResult.nativeError.rawValue }
    count.pointee = device == kAudioObjectUnknown ? 0 : 1
    return BridgeResult.ok.rawValue
}

@_cdecl("recorder_create")
public func recorderCreate(_ output: UnsafeMutablePointer<UnsafeMutableRawPointer?>?) -> Int32 {
    createHandle(output, calls: .live, systemCalls: .live)
}

func createHandle(
    _ output: UnsafeMutablePointer<UnsafeMutableRawPointer?>?, calls: CoreAudioCalls, systemCalls: SystemAudioCalls
) -> Int32 {
    guard let output else { return BridgeResult.invalidArgument.rawValue }
    output.pointee = Unmanaged.passRetained(RecorderHandle(calls: calls, systemCalls: systemCalls)).toOpaque()
    return BridgeResult.ok.rawValue
}

@_cdecl("recorder_prepare")
public func recorderPrepare(
    _ handle: UnsafeMutableRawPointer?, _ sourceKind: UInt32, _ format: UnsafeMutableRawPointer?
) -> Int32 {
    guard let handle, let format, (sourceKindMicrophone...sourceKindApplication).contains(sourceKind) else {
        return BridgeResult.invalidArgument.rawValue
    }
    let recorder = recorder(handle)
    let selected = recorder.select(sourceKind)
    guard selected == .ok else { return selected.rawValue }
    var negotiated = RecorderFormat()
    let result = recorder.capture.prepare(into: &negotiated)
    if result == .ok {
        format.assumingMemoryBound(to: RecorderFormat.self).pointee = negotiated
    }
    return result.rawValue
}

@_cdecl("recorder_set_processes")
public func recorderSetProcesses(_ handle: UnsafeMutableRawPointer?, _ pids: UnsafePointer<Int32>?, _ count: UInt32)
    -> Int32
{
    guard let handle, let pids, (1...maxApplicationProcesses).contains(count) else {
        return BridgeResult.invalidArgument.rawValue
    }
    let copied = Array(UnsafeBufferPointer(start: pids, count: Int(count)))
    guard copied.allSatisfy({ $0 > 0 }) else { return BridgeResult.invalidArgument.rawValue }
    return recorder(handle).setProcesses(copied).rawValue
}

@_cdecl("recorder_list_processes")
public func recorderListProcesses(
    _ processes: UnsafeMutableRawPointer?, _ capacity: UInt32, _ count: UnsafeMutablePointer<UInt32>?
) -> Int32 {
    listProcesses(processes, capacity, count, calls: .live)
}

/// Writes up to `capacity` entries and the number of processes listed. A process that exits
/// between the list and its details is skipped.
func listProcesses(
    _ processes: UnsafeMutableRawPointer?, _ capacity: UInt32, _ count: UnsafeMutablePointer<UInt32>?,
    calls: SystemAudioCalls
) -> Int32 {
    guard let count, capacity == 0 || processes != nil else { return BridgeResult.invalidArgument.rawValue }
    count.pointee = 0
    var objects: [AudioObjectID] = []
    guard calls.processObjects(&objects) == noErr else { return BridgeResult.nativeError.rawValue }
    var listed: UInt32 = 0
    for object in objects {
        var details = AudioProcessDetails()
        guard calls.processDetails(object, &details) == noErr else { continue }
        if listed < capacity, let processes {
            let entry = processes.advanced(by: Int(listed) * processEntryBytes)
            entry.storeBytes(of: details.pid, toByteOffset: 0, as: Int32.self)
            let flags =
                (details.isRunningOutput ? processRunningOutput : 0) | (details.isRunningInput ? processRunningInput : 0)
            entry.storeBytes(of: flags, toByteOffset: 4, as: UInt32.self)
            let bundleID = entry.advanced(by: 8)
            bundleID.initializeMemory(as: UInt8.self, repeating: 0, count: processBundleIDBytes)
            // Truncated to leave room for the terminating NUL.
            let bytes = Array(details.bundleID.utf8.prefix(processBundleIDBytes - 1))
            bundleID.copyMemory(from: bytes, byteCount: bytes.count)
        }
        listed += 1
    }
    count.pointee = listed
    return BridgeResult.ok.rawValue
}

@_cdecl("recorder_start")
public func recorderStart(
    _ handle: UnsafeMutableRawPointer?, _ callback: RecorderBlockCallback?, _ context: UnsafeMutableRawPointer?
) -> Int32 {
    guard let handle, let callback else { return BridgeResult.invalidArgument.rawValue }
    return recorder(handle).capture.start(callback: callback, context: context).rawValue
}

@_cdecl("recorder_stop")
public func recorderStop(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard let handle else { return BridgeResult.invalidArgument.rawValue }
    return recorder(handle).capture.stop().rawValue
}

@_cdecl("recorder_destroy")
public func recorderDestroy(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard let handle else { return BridgeResult.invalidArgument.rawValue }
    let result = recorder(handle).capture.stop()
    Unmanaged<RecorderHandle>.fromOpaque(handle).release()
    return result.rawValue
}

@_cdecl("recorder_last_os_status")
public func recorderLastOSStatus(_ handle: UnsafeMutableRawPointer?, _ status: UnsafeMutablePointer<Int32>?)
    -> Int32
{
    guard let handle, let status else { return BridgeResult.invalidArgument.rawValue }
    status.pointee = recorder(handle).capture.lastOSStatus
    return BridgeResult.ok.rawValue
}

private func recorder(_ handle: UnsafeMutableRawPointer) -> RecorderHandle {
    Unmanaged<RecorderHandle>.fromOpaque(handle).takeUnretainedValue()
}
