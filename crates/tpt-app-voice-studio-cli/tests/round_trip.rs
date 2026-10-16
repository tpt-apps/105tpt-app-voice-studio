//! End-to-end round trip (spec §18.3): edit → EDL → persist → reload → export.
//!
//! Builds a project with a recorded edit, saves it as a `.tptproj`, drives the
//! real `tpt-voice-studio export` binary against it, and checks that the
//! rendered output reflects the edit, is deterministic, and that the saved
//! project reloads without losing data.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use tpt_app_voice_studio_core::confidence::Confidence;
use tpt_app_voice_studio_core::id::{ProjectId, RecordingId, SegmentId};
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::project::Project;
use tpt_app_voice_studio_model::project_file::ProjectFile;
use tpt_app_voice_studio_model::recording::{Recording, RecordingFingerprint, TrackRole};
use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

fn word(text: &str, start_ms: u64, end_ms: u64) -> Word {
    Word {
        text: text.to_string(),
        start: Duration::from_millis(start_ms),
        end: Duration::from_millis(end_ms),
        confidence: Confidence::new(0.9).expect("valid confidence"),
        source_recording: RecordingId::new(1),
    }
}

/// Three words, one segment. The middle filler "um" is deleted by the EDL.
fn project_with_deleted_filler() -> Project {
    Project {
        id: ProjectId::new(7),
        name: "round-trip".to_string(),
        recordings: vec![RecordingId::new(1)],
        recording_registry: vec![Recording {
            id: RecordingId::new(1),
            path: PathBuf::from("round-trip.wav"),
            fingerprint: RecordingFingerprint(0x7),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(10),
            sample_rate: 16_000,
            channels: 1,
        }],
        transcript: Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: None,
            words: vec![
                word("keep", 0, 1_000),
                word("um", 1_000, 2_000),
                word("go", 2_000, 3_000),
            ],
        }]),
        edit_history: vec![EditOperation::DeleteRange {
            segment: SegmentId::new(1),
            word_range: 1..2,
        }],
        speakers: Vec::new(),
    }
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tvs-roundtrip-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_srt_export(project: &Path, output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_tpt-voice-studio"))
        .args(["export", "--project"])
        .arg(project)
        .args(["--format", "srt", "--output"])
        .arg(output)
        .status()
        .expect("runs export");
    assert!(status.success(), "export exited with {status}");
}

#[test]
fn edit_persist_reload_export_round_trip() {
    let dir = scratch_dir("main");
    let project_path = dir.join("episode.tptproj");
    let project = project_with_deleted_filler();
    let original_transcript = project.transcript.clone();

    ProjectFile::new(project)
        .save(&project_path)
        .expect("save project");

    // Export renders the saved edit: "um" must be gone, the rest kept.
    let first = dir.join("first.srt");
    run_srt_export(&project_path, &first);
    let rendered = std::fs::read_to_string(&first).expect("read export");
    assert!(rendered.contains("keep"), "kept word missing:\n{rendered}");
    assert!(rendered.contains("go"), "kept word missing:\n{rendered}");
    assert!(
        !rendered.contains("um"),
        "deleted filler leaked:\n{rendered}"
    );

    // Rendering the same project twice is byte-for-byte identical.
    let second = dir.join("second.srt");
    run_srt_export(&project_path, &second);
    assert_eq!(
        std::fs::read(&first).expect("first"),
        std::fs::read(&second).expect("second"),
        "export is not deterministic"
    );

    // Persistence is lossless: the reloaded project keeps the original
    // transcript and the recorded edit history.
    let reloaded = ProjectFile::load(&project_path).expect("reload").project;
    assert_eq!(reloaded.transcript, original_transcript);
    assert_eq!(reloaded.edit_history.len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}
