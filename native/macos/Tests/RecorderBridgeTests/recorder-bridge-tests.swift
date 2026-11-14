import AVFoundation
import AudioToolbox
import XCTest

@testable import RecorderBridge

final class RecorderBridgeTests: XCTestCase {
    func testLayoutsMatchTheCHeader() {
        XCTAssertEqual(MemoryLayout<RecorderFormat>.size, 16)
        XCTAssertEqual(MemoryLayout<RecorderBlock>.size, 32)
        XCTAssertEqual(MemoryLayout<RecorderBlock>.offset(of: \.hostTimeNs), 8)
        XCTAssertEqual(MemoryLayout<RecorderBlock>.offset(of: \.sampleTime), 16)
        XCTAssertEqual(MemoryLayout<RecorderBlock>.offset(of: \.frameCount), 24)
        XCTAssertEqual(MemoryLayout<RecorderBlock>.offset(of: \.flags), 28)
    }

    func testHostClockConvertsTicksExactlyAndSaturates() {
        let appleSilicon = HostClock(numerator: 125, denominator: 3)
        XCTAssertEqual(appleSilicon.nanoseconds(3), 125)
        XCTAssertEqual(appleSilicon.nanoseconds(24_000_000), 1_000_000_000)
        XCTAssertEqual(appleSilicon.nanoseconds(UInt64.max), UInt64.max)
        XCTAssertEqual(HostClock(numerator: 1, denominator: 0).nanoseconds(7), 7)
        XCTAssertNotEqual(HostClock.system().denominator, 0)
    }

