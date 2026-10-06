//! Recordings: the immutable source-audio entries of a project (spec §6.2).
//!
//! A project may hold multiple recordings representing separate microphone
//! tracks of the same session (multi-track podcast/interview recording).
//! Recordings are referenced, never modified: all editing happens in the
//! edit-decision-list (spec §3.3, §15).

use std::path::PathBuf;
use std::time::Duration;

use tpt_app_voice_studio_core::id::{RecordingId, SpeakerId};

/// A content fingerprint of a recording (e.g. a hash of the decoded or raw
/// bytes), used to detect moved/renamed/changed files without storing audio
/// in the project database (spec §15).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct RecordingFingerprint(pub u64);

/// The role a recording plays in a project (spec §6.2).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum TrackRole {
    /// A recording of the full session mix (all speakers).
    Mixed,
    /// A per-participant microphone track assigned to one speaker.
    SingleSpeakerIsolated(SpeakerId),
    /// A supporting recording (e.g. room tone, backup feed) not part of the
    /// primary edit.
    Reference,
}

/// A source recording of a project (spec §6.2).
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct Recording {
    /// Unique id of this recording within the project.
    pub id: RecordingId,
    /// Filesystem path of the (never-modified) source file.
    pub path: PathBuf,
    /// Content fingerprint for moved/changed-file detection.
    pub fingerprint: RecordingFingerprint,
    /// The role this track plays (mixed / per-speaker / reference).
    pub track_role: TrackRole,
    /// Total duration of the recording.
    pub duration: Duration,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
}

impl Recording {
    /// True if this recording's time is part of the primary edit (i.e. not a
    /// [`TrackRole::Reference`] track).
    #[must_use]
    pub fn is_editable(&self) -> bool {
        !matches!(self.track_role, TrackRole::Reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording(track_role: TrackRole) -> Recording {
        Recording {
            id: RecordingId::new(1),
            path: PathBuf::from("episode-042-host.wav"),
            fingerprint: RecordingFingerprint(0xDEAD_BEEF),
            track_role,
            duration: Duration::from_secs(3600),
            sample_rate: 48_000,
            channels: 1,
        }
    }

    #[test]
    fn reference_tracks_are_not_editable() {
        let host = recording(TrackRole::SingleSpeakerIsolated(SpeakerId::new(1)));
        let mixed = recording(TrackRole::Mixed);
        let room_tone = recording(TrackRole::Reference);
        assert!(host.is_editable());
        assert!(mixed.is_editable());
        assert!(!room_tone.is_editable());
    }
}
