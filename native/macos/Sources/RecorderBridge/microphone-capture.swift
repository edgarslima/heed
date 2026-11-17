import AVFoundation
import AudioToolbox
import CoreAudio

/// Result codes of `recorder-bridge.h`.
enum BridgeResult: Int32 {
    case ok = 0
    case invalidArgument = 1
    case sourceUnavailable = 2
    case invalidState = 3
    case permissionDenied = 4
    case nativeError = 5
}

/// Mirrors `recorder_format`; XCTest pins the layout to the C header.
struct RecorderFormat: Equatable {
    var sampleRateHz: UInt32 = 0
    var channelCount: UInt32 = 0
    var sampleFormat: UInt32 = 0
    var maxBlockFrames: UInt32 = 0
}

/// Mirrors `recorder_block`; XCTest pins the layout to the C header.
struct RecorderBlock {
    var samples: UnsafePointer<Float>?
    var hostTimeNs: UInt64
    var sampleTime: Double
    var frameCount: UInt32
    var flags: UInt32
}

extension RecorderBlock {
    /// Builds a block from an audio-thread timestamp; never allocates.
    init(
        samples: UnsafePointer<Float>, frameCount: UInt32, stamp: AudioTimeStamp, clock: HostClock,
        discontinuity: Bool
    ) {
        var flags: UInt32 = discontinuity ? blockDiscontinuity : 0
        var hostTimeNs: UInt64 = 0
        if stamp.mFlags.contains(.hostTimeValid) {
            flags |= blockHostTimeValid
            hostTimeNs = clock.nanoseconds(stamp.mHostTime)
        }
        if stamp.mFlags.contains(.sampleTimeValid) {
            flags |= blockSampleTimeValid
        }
        self.init(
            samples: samples, hostTimeNs: hostTimeNs, sampleTime: stamp.mSampleTime, frameCount: frameCount,
            flags: flags)
    }
}

/// One capture source behind a handle. All methods run on the handle's control thread.
protocol CaptureSession: AnyObject {
    var isIdle: Bool { get }
    var lastOSStatus: OSStatus { get }
    func prepare(into format: inout RecorderFormat) -> BridgeResult
    func start(callback: @escaping RecorderBlockCallback, context: UnsafeMutableRawPointer?) -> BridgeResult
    @discardableResult func stop() -> BridgeResult
}

/// `recorder_block_callback`; the block argument points to a `RecorderBlock`.
public typealias RecorderBlockCallback = @convention(c) (UnsafeMutableRawPointer?, UnsafeRawPointer?) -> Void

let sampleFormatF32: UInt32 = 1
let blockHostTimeValid: UInt32 = 1 << 0
let blockSampleTimeValid: UInt32 = 1 << 1
let blockDiscontinuity: UInt32 = 1 << 2

/// Largest block the bridge preallocates for, about 340 ms at 48 kHz.
let maxSupportedBlockFrames: UInt32 = 16_384
let supportedSampleRatesHz: ClosedRange<Double> = 8_000...384_000
let supportedChannelCounts: ClosedRange<UInt32> = 1...8

/// AUHAL input element (bus 1) and output element (bus 0).
private let inputElement: AudioUnitElement = 1
private let outputElement: AudioUnitElement = 0

/// Captures the default input device through AUHAL.
///
/// Lifecycle: `prepare` opens and configures the unit and preallocates the render buffer;
/// `start` begins delivering blocks; `stop` stops, uninitializes, and disposes the unit
/// in reverse creation order and returns to idle. All methods run on one control thread,
/// except `deliver`, which runs on the audio thread and never allocates, locks, or waits.
final class MicrophoneCapture: CaptureSession {
    private enum State {
        case idle
        case prepared
        case running
    }

    private let calls: CoreAudioCalls
    private let clock: HostClock
    private var state = State.idle
    private var unit: AudioUnit?
    private var samples: UnsafeMutablePointer<Float>?
    private var capacityFrames: UInt32 = 0
    private var channelCount: UInt32 = 0
    private var bufferList = AudioBufferList()
    private var callback: RecorderBlockCallback?
    private var context: UnsafeMutableRawPointer?
    /// Read and written only by `deliver` while running.
    private var pendingDiscontinuity = false

