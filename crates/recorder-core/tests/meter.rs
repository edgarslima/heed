//! 100 ms meter windows, dBFS conversion, and the latest-value snapshot slot.
//! dB values are compared with an absolute tolerance of 0.001 dB unless stated.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    reason = "test code: a failed precondition aborts the test; exact float values are expected"
)]

use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
use recorder_core::audio::meter::{
    CLIP_LEVEL, DBFS_FLOOR, Meter, MeterSnapshot, SnapshotSlot, amplitude_to_dbfs,
};

const DB_TOLERANCE: f32 = 0.001;

fn format(layout: ChannelLayout) -> AudioFormat {
    AudioFormat::new(48_000, layout, SampleFormat::F32, SampleLayout::Interleaved)
        .expect("valid format")
}

fn assert_db(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} dB is not within {tolerance} of {expected} dB"
    );
}

fn window_of(samples: &[f32]) -> Vec<f32> {
    let mut window = samples.to_vec();
    window.resize(4_800, 0.0);
    window
}

#[test]
fn dbfs_of_full_scale_is_zero() {
    assert_db(amplitude_to_dbfs(1.0), 0.0, DB_TOLERANCE);
    assert_db(amplitude_to_dbfs(-1.0), 0.0, DB_TOLERANCE);
}

#[test]
fn dbfs_of_half_scale_is_minus_6_0206() {
    assert_db(amplitude_to_dbfs(0.5), -6.0206, DB_TOLERANCE);
}

#[test]
fn dbfs_of_silence_is_the_floor() {
    assert_eq!(amplitude_to_dbfs(0.0), DBFS_FLOOR);
    assert_eq!(amplitude_to_dbfs(1.0e-7), DBFS_FLOOR);
}

#[test]
fn dbfs_of_nan_is_the_floor() {
    assert_eq!(amplitude_to_dbfs(f32::NAN), DBFS_FLOOR);
}

#[test]
fn full_scale_sine_has_0_dbfs_peak_and_minus_3_0103_rms() {
    let samples: Vec<f32> = (0..4_800)
        .map(|frame| (std::f64::consts::TAU * 1_000.0 * f64::from(frame) / 48_000.0).sin() as f32)
        .collect();
    let mut meter = Meter::new(&format(ChannelLayout::Mono));

    let snapshot = meter.process(&samples, 0).expect("one window");

    assert_db(snapshot.peak_dbfs, 0.0, DB_TOLERANCE);
    assert_db(snapshot.rms_dbfs, -3.0103, 0.01);
}

#[test]
fn stereo_peak_uses_the_louder_channel_and_rms_averages_energy() {
    let samples: Vec<f32> = [0.5, 0.0].repeat(4_800);
    let mut meter = Meter::new(&format(ChannelLayout::Stereo));

    let snapshot = meter.process(&samples, 0).expect("one window");

    assert_db(snapshot.peak_dbfs, -6.0206, DB_TOLERANCE);
    assert_db(snapshot.rms_dbfs, -9.0309, DB_TOLERANCE);
    assert!(!snapshot.clipped);
}

#[test]
fn flags_clipping_at_the_i16_full_scale_code() {
    let mono = format(ChannelLayout::Mono);

    let at_full_scale = Meter::new(&mono)
        .process(&window_of(&[CLIP_LEVEL]), 0)
        .expect("window");
    let below = Meter::new(&mono)
        .process(&window_of(&[0.999]), 0)
        .expect("window");

    assert_eq!(CLIP_LEVEL, 32_767.0 / 32_768.0);
    assert!(at_full_scale.clipped);
    assert!(!below.clipped);
}

#[test]
fn emits_a_window_every_100_ms() {
    let mut meter = Meter::new(&format(ChannelLayout::Mono));

    assert_eq!(meter.process(&[0.1; 4_799], 0), None);
    let snapshot = meter.process(&[0.1], 4_799).expect("window completed");
    let next = meter.process(&[0.1; 4_800], 4_800).expect("second window");

    assert_eq!((snapshot.end_frame, snapshot.sequence), (4_800, 1));
    assert_eq!((next.end_frame, next.sequence), (9_600, 2));
}

#[test]
fn returns_the_last_window_completed_in_one_call() {
    let mut meter = Meter::new(&format(ChannelLayout::Mono));
    let mut samples = vec![0.5; 4_800];
    samples.extend(vec![0.25; 4_800]);

    let snapshot = meter.process(&samples, 0).expect("two windows");

    assert_eq!((snapshot.end_frame, snapshot.sequence), (9_600, 2));
    assert_db(snapshot.peak_dbfs, -12.0412, DB_TOLERANCE);
}

#[test]
fn reports_the_signed_envelope() {
    let mut meter = Meter::new(&format(ChannelLayout::Mono));

    let snapshot = meter
        .process(&window_of(&[-0.25, 0.75, 0.1]), 0)
        .expect("window");

    assert_eq!(
        (snapshot.envelope_min, snapshot.envelope_max),
        (-0.25, 0.75)
    );
}

#[test]
fn envelope_of_a_constant_window_is_that_constant() {
    let mut meter = Meter::new(&format(ChannelLayout::Mono));

    let first = meter.process(&[0.5; 4_800], 0).expect("window");
    let second = meter.process(&[-0.25; 4_800], 4_800).expect("window");

    assert_eq!((first.envelope_min, first.envelope_max), (0.5, 0.5));
    assert_eq!((second.envelope_min, second.envelope_max), (-0.25, -0.25));
}

#[test]
fn snapshot_slot_keeps_only_the_latest() {
    let slot = SnapshotSlot::new();
    let reader = slot.clone();
    let snapshot = |sequence: u64| MeterSnapshot {
        sequence,
        end_frame: sequence * 4_800,
        peak_dbfs: -6.0,
        rms_dbfs: -9.0,
        clipped: false,
        envelope_min: -0.5,
        envelope_max: 0.5,
    };

    slot.publish(snapshot(1));
    slot.publish(snapshot(2));

    assert_eq!(reader.take_latest(), Some(snapshot(2)));
    assert_eq!(reader.take_latest(), None);
}
