//! macOS bridge ABI: layouts and argument checks that need no device or microphone
//! access, so they run on CI.

#![cfg(target_os = "macos")]
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use std::ffi::c_void;
use std::mem::{offset_of, size_of};

const RECORDER_OK: i32 = 0;
const RECORDER_INVALID_ARGUMENT: i32 = 1;
const RECORDER_INVALID_STATE: i32 = 3;
const RECORDER_SOURCE_MICROPHONE: u32 = 1;
const RECORDER_ACCESS_GRANTED: u32 = 2;

/// Mirrors `recorder_format` in `native/macos/include/recorder-bridge.h`.
#[repr(C)]
#[derive(Default)]
struct RecorderFormat {
    sample_rate_hz: u32,
    channel_count: u32,
    sample_format: u32,
    max_block_frames: u32,
}

/// Mirrors `recorder_block`.
#[repr(C)]
struct RecorderBlock {
    samples: *const f32,
    host_time_ns: u64,
    sample_time: f64,
    frame_count: u32,
    flags: u32,
}

type BlockCallback = unsafe extern "C" fn(*mut c_void, *const RecorderBlock);

#[link(name = "RecorderBridge", kind = "static")]
unsafe extern "C" {
    fn recorder_microphone_access(request: u32, access: *mut u32) -> i32;
    fn recorder_list(count: *mut u32) -> i32;
    fn recorder_create(handle: *mut *mut c_void) -> i32;
    fn recorder_prepare(handle: *mut c_void, source_kind: u32, format: *mut RecorderFormat) -> i32;
    fn recorder_start(
        handle: *mut c_void,
        callback: Option<BlockCallback>,
        context: *mut c_void,
    ) -> i32;
    fn recorder_stop(handle: *mut c_void) -> i32;
    fn recorder_destroy(handle: *mut c_void) -> i32;
    fn recorder_last_os_status(handle: *mut c_void, status: *mut i32) -> i32;
}

unsafe extern "C" fn ignore_block(_context: *mut c_void, _block: *const RecorderBlock) {}

#[test]
fn rust_layouts_match_the_c_header() {
    assert_eq!(size_of::<RecorderFormat>(), 16);
    assert_eq!(size_of::<RecorderBlock>(), 32);
    assert_eq!(offset_of!(RecorderBlock, host_time_ns), 8);
    assert_eq!(offset_of!(RecorderBlock, sample_time), 16);
    assert_eq!(offset_of!(RecorderBlock, frame_count), 24);
    assert_eq!(offset_of!(RecorderBlock, flags), 28);
}

#[test]
fn swift_bridge_links_and_validates_arguments() {
    let mut handle = std::ptr::null_mut();
    let mut format = RecorderFormat::default();
    let mut status = -1;
    let mut count = u32::MAX;
    let mut access = u32::MAX;
    // SAFETY: Every pointer refers to a live local of the declared C type. The handle
    // stays owned here until the single matching destroy call; it is never started, so
    // no callback can run and `ignore_block` is never called.
    unsafe {
        assert_eq!(recorder_microphone_access(0, &raw mut access), RECORDER_OK);
        assert!(access <= RECORDER_ACCESS_GRANTED);
        assert_eq!(recorder_list(&raw mut count), RECORDER_OK);
        assert!(count <= 1);
        assert_eq!(recorder_create(&raw mut handle), RECORDER_OK);
        assert!(!handle.is_null());
        assert_eq!(
            recorder_prepare(handle, 9, &raw mut format),
            RECORDER_INVALID_ARGUMENT
        );
        assert_eq!(
            recorder_prepare(handle, RECORDER_SOURCE_MICROPHONE, std::ptr::null_mut()),
            RECORDER_INVALID_ARGUMENT
        );
        assert_eq!(
            recorder_start(handle, None, std::ptr::null_mut()),
            RECORDER_INVALID_ARGUMENT
        );
        assert_eq!(
            recorder_start(handle, Some(ignore_block), std::ptr::null_mut()),
            RECORDER_INVALID_STATE
        );
        assert_eq!(
            recorder_last_os_status(handle, &raw mut status),
            RECORDER_OK
        );
        assert_eq!(status, 0);
        assert_eq!(recorder_stop(handle), RECORDER_OK);
        assert_eq!(recorder_stop(handle), RECORDER_OK);
        assert_eq!(recorder_destroy(handle), RECORDER_OK);
    }
}
