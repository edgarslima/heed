import CoreAudio
import Foundation

/// Captures everything the Mac plays through a global process tap and a private aggregate device.
///
/// Lifecycle: `prepare` creates the tap, then an aggregate device clocked by the default output
/// device with the tap in its tap list, then the IOProc, and preallocates the interleave buffer;
/// `start` starts the device; `stop` stops it and destroys the IOProc, aggregate, and tap in
/// reverse creation order. All methods run on one control thread, except `deliver`, which runs
/// on the IO thread and never allocates, locks, or waits.
///
/// No public API reports the System Audio Recording grant: without it the tap delivers zeros.
final class SystemCapture: CaptureSession {
    private enum State {
        case idle
        case prepared
        case running
    }

    private let calls: SystemAudioCalls
    private let clock: HostClock
    private var state = State.idle
    private var tap = AudioObjectID(kAudioObjectUnknown)
    private var aggregate = AudioObjectID(kAudioObjectUnknown)
    private var procID: AudioDeviceIOProcID?
    private var samples: UnsafeMutablePointer<Float>?
    private var capacityFrames: UInt32 = 0
    private var channelCount: UInt32 = 0
    private var callback: RecorderBlockCallback?
    private var context: UnsafeMutableRawPointer?
    /// Read and written only by `deliver` while running.
    private var pendingDiscontinuity = false

    private(set) var lastOSStatus: OSStatus = noErr
    /// Identities of the last created tap and aggregate; tests check they no longer resolve.
    private(set) var tapUID = ""
    private(set) var aggregateUID = ""

    var isIdle: Bool { state == .idle }

    init(calls: SystemAudioCalls) {
        self.calls = calls
        clock = calls.hostClock()
    }

    deinit {
        stop()
    }

    func prepare(into format: inout RecorderFormat) -> BridgeResult {
        guard state == .idle else { return .invalidState }
        let result = open(&format)
        guard result == .ok else {
            release()
            return result
        }
        state = .prepared
        return .ok
    }

    func start(callback: @escaping RecorderBlockCallback, context: UnsafeMutableRawPointer?) -> BridgeResult {
        guard state == .prepared, let procID else { return .invalidState }
        self.callback = callback
        self.context = context
        pendingDiscontinuity = false
        guard check(calls.start(aggregate, procID)) else {
            self.callback = nil
            self.context = nil
            return .nativeError
        }
        state = .running
        return .ok
    }

    /// Idempotent. Every teardown step runs even when an earlier one fails. Once the IOProc is
    /// destroyed, Core Audio no longer calls it, so the caller may free the callback context.
    @discardableResult
    func stop() -> BridgeResult {
        guard state != .idle else { return .ok }
        var succeeded = true
        if state == .running, let procID {
            succeeded = check(calls.stop(aggregate, procID))
        }
        succeeded = release() && succeeded
        return succeeded ? .ok : .nativeError
    }

    private func open(_ format: inout RecorderFormat) -> BridgeResult {
        var output = AudioObjectID(kAudioObjectUnknown)
        guard check(calls.defaultOutputDevice(&output)) else { return .nativeError }
        guard output != kAudioObjectUnknown else { return .sourceUnavailable }
        var outputUID = ""
        guard check(calls.deviceUID(output, &outputUID)) else { return .nativeError }
        guard !outputUID.isEmpty else { return .sourceUnavailable }

        // The recorder plays no audio, so the global tap excludes no process.
        let description = CATapDescription(stereoGlobalTapButExcludeProcesses: [])
        description.uuid = UUID()
        description.name = "Heed system audio"
        description.isPrivate = true
        description.muteBehavior = .unmuted
        var created = AudioObjectID(kAudioObjectUnknown)
        guard check(calls.createTap(description, &created)) else { return .nativeError }
        tap = created
        tapUID = description.uuid.uuidString

        var tapFormat = AudioStreamBasicDescription()
        guard check(calls.tapFormat(tap, &tapFormat)) else { return .nativeError }
        let channels = tapFormat.mChannelsPerFrame
        guard tapFormat.mFormatID == kAudioFormatLinearPCM, tapFormat.mFormatFlags & kAudioFormatFlagIsFloat != 0,
            tapFormat.mBitsPerChannel == 32, supportedChannelCounts.contains(channels)
        else { return .sourceUnavailable }

        aggregateUID = UUID().uuidString
        // The default output device clocks the aggregate; drift compensation resamples the tap to it.
        let aggregateDescription: [String: Any] = [
            kAudioAggregateDeviceNameKey: "Heed system capture",
            kAudioAggregateDeviceUIDKey: aggregateUID,
            kAudioAggregateDeviceMainSubDeviceKey: outputUID,
            kAudioAggregateDeviceIsPrivateKey: true,
            kAudioAggregateDeviceIsStackedKey: false,
            kAudioAggregateDeviceTapAutoStartKey: true,
            kAudioAggregateDeviceSubDeviceListKey: [[kAudioSubDeviceUIDKey: outputUID]],
            kAudioAggregateDeviceTapListKey: [[kAudioSubTapUIDKey: tapUID, kAudioSubTapDriftCompensationKey: true]],
        ]
        var device = AudioObjectID(kAudioObjectUnknown)
        guard check(calls.createAggregate(aggregateDescription, &device)) else { return .nativeError }
        aggregate = device

        // The IOProc runs on the aggregate's clock, so its rate is the delivered rate.
        var sampleRate: Double = 0
        guard check(calls.nominalSampleRate(aggregate, &sampleRate)) else { return .nativeError }
        guard supportedSampleRatesHz.contains(sampleRate), sampleRate.rounded() == sampleRate else {
            return .sourceUnavailable
        }
        var maxFrames: UInt32 = 0
        guard check(calls.bufferFrameSize(aggregate, &maxFrames)) else { return .nativeError }
        guard (1...maxSupportedBlockFrames).contains(maxFrames) else { return .sourceUnavailable }

        // The IOProc holds an unretained reference: `stop` destroys it before this object goes away.
        var proc: AudioDeviceIOProcID?
        guard check(calls.createIOProc(aggregate, systemIOProc, Unmanaged.passUnretained(self).toOpaque(), &proc)),
            let proc
        else { return .nativeError }
        procID = proc

        let buffer = UnsafeMutablePointer<Float>.allocate(capacity: Int(maxFrames * channels))
        buffer.initialize(repeating: 0, count: Int(maxFrames * channels))
        samples = buffer
        capacityFrames = maxFrames
        channelCount = channels
        format = RecorderFormat(
            sampleRateHz: UInt32(sampleRate), channelCount: channels, sampleFormat: sampleFormatF32,
            maxBlockFrames: maxFrames)
        return .ok
    }

