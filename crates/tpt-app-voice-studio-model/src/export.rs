//! Export jobs and formats (spec §6.6, §11).

use tpt_app_voice_studio_core::id::{ExportJobId, ProjectId};
use tpt_app_voice_studio_core::time::TimeRange;

/// Audio codecs available for export (spec §11, via `tpt-cadence`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum AudioCodec {
    /// Uncompressed PCM WAV.
    Wav,
    /// MP3 (lossy).
    Mp3,
    /// AAC (lossy).
    Aac,
    /// FLAC (lossless).
    Flac,
}

/// Subtitle/caption formats generated from the aligned, edited transcript
/// (spec §11).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum SubtitleFormat {
    /// SubRip `.srt`.
    Srt,
    /// WebVTT `.vtt`.
    Vtt,
}

/// Transcript delivery formats (spec §11, §13).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum TranscriptFormat {
    /// Plain prose paragraphs, suitable for reading.
    PlainText,
    /// Timestamped and speaker-labeled, suitable for client delivery or
    /// legal/corporate record-keeping (spec §11).
    Timestamped,
    /// Machine-readable JSON, matching the CLI's result format (spec §13).
    Json,
}

/// What kind of deliverable an export job produces (spec §6.6, §11).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum ExportFormat {
    /// Audio rendered from the edit-decision-list.
    Audio(AudioCodec),
    /// Subtitles generated from the edited transcript.
    Subtitles(SubtitleFormat),
    /// Transcript in a delivery format.
    Transcript(TranscriptFormat),
}

/// Lifecycle of an export job (spec §6.6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum ExportStatus {
    /// Accepted, waiting to render.
    Queued,
    /// Currently rendering.
    Running,
    /// Rendered successfully.
    Completed,
    /// Rendering failed.
    Failed,
}

/// One export request for a project (spec §6.6).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportJob {
    /// Unique id of the job.
    pub id: ExportJobId,
    /// The project being exported.
    pub project: ProjectId,
    /// The deliverable format.
    pub format: ExportFormat,
    /// Optional time-range restriction (e.g. clip/highlight export, spec §11);
    /// `None` exports the whole edit.
    pub range: Option<TimeRange>,
    /// Current job status.
    pub status: ExportStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_fields_round_trip_equality() {
        let job = ExportJob {
            id: ExportJobId::new(1),
            project: ProjectId::new(2),
            format: ExportFormat::Subtitles(SubtitleFormat::Vtt),
            range: None,
            status: ExportStatus::Queued,
        };
        let other = job.clone();
        assert_eq!(job, other);
        assert_ne!(job.format, ExportFormat::Audio(AudioCodec::Flac));
    }
}
