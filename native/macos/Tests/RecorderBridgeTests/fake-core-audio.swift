import AVFoundation
import AudioToolbox
import CoreAudio

@testable import RecorderBridge

/// Scripted stand-in for Core Audio. Records each call in `log` and lets a test fail any
/// step, change the device format, or play the device by invoking the input callback.
final class FakeCoreAudio {
    var accessStatus = AVAuthorizationStatus.authorized
    var requestAnswer = true
    var requestCount = 0
    var device = AudioObjectID(42)
    var returnsUnit = true
    var hardwareFormat = FakeCoreAudio.format(sampleRate: 48_000, channels: 1)
    var maxFrames: UInt32 = 512
    var reportedGetSize: UInt32?
    var renderValue: Float = 0.25
    /// Operation name → status to return instead of `noErr`.
    var failures: [String: OSStatus] = [:]
    private(set) var log: [String] = []
    private(set) var clientFormat: AudioStreamBasicDescription?
    private(set) var inputCallback: AURenderCallbackStruct?
    private(set) var renderBus: UInt32?
    let unit = AudioUnit(bitPattern: 0x1000)!

    static func format(sampleRate: Double, channels: UInt32) -> AudioStreamBasicDescription {
        AudioStreamBasicDescription(
            mSampleRate: sampleRate, mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 2 * channels, mFramesPerPacket: 1, mBytesPerFrame: 2 * channels,
            mChannelsPerFrame: channels, mBitsPerChannel: 16, mReserved: 0)
    }

    var calls: CoreAudioCalls {
        CoreAudioCalls(
            defaultInputDevice: { [self] output in
                step("device") {
                    output.pointee = device
                }
            },
            newInputUnit: { [self] output in
                step("newUnit") {
                    output.pointee = returnsUnit ? unit : nil
                }
            },
            setProperty: { [self] _, property, scope, _, value, _ in
                step("set \(Self.name(property, scope))") {
                    switch property {
                    case kAudioUnitProperty_StreamFormat:
                        clientFormat = value.load(as: AudioStreamBasicDescription.self)
                    case kAudioOutputUnitProperty_SetInputCallback:
                        inputCallback = value.load(as: AURenderCallbackStruct.self)
                    default:
                        break
                    }
                }
            },
            getProperty: { [self] _, property, scope, _, value, size in
                step("get \(Self.name(property, scope))") {
                    if property == kAudioUnitProperty_StreamFormat {
                        value.storeBytes(of: hardwareFormat, as: AudioStreamBasicDescription.self)
                    } else {
                        value.storeBytes(of: maxFrames, as: UInt32.self)
                    }
                    size.pointee = reportedGetSize ?? size.pointee
                }
            },
            initialize: { [self] _ in step("initialize") {} },
            start: { [self] _ in step("start") {} },
            stop: { [self] _ in step("stop") {} },
            uninitialize: { [self] _ in step("uninitialize") {} },
            dispose: { [self] _ in step("dispose") {} },
            render: { [self] _, _, _, bus, _, buffers in
                renderBus = bus
                if let status = failures["render"] { return status }
                let buffer = buffers.pointee.mBuffers
                let count = Int(buffer.mDataByteSize) / MemoryLayout<Float>.size
                buffer.mData?.assumingMemoryBound(to: Float.self).update(repeating: renderValue, count: count)
                return noErr
            },
            microphoneAccess: { [self] in accessStatus },
            requestMicrophoneAccess: { [self] completion in
                requestCount += 1
                let answer = requestAnswer
                DispatchQueue.global().async { completion(answer) }
            },
            hostClock: { HostClock(numerator: 125, denominator: 3) })
    }

    /// Plays one device cycle through the registered input callback.
    func play(frames: UInt32, flags: AudioTimeStampFlags = [.hostTimeValid, .sampleTimeValid], hostTime: UInt64 = 3_000,
        sampleTime: Double = 0) -> OSStatus
    {
        guard let input = inputCallback, let procedure = input.inputProc, let refCon = input.inputProcRefCon else {
            return kAudioUnitErr_Uninitialized
        }
        var stamp = AudioTimeStamp()
        stamp.mFlags = flags
        stamp.mHostTime = hostTime
        stamp.mSampleTime = sampleTime
        var actionFlags = AudioUnitRenderActionFlags()
        return procedure(refCon, &actionFlags, &stamp, 1, frames, nil)
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

    private static func name(_ property: AudioUnitPropertyID, _ scope: AudioUnitScope) -> String {
        let scopeName = scope == kAudioUnitScope_Input ? "input" : scope == kAudioUnitScope_Output ? "output" : "global"
        switch property {
        case kAudioOutputUnitProperty_EnableIO: return "enableIO \(scopeName)"
        case kAudioOutputUnitProperty_CurrentDevice: return "currentDevice"
        case kAudioUnitProperty_StreamFormat: return "streamFormat \(scopeName)"
        case kAudioUnitProperty_MaximumFramesPerSlice: return "maxFrames"
        case kAudioOutputUnitProperty_SetInputCallback: return "inputCallback"
        default: return "property \(property)"
        }
    }
}

/// Receives blocks through the C callback, as the Rust side does.
final class BlockCollector {
    struct Received {
        let frameCount: UInt32
        let flags: UInt32
        let hostTimeNs: UInt64
        let sampleTime: Double
        let samples: [Float]
    }

    var blocks: [Received] = []
    var channelCount = 1

    var context: UnsafeMutableRawPointer {
        Unmanaged.passUnretained(self).toOpaque()
    }

    static let callback: RecorderBlockCallback = { context, block in
        guard let context, let block else { return }
        let collector = Unmanaged<BlockCollector>.fromOpaque(context).takeUnretainedValue()
        let value = block.load(as: RecorderBlock.self)
        let count = Int(value.frameCount) * collector.channelCount
        let samples = value.samples.map { Array(UnsafeBufferPointer(start: $0, count: count)) } ?? []
        collector.blocks.append(
            Received(
                frameCount: value.frameCount, flags: value.flags, hostTimeNs: value.hostTimeNs,
                sampleTime: value.sampleTime, samples: samples))
    }
}
