//! Speakers: the participant registry of a project (spec §6.4).
//!
//! Diarisation assigns labels automatically; the editor can rename, merge,
//! and split speakers when diarisation is imperfect (spec §9). The model
//! stores the registry; merge/split mechanics land with the speaker-handling
//! work (todo.md, "Speaker & Multi-Track Handling").

use std::collections::BTreeSet;

use tpt_app_voice_studio_core::id::{RecordingId, SpeakerId};

/// A participant of a session and the recordings they appear in (spec §6.4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Speaker {
    /// Unique id of this speaker within the project.
    pub id: SpeakerId,
    /// Human-editable display label (diarisation defaults to e.g. "Speaker 1").
    pub label: String,
    /// Recordings this speaker was detected (or assigned) in.
    pub recordings: BTreeSet<RecordingId>,
}

impl Speaker {
    /// Creates a speaker with an empty recording set.
    #[must_use]
    pub fn new(id: SpeakerId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            recordings: BTreeSet::new(),
        }
    }

    /// Renames the speaker (spec §9).
    pub fn rename(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_updates_label() {
        let mut speaker = Speaker::new(SpeakerId::new(1), "Speaker 1");
        assert_eq!(speaker.label, "Speaker 1");
        speaker.rename("Host");
        assert_eq!(speaker.label, "Host");
    }

    #[test]
    fn recording_membership_is_tracked() {
        let mut speaker = Speaker::new(SpeakerId::new(1), "Guest");
        speaker.recordings.insert(RecordingId::new(2));
        assert!(speaker.recordings.contains(&RecordingId::new(2)));
        assert!(!speaker.recordings.contains(&RecordingId::new(1)));
    }
}