    func testNullArgumentsAreRejected() throws {
        let fake = FakeCoreAudio()
        var status: Int32 = 0
        var format = RecorderFormat()
        XCTAssertEqual(recorderMicrophoneAccess(0, nil), 1)
        XCTAssertEqual(recorderList(nil), 1)
        XCTAssertEqual(recorderCreate(nil), 1)
        XCTAssertEqual(recorderStart(nil, BlockCollector.callback, nil), 1)
        XCTAssertEqual(recorderStop(nil), 1)
        XCTAssertEqual(recorderDestroy(nil), 1)
        XCTAssertEqual(recorderLastOSStatus(nil, &status), 1)
        XCTAssertEqual(queryMicrophoneAccess(0, nil, calls: fake.calls), 1)
        XCTAssertEqual(listSources(nil, calls: fake.calls), 1)
        let handle = try makeHandle(fake)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(nil, sourceKindMicrophone, pointer), 1)
            XCTAssertEqual(recorderPrepare(handle, 9, pointer), 1)
        }
        XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, nil), 1)
        XCTAssertEqual(recorderStart(handle, nil, nil), 1)
        XCTAssertEqual(recorderLastOSStatus(handle, nil), 1)
        XCTAssertEqual(fake.log, [])
        XCTAssertEqual(recorderDestroy(handle), 0)
    }

    func testMicrophoneAccessIsReportedAndRequested() {
        let fake = FakeCoreAudio()
        var access: UInt32 = 99
        for (status, expected) in [
            (AVAuthorizationStatus.authorized, MicrophoneAccess.granted),
            (.denied, .denied),
            (.restricted, .denied),
            (.notDetermined, .notDetermined),
        ] {
            fake.accessStatus = status
            XCTAssertEqual(queryMicrophoneAccess(0, &access, calls: fake.calls), 0)
            XCTAssertEqual(access, expected.rawValue)
        }
        XCTAssertEqual(fake.requestCount, 0)
        for (answer, expected) in [(true, MicrophoneAccess.granted), (false, .denied)] {
            fake.requestAnswer = answer
            XCTAssertEqual(queryMicrophoneAccess(1, &access, calls: fake.calls), 0)
            XCTAssertEqual(access, expected.rawValue)
        }
        XCTAssertEqual(fake.requestCount, 2)
    }

    func testListCountsTheDefaultInputDevice() {
        let fake = FakeCoreAudio()
        var count: UInt32 = 99
        XCTAssertEqual(listSources(&count, calls: fake.calls), 0)
        XCTAssertEqual(count, 1)
        fake.device = AudioObjectID(kAudioObjectUnknown)
        XCTAssertEqual(listSources(&count, calls: fake.calls), 0)
        XCTAssertEqual(count, 0)
        fake.failures["device"] = kAudioHardwareNotRunningError
        count = 99
        XCTAssertEqual(listSources(&count, calls: fake.calls), 5)
        XCTAssertEqual(count, 0)
    }

    func testPrepareNegotiatesFloatAtTheDeviceRate() {
        let fake = FakeCoreAudio()
        fake.hardwareFormat = FakeCoreAudio.format(sampleRate: 44_100, channels: 2)
        let capture = MicrophoneCapture(calls: fake.calls)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        XCTAssertEqual(
            format, RecorderFormat(sampleRateHz: 44_100, channelCount: 2, sampleFormat: 1, maxBlockFrames: 512))
        let client = fake.clientFormat
        XCTAssertEqual(client?.mSampleRate, 44_100)
        XCTAssertEqual(client?.mChannelsPerFrame, 2)
        XCTAssertEqual(client?.mFormatFlags, kAudioFormatFlagsNativeFloatPacked)
        XCTAssertEqual(client?.mBytesPerFrame, 8)
        XCTAssertEqual(
            fake.log,
            [
                "device", "newUnit", "set enableIO input", "set enableIO output", "set currentDevice",
                "get streamFormat input", "set streamFormat output", "get maxFrames", "set inputCallback",
                "initialize",
            ])
        XCTAssertEqual(capture.prepare(into: &format), .invalidState)
    }

    func testPrepareRequiresGrantedAccessAndADevice() {
        let fake = FakeCoreAudio()
        var format = RecorderFormat()
        for status in [AVAuthorizationStatus.notDetermined, .denied, .restricted] {
            fake.accessStatus = status
            XCTAssertEqual(MicrophoneCapture(calls: fake.calls).prepare(into: &format), .permissionDenied)
        }
        XCTAssertEqual(fake.log, [])
        fake.accessStatus = .authorized
        fake.device = AudioObjectID(kAudioObjectUnknown)
        XCTAssertEqual(MicrophoneCapture(calls: fake.calls).prepare(into: &format), .sourceUnavailable)
        XCTAssertEqual(fake.log, ["device"])
        XCTAssertEqual(format, RecorderFormat())
    }

    func testPrepareRejectsUnsupportedDeviceFormats() {
        let cases: [(Double, UInt32, UInt32)] = [
            (44_100.5, 1, 512), (4_000, 1, 512), (400_000, 1, 512), (48_000, 0, 512), (48_000, 9, 512),
            (48_000, 1, 0), (48_000, 1, 16_385),
        ]
        for (sampleRate, channels, maxFrames) in cases {
            let fake = FakeCoreAudio()
            fake.hardwareFormat = FakeCoreAudio.format(sampleRate: sampleRate, channels: channels)
            fake.maxFrames = maxFrames
            let capture = MicrophoneCapture(calls: fake.calls)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .sourceUnavailable, "\(sampleRate) \(channels) \(maxFrames)")
            XCTAssertEqual(fake.log.last, "dispose")
            XCTAssertFalse(fake.log.contains("initialize"))
        }
    }

    func testPrepareFailuresReleaseTheUnitAndKeepTheStatus() {
        let steps = [
            "newUnit", "set enableIO input", "set enableIO output", "set currentDevice", "get streamFormat input",
            "set streamFormat output", "get maxFrames", "set inputCallback", "initialize",
        ]
        for step in steps {
            let fake = FakeCoreAudio()
            fake.failures[step] = -50
            let capture = MicrophoneCapture(calls: fake.calls)
            var format = RecorderFormat()
            XCTAssertEqual(capture.prepare(into: &format), .nativeError, step)
            XCTAssertEqual(capture.lastOSStatus, -50, step)
            XCTAssertEqual(fake.log.last, step == "newUnit" ? "newUnit" : "dispose", step)
            fake.failures.removeAll()
            XCTAssertEqual(capture.prepare(into: &format), .ok, step)
        }
        let fake = FakeCoreAudio()
        fake.failures["device"] = -50
        var format = RecorderFormat()
        XCTAssertEqual(MicrophoneCapture(calls: fake.calls).prepare(into: &format), .nativeError)
    }

    func testPrepareRejectsAMissingUnitOrAShortProperty() {
        var format = RecorderFormat()
        let missing = FakeCoreAudio()
        missing.returnsUnit = false
        XCTAssertEqual(MicrophoneCapture(calls: missing.calls).prepare(into: &format), .nativeError)
        let short = FakeCoreAudio()
        short.reportedGetSize = 2
        XCTAssertEqual(MicrophoneCapture(calls: short.calls).prepare(into: &format), .nativeError)
        XCTAssertEqual(short.log.last, "dispose")
    }

    func testStartRequiresAPreparedUnitAndRecoversFromAFailedStart() {
        let fake = FakeCoreAudio()
        let collector = BlockCollector()
        let capture = MicrophoneCapture(calls: fake.calls)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .invalidState)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        fake.failures["start"] = -10_851
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .nativeError)
        XCTAssertEqual(capture.lastOSStatus, -10_851)
        XCTAssertEqual(fake.play(frames: 64), noErr)
        XCTAssertTrue(collector.blocks.isEmpty)
        fake.failures.removeAll()
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .invalidState)
    }

    func testDeliveryForwardsRenderedFramesWithTimestamps() {
        let fake = FakeCoreAudio()
        fake.hardwareFormat = FakeCoreAudio.format(sampleRate: 48_000, channels: 2)
        let collector = BlockCollector()
        collector.channelCount = 2
        let capture = startedCapture(fake, collector)
        XCTAssertEqual(fake.play(frames: 256, hostTime: 3_000, sampleTime: 512), noErr)
        XCTAssertEqual(fake.play(frames: 128, flags: []), noErr)
        XCTAssertEqual(fake.renderBus, 1)
        XCTAssertEqual(collector.blocks.count, 2)
        let first = collector.blocks[0]
        XCTAssertEqual(first.frameCount, 256)
        XCTAssertEqual(first.flags, blockHostTimeValid | blockSampleTimeValid)
        XCTAssertEqual(first.hostTimeNs, 125_000)
        XCTAssertEqual(first.sampleTime, 512)
        XCTAssertEqual(first.samples, [Float](repeating: 0.25, count: 512))
        XCTAssertEqual(collector.blocks[1].flags, 0)
        XCTAssertEqual(collector.blocks[1].hostTimeNs, 0)
        XCTAssertEqual(capture.stop(), .ok)
    }

    func testOversizedOrFailedRendersMarkTheNextBlock() {
        let fake = FakeCoreAudio()
        let collector = BlockCollector()
        let capture = startedCapture(fake, collector)
        XCTAssertEqual(fake.play(frames: 513), noErr)
        XCTAssertEqual(fake.play(frames: 512), noErr)
        XCTAssertEqual(fake.play(frames: 512), noErr)
        fake.failures["render"] = kAudioUnitErr_CannotDoInCurrentContext
        XCTAssertEqual(fake.play(frames: 64), kAudioUnitErr_CannotDoInCurrentContext)
        fake.failures.removeAll()
        XCTAssertEqual(fake.play(frames: 64), noErr)
        XCTAssertEqual(collector.blocks.map(\.frameCount), [512, 512, 64])
        XCTAssertEqual(collector.blocks.map { $0.flags & blockDiscontinuity }, [blockDiscontinuity, 0, blockDiscontinuity])
        XCTAssertEqual(capture.stop(), .ok)
    }

    func testStopTearsDownInReverseOrderAndIsIdempotent() {
        let fake = FakeCoreAudio()
        let collector = BlockCollector()
        let capture = startedCapture(fake, collector)
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log, ["stop", "uninitialize", "dispose"])
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log, ["stop", "uninitialize", "dispose"])
        XCTAssertEqual(fake.play(frames: 64), noErr)
        XCTAssertTrue(collector.blocks.isEmpty)

        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .ok)
        XCTAssertEqual(fake.log, ["uninitialize", "dispose"])
    }

    func testStopRunsEveryTeardownStepWhenOneFails() {
        let fake = FakeCoreAudio()
        let capture = startedCapture(fake, BlockCollector())
        fake.failures["stop"] = -1
        fake.failures["uninitialize"] = -2
        fake.clearLog()
        XCTAssertEqual(capture.stop(), .nativeError)
        XCTAssertEqual(fake.log, ["stop", "uninitialize", "dispose"])
        XCTAssertEqual(capture.lastOSStatus, -2)
        fake.failures.removeAll()
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
    }

    func testTheABIDrivesACaptureAndDestroyStopsIt() throws {
        let fake = FakeCoreAudio()
        let collector = BlockCollector()
        let handle = try makeHandle(fake)
        var format = RecorderFormat()
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, pointer), 0)
        }
        XCTAssertEqual(format.sampleRateHz, 48_000)
        XCTAssertEqual(recorderStart(handle, BlockCollector.callback, collector.context), 0)
        XCTAssertEqual(fake.play(frames: 32), noErr)
        XCTAssertEqual(collector.blocks.count, 1)
        var status: Int32 = 99
        XCTAssertEqual(recorderLastOSStatus(handle, &status), 0)
        XCTAssertEqual(status, 0)
        XCTAssertEqual(recorderStop(handle), 0)
        XCTAssertEqual(recorderStart(handle, BlockCollector.callback, collector.context), 3)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, pointer), 0)
        }
        XCTAssertEqual(recorderStart(handle, BlockCollector.callback, collector.context), 0)
        fake.clearLog()
        XCTAssertEqual(recorderDestroy(handle), 0)
        XCTAssertEqual(fake.log, ["stop", "uninitialize", "dispose"])
    }

    func testFailedPrepareLeavesTheFormatUntouched() throws {
        let fake = FakeCoreAudio()
        fake.accessStatus = .denied
        let handle = try makeHandle(fake)
        var format = RecorderFormat(sampleRateHz: 7, channelCount: 7, sampleFormat: 7, maxBlockFrames: 7)
        withUnsafeMutablePointer(to: &format) { pointer in
            XCTAssertEqual(recorderPrepare(handle, sourceKindMicrophone, pointer), 4)
        }
        XCTAssertEqual(format.sampleRateHz, 7)
        XCTAssertEqual(recorderDestroy(handle), 0)
    }

    private func makeHandle(_ fake: FakeCoreAudio) throws -> UnsafeMutableRawPointer {
        var handle: UnsafeMutableRawPointer?
        XCTAssertEqual(createHandle(&handle, calls: fake.calls), 0)
        return try XCTUnwrap(handle)
    }

    private func startedCapture(_ fake: FakeCoreAudio, _ collector: BlockCollector) -> MicrophoneCapture {
        let capture = MicrophoneCapture(calls: fake.calls)
        var format = RecorderFormat()
        XCTAssertEqual(capture.prepare(into: &format), .ok)
        XCTAssertEqual(capture.start(callback: BlockCollector.callback, context: collector.context), .ok)
        return capture
    }
}
