//! Export pipeline for TPT Voice Studio.
//!
//! Renders deliverables from the edit-decision-list and the aligned, edited
//! transcript (spec §11). All text-timeline exports are positioned on the
//! *rendered* output timeline (`tpt-app-voice-studio-edit::timeline`), so
//! subtitles and transcripts stay in sync with the final edited audio
//! without a manual re-timing pass.
//!
//! - Subtitles/captions: SRT/VTT cue generation ([`subtitles`]).
//! - Transcript delivery: plain text, timestamped/speaker-labeled, and
//!   machine-readable JSON ([`transcript`]), with the formal-use disclaimer
//!   on opt-in (spec §17.3).
//! - Clips/highlights: selecting the exact source ranges a renderer must cut
//!   for a rendered-time selection ([`clip`]).
//!
//! Audio export (WAV/MP3/AAC/FLAC) renders these same selections through the
//! codec foundation (`tpt-cadence`) and lands with the Phase 1 codec
//! integration; its selection logic is already defined here.

pub mod clip;
pub mod subtitles;
pub mod transcript;

pub use clip::{select_clip, ClipSelection, ClipWord};
pub use subtitles::{build_cues, render_subtitles, SubtitleCue, SubtitleOptions};
pub use transcript::{
    render_transcript, DisclaimerMode, TranscriptExportOptions, REVIEWED_NOTE,
    TRANSCRIPT_DISCLAIMER,
};
