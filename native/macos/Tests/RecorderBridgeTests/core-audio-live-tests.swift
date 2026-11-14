import AVFoundation
import AudioToolbox
import XCTest

@testable import RecorderBridge

/// Exercises the real Core Audio calls. The default run never starts input, so it cannot
/// trigger a microphone prompt. Set `HEED_HARDWARE_TESTS=1` (with `xcodebuild`, pass
/// `TEST_RUNNER_HEED_HARDWARE_TESTS=1`) on a Mac with granted microphone access to capture.
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
}
