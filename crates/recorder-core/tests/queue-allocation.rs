//! Proves that the queue's callback-side operations do not allocate.
//!
//! The counting allocator tracks allocations per thread, so allocations made by the
//! test harness on other threads do not affect the result.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
use recorder_core::capture::block::{BlockFlags, BlockHeader, SourceId};
use recorder_core::capture::queue::{DropReason, PushOutcome, QueueSizing, block_queue};

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static DEALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

// SAFETY: every method forwards to the system allocator with the caller's arguments
// unchanged, so the GlobalAlloc contract is upheld by `System`. The thread-local
// counter is const-initialized and never allocates.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        // SAFETY: same layout the caller passed, which satisfies `alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.with(|count| count.set(count.get() + 1));
        // SAFETY: `ptr` was allocated by `System` with this `layout` through this allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        // SAFETY: `ptr` and `layout` come from a previous allocation by `System`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Allocations and deallocations made on this thread so far.
fn heap_operations() -> (u64, u64) {
    (ALLOCATIONS.with(Cell::get), DEALLOCATIONS.with(Cell::get))
}

#[test]
fn push_pop_and_recycle_do_not_allocate() {
    let format = AudioFormat::new(
        48_000,
        ChannelLayout::Stereo,
        SampleFormat::F32,
        SampleLayout::Interleaved,
    )
    .expect("valid format");
    let (mut producer, mut consumer) = block_queue(QueueSizing::for_format(&format, 480, 480));
    let pcm = vec![0_u8; 480 * format.bytes_per_frame()];
    let header = |sequence: u64| BlockHeader {
        source_id: SourceId::new(1),
        sequence,
        start_frame: sequence * 480,
        frame_count: 480,
        format,
        first_frame_host_time_ns: None,
        flags: BlockFlags::default(),
    };

    let mut run = |from: u64, to: u64| {
        let mut queued = 0_u64;
        // Two blocks in flight, recycled in reverse order of arrival.
        for sequence in (from..to).step_by(2) {
            queued += u64::from(producer.push(header(sequence), &pcm) == PushOutcome::Queued);
            queued += u64::from(producer.push_silence(header(sequence + 1)) == PushOutcome::Queued);
            let first = consumer.pop();
            let second = consumer.pop();
            if let Some(block) = second {
                consumer.recycle(block);
            }
            if let Some(block) = first {
                consumer.recycle(block);
            }
        }
        queued
    };

    run(0, 20);
    let before = heap_operations();
    let queued = run(20, 20_020);
    let after = heap_operations();

    assert_eq!(queued, 20_000);
    assert_eq!(
        after, before,
        "allocations and deallocations during push, pop, and recycle"
    );
}

#[test]
fn recycling_a_foreign_block_keeps_the_producer_allocation_free() {
    let format = AudioFormat::new(
        48_000,
        ChannelLayout::Mono,
        SampleFormat::I16,
        SampleLayout::Interleaved,
    )
    .expect("valid format");
    let header = |frame_count: u32| BlockHeader {
        source_id: SourceId::new(1),
        sequence: 0,
        start_frame: 0,
        frame_count,
        format,
        first_frame_host_time_ns: None,
        flags: BlockFlags::default(),
    };
    let (mut small_producer, mut small_consumer) = block_queue(QueueSizing {
        slot_count: 2,
        slot_bytes: 2,
    });
    let (mut large_producer, mut large_consumer) = block_queue(QueueSizing {
        slot_count: 2,
        slot_bytes: 3_840,
    });
    let pcm = vec![0_u8; 3_840];
    assert_eq!(small_producer.push(header(1), &[1, 1]), PushOutcome::Queued);
    let foreign = small_consumer.pop().expect("small block");
    // Keep one large buffer in flight so the large queue's free ring has room.
    assert_eq!(
        large_producer.push(header(1_920), &pcm),
        PushOutcome::Queued
    );
    let _in_flight = large_consumer.pop().expect("large block");
    let _ = large_consumer.recycle(foreign);

    let before = heap_operations();
    let first = large_producer.push(header(1_920), &pcm);
    let second = large_producer.push(header(1_920), &pcm);
    let after = heap_operations();

    assert_eq!(first, PushOutcome::Queued);
    assert_eq!(second, PushOutcome::Dropped(DropReason::QueueFull));
    assert_eq!(
        after, before,
        "heap operations in the producer after a foreign recycle"
    );
}
