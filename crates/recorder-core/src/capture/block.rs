//! Audio blocks as delivered by a capture source (architecture section 2, `AudioBlock`).

use crate::audio::format::AudioFormat;

/// Opaque identifier of a source within one session. Never persisted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    /// Wraps a session-scoped source number.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The wrapped number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Conditions reported with a block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockFlags {
    /// The device marked the block as silent; its samples are zeros.
    pub silent: bool,
    /// Frames were lost before this block (device discontinuity or queue overflow).
    pub discontinuity: bool,
    /// The device timestamp for this block is not valid.
    pub timestamp_invalid: bool,
}

/// Metadata of one block of PCM frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    /// Source that produced the block.
    pub source_id: SourceId,
    /// Per-source block counter, starting at 0.
    pub sequence: u64,
    /// Index of the first frame in the source clock.
    pub start_frame: u64,
    /// Frames in the block. One frame holds one sample per channel.
    pub frame_count: u32,
    /// Rate, sample format, channel layout, and sample layout of the PCM bytes.
    pub format: AudioFormat,
    /// Host monotonic time of the first frame, in nanoseconds, when available.
    pub first_frame_host_time_ns: Option<u64>,
    /// Conditions reported with the block.
    pub flags: BlockFlags,
}

impl BlockHeader {
    /// Number of PCM bytes the block must carry.
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.frame_count as usize * self.format.bytes_per_frame()
    }
}
