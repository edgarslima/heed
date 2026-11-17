import CoreAudio
import Foundation

@testable import RecorderBridge

/// Scripted stand-in for the tap and aggregate-device calls. Records each call in `log`, lets
/// a test fail any step or change what the system reports, and plays IO cycles.
final class FakeSystemAudio {
    var outputDevice = AudioObjectID(7)
    var outputUID = "BuiltInSpeakerDevice"
    var tapFormat = FakeSystemAudio.floatFormat(channels: 2, flags: kAudioFormatFlagsNativeFloatPacked)
    var sampleRate: Double = 48_000
    var bufferFrames: UInt32 = 512
    var returnsProc = true
    /// Operation name → status to return instead of `noErr`.
    var failures: [String: OSStatus] = [:]
    private(set) var log: [String] = []
    private(set) var tapDescription: CATapDescription?
    private(set) var aggregateDescription: [String: Any]?
    private(set) var ioProc: AudioDeviceIOProc?
    private(set) var clientData: UnsafeMutableRawPointer?
    let tap = AudioObjectID(70)
    let aggregate = AudioObjectID(71)

    static func floatFormat(channels: UInt32, flags: AudioFormatFlags) -> AudioStreamBasicDescription {
        AudioStreamBasicDescription(
            mSampleRate: 48_000, mFormatID: kAudioFormatLinearPCM, mFormatFlags: flags, mBytesPerPacket: 4 * channels,
            mFramesPerPacket: 1, mBytesPerFrame: 4 * channels, mChannelsPerFrame: channels, mBitsPerChannel: 32,
            mReserved: 0)
    }

    var calls: SystemAudioCalls {
        SystemAudioCalls(
            defaultOutputDevice: { [self] device in step("outputDevice") { device = outputDevice } },
            deviceUID: { [self] _, uid in step("outputUID") { uid = outputUID } },
            createTap: { [self] description, tapID in
                step("createTap") {
                    tapDescription = description
                    tapID = tap
                }
            },
            tapFormat: { [self] _, format in step("tapFormat") { format = tapFormat } },
            createAggregate: { [self] description, device in
                step("createAggregate") {
                    aggregateDescription = description
                    device = aggregate
                }
            },
            nominalSampleRate: { [self] _, rate in step("sampleRate") { rate = sampleRate } },
            bufferFrameSize: { [self] _, frames in step("bufferFrames") { frames = bufferFrames } },
            createIOProc: { [self] _, proc, data, procID in
                step("createIOProc") {
                    ioProc = proc
                    clientData = data
                    // The fake uses the procedure itself as its identifier.
                    procID = returnsProc ? proc : nil
                }
            },
            start: { [self] _, _ in step("start") {} },
            stop: { [self] _, _ in step("stop") {} },
            destroyIOProc: { [self] _, _ in step("destroyIOProc") {} },
            destroyAggregate: { [self] _ in step("destroyAggregate") {} },
            destroyTap: { [self] _ in step("destroyTap") {} },
            hostClock: { HostClock(numerator: 125, denominator: 3) })
    }

    /// Plays one IO cycle with one buffer per entry of `channels`, each with its own channel
    /// count and samples. An empty list plays a cycle without buffers.
    @discardableResult
    func play(
        _ channels: [(count: UInt32, samples: [Float])], flags: AudioTimeStampFlags = [.hostTimeValid, .sampleTimeValid],
        hostTime: UInt64 = 3_000, sampleTime: Double = 0, nullData: Bool = false
    ) -> OSStatus {
        guard let ioProc else { return kAudioHardwareNotRunningError }
        let list = AudioBufferList.allocate(maximumBuffers: max(channels.count, 1))
        let storage = channels.map { channel -> UnsafeMutablePointer<Float> in
            let pointer = UnsafeMutablePointer<Float>.allocate(capacity: max(channel.samples.count, 1))
            pointer.initialize(from: channel.samples, count: channel.samples.count)
            return pointer
        }
        defer {
            storage.forEach { $0.deallocate() }
            free(list.unsafeMutablePointer)
        }
        list.count = channels.count
        for (index, channel) in channels.enumerated() {
            list[index] = AudioBuffer(
                mNumberChannels: channel.count, mDataByteSize: UInt32(channel.samples.count * 4),
                mData: nullData ? nil : UnsafeMutableRawPointer(storage[index]))
        }
        var stamp = AudioTimeStamp()
        stamp.mFlags = flags
        stamp.mHostTime = hostTime
        stamp.mSampleTime = sampleTime
        var now = AudioTimeStamp()
        var outputTime = AudioTimeStamp()
        var output = AudioBufferList()
        return ioProc(aggregate, &now, list.unsafePointer, &stamp, &output, &outputTime, clientData)
    }

    func clearLog() {
        log.removeAll()
    }

    private func step(_ name: String, _ effect: () -> Void) -> OSStatus {
        log.append(name)
        if let status = failures[name] { return status }
        effect()
        return noErr
    }
}
