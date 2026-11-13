//! Bounded block queue: sizing, delivery, overflow accounting, and buffer reuse.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
use recorder_core::capture::queue::QueueSizing;

fn format(rate: u32, layout: ChannelLayout, sample_format: SampleFormat) -> AudioFormat {
    AudioFormat::new(rate, layout, sample_format, SampleLayout::Interleaved).expect("valid format")
}

#[test]
fn sizes_one_second_of_10_ms_blocks_at_48_khz() {
    let sizing = QueueSizing::for_format(
        &format(48_000, ChannelLayout::Stereo, SampleFormat::F32),
        480,
    );

    assert_eq!(
        sizing,
        QueueSizing {
            slot_count: 100,
            slot_bytes: 3_840
        }
    );
}

#[test]
fn rounds_slot_count_up_for_uneven_blocks() {
    let sizing = QueueSizing::for_format(
        &format(44_100, ChannelLayout::Mono, SampleFormat::I16),
        1_024,
    );

    assert_eq!(sizing.slot_count, 44);
}

#[test]
fn keeps_at_least_two_slots() {
    let sizing = QueueSizing::for_format(
        &format(8_000, ChannelLayout::Mono, SampleFormat::I16),
        16_000,
    );

    assert_eq!(sizing.slot_count, 2);
}
