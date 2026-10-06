//! Project file persistence: the `.tptproj` envelope referenced by the CLI
//! (spec §13) and the desktop app.
//!
//! A project file is JSON: a small versioned envelope around the full
//! [`Project`] (recordings metadata, transcript, speakers, and the
//! edit-decision-list — spec §6, §15). Original audio is never embedded;
//! recordings are stored as paths plus fingerprints (spec §15).
//!
//! SQLite storage of the *library* state (project list, preferences, job
//! metadata) is a separate persistence layer (spec §15) and lands with the
//! foundation integration; the project file is the portable unit of work.

use std::path::Path;

use crate::project::Project;

/// Current [`ProjectFile::format_version`]. Bump on breaking changes and
/// provide a migration path before writing the new version.
pub const PROJECT_FILE_VERSION: u32 = 1;

/// Versioned serialization envelope for a project.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    /// Envelope version, must equal [`PROJECT_FILE_VERSION`] when loading.
    pub format_version: u32,
    /// The project payload.
    pub project: Project,
}

impl ProjectFile {
    /// Wraps a project in a current-version envelope.
    #[must_use]
    pub fn new(project: Project) -> Self {
        Self {
            format_version: PROJECT_FILE_VERSION,
            project,
        }
    }

    /// Serializes to pretty-printed JSON.
    ///
    /// # Errors
    /// Returns [`ProjectFileError::Json`] if serialization fails (should not
    /// happen for valid projects).
    pub fn to_json(&self) -> Result<String, ProjectFileError> {
        serde_json::to_string_pretty(self).map_err(ProjectFileError::Json)
    }

    /// Deserializes from JSON, validating the envelope version.
    ///
    /// # Errors
    /// Returns [`ProjectFileError::Json`] for malformed JSON and
    /// [`ProjectFileError::UnsupportedVersion`] for envelopes written by a
    /// different version.
    pub fn from_json(json: &str) -> Result<Self, ProjectFileError> {
        let file: Self = serde_json::from_str(json)?;
        if file.format_version != PROJECT_FILE_VERSION {
            return Err(ProjectFileError::UnsupportedVersion {
                found: file.format_version,
                expected: PROJECT_FILE_VERSION,
            });
        }
        Ok(file)
    }

    /// Writes to `path` (typically with a `.tptproj` extension).
    ///
    /// # Errors
    /// See [`ProjectFile::to_json`]; I/O failures map to
    /// [`ProjectFileError::Io`].
    pub fn save(&self, path: &Path) -> Result<(), ProjectFileError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads and validates from `path`.
    ///
    /// # Errors
    /// See [`ProjectFile::from_json`]; I/O failures map to
    /// [`ProjectFileError::Io`].
    pub fn load(path: &Path) -> Result<Self, ProjectFileError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

/// Project file persistence failures.
#[derive(Debug, thiserror::Error)]
pub enum ProjectFileError {
    /// The file is not valid JSON or does not match the schema.
    #[error("invalid project file: {0}")]
    Json(#[from] serde_json::Error),
    /// The envelope was written by a different format version.
    #[error("unsupported project file version {found} (expected {expected})")]
    UnsupportedVersion {
        /// The version found in the file.
        found: u32,
        /// The version this build understands.
        expected: u32,
    },
    /// The file could not be read or written.
    #[error("project file I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{ProjectId, RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_core::time::TimeRange;

    use crate::edit_operation::EditOperation;
    use crate::project::Project;
    use crate::recording::{Recording, RecordingFingerprint, TrackRole};
    use crate::speaker::Speaker;
    use crate::transcript::{Segment, Transcript, Word};

    use super::*;

    fn sample_project() -> Project {
        Project {
            id: ProjectId::new(42),
            name: "episode-042".to_string(),
            recordings: vec![RecordingId::new(1)],
            recording_registry: vec![Recording {
                id: RecordingId::new(1),
                path: PathBuf::from("episode-042.wav"),
                fingerprint: RecordingFingerprint(0xABCD),
                track_role: TrackRole::SingleSpeakerIsolated(SpeakerId::new(1)),
                duration: Duration::from_secs(3_600),
                sample_rate: 48_000,
                channels: 1,
            }],
            transcript: Transcript::from_segments(vec![Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![Word {
                    text: "um".to_string(),
                    start: Duration::from_millis(100),
                    end: Duration::from_millis(400),
                    confidence: Confidence::new(0.42).expect("valid"),
                    source_recording: RecordingId::new(1),
                }],
            }]),
            edit_history: vec![EditOperation::Mute {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_secs(1), Duration::from_secs(2))
                    .expect("valid"),
            }],
            speakers: vec![Speaker::new(SpeakerId::new(1), "Host")],
        }
    }

    #[test]
    fn round_trips_through_json_losing_nothing() {
        let file = ProjectFile::new(sample_project());
        let json = file.to_json().expect("serializes");
        let loaded = ProjectFile::from_json(&json).expect("deserializes");
        assert_eq!(loaded.format_version, PROJECT_FILE_VERSION);
        assert_eq!(loaded.project, file.project);
    }

    #[test]
    fn rejects_unknown_envelope_versions() {
        let mut file = ProjectFile::new(sample_project());
        file.format_version = 99;
        let json = file.to_json().expect("serializes");
        let err = ProjectFile::from_json(&json).expect_err("version mismatch");
        assert!(matches!(
            err,
            ProjectFileError::UnsupportedVersion {
                found: 99,
                expected: PROJECT_FILE_VERSION
            }
        ));
    }

    #[test]
    fn rejects_malformed_json() {
        let err = ProjectFile::from_json("{ not json").expect_err("malformed");
        assert!(matches!(err, ProjectFileError::Json(_)));
    }

    #[test]
    fn confidence_survives_a_cycle_as_a_number() {
        let file = ProjectFile::new(sample_project());
        let json = file.to_json().expect("serializes");
        assert!(json.contains("\"confidence\": 0.42"));
    }

    #[test]
    fn saves_and_loads_from_disk() {
        let dir = std::env::temp_dir().join(format!("tvs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("episode-042.tptproj");

        let file = ProjectFile::new(sample_project());
        file.save(&path).expect("saves");
        let loaded = ProjectFile::load(&path).expect("loads");
        assert_eq!(loaded.project, file.project);

        std::fs::remove_file(&path).ok();
    }
}
