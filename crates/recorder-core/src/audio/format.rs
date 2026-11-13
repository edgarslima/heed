//! PCM formats as negotiated with a capture device.

/// Lowest sample rate the engine accepts.
pub const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
/// Highest sample rate the engine accepts.
pub const MAX_SAMPLE_RATE_HZ: u32 = 384_000;
/// Highest channel count the engine accepts.
pub const MAX_CHANNELS: u16 = 8;

/// Encoding of one sample. All multi-byte encodings are little-endian.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    /// Signed 16-bit integer.
    I16,
    /// Signed 24-bit integer packed in 3 bytes.
    I24,
    /// Signed 32-bit integer.
    I32,
    /// 32-bit IEEE float.
    F32,
}

impl SampleFormat {
    /// Number of bytes one sample occupies.
    #[must_use]
    pub const fn bytes_per_sample(self) -> usize {
        match self {
            Self::I16 => 2,
            Self::I24 => 3,
            Self::I32 | Self::F32 => 4,
        }
    }
}

/// How channels are arranged in a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleLayout {
    /// Frame by frame: one sample per channel, then the next frame.
    Interleaved,
    /// Channel by channel: all frames of channel 0, then channel 1, and so on.
    Planar,
}

/// Channel arrangement of a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelLayout {
    /// One channel.
    Mono,
    /// Left and right.
    Stereo,
    /// Three or more channels without a named mapping.
    Discrete(u16),
}

impl ChannelLayout {
    /// Maps a device channel count to a layout.
    ///
    /// # Errors
    ///
    /// Returns [`FormatError::UnsupportedChannelCount`] for 0 or more than [`MAX_CHANNELS`].
    pub const fn from_channel_count(count: u16) -> Result<Self, FormatError> {
        match count {
            1 => Ok(Self::Mono),
            2 => Ok(Self::Stereo),
            3..=MAX_CHANNELS => Ok(Self::Discrete(count)),
            _ => Err(FormatError::UnsupportedChannelCount(count)),
        }
    }

    /// Number of channels in the layout.
    #[must_use]
    pub const fn channel_count(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Discrete(count) => count,
        }
    }
}

/// A rejected format parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// The rate is outside [`MIN_SAMPLE_RATE_HZ`]..=[`MAX_SAMPLE_RATE_HZ`].
    UnsupportedSampleRate(u32),
    /// The channel count is 0 or above [`MAX_CHANNELS`].
    UnsupportedChannelCount(u16),
}

/// A validated PCM format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioFormat {
    sample_rate_hz: u32,
    channel_layout: ChannelLayout,
    sample_format: SampleFormat,
    sample_layout: SampleLayout,
}

impl AudioFormat {
    /// Validates and creates a format.
    ///
    /// # Errors
    ///
    /// Returns a [`FormatError`] when the rate or the channel count is out of range.
    pub const fn new(
        sample_rate_hz: u32,
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        sample_layout: SampleLayout,
    ) -> Result<Self, FormatError> {
        if sample_rate_hz < MIN_SAMPLE_RATE_HZ || sample_rate_hz > MAX_SAMPLE_RATE_HZ {
            return Err(FormatError::UnsupportedSampleRate(sample_rate_hz));
        }
        if let ChannelLayout::Discrete(count) = channel_layout
            && (count < 3 || count > MAX_CHANNELS)
        {
            return Err(FormatError::UnsupportedChannelCount(count));
        }
        Ok(Self {
            sample_rate_hz,
            channel_layout,
            sample_format,
            sample_layout,
        })
    }

    /// Frames per second.
    #[must_use]
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Channel arrangement.
    #[must_use]
    pub const fn channel_layout(&self) -> ChannelLayout {
        self.channel_layout
    }

    /// Sample encoding.
    #[must_use]
    pub const fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    /// Interleaved or planar arrangement.
    #[must_use]
    pub const fn sample_layout(&self) -> SampleLayout {
        self.sample_layout
    }

    /// Bytes in one frame (one sample per channel).
    #[must_use]
    pub const fn bytes_per_frame(&self) -> usize {
        self.sample_format.bytes_per_sample() * self.channel_layout.channel_count() as usize
    }

    /// Frames needed to cover `duration_ms`, rounded up.
    #[must_use]
    pub const fn frames_in_ms(&self, duration_ms: u32) -> u64 {
        (self.sample_rate_hz as u64 * duration_ms as u64).div_ceil(1_000)
    }
}
