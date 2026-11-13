//! Deterministic synthetic source driven on the test thread.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a failed precondition aborts the test"
)]

use std::sync::mpsc::{Receiver, channel};

use recorder_core::audio::convert::convert_to_f32;
use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
use recorder_core::capture::block::SourceId;
use recorder_core::capture::event::{CaptureError, CaptureEvent};
use recorder_core::capture::queue::{BlockConsumer, QueueSizing, block_queue};
use recorder_core::capture::source::CaptureSource;
use recorder_core::capture::synthetic::{
    ScriptedEvent, SyntheticConfig, SyntheticSignal, SyntheticSource, host_time_ns,
};

const SOURCE: SourceId = SourceId::new(4);

fn mono(sample_format: SampleFormat) -> AudioFormat {
    AudioFormat::new(
        48_000,
        ChannelLayout::Mono,
        sample_format,
        SampleLayout::Interleaved,
    )
    .expect("valid format")
}

fn config(signal: SyntheticSignal, script: Vec<(u64, ScriptedEvent)>) -> SyntheticConfig {
    SyntheticConfig {
        source_id: SOURCE,
        format: mono(SampleFormat::I16),
        signal,
        block_frames: 480,
        start_host_time_ns: 1_000,
        drift_ppm: 0,
        script,
    }
}

fn started(config: SyntheticConfig) -> (SyntheticSource, BlockConsumer, Receiver<CaptureEvent>) {
    let sizing = QueueSizing::for_format(&config.format, config.block_frames);
    let (producer, consumer) = block_queue(sizing);
    let (events, receiver) = channel();
    let mut source = SyntheticSource::new(config);
    assert_eq!(source.prepare(), Ok(mono(SampleFormat::I16)));
    source.start(producer, events).expect("start");
    (source, consumer, receiver)
}

#[test]
fn pumps_sequential_blocks_with_contiguous_frames() {
    let (mut source, mut consumer, _events) =
        started(config(SyntheticSignal::Constant(0.25), Vec::new()));

    assert_eq!(source.pump(3), Ok(3));

    for (index, expected_start) in [0_u64, 480, 960].into_iter().enumerate() {
        let block = consumer.pop().expect("block");
        assert_eq!(block.header().sequence, index as u64);
        assert_eq!(block.header().start_frame, expected_start);
        assert_eq!(block.header().frame_count, 480);
        assert_eq!(block.header().source_id, SOURCE);
        assert_eq!(block.pcm().len(), 960);
    }
    assert_eq!(source.source_id(), SOURCE);
}

#[test]
fn sine_round_trips_through_conversion_within_one_i16_step() {
    let (mut source, mut consumer, _events) = started(config(
        SyntheticSignal::Sine {
            frequency_hz: 1_000.0,
            amplitude: 0.5,
        },
        Vec::new(),
    ));
    source.pump(1).expect("pump");
    let block = consumer.pop().expect("block");
    let mut samples = Vec::new();

    convert_to_f32(block.pcm(), &block.header().format, &mut samples).expect("whole frames");

    for (frame, sample) in samples.iter().enumerate() {
        let expected = 0.5 * (std::f64::consts::TAU * 1_000.0 * frame as f64 / 48_000.0).sin();
        assert!(
            (f64::from(*sample) - expected).abs() <= 1.0 / 32_768.0,
            "frame {frame}"
        );
    }
}

#[test]
fn encodes_every_sample_format_and_planar_layout() {
    for sample_format in [
        SampleFormat::I16,
        SampleFormat::I24,
        SampleFormat::I32,
        SampleFormat::F32,
    ] {
        let format = AudioFormat::new(
            48_000,
            ChannelLayout::Stereo,
            sample_format,
            SampleLayout::Planar,
        )
        .expect("valid format");
        let mut config = config(SyntheticSignal::Constant(0.5), Vec::new());
        config.format = format;
        config.block_frames = 4;
        let (producer, mut consumer) = block_queue(QueueSizing::for_format(&format, 4));
        let mut source = SyntheticSource::new(config);
        source.start(producer, channel().0).expect("start");
        source.pump(1).expect("pump");
        let mut samples = Vec::new();

        convert_to_f32(consumer.pop().expect("block").pcm(), &format, &mut samples)
            .expect("whole frames");

        assert_eq!(samples, vec![0.5; 8], "{sample_format:?}");
    }
}

