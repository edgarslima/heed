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
use recorder_core::capture::queue::{PushOutcome, QueueSizing, block_queue};

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
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

fn allocations() -> u64 {
    ALLOCATIONS.with(Cell::get)
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
    let (mut producer, mut consumer) = block_queue(QueueSizing::for_format(&format, 480));
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
    let before = allocations();
    let queued = run(20, 20_020);
    let after = allocations();

    assert_eq!(queued, 20_000);
    assert_eq!(
        after - before,
        0,
        "allocations during push, pop, and recycle"
    );
}