    /// Releases whatever `open` created, in reverse order, and returns to idle.
    @discardableResult
    private func release() -> Bool {
        var succeeded = true
        if let procID {
            succeeded = check(calls.destroyIOProc(aggregate, procID)) && succeeded
        }
        if aggregate != kAudioObjectUnknown {
            succeeded = check(calls.destroyAggregate(aggregate)) && succeeded
        }
        if tap != kAudioObjectUnknown {
            succeeded = check(calls.destroyTap(tap)) && succeeded
        }
        procID = nil
        aggregate = AudioObjectID(kAudioObjectUnknown)
        tap = AudioObjectID(kAudioObjectUnknown)
        callback = nil
        context = nil
        samples?.deallocate()
        samples = nil
        capacityFrames = 0
        state = .idle
        return succeeded
    }

    /// IO thread. Forwards interleaved input as borrowed, or interleaves planar input into the
    /// preallocated buffer first.
    fileprivate func deliver(_ input: UnsafePointer<AudioBufferList>, _ time: UnsafePointer<AudioTimeStamp>) {
        guard let callback, let samples else { return }
        let buffers = UnsafeMutableAudioBufferListPointer(UnsafeMutablePointer(mutating: input))
        let channels = Int(channelCount)
        guard let first = buffers.first, let firstData = first.mData else {
            pendingDiscontinuity = true
            return
        }
        let planar: Bool
        let frames: Int
        if buffers.count == 1, first.mNumberChannels == channelCount {
            planar = false
            frames = Int(first.mDataByteSize) / (4 * channels)
        } else if buffers.count == channels,
            buffers.allSatisfy({ $0.mNumberChannels == 1 && $0.mData != nil && $0.mDataByteSize == first.mDataByteSize })
        {
            planar = true
            frames = Int(first.mDataByteSize) / 4
        } else {
            pendingDiscontinuity = true
            return
        }
        guard frames > 0 else { return }
        guard frames <= Int(capacityFrames) else {
            pendingDiscontinuity = true
            return
        }
        if planar {
            for (channel, buffer) in buffers.enumerated() {
                guard let data = buffer.mData else { continue }
                let source = data.assumingMemoryBound(to: Float.self)
                for frame in 0..<frames {
                    samples[frame * channels + channel] = source[frame]
                }
            }
        }
        let interleaved = planar ? UnsafePointer(samples) : UnsafePointer(firstData.assumingMemoryBound(to: Float.self))
        var block = RecorderBlock(
            samples: interleaved, frameCount: UInt32(frames), stamp: time.pointee, clock: clock,
            discontinuity: pendingDiscontinuity)
        withUnsafePointer(to: &block) { callback(context, UnsafeRawPointer($0)) }
        pendingDiscontinuity = false
    }

    @discardableResult
    private func check(_ status: OSStatus) -> Bool {
        guard status == noErr else {
            lastOSStatus = status
            return false
        }
        return true
    }
}

private let systemIOProc: AudioDeviceIOProc = { _, _, input, inputTime, _, _, clientData in
    guard let clientData else { return noErr }
    Unmanaged<SystemCapture>.fromOpaque(clientData).takeUnretainedValue().deliver(input, inputTime)
    return noErr
}
