//! Bounded single-producer, single-consumer block queue per source
//! (architecture section 2, "Capture contract and concurrency").

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Source of unique queue identities, so a block can only be recycled into the queue
/// it came from.
static NEXT_QUEUE_ID: AtomicU64 = AtomicU64::new(1);

use rtrb::RingBuffer;

use crate::audio::format::AudioFormat;
use crate::capture::block::BlockHeader;

/// Number and size of the preallocated buffers of one queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueSizing {
    /// Buffers in the queue.
    pub slot_count: usize,
    /// Capacity of each buffer in bytes.
    pub slot_bytes: usize,
}

impl QueueSizing {
    /// Sizes a queue to hold about one second of audio even when every block has only
    /// `min_block_frames` frames, with buffers large enough for `max_block_frames`
    /// frames and at least two buffers. Each buffer holds one block, so the slot count
    /// follows the smallest block a device may deliver.
    #[must_use]
    pub fn for_format(format: &AudioFormat, min_block_frames: u32, max_block_frames: u32) -> Self {
        let smallest = min_block_frames.max(1);
        let largest = max_block_frames.max(smallest);
        let slots_per_second = format.sample_rate_hz().div_ceil(smallest);
        Self {
            slot_count: usize::try_from(slots_per_second)
                .unwrap_or(usize::MAX)
                .max(2),
            slot_bytes: largest as usize * format.bytes_per_frame(),
        }
    }
}

/// Result of handing a block to the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushOutcome {
    /// The block was copied into a free buffer and queued.
    Queued,
    /// The block was discarded and counted.
    Dropped(DropReason),
}

/// Why a block was discarded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    /// No free buffer: the consumer is behind or has not recycled buffers.
    QueueFull,
    /// The block is larger than a buffer.
    Oversize,
    /// The byte length does not match the header.
    SizeMismatch,
}

/// Counters shared by both ends of a queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueueStats {
    /// Blocks queued since creation.
    pub queued_blocks: u64,
    /// Blocks discarded since creation.
    pub dropped_blocks: u64,
    /// Frames in discarded blocks.
    pub dropped_frames: u64,
}

#[derive(Default)]
struct SharedStats {
    queued_blocks: AtomicU64,
    dropped_blocks: AtomicU64,
    dropped_frames: AtomicU64,
}

impl SharedStats {
    fn snapshot(&self) -> QueueStats {
        QueueStats {
            queued_blocks: self.queued_blocks.load(Ordering::Relaxed),
            dropped_blocks: self.dropped_blocks.load(Ordering::Relaxed),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
        }
    }
}

/// A queued block with its PCM bytes, owned by the consumer until recycled.
#[derive(Debug)]
pub struct ReceivedBlock {
    header: BlockHeader,
    pcm: Vec<u8>,
    queue_id: u64,
}

impl ReceivedBlock {
    /// Block metadata.
    #[must_use]
    pub const fn header(&self) -> &BlockHeader {
        &self.header
    }

    /// PCM bytes in the header's format.
    #[must_use]
    pub fn pcm(&self) -> &[u8] {
        &self.pcm
    }
}

/// Capture-side end of a block queue. Its methods never allocate, free, lock, or block,
/// so they are safe to call from an audio callback. `BlockConsumer::recycle` rejects
/// buffers from other queues to keep that guarantee.
pub struct BlockProducer {
    queue_id: u64,
    filled: rtrb::Producer<ReceivedBlock>,
    free: rtrb::Consumer<Vec<u8>>,
    slot_bytes: usize,
    pending_discontinuity: bool,
    stats: Arc<SharedStats>,
}

impl BlockProducer {
    /// Copies `pcm` into a free buffer and queues it with `header`.
    pub fn push(&mut self, header: BlockHeader, pcm: &[u8]) -> PushOutcome {
        if pcm.len() != header.byte_len() {
            return self.drop_block(&header, DropReason::SizeMismatch);
        }
        self.enqueue(header, |buffer| buffer.extend_from_slice(pcm))
    }

