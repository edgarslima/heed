//! PCM to f32 conversion. Integer conversions are exact, so tests use equality.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    reason = "test code: a failed precondition aborts the test; exact float values are expected"
)]

use recorder_core::audio::convert::{ConversionError, convert_to_f32};
use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};

fn format(
    layout: ChannelLayout,
    sample_format: SampleFormat,
    sample_layout: SampleLayout,
) -> AudioFormat {
    AudioFormat::new(48_000, layout, sample_format, sample_layout).expect("valid format")
}

fn mono(sample_format: SampleFormat) -> AudioFormat {
    format(
        ChannelLayout::Mono,
        sample_format,
        SampleLayout::Interleaved,
    )
}

fn f32_bytes(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

#[test]
fn converts_i16_extremes() {
    let mut out = Vec::new();

    let report = convert_to_f32(
        &[0x00, 0x80, 0xff, 0x7f, 0x00, 0x00],
        &mono(SampleFormat::I16),
        &mut out,
    )
    .expect("whole frames");

    assert_eq!(out, vec![-1.0, 32_767.0 / 32_768.0, 0.0]);
    assert_eq!(report.frame_count, 3);
    assert_eq!(report.sanitized_samples, 0);
}

#[test]
fn converts_i24_extremes() {
    let mut out = Vec::new();

    convert_to_f32(
        &[0x00, 0x00, 0x80, 0xff, 0xff, 0x7f],
        &mono(SampleFormat::I24),
        &mut out,
    )
    .expect("whole frames");

    assert_eq!(out, vec![-1.0, 8_388_607.0 / 8_388_608.0]);
}

#[test]
fn converts_i32_minimum_to_minus_one() {
    let mut out = Vec::new();

    convert_to_f32(&i32::MIN.to_le_bytes(), &mono(SampleFormat::I32), &mut out)
        .expect("whole frames");

    assert_eq!(out, vec![-1.0]);
}

#[test]
fn sanitizes_non_finite_float_samples() {
    let mut out = Vec::new();
    let pcm = f32_bytes(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5]);

    let report = convert_to_f32(&pcm, &mono(SampleFormat::F32), &mut out).expect("whole frames");

    assert_eq!(out, vec![0.0, 0.0, 0.0, 0.5]);
    assert_eq!(report.sanitized_samples, 3);
}

#[test]
fn keeps_finite_float_samples_above_full_scale() {
    let mut out = Vec::new();

    convert_to_f32(&f32_bytes(&[1.5, -2.0]), &mono(SampleFormat::F32), &mut out)
        .expect("whole frames");

    assert_eq!(out, vec![1.5, -2.0]);
}

#[test]
fn interleaves_planar_stereo() {
    let mut out = Vec::new();
    let pcm = f32_bytes(&[0.25, 0.5, -0.25, -0.5]);
    let planar = format(
        ChannelLayout::Stereo,
        SampleFormat::F32,
        SampleLayout::Planar,
    );

    let report = convert_to_f32(&pcm, &planar, &mut out).expect("whole frames");

    assert_eq!(out, vec![0.25, -0.25, 0.5, -0.5]);
    assert_eq!(report.frame_count, 2);
}

#[test]
fn rejects_partial_frames() {
    let mut out = Vec::new();
    let stereo_i24 = format(
        ChannelLayout::Stereo,
        SampleFormat::I24,
        SampleLayout::Interleaved,
    );

    let result = convert_to_f32(&[0; 5], &stereo_i24, &mut out);

    assert_eq!(
        result,
        Err(ConversionError::PartialFrame {
            byte_len: 5,
            bytes_per_frame: 6
        })
    );
}

#[test]
fn reuses_output_capacity() {
    let mut out: Vec<f32> = Vec::with_capacity(16);
    out.push(9.0);
    let pointer = out.as_ptr();

    convert_to_f32(&[0x00, 0x40], &mono(SampleFormat::I16), &mut out).expect("whole frames");

    assert_eq!(out, vec![0.5]);
    assert_eq!(out.as_ptr(), pointer);
    assert_eq!(out.capacity(), 16);
}