#[test]
fn host_time_without_drift_is_exact() {
    assert_eq!(
        host_time_ns(1_000, 48_000, 48_000, 0),
        1_000 + 1_000_000_000
    );
}

#[test]
fn host_time_with_100_ppm_fast_clock() {
    assert_eq!(
        host_time_ns(1_000, 4_800_000, 48_000, 100),
        1_000 + 99_990_000_999
    );
}

#[test]
fn host_time_survives_a_day_at_384_khz() {
    let frames = 384_000_u64 * 86_400;

    assert_eq!(host_time_ns(0, frames, 384_000, 1_000), 86_313_686_313_686);
    assert!(host_time_ns(0, frames, 384_000, -1_000) > 86_400_000_000_000);
}

#[test]
fn scripted_silence_discontinuity_and_invalid_timestamp() {
    let script = vec![
        (1, ScriptedEvent::SilentBlock),
        (2, ScriptedEvent::Discontinuity { lost_frames: 100 }),
        (3, ScriptedEvent::InvalidTimestamp),
    ];
    let (mut source, mut consumer, events) =
        started(config(SyntheticSignal::Constant(0.25), script));

    assert_eq!(source.pump(4), Ok(4));

    let first = consumer.pop().expect("block 0");
    assert_eq!(first.header().first_frame_host_time_ns, Some(1_000));
    let silent = consumer.pop().expect("block 1");
    assert!(silent.header().flags.silent);
    assert!(silent.pcm().iter().all(|byte| *byte == 0));
    let after_gap = consumer.pop().expect("block 2");
    assert!(after_gap.header().flags.discontinuity);
    assert_eq!(after_gap.header().start_frame, 960 + 100);
    assert_eq!(
        events.try_recv(),
        Ok(CaptureEvent::Discontinuity {
            source_id: SOURCE,
            at_frame: 960,
            lost_frames: Some(100)
        })
    );
    let invalid = consumer.pop().expect("block 3");
    assert!(invalid.header().flags.timestamp_invalid);
    assert_eq!(invalid.header().first_frame_host_time_ns, None);
    assert_eq!(invalid.header().start_frame, 1_540);
}

#[test]
fn disconnect_stops_emission_and_reports_the_frame() {
    let (mut source, mut consumer, events) = started(config(
        SyntheticSignal::Silence,
        vec![(2, ScriptedEvent::Disconnect)],
    ));

    assert_eq!(source.pump(5), Ok(2));

    assert_eq!(
        events.try_recv(),
        Ok(CaptureEvent::Disconnected {
            source_id: SOURCE,
            at_frame: Some(960)
        })
    );
    assert_eq!(source.pump(1), Err(CaptureError::SourceUnavailable));
    assert!(consumer.pop().is_some());
    assert!(consumer.pop().is_some());
    assert!(consumer.pop().is_none());
}

#[test]
fn stop_is_idempotent_and_sends_ended_once() {
    let (mut source, _consumer, events) = started(config(SyntheticSignal::Silence, Vec::new()));
    source.pump(2).expect("pump");

    let first = source.stop();
    let second = source.stop();

    assert_eq!((first.final_frame, first.already_stopped), (960, false));
    assert_eq!((second.final_frame, second.already_stopped), (960, true));
    assert_eq!(
        events.try_recv(),
        Ok(CaptureEvent::Ended {
            source_id: SOURCE,
            final_frame: 960
        })
    );
    assert!(events.try_recv().is_err());
    assert_eq!(source.pump(1), Err(CaptureError::SourceUnavailable));
}

#[test]
fn pump_before_start_is_an_error() {
    let mut source = SyntheticSource::new(config(SyntheticSignal::Silence, Vec::new()));

    assert_eq!(source.pump(1), Err(CaptureError::SourceUnavailable));
}

#[test]
fn start_twice_is_an_error() {
    let (mut source, _consumer, _events) = started(config(SyntheticSignal::Silence, Vec::new()));
    let (producer, _second_consumer) = block_queue(QueueSizing {
        slot_count: 2,
        slot_bytes: 960,
    });

    assert_eq!(
        source.start(producer, channel().0),
        Err(CaptureError::SourceUnavailable)
    );
}
