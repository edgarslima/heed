import CoreAudio
import XCTest

@testable import RecorderBridge

final class SystemCaptureTests: XCTestCase {
    private let openSteps = [
        "outputDevice", "outputUID", "createTap", "tapFormat", "createAggregate", "sampleRate", "bufferFrames",
        "createIOProc",
    ]

    func testPrepareCreatesAnUnmutedGlobalTapInAPrivateAggregate() throws {
        let fake = FakeSystemAudio()
        fake.sampleRate = 44_100
        fake.bufferFrames = 1_024
        let capture = SystemCapture(calls: fake.calls)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        XCTAssertEqual(
            format, RecorderFormat(sampleRateHz: 44_100, channelCount: 2, sampleFormat: 1, maxBlockFrames: 1_024))
        XCTAssertEqual(fake.log, openSteps)

        let tap = try XCTUnwrap(fake.tapDescription)
        XCTAssertEqual(tap.muteBehavior, .unmuted)
        XCTAssertTrue(tap.isPrivate)
        XCTAssertTrue(tap.isExclusive)
        XCTAssertTrue(tap.isMixdown)
        XCTAssertFalse(tap.isMono)
        XCTAssertEqual(capture.tapUID, tap.uuid.uuidString)

        let aggregate = try XCTUnwrap(fake.aggregateDescription)
        XCTAssertEqual(aggregate[kAudioAggregateDeviceUIDKey] as? String, capture.aggregateUID)
        XCTAssertEqual(aggregate[kAudioAggregateDeviceMainSubDeviceKey] as? String, "BuiltInSpeakerDevice")
        XCTAssertEqual(aggregate[kAudioAggregateDeviceIsPrivateKey] as? Bool, true)
        XCTAssertEqual(aggregate[kAudioAggregateDeviceTapAutoStartKey] as? Bool, true)
        let subDevices = try XCTUnwrap(aggregate[kAudioAggregateDeviceSubDeviceListKey] as? [[String: Any]])
        XCTAssertEqual(subDevices.first?[kAudioSubDeviceUIDKey] as? String, "BuiltInSpeakerDevice")
        let taps = try XCTUnwrap(aggregate[kAudioAggregateDeviceTapListKey] as? [[String: Any]])
        XCTAssertEqual(taps.count, 1)
        XCTAssertEqual(taps.first?[kAudioSubTapUIDKey] as? String, capture.tapUID)
        XCTAssertEqual(taps.first?[kAudioSubTapDriftCompensationKey] as? Bool, true)

        XCTAssertFalse(capture.isIdle)
        XCTAssertEqual(capture.prepare(into: &format), .invalidState)
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertTrue(capture.isIdle)
    }

