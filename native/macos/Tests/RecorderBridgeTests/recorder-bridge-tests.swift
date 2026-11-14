import XCTest
@testable import RecorderBridge

final class RecorderBridgeTests: XCTestCase {
    func testNullArgumentsAreRejected() {
        XCTAssertEqual(recorderList(nil), 1)
        XCTAssertEqual(recorderCreate(nil), 1)
        XCTAssertEqual(recorderPrepare(nil), 1)
        XCTAssertEqual(recorderStart(nil), 1)
        XCTAssertEqual(recorderStop(nil), 1)
        XCTAssertEqual(recorderDestroy(nil), 1)
    }

    func testUnimplementedSourceDoesNotPretendToCapture() throws {
        var count: UInt32 = 99
        XCTAssertEqual(recorderList(&count), 2)
        XCTAssertEqual(count, 0)
        var handle: UnsafeMutableRawPointer?
        XCTAssertEqual(recorderCreate(&handle), 0)
        let owned = try XCTUnwrap(handle)
        XCTAssertEqual(recorderPrepare(owned), 2)
        XCTAssertEqual(recorderStart(owned), 3)
        XCTAssertEqual(recorderStop(owned), 0)
        XCTAssertEqual(recorderStop(owned), 0)
        XCTAssertEqual(recorderDestroy(owned), 0)
    }
}
