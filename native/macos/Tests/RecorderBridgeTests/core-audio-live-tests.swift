import AVFoundation
import AudioToolbox
import XCTest

@testable import RecorderBridge

/// Exercises the real Core Audio calls. The default run never starts input, so it cannot
/// trigger a microphone prompt. To capture, run `HEED_HARDWARE_TESTS=1 swift test --filter
/// CoreAudioLiveTests` from a terminal that has microphone access: `swift test` inherits the
/// terminal's TCC grant, while the runner that `xcodebuild` launches has its own identity and
/// skips the capture test unless that runner was granted access too. The system-audio test
/// needs the terminal's System Audio Recording grant; without it the tap delivers zeros.
final class CoreAudioLiveTests: XCTestCase {
    func testLiveCallsOpenAndReleaseAnInputUnitWithoutStartingIt() throws {
        let calls = CoreAudioCalls.live
        XCTAssertNotEqual(calls.hostClock().denominator, 0)
        _ = calls.microphoneAccess()
        var device = AudioObjectID(kAudioObjectUnknown)
        XCTAssertEqual(calls.defaultInputDevice(&device), noErr)
        var unit: AudioUnit?
        XCTAssertEqual(calls.newInputUnit(&unit), noErr)
        let created = try XCTUnwrap(unit)
        var enable: UInt32 = 1
        XCTAssertEqual(
            calls.setProperty(
                created, kAudioOutputUnitProperty_EnableIO, kAudioUnitScope_Input, 1, &enable,
                UInt32(MemoryLayout<UInt32>.size)),
            noErr)
        var maxFrames: UInt32 = 0
        var size = UInt32(MemoryLayout<UInt32>.size)
        XCTAssertEqual(
            calls.getProperty(created, kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitScope_Global, 0, &maxFrames, &size),
            noErr)
        XCTAssertGreaterThan(maxFrames, 0)
        XCTAssertEqual(calls.dispose(created), noErr)
    }