    func testPrepareRejectsAMissingOutputOrAnUnsupportedFormat() {
        typealias Scenario = (name: String, change: (FakeSystemAudio) -> Void, lastStep: String)
        let scenarios: [Scenario] = [
            ("no output device", { $0.outputDevice = AudioObjectID(kAudioObjectUnknown) }, "outputDevice"),
            ("empty output UID", { $0.outputUID = "" }, "outputUID"),
            ("integer tap", { $0.tapFormat.mFormatFlags = kAudioFormatFlagIsSignedInteger }, "destroyTap"),
            ("not PCM", { $0.tapFormat.mFormatID = kAudioFormatMPEG4AAC }, "destroyTap"),
            ("64-bit float", { $0.tapFormat.mBitsPerChannel = 64 }, "destroyTap"),
            ("no channels", { $0.tapFormat.mChannelsPerFrame = 0 }, "destroyTap"),
            ("nine channels", { $0.tapFormat.mChannelsPerFrame = 9 }, "destroyTap"),
            ("fractional rate", { $0.sampleRate = 44_100.5 }, "destroyTap"),
            ("rate too low", { $0.sampleRate = 4_000 }, "destroyTap"),
            ("zero buffer", { $0.bufferFrames = 0 }, "destroyTap"),
            ("oversized buffer", { $0.bufferFrames = 16_385 }, "destroyTap"),
        ]
        for scenario in scenarios {
            let fake = FakeSystemAudio()
            scenario.change(fake)
            let capture = SystemCapture(calls: fake.calls)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .sourceUnavailable, scenario.name)
            XCTAssertEqual(fake.log.last, scenario.lastStep, scenario.name)
            XCTAssertFalse(fake.log.contains("createIOProc"), scenario.name)
            XCTAssertEqual(format, RecorderFormat(), scenario.name)
            XCTAssertTrue(capture.isIdle, scenario.name)
        }
    }

    func testPrepareFailuresUnwindInReverseOrderAndKeepTheStatus() {
        for (index, step) in openSteps.enumerated() {
            let fake = FakeSystemAudio()
            fake.failures[step] = -50
            let capture = SystemCapture(calls: fake.calls)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .nativeError, step)
            XCTAssertEqual(capture.lastOSStatus, -50, step)
            // Steps 2 and 4 create the tap and the aggregate; later failures release both.
            var expectedUnwind: [String] = []
            if index > 4 { expectedUnwind.append("destroyAggregate") }
            if index > 2 { expectedUnwind.append("destroyTap") }
            XCTAssertEqual(Array(fake.log[(index + 1)...]), expectedUnwind, step)
            fake.failures.removeAll()
            fake.clearLog()
            XCTAssertEqual(capture.prepare(into: &format), .ok, step)
            XCTAssertEqual(fake.log, openSteps, step)
        }
    }

    func testPrepareRejectsAMissingIOProc() {
        let fake = FakeSystemAudio()
        fake.returnsProc = false
        let capture = SystemCapture(calls: fake.calls)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .nativeError)
        XCTAssertEqual(Array(fake.log.suffix(2)), ["destroyAggregate", "destroyTap"])
    }

    func testStartRequiresAPreparedDeviceAndRecoversFromAFailedStart() {
        let fake = FakeSystemAudio()
        let collector = BlockCollector()
        let capture = SystemCapture(calls: fake.calls)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .invalidState)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        fake.failures["start"] = kAudioHardwareNotRunningError
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .nativeError)
        XCTAssertEqual(capture.lastOSStatus, kAudioHardwareNotRunningError)
        fake.play([(2, [0.5, 0.5])])
        XCTAssertTrue(collector.blocks.isEmpty)
        fake.failures.removeAll()
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .invalidState)
        XCTAssertEqual(capture.stop(), .ok)
    }

    func testDeliveryPassesInterleavedInputAndInterleavesPlanarInput() {
        let fake = FakeSystemAudio()
        let collector = BlockCollector()
        collector.channelCount = 2
        let capture = startedCapture(fake, collector)
        XCTAssertEqual(fake.play([(2, [1, -1, 2, -2, 3, -3])], hostTime: 3_000, sampleTime: 256), noErr)
        XCTAssertEqual(fake.play([(1, [1, 2, 3, 4]), (1, [-1, -2, -3, -4])], flags: []), noErr)
        XCTAssertEqual(collector.blocks.count, 2)
        let first = collector.blocks[0]
        XCTAssertEqual(first.frameCount, 3)
        XCTAssertEqual(first.samples, [1, -1, 2, -2, 3, -3])
        XCTAssertEqual(first.flags, blockHostTimeValid | blockSampleTimeValid)
        XCTAssertEqual(first.hostTimeNs, 125_000)
        XCTAssertEqual(first.sampleTime, 256)
        let second = collector.blocks[1]
        XCTAssertEqual(second.frameCount, 4)
        XCTAssertEqual(second.samples, [1, -1, 2, -2, 3, -3, 4, -4])
        XCTAssertEqual(second.flags, 0)
        XCTAssertEqual(capture.stop(), .ok)
    }

    func testUnexpectedOrOversizedCyclesMarkTheNextBlock() {
        let fake = FakeSystemAudio()
        fake.bufferFrames = 4
        let collector = BlockCollector()
        collector.channelCount = 2
        let capture = startedCapture(fake, collector)
        let frame: [Float] = [0.1, 0.2]
        fake.play([])
        fake.play([(2, frame)])
        fake.play([(2, frame)], nullData: true)
        fake.play([(1, [0.1]), (2, frame)])
        fake.play([(1, [0.1]), (1, [0.2, 0.3])])
        fake.play([(3, [0.1, 0.2, 0.3])])
        fake.play([(2, [Float](repeating: 0, count: 10))])
        fake.play([(2, frame)])
        fake.play([(2, [])])
        fake.play([(2, frame)])
        XCTAssertEqual(collector.blocks.map(\.frameCount), [1, 1, 1])
        XCTAssertEqual(
            collector.blocks.map { $0.flags & blockDiscontinuity }, [blockDiscontinuity, blockDiscontinuity, 0])
        XCTAssertEqual(capture.stop(), .ok)
    }

    func testStopTearsDownInReverseOrderAndIsIdempotent() {
        let fake = FakeSystemAudio()
        let collector = BlockCollector()
        let capture = startedCapture(fake, collector)
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log, ["stop", "destroyIOProc", "destroyAggregate", "destroyTap"])
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log.count, 4)
        fake.play([(2, [0.5, 0.5])])
        XCTAssertTrue(collector.blocks.isEmpty)

        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log, ["destroyIOProc", "destroyAggregate", "destroyTap"])
    }

    func testStopRunsEveryTeardownStepWhenOneFails() {
        let fake = FakeSystemAudio()
        let capture = startedCapture(fake, BlockCollector())
        fake.failures["stop"] = -1
        fake.failures["destroyAggregate"] = -2
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .nativeError)
        XCTAssertEqual(fake.log, ["stop", "destroyIOProc", "destroyAggregate", "destroyTap"])
        XCTAssertEqual(capture.lastOSStatus, -2)
        XCTAssertTrue(capture.isIdle)
        fake.failures.removeAll()
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
    }

    func testAHandleSwitchesSourceOnlyWhenIdle() throws {
        let microphone = FakeCoreAudio()
        let system = FakeSystemAudio()
        let collector = BlockCollector()
        collector.channelCount = 2
        var output: UnsafeMutableRawPointer?
        XCTAssertEqual(createHandle(&output, calls: microphone.calls, systemCalls: system.calls), 0)
        let handle = try XCTUnwrap(output)
        var format = RecorderFormat()
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindSystem, pointer), 0)
            XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, pointer), 3)
            XCTAssertEqual(recorderPrepare(handle, sourceKindSystem, pointer), 3)
        }
        XCTAssertEqual(format.channelCount, 2)
        XCTAssertEqual(microphone.log, [])
        XCTAssertEqual(recorderStart(handle, BlockCollector.callback, collector.context), 0)
        system.play([(2, [0.5, -0.5])])
        XCTAssertEqual(collector.blocks.count, 1)
        system.failures["createTap"] = -7
        XCTAssertEqual(recorderStop(handle), 0)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindSystem, pointer), 5)
        }
        var status: Int32 = 0
        XCTAssertEqual(recorderLastOSStatus(handle, &status), 0)
        XCTAssertEqual(status, -7)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, pointer), 0)
        }
        XCTAssertEqual(format.channelCount, 1)
        system.failures.removeAll()
        XCTAssertEqual(recorderStop(handle), 0)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindSystem, pointer), 0)
        }
        system.clearLog()
        XCTAssertEqual(recorderDestroy(handle), 0)
        XCTAssertEqual(system.log, ["destroyIOProc", "destroyAggregate", "destroyTap"])
    }

    private func startedCapture(_ fake: FakeSystemAudio, _ collector: BlockCollector) -> SystemCapture {
        let capture = SystemCapture(calls: fake.calls)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
        return capture
    }
}
