//! Capture contract types: block headers, flags, events, and error codes.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
use recorder_core::capture::block::{BlockFlags, BlockHeader, SourceId};
use recorder_core::capture::event::{CaptureError, CaptureEvent};

fn stereo_i16() -> AudioFormat {
    AudioFormat::new(
        48_000,
        ChannelLayout::Stereo,
        SampleFormat::I16,
        SampleLayout::Interleaved,
    )
    .expect("valid format")
}

#[test]
fn header_byte_len_is_frames_times_bytes_per_frame() {
    let header = BlockHeader {
        source_id: SourceId::new(7),
        sequence: 0,
        start_frame: 0,
        frame_count: 480,
        format: stereo_i16(),
        first_frame_host_time_ns: None,
        flags: BlockFlags::default(),
    };

    assert_eq!(header.byte_len(), 1_920);
    assert_eq!(header.source_id.get(), 7);
}

#[test]
fn capture_error_codes_match_the_architecture() {
    assert_eq!(CaptureError::PermissionDenied.code(), "permissionDenied");
    assert_eq!(CaptureError::SourceUnavailable.code(), "sourceUnavailable");
    assert_eq!(CaptureError::FormatUnsupported.code(), "formatUnsupported");
    assert_eq!(
        CaptureError::CaptureInterrupted.code(),
        "captureInterrupted"
    );
}

#[test]
fn every_event_reports_its_source() {
    let source_id = SourceId::new(3);
    let events = [
        CaptureEvent::Disconnected {
            source_id,
            at_frame: Some(10),
        },
        CaptureEvent::FormatChanged {
            source_id,
            format: stereo_i16(),
        },
        CaptureEvent::Discontinuity {
            source_id,
            at_frame: 20,
            lost_frames: None,
        },
        CaptureEvent::Failed {
            source_id,
            error: CaptureError::CaptureInterrupted,
        },
        CaptureEvent::Ended {
            source_id,
            final_frame: 30,
        },
    ];

    for event in events {
        assert_eq!(event.source_id(), source_id);
    }
}

#[test]
fn default_flags_are_all_clear() {
    assert_eq!(
        BlockFlags::default(),
        BlockFlags {
            silent: false,
            discontinuity: false,
            timestamp_invalid: false
        }
    );
}
