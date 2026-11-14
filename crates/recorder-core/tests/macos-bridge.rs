#![cfg(target_os = "macos")]

use std::ffi::c_void;

#[link(name = "RecorderBridge", kind = "static")]
unsafe extern "C" {
    fn recorder_create(handle: *mut *mut c_void) -> i32;
    fn recorder_prepare(handle: *mut c_void) -> i32;
    fn recorder_stop(handle: *mut c_void) -> i32;
    fn recorder_destroy(handle: *mut c_void) -> i32;
}

#[test]
fn swift_bridge_links_and_reports_an_unavailable_source() {
    let mut handle = std::ptr::null_mut();
    // SAFETY: The output pointer is valid; the returned handle remains owned here
    // until the single matching destroy call, and no callback can run in this stub.
    unsafe {
        assert_eq!(recorder_create(&raw mut handle), 0);
        assert!(!handle.is_null());
        assert_eq!(recorder_prepare(handle), 2);
        assert_eq!(recorder_stop(handle), 0);
        assert_eq!(recorder_destroy(handle), 0);
    }
}
