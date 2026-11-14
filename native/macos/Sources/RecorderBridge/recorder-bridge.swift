import CoreAudio

/// The object behind an opaque `recorder_handle`.
final class RecorderHandle {
    let capture: MicrophoneCapture

    init(calls: CoreAudioCalls) {
        capture = MicrophoneCapture(calls: calls)
    }
}

let sourceKindMicrophone: UInt32 = 1

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
    createHandle(output, calls: .live)
}

func createHandle(_ output: UnsafeMutablePointer<UnsafeMutableRawPointer?>?, calls: CoreAudioCalls) -> Int32 {
    guard let output else { return BridgeResult.invalidArgument.rawValue }
    output.pointee = Unmanaged.passRetained(RecorderHandle(calls: calls)).toOpaque()
    return BridgeResult.ok.rawValue
}

@_cdecl("recorder_prepare")
public func recorderPrepare(
    _ handle: UnsafeMutableRawPointer?, _ sourceKind: UInt32, _ format: UnsafeMutableRawPointer?
) -> Int32 {
    guard let handle, let format, sourceKind == sourceKindMicrophone else {
        return BridgeResult.invalidArgument.rawValue
    }
    var negotiated = RecorderFormat()
    let result = recorder(handle).capture.prepare(into: &negotiated)
    if result == .ok {
        format.assumingMemoryBound(to: RecorderFormat.self).pointee = negotiated
    }
    return result.rawValue
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