    /// Queues a block of zeros marked silent, without reading device memory.
    pub fn push_silence(&mut self, mut header: BlockHeader) -> PushOutcome {
        header.flags.silent = true;
        let byte_len = header.byte_len();
        self.enqueue(header, |buffer| buffer.resize(byte_len, 0))
    }

    /// Current counters.
    #[must_use]
    pub fn stats(&self) -> QueueStats {
        self.stats.snapshot()
    }

    fn enqueue(&mut self, mut header: BlockHeader, fill: impl FnOnce(&mut Vec<u8>)) -> PushOutcome {
        if header.byte_len() > self.slot_bytes {
            return self.drop_block(&header, DropReason::Oversize);
        }
        let Ok(mut buffer) = self.free.pop() else {
            return self.drop_block(&header, DropReason::QueueFull);
        };
        buffer.clear();
        fill(&mut buffer);
        if self.pending_discontinuity {
            header.flags.discontinuity = true;
            self.pending_discontinuity = false;
        }
        // The filled ring holds as many entries as there are buffers, so a buffer taken
        // from the free ring always fits.
        match self.filled.push(ReceivedBlock {
            header,
            pcm: buffer,
            queue_id: self.queue_id,
        }) {
            Ok(()) => {
                self.stats.queued_blocks.fetch_add(1, Ordering::Relaxed);
                PushOutcome::Queued
            }
            Err(rtrb::PushError::Full(block)) => {
                self.drop_block(&block.header, DropReason::QueueFull)
            }
        }
    }

    fn drop_block(&mut self, header: &BlockHeader, reason: DropReason) -> PushOutcome {
        self.stats.dropped_blocks.fetch_add(1, Ordering::Relaxed);
        self.stats
            .dropped_frames
            .fetch_add(u64::from(header.frame_count), Ordering::Relaxed);
        self.pending_discontinuity = true;
        PushOutcome::Dropped(reason)
    }
}

/// Engine-side end of a block queue.
pub struct BlockConsumer {
    queue_id: u64,
    filled: rtrb::Consumer<ReceivedBlock>,
    free: rtrb::Producer<Vec<u8>>,
    stats: Arc<SharedStats>,
}

impl BlockConsumer {
    /// Takes the oldest queued block, if any.
    pub fn pop(&mut self) -> Option<ReceivedBlock> {
        self.filled.pop().ok()
    }

    /// Returns a block's buffer to the producer for reuse and reports whether it was
    /// accepted. A block from another queue is rejected and its buffer is freed here,
    /// on the consumer thread, so the producer never receives a buffer of the wrong
    /// size. A block that is dropped instead of recycled permanently removes one buffer
    /// from its queue.
    pub fn recycle(&mut self, block: ReceivedBlock) -> bool {
        if block.queue_id != self.queue_id {
            return false;
        }
        let mut buffer = block.pcm;
        buffer.clear();
        // The free ring holds every buffer of this queue, so it cannot be full here.
        self.free.push(buffer).is_ok()
    }

    /// Current counters.
    #[must_use]
    pub fn stats(&self) -> QueueStats {
        self.stats.snapshot()
    }
}

/// Creates a queue with `sizing.slot_count` buffers of `sizing.slot_bytes` bytes,
/// all allocated up front.
#[must_use]
pub fn block_queue(sizing: QueueSizing) -> (BlockProducer, BlockConsumer) {
    let slot_count = sizing.slot_count.max(1);
    let (filled_producer, filled_consumer) = RingBuffer::new(slot_count);
    let (mut free_producer, free_consumer) = RingBuffer::new(slot_count);
    for _ in 0..slot_count {
        let _ = free_producer.push(Vec::with_capacity(sizing.slot_bytes));
    }
    let stats = Arc::new(SharedStats::default());
    let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::Relaxed);
    (
        BlockProducer {
            queue_id,
            filled: filled_producer,
            free: free_consumer,
            slot_bytes: sizing.slot_bytes,
            pending_discontinuity: false,
            stats: Arc::clone(&stats),
        },
        BlockConsumer {
            queue_id,
            filled: filled_consumer,
            free: free_producer,
            stats,
        },
    )
}
