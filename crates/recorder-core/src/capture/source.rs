//! The contract every capture backend implements (architecture section 2).

use std::sync::mpsc::Sender;

use crate::audio::format::AudioFormat;
use crate::capture::block::SourceId;
use crate::capture::event::{CaptureError, CaptureEvent};
use crate::capture::queue::BlockProducer;

/// Result of stopping a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopSummary {
    /// Source frame index where capture ended.
    pub final_frame: u64,
    /// True when the source had already been stopped; nothing was done.
    pub already_stopped: bool,
}

/// A source of PCM blocks: an OS capture backend or the synthetic test source.
pub trait CaptureSource {
    /// Session-scoped identifier of the source.
    fn source_id(&self) -> SourceId;

    /// Resolves permissions, identity, and the negotiated format.
    ///
    /// # Errors
    ///
    /// Returns a [`CaptureError`] when the source cannot be opened.
    fn prepare(&mut self) -> Result<AudioFormat, CaptureError>;

    /// Starts delivering blocks to `producer` and session events to `events`.
    ///
    /// # Errors
    ///
    /// Returns a [`CaptureError`] when capture cannot start.
    fn start(
        &mut self,
        producer: BlockProducer,
        events: Sender<CaptureEvent>,
    ) -> Result<(), CaptureError>;

    /// Stops delivery and reports where capture ended. Idempotent: only the first call
    /// sends [`CaptureEvent::Ended`].
    fn stop(&mut self) -> StopSummary;
}
