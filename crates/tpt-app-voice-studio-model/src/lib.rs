//! Domain model for TPT Voice Studio.
//!
//! Defines the persistent, engine-level representation of a Voice Studio
//! project per spec §6:
//!
//! - `Project` and `Recording` (§6.1–6.2), including multi-track multi-mic
//!   sessions via `TrackRole`.
//! - `Transcript`, `Segment`, `Word` (§6.3), where every word carries a
//!   timestamp range, a confidence score, and the source recording it was
//!   aligned against — the binding that makes "edit the transcript, edit the
//!   audio" possible (spec §3.2).
//! - `Speaker` (§6.4) with rename/merge/split support (spec §9).
//! - `EditOperation` (§6.5): the reversible, ordered edit-decision-list
//!   entries (`DeleteRange`, `Reorder`, `Trim`, `Mute`, `SelectTake`).
//! - `ExportJob` and `ExportFormat` (§6.6).
//!
//! The model is plain data plus invariants and query helpers. It performs no
//! I/O, depends on no codecs, and links against no speech engine; the
//! pipeline crates own behaviour (spec §3.6).
//!
//! Durations are `std::time::Duration` measured on the recording's own
//! timeline; ranges are half-open, per [`tpt_app_voice_studio_core::time`].

pub mod edit_operation;
pub mod export;
pub mod project;
pub mod project_file;
pub mod recording;
pub mod speaker;
pub mod transcript;

pub use edit_operation::EditOperation;
pub use export::{
    AudioCodec, ExportFormat, ExportJob, ExportStatus, SubtitleFormat, TranscriptFormat,
};
pub use project::Project;
pub use project_file::{ProjectFile, ProjectFileError, PROJECT_FILE_VERSION};
pub use recording::{Recording, RecordingFingerprint, TrackRole};
pub use speaker::Speaker;
pub use transcript::{Segment, Transcript, Word};