    /// The last failed Core Audio status from a control-thread call, or `noErr`.
    private(set) var lastOSStatus: OSStatus = noErr

    var isIdle: Bool { state == .idle }

    init(calls: CoreAudioCalls) {
        self.calls = calls
        clock = calls.hostClock()
    }

    deinit {
        stop()
    }

    func prepare(into format: inout RecorderFormat) -> BridgeResult {
        guard state == .idle else { return .invalidState }
        guard calls.microphoneAccess() == .authorized else { return .permissionDenied }
        var device = AudioObjectID(kAudioObjectUnknown)
        guard check(calls.defaultInputDevice(&device)) else { return .nativeError }
        guard device != kAudioObjectUnknown else { return .sourceUnavailable }
        var created: AudioUnit?
        guard check(calls.newInputUnit(&created)), let created else { return .nativeError }
        let result = configure(created, device: device, format: &format)
        guard result == .ok else {
            check(calls.dispose(created))
            return result
        }
        unit = created
        state = .prepared
        return .ok
    }

    func start(callback: @escaping RecorderBlockCallback, context: UnsafeMutableRawPointer?) -> BridgeResult {
        guard state == .prepared, let unit else { return .invalidState }
        self.callback = callback
        self.context = context
        pendingDiscontinuity = false
        guard check(calls.start(unit)) else {
            self.callback = nil
            self.context = nil
            return .nativeError
        }
        state = .running
        return .ok
    }

    /// Idempotent. Every teardown step runs even when an earlier one fails. Once dispose
    /// returns, Core Audio no longer calls the input callback, so the caller may free the
    /// callback context.
    @discardableResult
    func stop() -> BridgeResult {
        guard let unit else { return .ok }
        var succeeded = true
        if state == .running {
            succeeded = check(calls.stop(unit)) && succeeded
        }
        succeeded = check(calls.uninitialize(unit)) && succeeded
        succeeded = check(calls.dispose(unit)) && succeeded
        self.unit = nil
        callback = nil
        context = nil
        samples?.deallocate()
        samples = nil
        capacityFrames = 0
        state = .idle
        return succeeded ? .ok : .nativeError
    }

    private func configure(_ unit: AudioUnit, device: AudioObjectID, format: inout RecorderFormat) -> BridgeResult {
        var enable: UInt32 = 1
        var disable: UInt32 = 0
        var device = device
        guard set(unit, kAudioOutputUnitProperty_EnableIO, kAudioUnitScope_Input, inputElement, &enable),
            set(unit, kAudioOutputUnitProperty_EnableIO, kAudioUnitScope_Output, outputElement, &disable),
            set(unit, kAudioOutputUnitProperty_CurrentDevice, kAudioUnitScope_Global, 0, &device)
        else { return .nativeError }

        var hardware = AudioStreamBasicDescription()
        guard get(unit, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Input, inputElement, &hardware) else {
            return .nativeError
        }
        let sampleRate = hardware.mSampleRate
        let channels = hardware.mChannelsPerFrame
        guard supportedSampleRatesHz.contains(sampleRate), sampleRate.rounded() == sampleRate,
            supportedChannelCounts.contains(channels)
        else { return .sourceUnavailable }

        // AUHAL converts the sample format but not the rate, so the client keeps the device rate.
        var client = AudioStreamBasicDescription(
            mSampleRate: sampleRate,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagsNativeFloatPacked,
            mBytesPerPacket: 4 * channels,
            mFramesPerPacket: 1,
            mBytesPerFrame: 4 * channels,
            mChannelsPerFrame: channels,
            mBitsPerChannel: 32,
            mReserved: 0)
        guard set(unit, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Output, inputElement, &client) else {
            return .nativeError
        }

        var maxFrames: UInt32 = 0
        guard get(unit, kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitScope_Global, 0, &maxFrames) else {
            return .nativeError
        }
        guard (1...maxSupportedBlockFrames).contains(maxFrames) else { return .sourceUnavailable }

        // The unit holds an unretained reference: `stop` disposes it before this object goes away.
        var input = AURenderCallbackStruct(
            inputProc: microphoneInputCallback,
            inputProcRefCon: Unmanaged.passUnretained(self).toOpaque())
        guard set(unit, kAudioOutputUnitProperty_SetInputCallback, kAudioUnitScope_Global, 0, &input),
            check(calls.initialize(unit))
        else { return .nativeError }

        let buffer = UnsafeMutablePointer<Float>.allocate(capacity: Int(maxFrames * channels))
        buffer.initialize(repeating: 0, count: Int(maxFrames * channels))
        samples = buffer
        capacityFrames = maxFrames
        channelCount = channels
        format = RecorderFormat(
            sampleRateHz: UInt32(sampleRate),
            channelCount: channels,
            sampleFormat: sampleFormatF32,
            maxBlockFrames: maxFrames)
        return .ok
    }

