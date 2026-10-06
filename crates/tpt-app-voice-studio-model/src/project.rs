//! Projects: the top-level container tying recordings, transcript, speakers,
//! and edit history together (spec §6.1).

use tpt_app_voice_studio_core::id::{ProjectId, RecordingId, SegmentId, SpeakerId};

use crate::edit_operation::EditOperation;
use crate::recording::Recording;
use crate::speaker::Speaker;
use crate::transcript::{Segment, Transcript};

/// A Voice Studio project (spec §6.1).
///
/// Two parallel views of recordings are kept, mirroring spec §6.1 and §15:
///
/// - `recordings` — the ordered id list defining membership and playback
///   order of the session's tracks.
/// - `recording_registry` — the full [`Recording`] metadata (paths,
///   fingerprints, roles, timing) persisted alongside the project; original
///   audio itself is never stored or modified (spec §3.3, §15).
///
/// `edit_history` is the ordered, reversible edit-decision-list, applied
/// against source audio only at playback and export time (spec §3.3).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Project {
    /// Unique id of the project.
    pub id: ProjectId,
    /// Human-editable project name.
    pub name: String,
    /// Ordered ids of the project's recordings (one or more mic tracks).
    pub recordings: Vec<RecordingId>,
    /// Full metadata for every recording referenced by `recordings`.
    pub recording_registry: Vec<Recording>,
    /// The working transcript; the edit engine retains the original aligned
    /// transcript so every edit stays reversible.
    pub transcript: Transcript,
    /// The edit-decision-list: appended in order, applied at playback/export.
    pub edit_history: Vec<EditOperation>,
    /// The project's speaker registry.
    pub speakers: Vec<Speaker>,
}

impl Project {
    /// Looks up full recording metadata by id.
    #[must_use]
    pub fn recording(&self, id: RecordingId) -> Option<&Recording> {
        self.recording_registry.iter().find(|r| r.id == id)
    }

    /// The speaker with the given id, if present in the registry.
    #[must_use]
    pub fn speaker(&self, id: SpeakerId) -> Option<&Speaker> {
        self.speakers.iter().find(|s| s.id == id)
    }

    /// The segment with the given id, if present in the transcript.
    #[must_use]
    pub fn segment(&self, id: SegmentId) -> Option<&Segment> {
        self.transcript.segment(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::{RecordingFingerprint, TrackRole};
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn looks_up_recordings_speakers_and_segments() {
        let host = Recording {
            id: RecordingId::new(1),
            path: PathBuf::from("host.wav"),
            fingerprint: RecordingFingerprint(1),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(60),
            sample_rate: 48_000,
            channels: 2,
        };
        let project = Project {
            id: ProjectId::new(1),
            name: "episode-042".to_string(),
            recordings: vec![RecordingId::new(1)],
            recording_registry: vec![host],
            transcript: Transcript::from_segments(vec![Segment {
                id: SegmentId::new(10),
                speaker: Some(SpeakerId::new(1)),
                words: Vec::new(),
            }]),
            edit_history: Vec::new(),
            speakers: vec![Speaker::new(SpeakerId::new(1), "Host")],
        };

        assert_eq!(
            project.recording(RecordingId::new(1)).expect("exists").path,
            PathBuf::from("host.wav")
        );
        assert!(project.recording(RecordingId::new(2)).is_none());
        assert_eq!(
            project.speaker(SpeakerId::new(1)).expect("exists").label,
            "Host"
        );
        assert!(project.speaker(SpeakerId::new(9)).is_none());
        assert!(project.segment(SegmentId::new(10)).is_some());
    }
}
