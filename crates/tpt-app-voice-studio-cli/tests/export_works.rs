//! End-to-end CLI tests: build a project file, run the `tpt-voice-studio`
//! binary, and verify the export output and the exit-code contract (spec §13).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use tpt_app_voice_studio_core::confidence::Confidence;
use tpt_app_voice_studio_core::id::{ProjectId, RecordingId, SegmentId, SpeakerId};
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::project::Project;
use tpt_app_voice_studio_model::recording::{Recording, RecordingFingerprint, TrackRole};
use tpt_app_voice_studio_model::speaker::Speaker;
use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};
use tpt_app_voice_studio_model::ProjectFile;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tvs-cli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Two-speaker project with one applied edit (a filler deletion), so the
/// rendered timeline differs from the source timeline.
fn write_project_file(path: &Path) {
    let project = Project {
        id: ProjectId::new(42),
        name: "episode-042".to_string(),
        recordings: vec![RecordingId::new(1)],
        recording_registry: vec![Recording {
            id: RecordingId::new(1),
            path: PathBuf::from("episode-042.wav"),
            fingerprint: RecordingFingerprint(7),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(3_600),
            sample_rate: 48_000,
            channels: 2,
        }],
        transcript: Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![
                    word("um", 0, 300),
                    word("Welcome", 300, 1_000),
                    word("back", 1_000, 2_000),
                ],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![word("Thanks", 20_000, 21_000)],
            },
        ]),
        edit_history: vec![EditOperation::DeleteRange {
            segment: SegmentId::new(1),
            word_range: 0..1,
        }],
        speakers: vec![
            Speaker::new(SpeakerId::new(1), "Host"),
            Speaker::new(SpeakerId::new(2), "Guest"),
        ],
    };
    ProjectFile::new(project).save(path).expect("save project");
}

fn word(text: &str, start_ms: u64, end_ms: u64) -> Word {
    Word {
        text: text.to_string(),
        start: Duration::from_millis(start_ms),
        end: Duration::from_millis(end_ms),
        confidence: Confidence::new(0.91).expect("valid"),
        source_recording: RecordingId::new(1),
    }
}

fn run_cli(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tpt-voice-studio"))
        .args(args)
        .output()
        .expect("binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status.code().unwrap_or(-1), stdout)
}

#[test]
fn export_srt_from_project_file() {
    let dir = temp_dir("srt");
    let project_path = dir.join("episode-042.tptproj");
    let output_path = dir.join("episode-042.srt");
    write_project_file(&project_path);

    let (code, stdout) = run_cli(&[
        "--json",
        "export",
        "--project",
        project_path.to_str().expect("utf-8"),
        "--format",
        "srt",
        "--output",
        output_path.to_str().expect("utf-8"),
    ]);
    assert_eq!(code, 0, "stdout: {stdout}");

    let result: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json result");
    assert_eq!(result["status"], "success");
    assert_eq!(result["project"], "episode-042");
    assert_eq!(result["words"], 3);

    let srt = std::fs::read_to_string(&output_path).expect("srt written");
    // The filler was deleted by the saved history, so cue 1 starts at
    // "Welcome" on the rendered timeline, labeled with the speaker.
    assert!(
        srt.contains("1\n00:00:00,000 --> 00:00:01,700\nHost: Welcome back"),
        "{srt}"
    );
    assert!(
        srt.contains("2\n00:00:01,700 --> 00:00:02,700\nGuest: Thanks"),
        "{srt}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_transcript_with_disclaimer() {
    let dir = temp_dir("transcript");
    let project_path = dir.join("episode-042.tptproj");
    let output_path = dir.join("episode-042.txt");
    write_project_file(&project_path);

    let (code, _) = run_cli(&[
        "export",
        "--project",
        project_path.to_str().expect("utf-8"),
        "--format",
        "transcript",
        "--output",
        output_path.to_str().expect("utf-8"),
        "--disclaimer",
        "unverified",
    ]);
    assert_eq!(code, 0);

    let text = std::fs::read_to_string(&output_path).expect("transcript written");
    assert!(text.starts_with("[00:00:00] Host: Welcome back\n[00:00:01] Guest: Thanks"));
    assert!(text.contains("automated speech recognition"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_project_file_is_an_input_error() {
    let dir = temp_dir("missing");
    let missing = dir.join("nope.tptproj");
    let output_path = dir.join("out.srt");

    let (code, stdout) = run_cli(&[
        "--json",
        "export",
        "--project",
        missing.to_str().expect("utf-8"),
        "--format",
        "srt",
        "--output",
        output_path.to_str().expect("utf-8"),
    ]);
    assert_eq!(code, 6, "exit code must be INPUT_ERROR");
    let result: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json result");
    assert_eq!(result["status"], "error");
    assert_eq!(result["code"], 6);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn audio_format_reports_the_missing_codec_engine() {
    let dir = temp_dir("audio");
    let project_path = dir.join("episode-042.tptproj");
    let output_path = dir.join("episode-042.wav");
    write_project_file(&project_path);

    let (code, stdout) = run_cli(&[
        "--json",
        "export",
        "--project",
        project_path.to_str().expect("utf-8"),
        "--format",
        "audio",
        "--output",
        output_path.to_str().expect("utf-8"),
    ]);
    assert_eq!(code, 4, "exit code must be EXPORT_FAILED");
    let result: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json result");
    assert_eq!(result["code"], 4);
    assert!(result["message"]
        .as_str()
        .expect("message")
        .contains("tpt-cadence"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn transcribe_reports_the_missing_speech_engine() {
    let dir = temp_dir("transcribe");
    let input = dir.join("episode.wav");
    std::fs::write(&input, b"not really audio").expect("write input");
    let output = dir.join("episode.json");

    let (code, stdout) = run_cli(&[
        "--json",
        "transcribe",
        "--input",
        input.to_str().expect("utf-8"),
        "--output",
        output.to_str().expect("utf-8"),
    ]);
    assert_eq!(code, 2, "exit code must be TRANSCRIPTION_FAILED");
    let result: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json result");
    assert_eq!(result["status"], "error");
    assert_eq!(result["code"], 2);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unapplicable_history_is_an_input_error() {
    let dir = temp_dir("badhistory");
    let project_path = dir.join("broken.tptproj");
    let output_path = dir.join("out.srt");
    write_project_file(&project_path);

    // Corrupt the saved history: delete words beyond the segment length.
    let mut file = ProjectFile::load(&project_path).expect("loads");
    file.project.edit_history = vec![EditOperation::DeleteRange {
        segment: SegmentId::new(1),
        word_range: 0..99,
    }];
    file.save(&project_path).expect("saves");

    let (code, _) = run_cli(&[
        "--json",
        "export",
        "--project",
        project_path.to_str().expect("utf-8"),
        "--format",
        "srt",
        "--output",
        output_path.to_str().expect("utf-8"),
    ]);
    assert_eq!(code, 6);

    std::fs::remove_dir_all(&dir).ok();
}
