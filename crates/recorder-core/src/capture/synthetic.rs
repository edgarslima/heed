//! Deterministic synthetic source for tests (architecture section 1: "a synthetic
//! source only for testing"). It runs on the caller's thread through [`SyntheticSource::pump`].

use std::f64::consts::TAU;
use std::sync::mpsc::Sender;

use crate::audio::format::{AudioFormat, SampleFormat, SampleLayout};
use crate::capture::block::{BlockFlags, BlockHeader, SourceId};
use crate::capture::event::{CaptureError, CaptureEvent};
use crate::capture::queue::BlockProducer;
use crate::capture::source::{CaptureSource, StopSummary};

const NANOS_PER_SECOND_TIMES_PPM_BASE: u128 = 1_000_000_000 * 1_000_000;
const PPM_BASE: i64 = 1_000_000;

/// Signal written into every channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SyntheticSignal {
    /// All zeros.
    Silence,
    /// The same value in every sample.
    Constant(f32),
    /// A sine starting at phase 0 on frame 0 of the source clock.
    Sine { frequency_hz: f32, amplitude: f32 },
}

/// A condition injected at a given block index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptedEvent {
    /// The block is delivered as a device-silent block.
    SilentBlock,
    /// `lost_frames` frames are skipped before the block.
    Discontinuity { lost_frames: u64 },
    /// The block carries no valid timestamp.
    InvalidTimestamp,
    /// The source disappears before the block.
    Disconnect,
}

/// Configuration of a synthetic source.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticConfig {
    /// Session-scoped identifier.
    pub source_id: SourceId,
    /// Format of the produced PCM.
    pub format: AudioFormat,
    /// Signal written into every channel.
    pub signal: SyntheticSignal,
    /// Frames per block.
    pub block_frames: u32,
    /// Host time of frame 0, in nanoseconds.
    pub start_host_time_ns: u64,
    /// Source clock error in parts per million; positive runs fast.
    pub drift_ppm: i32,
    /// Conditions keyed by block index.
    pub script: Vec<(u64, ScriptedEvent)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Running,
    Disconnected,
    Stopped,
}

/// A capture source that generates blocks on demand.
pub struct SyntheticSource {
    config: SyntheticConfig,
    producer: Option<BlockProducer>,
    events: Option<Sender<CaptureEvent>>,
    state: State,
    next_block: u64,
    next_frame: u64,
    scratch: Vec<u8>,
}

/// Host time of source `frame` for a clock that runs `drift_ppm` fast, in nanoseconds.
/// Uses 128-bit integers so day-long sessions at 384 kHz cannot overflow.
#[must_use]
pub fn host_time_ns(
    start_host_time_ns: u64,
    frame: u64,
    sample_rate_hz: u32,
    drift_ppm: i32,
) -> u64 {
    let ticks_per_million_seconds = u128::from(sample_rate_hz)
        * u128::try_from((PPM_BASE + i64::from(drift_ppm)).max(1)).unwrap_or(1);
    let elapsed =
        u128::from(frame) * NANOS_PER_SECOND_TIMES_PPM_BASE / ticks_per_million_seconds.max(1);
    u64::try_from(u128::from(start_host_time_ns) + elapsed).unwrap_or(u64::MAX)
}

impl SyntheticSource {
    /// Creates an idle source with its block buffer allocated.
    #[must_use]
    pub fn new(config: SyntheticConfig) -> Self {
        let scratch =
            Vec::with_capacity(config.block_frames as usize * config.format.bytes_per_frame());
        Self {
            config,
            producer: None,
            events: None,
            state: State::Idle,
            next_block: 0,
            next_frame: 0,
            scratch,
        }
    }

