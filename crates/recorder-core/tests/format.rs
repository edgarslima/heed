//! Audio format invariants: sizes, durations, and accepted ranges.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use recorder_core::audio::format::{
    AudioFormat, ChannelLayout, FormatError, SampleFormat, SampleLayout,
};

fn format(sample_rate_hz: u32, layout: ChannelLayout, sample_format: SampleFormat) -> AudioFormat {
    match AudioFormat::new(
        sample_rate_hz,
        layout,
        sample_format,
        SampleLayout::Interleaved,
    ) {
        Ok(format) => format,
        Err(error) => panic!("valid test format rejected: {error:?}"),
    }
}

#[test]
fn bytes_per_frame_for_stereo_i24_is_6() {
    assert_eq!(
        format(48_000, ChannelLayout::Stereo, SampleFormat::I24).bytes_per_frame(),
        6
    );
}

#[test]
fn frames_in_100_ms_at_48_khz_is_4800() {
    assert_eq!(
        format(48_000, ChannelLayout::Mono, SampleFormat::F32).frames_in_ms(100),
        4_800
    );
}

#[test]
fn frames_in_10_ms_at_44_1_khz_is_441() {
    assert_eq!(
        format(44_100, ChannelLayout::Mono, SampleFormat::F32).frames_in_ms(10),
        441
    );
}

#[test]
fn frames_in_1_ms_at_44_1_khz_rounds_up_to_45() {
    assert_eq!(
        format(44_100, ChannelLayout::Mono, SampleFormat::F32).frames_in_ms(1),
        45
    );
}

#[test]
fn rejects_sample_rates_outside_8_khz_to_384_khz() {
    for rate in [7_999, 384_001] {
        assert_eq!(
            AudioFormat::new(
                rate,
                ChannelLayout::Mono,
                SampleFormat::I16,
                SampleLayout::Interleaved
            ),
            Err(FormatError::UnsupportedSampleRate(rate))
        );
    }
    for rate in [8_000, 384_000] {
        assert!(
            AudioFormat::new(
                rate,
                ChannelLayout::Mono,
                SampleFormat::I16,
                SampleLayout::Interleaved
            )
            .is_ok()
        );
    }
}

#[test]
fn maps_channel_counts_to_layouts() {
    assert_eq!(
        ChannelLayout::from_channel_count(1),
        Ok(ChannelLayout::Mono)
    );
    assert_eq!(
        ChannelLayout::from_channel_count(2),
        Ok(ChannelLayout::Stereo)
    );
    assert_eq!(
        ChannelLayout::from_channel_count(6),
        Ok(ChannelLayout::Discrete(6))
    );
    assert_eq!(
        ChannelLayout::from_channel_count(6).map(ChannelLayout::channel_count),
        Ok(6)
    );
    for count in [0, 9] {
        assert_eq!(
            ChannelLayout::from_channel_count(count),
            Err(FormatError::UnsupportedChannelCount(count))
        );
    }
}

#[test]
fn sample_formats_report_their_sizes() {
    assert_eq!(SampleFormat::I16.bytes_per_sample(), 2);
    assert_eq!(SampleFormat::I24.bytes_per_sample(), 3);
    assert_eq!(SampleFormat::I32.bytes_per_sample(), 4);
    assert_eq!(SampleFormat::F32.bytes_per_sample(), 4);
}

#[test]
fn rejects_discrete_layouts_outside_3_to_8_channels() {
    for count in [2, 9] {
        assert_eq!(
            AudioFormat::new(
                48_000,
                ChannelLayout::Discrete(count),
                SampleFormat::I16,
                SampleLayout::Planar
            ),
            Err(FormatError::UnsupportedChannelCount(count))
        );
    }
    let six = format(48_000, ChannelLayout::Discrete(6), SampleFormat::I16);
    assert_eq!(six.bytes_per_frame(), 12);
}

#[test]
fn exposes_its_parameters() {
    let mono = AudioFormat::new(
        44_100,
        ChannelLayout::Mono,
        SampleFormat::I16,
        SampleLayout::Planar,
    )
    .expect("valid format");

    assert_eq!(mono.sample_rate_hz(), 44_100);
    assert_eq!(mono.channel_layout(), ChannelLayout::Mono);
    assert_eq!(mono.sample_format(), SampleFormat::I16);
    assert_eq!(mono.sample_layout(), SampleLayout::Planar);
    assert_eq!(mono.bytes_per_frame(), 2);
}
