//! Conversion of captured PCM bytes to finite interleaved `f32` samples
//! (architecture section 4, "Conversion").

use crate::audio::format::{AudioFormat, SampleFormat, SampleLayout};

const I16_SCALE: f32 = 32_768.0;
const I24_SCALE: f32 = 8_388_608.0;
const I32_SCALE: f32 = 2_147_483_648.0;

/// Diagnostics of one conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversionReport {
    /// Frames converted.
    pub frame_count: u32,
    /// Non-finite float samples replaced with 0.0.
    pub sanitized_samples: u32,
}

/// A buffer that cannot be converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionError {
    /// The byte length is not a whole number of frames.
    PartialFrame {
        byte_len: usize,
        bytes_per_frame: usize,
    },
}

/// Converts `pcm` in `format` to interleaved `f32` samples in `out`.
///
/// `out` is cleared and refilled; it grows only when its capacity is too small, so a
/// preallocated buffer is reused without allocation. Integer samples are scaled to
/// [-1.0, 1.0). Finite float samples are kept as they are, including values beyond
/// full scale; NaN and infinities become 0.0 and are counted.
///
/// # Errors
///
/// Returns [`ConversionError::PartialFrame`] when `pcm` does not hold whole frames.
pub fn convert_to_f32(
    pcm: &[u8],
    format: &AudioFormat,
    out: &mut Vec<f32>,
) -> Result<ConversionReport, ConversionError> {
    let bytes_per_frame = format.bytes_per_frame();
    if !pcm.len().is_multiple_of(bytes_per_frame) {
        return Err(ConversionError::PartialFrame {
            byte_len: pcm.len(),
            bytes_per_frame,
        });
    }
    let channels = usize::from(format.channel_layout().channel_count());
    let frame_count = pcm.len() / bytes_per_frame;
    let sample_format = format.sample_format();

    out.clear();
    out.resize(frame_count * channels, 0.0);

    let mut sanitized_samples: u32 = 0;
    for (index, bytes) in pcm
        .chunks_exact(sample_format.bytes_per_sample())
        .enumerate()
    {
        let decoded = decode_sample(bytes, sample_format);
        let sample = if decoded.is_finite() {
            decoded
        } else {
            sanitized_samples = sanitized_samples.saturating_add(1);
            0.0
        };
        let target = match format.sample_layout() {
            SampleLayout::Interleaved => index,
            SampleLayout::Planar => (index % frame_count) * channels + index / frame_count,
        };
        if let Some(slot) = out.get_mut(target) {
            *slot = sample;
        }
    }

    Ok(ConversionReport {
        frame_count: u32::try_from(frame_count).unwrap_or(u32::MAX),
        sanitized_samples,
    })
}

fn decode_sample(bytes: &[u8], sample_format: SampleFormat) -> f32 {
    match (sample_format, bytes) {
        (SampleFormat::I16, &[b0, b1]) => f32::from(i16::from_le_bytes([b0, b1])) / I16_SCALE,
        // Placing the 3 bytes in the high part and shifting back sign-extends the value.
        (SampleFormat::I24, &[b0, b1, b2]) => {
            (i32::from_le_bytes([0, b0, b1, b2]) >> 8) as f32 / I24_SCALE
        }
        (SampleFormat::I32, &[b0, b1, b2, b3]) => {
            i32::from_le_bytes([b0, b1, b2, b3]) as f32 / I32_SCALE
        }
        (SampleFormat::F32, &[b0, b1, b2, b3]) => f32::from_le_bytes([b0, b1, b2, b3]),
        // Unreachable: chunks always have the sample format's width.
        _ => 0.0,
    }
}