    func testHardwareCaptureDeliversBlocksAndStopsCallingBack() throws {
        guard ProcessInfo.processInfo.environment["HEED_HARDWARE_TESTS"] == "1" else {
            throw XCTSkip("Set HEED_HARDWARE_TESTS=1 to capture from the default input device.")
        }
        guard AVCaptureDevice.authorizationStatus(for: .audio) == .authorized else {
            throw XCTSkip("Microphone access is not granted to the test runner.")
        }
        for _ in 0..<3 {
            let collector = BlockCollector()
            let capture = MicrophoneCapture(calls: .live)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .ok)
            collector.channelCount = Int(format.channelCount)
            XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
            Thread.sleep(forTimeInterval: 1)
            XCTAssertEqual(capture.stop(), .ok)
            let delivered = collector.blocks.count
            Thread.sleep(forTimeInterval: 0.2)
            XCTAssertEqual(collector.blocks.count, delivered, "a callback ran after stop returned")
            let frames = collector.blocks.reduce(0) { $0 + Int($1.frameCount) }
            XCTAssertGreaterThan(frames, Int(format.sampleRateHz) * 8 / 10)
            XCTAssertTrue(collector.blocks.allSatisfy { $0.flags & blockHostTimeValid != 0 })
        }
    }

    func testHardwareSystemCaptureDeliversBlocksAndLeavesNoTapOrAggregate() throws {
        guard ProcessInfo.processInfo.environment["HEED_HARDWARE_TESTS"] == "1" else {
            throw XCTSkip("Set HEED_HARDWARE_TESTS=1 to capture system audio.")
        }
        // The tap delivers no IO cycles while nothing plays, so the test plays its own tone.
        let tone = try playTone()
        defer { tone.stop() }
        for _ in 0..<3 {
            let collector = BlockCollector()
            let capture = SystemCapture(calls: .live)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .ok, "OSStatus \(capture.lastOSStatus)")
            XCTAssertTrue(resolves(capture.aggregateUID, kAudioHardwarePropertyTranslateUIDToDevice))
            XCTAssertTrue(resolves(capture.tapUID, kAudioHardwarePropertyTranslateUIDToTap))
            collector.channelCount = Int(format.channelCount)
            XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
            Thread.sleep(forTimeInterval: 1)
            XCTAssertEqual(capture.stop(), .ok)
            let delivered = collector.blocks.count
            Thread.sleep(forTimeInterval: 0.2)
            XCTAssertEqual(collector.blocks.count, delivered, "a callback ran after stop returned")
            let frames = collector.blocks.reduce(0) { $0 + Int($1.frameCount) }
            XCTAssertGreaterThan(frames, Int(format.sampleRateHz) * 8 / 10)
            XCTAssertTrue(collector.blocks.allSatisfy { $0.flags & blockHostTimeValid != 0 })
            let peak = collector.blocks.lazy.flatMap(\.samples).map(abs).max() ?? 0
            XCTAssertGreaterThan(peak, 0, "all zeros: grant System Audio Recording to the terminal")
            XCTAssertTrue(disappears(capture.aggregateUID, kAudioHardwarePropertyTranslateUIDToDevice))
            XCTAssertTrue(disappears(capture.tapUID, kAudioHardwarePropertyTranslateUIDToTap))
        }

        // A prepare that fails after creating the tap and aggregate must release both.
        var failing = SystemAudioCalls.live
        failing.bufferFrameSize = { _, _ in OSStatus(kAudioHardwareUnspecifiedError) }
        let capture = SystemCapture(calls: failing)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .nativeError)
        XCTAssertEqual(capture.lastOSStatus, OSStatus(kAudioHardwareUnspecifiedError), "a teardown step failed")
        XCTAssertFalse(capture.aggregateUID.isEmpty)
        XCTAssertTrue(disappears(capture.aggregateUID, kAudioHardwarePropertyTranslateUIDToDevice))
        XCTAssertTrue(disappears(capture.tapUID, kAudioHardwarePropertyTranslateUIDToTap))
    }

    /// Plays a soft 440 Hz tone through the default output until the engine stops.
    private func playTone() throws -> AVAudioEngine {
        let engine = AVAudioEngine()
        var phase: Float = 0
        let step = 2 * Float.pi * 440 / 48_000
        let source = AVAudioSourceNode { _, _, frameCount, output in
            for frame in 0..<Int(frameCount) {
                let sample = 0.05 * sin(phase)
                phase = (phase + step).truncatingRemainder(dividingBy: 2 * Float.pi)
                for buffer in UnsafeMutableAudioBufferListPointer(output) {
                    buffer.mData?.assumingMemoryBound(to: Float.self)[frame] = sample
                }
            }
            return noErr
        }
        engine.attach(source)
        engine.connect(source, to: engine.mainMixerNode, format: AVAudioFormat(standardFormatWithSampleRate: 48_000, channels: 1))
        try engine.start()
        Thread.sleep(forTimeInterval: 0.3)
        return engine
    }

    /// The HAL drops a destroyed object from UID translation shortly after the call returns.
    private func disappears(_ uid: String, _ selector: AudioObjectPropertySelector) -> Bool {
        let deadline = Date().addingTimeInterval(1)
        while resolves(uid, selector) {
            guard Date() < deadline else { return false }
            Thread.sleep(forTimeInterval: 0.02)
        }
        return true
    }

    /// Whether the system object translates `uid` to a live device or tap.
    private func resolves(_ uid: String, _ selector: AudioObjectPropertySelector) -> Bool {
        var address = AudioObjectPropertyAddress(
            mSelector: selector, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var qualifier = uid as CFString
        var object = AudioObjectID(kAudioObjectUnknown)
        var size = UInt32(MemoryLayout<AudioObjectID>.size)
        let status = withUnsafePointer(to: &qualifier) {
            AudioObjectGetPropertyData(
                AudioObjectID(kAudioObjectSystemObject), &address, UInt32(MemoryLayout<CFString>.size), $0, &size,
                &object)
        }
        return status == noErr && object != kAudioObjectUnknown
    }
}
