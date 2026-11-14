private let ok: Int32 = 0
private let invalidArgument: Int32 = 1
private let sourceUnavailable: Int32 = 2
private let invalidState: Int32 = 3

private final class RecorderHandle {}

@_cdecl("recorder_list")
public func recorderList(_ count: UnsafeMutablePointer<UInt32>?) -> Int32 {
    guard let count else { return invalidArgument }
    count.pointee = 0
    return sourceUnavailable
}

@_cdecl("recorder_create")
public func recorderCreate(_ output: UnsafeMutablePointer<UnsafeMutableRawPointer?>?) -> Int32 {
    guard let output else { return invalidArgument }
    output.pointee = Unmanaged.passRetained(RecorderHandle()).toOpaque()
    return ok
}

@_cdecl("recorder_prepare")
public func recorderPrepare(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard handle != nil else { return invalidArgument }
    return sourceUnavailable
}

@_cdecl("recorder_start")
public func recorderStart(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard handle != nil else { return invalidArgument }
    return invalidState
}

@_cdecl("recorder_stop")
public func recorderStop(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard handle != nil else { return invalidArgument }
    return ok
}

@_cdecl("recorder_destroy")
public func recorderDestroy(_ handle: UnsafeMutableRawPointer?) -> Int32 {
    guard let handle else { return invalidArgument }
    Unmanaged<RecorderHandle>.fromOpaque(handle).release()
    return ok
}
