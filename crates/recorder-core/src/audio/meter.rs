//! Level metering per source (architecture section 4, "Metering"; design system
//! section 3, "Meters and waveform").

use std::sync::{Arc, Mutex, PoisonError, TryLockError};

use crate::audio::format::AudioFormat;

/// Lowest reported level. Silence and invalid values map here, which keeps meter
/// values finite for display and IPC.
pub const DBFS_FLOOR: f32 = -120.0;

/// Magnitude at or above which a sample counts as clipped: the largest positive
/// 16-bit code, 32767 / 32768.
pub const CLIP_LEVEL: f32 = 32_767.0 / 32_768.0;

const WINDOW_MS: u32 = 100;

/// Converts a linear amplitude to dBFS, clamped to [`DBFS_FLOOR`]. NaN maps to the floor.
#[must_use]
pub fn amplitude_to_dbfs(amplitude: f32) -> f32 {
    let level = 20.0 * amplitude.abs().log10();
    if level.is_nan() {
        DBFS_FLOOR
    } else {
        level.max(DBFS_FLOOR)
    }
}

/// Levels of one completed 100 ms window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeterSnapshot {
    /// Window counter, starting at 1.
    pub sequence: u64,
    /// Source frame index just after the window.
    pub end_frame: u64,
    /// Largest absolute sample across channels, in dBFS.
    pub peak_dbfs: f32,
    /// Root mean square over all channels (average channel energy), in dBFS.
    pub rms_dbfs: f32,
    /// True when any sample reached [`CLIP_LEVEL`].
    pub clipped: bool,
    /// Lowest signed sample, for the waveform envelope.
    pub envelope_min: f32,
    /// Highest signed sample, for the waveform envelope.
    pub envelope_max: f32,
}

/// Accumulates samples of one source into 100 ms windows. No allocation after creation.
#[derive(Clone, Debug)]
pub struct Meter {
    channels: usize,
    window_frames: u64,
    frames_in_window: u64,
    peak: f32,
    sum_of_squares: f64,
    clipped: bool,
    envelope_min: f32,
    envelope_max: f32,
    sequence: u64,
}

impl Meter {
    /// Creates a meter for `format` with a 100 ms window.
    #[must_use]
    pub fn new(format: &AudioFormat) -> Self {
        Self {
            channels: usize::from(format.channel_layout().channel_count()),
            window_frames: format.frames_in_ms(WINDOW_MS),
            frames_in_window: 0,
            peak: 0.0,
            sum_of_squares: 0.0,
            clipped: false,
            envelope_min: 0.0,
            envelope_max: 0.0,
            sequence: 0,
        }
    }

    /// Adds interleaved samples whose first frame is `start_frame` and returns the last
    /// window completed by this call, if any.
    pub fn process(&mut self, interleaved: &[f32], start_frame: u64) -> Option<MeterSnapshot> {
        let mut latest = None;
        for (offset, frame) in interleaved.chunks_exact(self.channels).enumerate() {
            for &sample in frame {
                let magnitude = sample.abs();
                if magnitude > self.peak {
                    self.peak = magnitude;
                }
                if magnitude >= CLIP_LEVEL {
                    self.clipped = true;
                }
                self.sum_of_squares += f64::from(sample) * f64::from(sample);
                self.envelope_min = self.envelope_min.min(sample);
                self.envelope_max = self.envelope_max.max(sample);
            }
            self.frames_in_window += 1;
            if self.frames_in_window == self.window_frames {
                latest = Some(self.finish_window(start_frame + offset as u64 + 1));
            }
        }
        latest
    }

    fn finish_window(&mut self, end_frame: u64) -> MeterSnapshot {
        self.sequence += 1;
        let samples = (self.window_frames * self.channels as u64) as f64;
        let snapshot = MeterSnapshot {
            sequence: self.sequence,
            end_frame,
            peak_dbfs: amplitude_to_dbfs(self.peak),
            rms_dbfs: amplitude_to_dbfs((self.sum_of_squares / samples).sqrt() as f32),
            clipped: self.clipped,
            envelope_min: self.envelope_min,
            envelope_max: self.envelope_max,
        };
        self.frames_in_window = 0;
        self.peak = 0.0;
        self.sum_of_squares = 0.0;
        self.clipped = false;
        self.envelope_min = 0.0;
        self.envelope_max = 0.0;
        snapshot
    }
}

/// Holds the most recent snapshot for a reader that may fall behind. Older snapshots
/// are discarded (architecture section 2: "discard stale snapshots").
#[derive(Clone, Debug, Default)]
pub struct SnapshotSlot {
    latest: Arc<Mutex<Option<MeterSnapshot>>>,
}

impl SnapshotSlot {
    /// Creates an empty slot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the stored snapshot without blocking. If the reader holds the lock at
    /// that instant, this snapshot is skipped; the next window replaces it 100 ms later.
    pub fn publish(&self, snapshot: MeterSnapshot) {
        let guard = match self.latest.try_lock() {
            Ok(guard) => Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        };
        if let Some(mut latest) = guard {
            *latest = Some(snapshot);
        }
    }

    /// Takes the newest snapshot, leaving the slot empty.
    pub fn take_latest(&self) -> Option<MeterSnapshot> {
        self.latest
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}
