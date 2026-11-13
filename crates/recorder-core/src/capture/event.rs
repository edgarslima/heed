//! Session events raised by capture sources outside the PCM path.

use crate::audio::format::AudioFormat;
use crate::capture::block::SourceId;

/// Capture failures, with the stable codes of architecture section 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureError {
    /// The OS denied or restricted access to the source.
    PermissionDenied,
    /// The device, endpoint, or application is not available.
    SourceUnavailable,
    /// The negotiated format cannot be captured.
    FormatUnsupported,
    /// Capture stopped unexpectedly.
    CaptureInterrupted,
}

impl CaptureError {
    /// Stable code used in IPC errors and logs.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PermissionDenied => "permissionDenied",
            Self::SourceUnavailable => "sourceUnavailable",
            Self::FormatUnsupported => "formatUnsupported",
            Self::CaptureInterrupted => "captureInterrupted",
        }
    }
}

/// An event a source reports to the session controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureEvent {
    /// The source disappeared, at the given source frame when observable.
    Disconnected {
        source_id: SourceId,
        at_frame: Option<u64>,
    },
    /// The device changed its format mid-session.
    FormatChanged {
        source_id: SourceId,
        format: AudioFormat,
    },
    /// Frames were lost at `at_frame`; the count is known only for some backends.
    Discontinuity {
        source_id: SourceId,
        at_frame: u64,
        lost_frames: Option<u64>,
    },
    /// Capture failed.
    Failed {
        source_id: SourceId,
        error: CaptureError,
    },
    /// Capture ended after a stop request, at `final_frame`.
    Ended {
        source_id: SourceId,
        final_frame: u64,
    },
}

impl CaptureEvent {
    /// Source that raised the event.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        match *self {
            Self::Disconnected { source_id, .. }
            | Self::FormatChanged { source_id, .. }
            | Self::Discontinuity { source_id, .. }
            | Self::Failed { source_id, .. }
            | Self::Ended { source_id, .. } => source_id,
        }
    }
}