    /// Audio thread. Renders into the preallocated buffer and forwards a borrowed block.
    fileprivate func deliver(
        actionFlags: UnsafeMutablePointer<AudioUnitRenderActionFlags>,
        timeStamp: UnsafePointer<AudioTimeStamp>,
        frameCount: UInt32
    ) -> OSStatus {
        guard let unit, let samples, let callback, frameCount <= capacityFrames else {
            pendingDiscontinuity = true
            return noErr
        }
        bufferList.mNumberBuffers = 1
        bufferList.mBuffers = AudioBuffer(
            mNumberChannels: channelCount,
            mDataByteSize: frameCount * channelCount * 4,
            mData: UnsafeMutableRawPointer(samples))
        let status = calls.render(unit, actionFlags, timeStamp, inputElement, frameCount, &bufferList)
        guard status == noErr else {
            pendingDiscontinuity = true
            return status
        }
        var block = RecorderBlock(
            samples: UnsafePointer(samples), frameCount: frameCount, stamp: timeStamp.pointee, clock: clock,
            discontinuity: pendingDiscontinuity)
        withUnsafePointer(to: &block) { callback(context, UnsafeRawPointer($0)) }
        pendingDiscontinuity = false
        return noErr
    }

    @discardableResult
    private func check(_ status: OSStatus) -> Bool {
        guard status == noErr else {
            lastOSStatus = status
            return false
        }
        return true
    }

    private func set<Value>(
        _ unit: AudioUnit, _ property: AudioUnitPropertyID, _ scope: AudioUnitScope,
        _ element: AudioUnitElement, _ value: inout Value
    ) -> Bool {
        let size = UInt32(MemoryLayout<Value>.size)
        return check(withUnsafePointer(to: &value) { calls.setProperty(unit, property, scope, element, $0, size) })
    }

    private func get<Value>(
        _ unit: AudioUnit, _ property: AudioUnitPropertyID, _ scope: AudioUnitScope,
        _ element: AudioUnitElement, _ value: inout Value
    ) -> Bool {
        var size = UInt32(MemoryLayout<Value>.size)
        let status = withUnsafeMutablePointer(to: &value) {
            calls.getProperty(unit, property, scope, element, $0, &size)
        }
        return check(status) && size == UInt32(MemoryLayout<Value>.size)
    }
}

private let microphoneInputCallback: AURenderCallback = { refCon, actionFlags, timeStamp, _, frameCount, _ in
    Unmanaged<MicrophoneCapture>.fromOpaque(refCon).takeUnretainedValue()
        .deliver(actionFlags: actionFlags, timeStamp: timeStamp, frameCount: frameCount)
}

/// Maps the TCC status; with `request`, asks the user and waits for the answer.
/// Must not run on the thread that delivers the completion handler.
func microphoneAccess(calls: CoreAudioCalls, request: Bool) -> MicrophoneAccess {
    switch calls.microphoneAccess() {
    case .authorized:
        return .granted
    case .notDetermined:
        guard request else { return .notDetermined }
        let answer = AccessAnswer()
        calls.requestMicrophoneAccess { granted in
            answer.granted = granted
            answer.done.signal()
        }
        answer.done.wait()
        return answer.granted ? .granted : .denied
    default:
        return .denied
    }
}

/// The semaphore orders the write in the completion handler before the read after `wait`.
private final class AccessAnswer: @unchecked Sendable {
    var granted = false
    let done = DispatchSemaphore(value: 0)
}