    /// Generates up to `blocks` blocks on the calling thread and returns how many were
    /// produced. Emission ends early at a scripted disconnect.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::SourceUnavailable`] when the source is not running.
    pub fn pump(&mut self, blocks: u32) -> Result<u32, CaptureError> {
        if self.state != State::Running {
            return Err(CaptureError::SourceUnavailable);
        }
        let mut produced = 0;
        for _ in 0..blocks {
            let scripted = self.scripted_event(self.next_block);
            let mut flags = BlockFlags::default();
            match scripted {
                Some(ScriptedEvent::Disconnect) => {
                    // Stands in for a device-notification thread, which is not real-time.
                    self.send(CaptureEvent::Disconnected {
                        source_id: self.config.source_id,
                        at_frame: Some(self.next_frame),
                    });
                    self.state = State::Disconnected;
                    break;
                }
                Some(ScriptedEvent::Discontinuity { lost_frames }) => {
                    // Losses seen on the callback path travel in the block (flag and
                    // start-frame gap), not through the event channel, which may allocate.
                    self.next_frame += lost_frames;
                    flags.discontinuity = true;
                }
                Some(ScriptedEvent::InvalidTimestamp) => flags.timestamp_invalid = true,
                Some(ScriptedEvent::SilentBlock) | None => {}
            }
            let header = BlockHeader {
                source_id: self.config.source_id,
                sequence: self.next_block,
                start_frame: self.next_frame,
                frame_count: self.config.block_frames,
                format: self.config.format,
                first_frame_host_time_ns: (!flags.timestamp_invalid).then(|| {
                    host_time_ns(
                        self.config.start_host_time_ns,
                        self.next_frame,
                        self.config.format.sample_rate_hz(),
                        self.config.drift_ppm,
                    )
                }),
                flags,
            };
            if scripted == Some(ScriptedEvent::SilentBlock) {
                if let Some(producer) = self.producer.as_mut() {
                    producer.push_silence(header);
                }
            } else {
                self.render(header.start_frame);
                if let Some(producer) = self.producer.as_mut() {
                    producer.push(header, &self.scratch);
                }
            }
            self.next_frame += u64::from(self.config.block_frames);
            self.next_block += 1;
            produced += 1;
        }
        Ok(produced)
    }

    fn scripted_event(&self, block: u64) -> Option<ScriptedEvent> {
        self.config
            .script
            .iter()
            .find(|(index, _)| *index == block)
            .map(|(_, event)| *event)
    }

    fn send(&self, event: CaptureEvent) {
        if let Some(events) = &self.events {
            // A closed receiver means nobody listens anymore; the event is not needed.
            let _ = events.send(event);
        }
    }

    fn render(&mut self, start_frame: u64) {
        let format = self.config.format;
        let channels = usize::from(format.channel_layout().channel_count());
        let frames = self.config.block_frames as usize;
        self.scratch.clear();
        for index in 0..frames * channels {
            let frame = match format.sample_layout() {
                SampleLayout::Interleaved => index / channels,
                SampleLayout::Planar => index % frames,
            };
            let value = self.sample_at(start_frame + frame as u64);
            encode_sample(value, format.sample_format(), &mut self.scratch);
        }
    }

    fn sample_at(&self, frame: u64) -> f64 {
        match self.config.signal {
            SyntheticSignal::Silence => 0.0,
            SyntheticSignal::Constant(value) => f64::from(value),
            SyntheticSignal::Sine {
                frequency_hz,
                amplitude,
            } => {
                let seconds = frame as f64 / f64::from(self.config.format.sample_rate_hz());
                f64::from(amplitude) * (TAU * f64::from(frequency_hz) * seconds).sin()
            }
        }
    }
}

fn encode_sample(value: f64, sample_format: SampleFormat, out: &mut Vec<u8>) {
    match sample_format {
        SampleFormat::I16 => {
            let scaled = (value * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16;
            out.extend_from_slice(&scaled.to_le_bytes());
        }
        SampleFormat::I24 => {
            let scaled = (value * 8_388_608.0)
                .round()
                .clamp(-8_388_608.0, 8_388_607.0) as i32;
            let [b0, b1, b2, _] = scaled.to_le_bytes();
            out.extend_from_slice(&[b0, b1, b2]);
        }
        SampleFormat::I32 => {
            let scaled = (value * 2_147_483_648.0)
                .round()
                .clamp(-2_147_483_648.0, 2_147_483_647.0) as i32;
            out.extend_from_slice(&scaled.to_le_bytes());
        }
        SampleFormat::F32 => out.extend_from_slice(&(value as f32).to_le_bytes()),
    }
}

impl CaptureSource for SyntheticSource {
    fn source_id(&self) -> SourceId {
        self.config.source_id
    }

    fn prepare(&mut self) -> Result<AudioFormat, CaptureError> {
        Ok(self.config.format)
    }

    fn start(
        &mut self,
        producer: BlockProducer,
        events: Sender<CaptureEvent>,
    ) -> Result<(), CaptureError> {
        if self.state != State::Idle {
            return Err(CaptureError::SourceUnavailable);
        }
        self.producer = Some(producer);
        self.events = Some(events);
        self.state = State::Running;
        Ok(())
    }

    fn stop(&mut self) -> StopSummary {
        let already_stopped = self.state == State::Stopped;
        if !already_stopped {
            self.send(CaptureEvent::Ended {
                source_id: self.config.source_id,
                final_frame: self.next_frame,
            });
            self.state = State::Stopped;
        }
        StopSummary {
            final_frame: self.next_frame,
            already_stopped,
        }
    }
}
