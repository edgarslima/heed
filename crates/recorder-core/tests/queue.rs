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

use recorder_core::capture::block::{BlockFlags, BlockHeader, SourceId};
use recorder_core::capture::queue::{DropReason, PushOutcome, block_queue};

fn mono_i16() -> AudioFormat {
    format(48_000, ChannelLayout::Mono, SampleFormat::I16)
}

fn header(sequence: u64, start_frame: u64, frame_count: u32) -> BlockHeader {
    BlockHeader {
        source_id: SourceId::new(1),
        sequence,
        start_frame,
        frame_count,
        format: mono_i16(),
        first_frame_host_time_ns: Some(1_000 + sequence),
        flags: BlockFlags::default(),
    }
}

fn small_queue(
    slot_count: usize,
) -> (
    recorder_core::capture::queue::BlockProducer,
    recorder_core::capture::queue::BlockConsumer,
) {
    block_queue(QueueSizing {
        slot_count,
        slot_bytes: 960,
    })
}

#[test]
fn delivers_blocks_in_order_with_their_bytes() {
    let (mut producer, mut consumer) = small_queue(4);

    assert_eq!(
        producer.push(header(0, 0, 2), &[1, 2, 3, 4]),
        PushOutcome::Queued
    );
    assert_eq!(producer.push(header(1, 2, 1), &[5, 6]), PushOutcome::Queued);

    let first = consumer.pop().expect("first block");
    assert_eq!(first.header(), &header(0, 0, 2));
    assert_eq!(first.pcm(), &[1, 2, 3, 4]);
    let second = consumer.pop().expect("second block");
    assert_eq!(second.header().sequence, 1);
    assert_eq!(second.pcm(), &[5, 6]);
    assert!(consumer.pop().is_none());
    assert_eq!(consumer.stats().queued_blocks, 2);
}

#[test]
fn push_silence_zero_fills_and_flags_silent() {
    let (mut producer, mut consumer) = small_queue(2);

    assert_eq!(producer.push_silence(header(0, 0, 3)), PushOutcome::Queued);

    let block = consumer.pop().expect("silent block");
    assert_eq!(block.pcm(), &[0, 0, 0, 0, 0, 0]);
    assert!(block.header().flags.silent);
}

#[test]
fn rejects_mismatched_and_oversize_blocks() {
    let (mut producer, mut consumer) = small_queue(2);

    assert_eq!(
        producer.push(header(0, 0, 2), &[1, 2]),
        PushOutcome::Dropped(DropReason::SizeMismatch)
    );
    assert_eq!(
        producer.push(header(1, 0, 481), &[0; 962]),
        PushOutcome::Dropped(DropReason::Oversize)
    );
    assert_eq!(
        producer.push_silence(header(2, 0, 481)),
        PushOutcome::Dropped(DropReason::Oversize)
    );
    assert!(consumer.pop().is_none());
    assert_eq!(producer.stats().dropped_blocks, 3);
    assert_eq!(producer.stats().dropped_frames, 2 + 481 + 481);
}

#[test]
fn pushes_drop_when_consumer_does_not_recycle() {
    let (mut producer, mut consumer) = small_queue(4);
    let pcm = [0_u8; 960];

    let outcomes: Vec<PushOutcome> = (0..6)
        .map(|index| producer.push(header(index, index * 480, 480), &pcm))
        .collect();
    while consumer.pop().is_some() {}

    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == PushOutcome::Queued)
            .count(),
        4
    );
    assert_eq!(outcomes[4], PushOutcome::Dropped(DropReason::QueueFull));
    let stats = producer.stats();
    assert_eq!(
        (
            stats.queued_blocks,
            stats.dropped_blocks,
            stats.dropped_frames
        ),
        (4, 2, 960)
    );
    assert_eq!(
        producer.push(header(6, 2_880, 480), &pcm),
        PushOutcome::Dropped(DropReason::QueueFull)
    );
}

#[test]
fn flags_the_block_after_a_drop_as_discontinuous() {
    let (mut producer, mut consumer) = small_queue(2);

    assert_eq!(producer.push(header(0, 0, 1), &[1, 1]), PushOutcome::Queued);
    assert_eq!(producer.push(header(1, 1, 1), &[2, 2]), PushOutcome::Queued);
    assert_eq!(
        producer.push(header(2, 2, 1), &[3, 3]),
        PushOutcome::Dropped(DropReason::QueueFull)
    );
    let first = consumer.pop().expect("first block");
    consumer.recycle(first);
    assert_eq!(producer.push(header(3, 3, 1), &[4, 4]), PushOutcome::Queued);

    let second = consumer.pop().expect("second block");
    assert!(!second.header().flags.discontinuity);
    let after_drop = consumer.pop().expect("block after the drop");
    assert_eq!(after_drop.header().sequence, 3);
    assert!(after_drop.header().flags.discontinuity);
}

#[test]
fn recycled_buffers_are_reused() {
    let (mut producer, mut consumer) = small_queue(2);

    for sequence in 0..10 {
        assert_eq!(
            producer.push(header(sequence, sequence, 1), &[7, 7]),
            PushOutcome::Queued
        );
        let block = consumer.pop().expect("block");
        assert_eq!(block.pcm(), &[7, 7]);
        consumer.recycle(block);
    }

    assert_eq!(producer.stats().dropped_blocks, 0);
    assert_eq!(consumer.stats().queued_blocks, 10);
}
