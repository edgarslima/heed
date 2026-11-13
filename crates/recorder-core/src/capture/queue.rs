//! Bounded single-producer, single-consumer block queue per source
//! (architecture section 2, "Capture contract and concurrency").

use crate::audio::format::AudioFormat;

/// Number and size of the preallocated buffers of one queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueSizing {
    /// Buffers in the queue.
    pub slot_count: usize,
    /// Capacity of each buffer in bytes.
    pub slot_bytes: usize,
}

impl QueueSizing {
    /// Sizes a queue to hold about one second of audio in blocks of at most
    /// `max_block_frames` frames, with at least two buffers.
    #[must_use]
    pub fn for_format(format: &AudioFormat, max_block_frames: u32) -> Self {
        let block_frames = max_block_frames.max(1);
        let slots_per_second = format.sample_rate_hz().div_ceil(block_frames);
        Self {
            slot_count: usize::try_from(slots_per_second)
                .unwrap_or(usize::MAX)
                .max(2),
            slot_bytes: block_frames as usize * format.bytes_per_frame(),
        }
    }
}
