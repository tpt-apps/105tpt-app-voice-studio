//! `tpt-voice-studio` — the TPT Voice Studio CLI.
//!
//! A first-class automation interface for studio-scale batch processing
//! (spec §13):
//!
//! - `transcribe` — transcribe a single recording to a transcript file.
//! - `batch-transcribe` — transcribe every recording in a directory.
//! - `export` — render SRT/VTT subtitles or a transcript (text/JSON) from a
//!   `.tptproj` project file; audio formats arrive with the codec
//!   integration.
//!
//! Results are machine-readable with `--json` and the exit-code contract is
//! stable (spec §13):
//!
//! - `0` — `SUCCESS`: operation completed fully.
//! - `1` — `PARTIAL_SUCCESS`: completed with per-item failures (batch modes).
//! - `2` — `TRANSCRIPTION_FAILED`.
//! - `3` — `ALIGNMENT_FAILED`.
//! - `4` — `EXPORT_FAILED`.
//! - `5` — `CONFIGURATION_ERROR`.
//! - `6` — `INPUT_ERROR`.
//! - `7` — `INTERNAL_ERROR`.
//!
//! The codes are exposed as constants in [`exit_code`]; the contract must
//! remain stable. The CLI drives the same engine crates as the desktop
//! application; it must never grow a second implementation of any pipeline
//! stage (spec §3.6).
//!
//! Status: `export` is end-to-end functional for text formats. `transcribe`
//! and `batch-transcribe` report `TRANSCRIPTION_FAILED` with an explicit
//! message until the `tpt-voice` speech engine is resolvable (dependency
//! risk, spec §5.1).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use tpt_app_voice_studio_edit::EditSession;
use tpt_app_voice_studio_export::{
    render_subtitles, render_transcript, DisclaimerMode, SubtitleOptions, TranscriptExportOptions,
};
use tpt_app_voice_studio_model::export::{SubtitleFormat, TranscriptFormat};
use tpt_app_voice_studio_model::ProjectFile;

/// Stable CLI exit codes (spec §13). Public so the desktop shell and tests
/// can reference the contract instead of hard-coding integers.
pub mod exit_code {
    /// Operation completed fully.
    pub const SUCCESS: u8 = 0;
    /// Operation completed with per-item failures (batch modes only).
    pub const PARTIAL_SUCCESS: u8 = 1;
    /// Transcription could not be completed.
    pub const TRANSCRIPTION_FAILED: u8 = 2;
    /// Forced alignment could not be completed.
    pub const ALIGNMENT_FAILED: u8 = 3;
    /// Export rendering failed.
    pub const EXPORT_FAILED: u8 = 4;
    /// Invalid or conflicting command-line configuration.
    pub const CONFIGURATION_ERROR: u8 = 5;
    /// Missing/unreadable/unsupported input file.
    pub const INPUT_ERROR: u8 = 6;
    /// Unexpected internal failure.
    pub const INTERNAL_ERROR: u8 = 7;
}

#[derive(Parser)]
#[command(
    name = "tpt-voice-studio",
    version,
    about = "TPT Voice Studio — transcript-driven audio editing"
)]
struct Cli {
    /// Emit a machine-readable JSON result instead of human-readable text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transcribe a single recording (requires the speech engine).
    Transcribe {
        /// Recording to transcribe.
        #[arg(long)]
        input: PathBuf,
        /// Transcript output path (JSON).
        #[arg(long)]
        output: PathBuf,
    },
    /// Transcribe every recording in a directory (requires the speech engine).
    BatchTranscribe {
        /// Directory of recordings.
        #[arg(long)]
        input: PathBuf,
        /// Directory for transcript output.
        #[arg(long)]
        output: PathBuf,
    },
    /// Render subtitles or a transcript from a project file.
    Export {
        /// Project file (`.tptproj`).
        #[arg(long)]
        project: PathBuf,
        /// Output format.
        #[arg(long)]
        format: ExportFormatArg,
        /// Output file path.
        #[arg(long)]
        output: PathBuf,
        /// Attach the formal-use disclaimer (spec §17.3).
        #[arg(long, value_enum, default_value_t = DisclaimerArg::None)]
        disclaimer: DisclaimerArg,
        /// Omit speaker labels from the output.
        #[arg(long)]
        no_speaker_labels: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ExportFormatArg {
    /// Audio (WAV/MP3/AAC/FLAC) — requires the codec foundation.
    Audio,
    /// SubRip subtitles.
    Srt,
    /// WebVTT subtitles.
    Vtt,
    /// Timestamped, speaker-labeled transcript.
    Transcript,
    /// Plain-text transcript.
    TranscriptPlain,
    /// Machine-readable JSON transcript.
    TranscriptJson,
}

#[derive(Clone, Copy, ValueEnum)]
enum DisclaimerArg {
    None,
    Unverified,
    Reviewed,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Transcribe { .. } | Commands::BatchTranscribe { .. } => {
            engine_not_available(cli.json)
        }
        Commands::Export {
            project,
            format,
            output,
            disclaimer,
            no_speaker_labels,
        } => run_export(
            &project,
            format,
            &output,
            disclaimer,
            no_speaker_labels,
            cli.json,
        ),
    };
    ExitCode::from(code)
}

/// Reports the not-yet-integrated speech engine with the contract exit code.
///
/// The message is explicit so no user mistakes this for a failed attempt at
/// their file: transcription requires `tpt-voice` (spec §5.1), which is not
/// yet resolvable.
fn engine_not_available(json: bool) -> u8 {
    let message = "transcription is unavailable: the speech engine (tpt-voice) is not \
integrated yet (see docs/architecture.md in the repository)";
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "code": exit_code::TRANSCRIPTION_FAILED,
                "message": message,
            })
        );
    } else {
        eprintln!("error: {message}");
    }
    exit_code::TRANSCRIPTION_FAILED
}

struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn run_export(
    project_path: &std::path::Path,
    format: ExportFormatArg,
    output_path: &std::path::Path,
    disclaimer: DisclaimerArg,
    no_speaker_labels: bool,
    json: bool,
) -> u8 {
    match render_export(
        project_path,
        format,
        output_path,
        disclaimer,
        no_speaker_labels,
    ) {
        Ok(summary) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "command": "export",
                        "project": summary.project_name,
                        "format": summary.format_name,
                        "output": output_path.display().to_string(),
                        "words": summary.words,
                    })
                );
            } else {
                println!(
                    "wrote {} ({}, {} words)",
                    output_path.display(),
                    summary.format_name,
                    summary.words
                );
            }
            exit_code::SUCCESS
        }
        Err(failure) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "export",
                        "code": failure.code,
                        "message": failure.message,
                    })
                );
            } else {
                eprintln!("error: {}", failure.message);
            }
            failure.code
        }
    }
}

struct ExportSummary {
    project_name: String,
    format_name: &'static str,
    words: usize,
}

fn render_export(
    project_path: &std::path::Path,
    format: ExportFormatArg,
    output_path: &std::path::Path,
    disclaimer: DisclaimerArg,
    no_speaker_labels: bool,
) -> Result<ExportSummary, Failure> {
    if format == ExportFormatArg::Audio {
        return Err(Failure::new(
            exit_code::EXPORT_FAILED,
            "audio export is unavailable: the codec foundation (tpt-cadence) is not integrated \
             yet (see docs/architecture.md in the repository)",
        ));
    }

    let file = ProjectFile::load(project_path)
        .map_err(|e| Failure::new(exit_code::INPUT_ERROR, e.to_string()))?;
    let project = &file.project;

    // Reconstruct the edited working transcript from the saved history.
    let session = EditSession::from_history(
        project.recording_registry.clone(),
        project.transcript.clone(),
        project.edit_history.clone(),
    )
    .map_err(|e| {
        Failure::new(
            exit_code::INPUT_ERROR,
            format!("project edit history does not apply: {e}"),
        )
    })?;
    let working = session.working();
    let words = working.word_count();

    let content = match format {
        ExportFormatArg::Audio => unreachable!("rejected above"),
        ExportFormatArg::Srt | ExportFormatArg::Vtt => {
            let options = SubtitleOptions {
                format: if format == ExportFormatArg::Srt {
                    SubtitleFormat::Srt
                } else {
                    SubtitleFormat::Vtt
                },
                include_speaker_labels: !no_speaker_labels,
                ..SubtitleOptions::default()
            };
            render_subtitles(working, &project.speakers, &options)
        }
        ExportFormatArg::Transcript
        | ExportFormatArg::TranscriptPlain
        | ExportFormatArg::TranscriptJson => {
            let options = TranscriptExportOptions {
                format: match format {
                    ExportFormatArg::Transcript => TranscriptFormat::Timestamped,
                    ExportFormatArg::TranscriptPlain => TranscriptFormat::PlainText,
                    _ => TranscriptFormat::Json,
                },
                include_speaker_labels: !no_speaker_labels,
                disclaimer: match disclaimer {
                    DisclaimerArg::None => DisclaimerMode::None,
                    DisclaimerArg::Unverified => DisclaimerMode::Unverified,
                    DisclaimerArg::Reviewed => DisclaimerMode::Reviewed,
                },
            };
            render_transcript(working, &project.speakers, &options)
        }
    };

    std::fs::write(output_path, content)
        .map_err(|e| Failure::new(exit_code::EXPORT_FAILED, format!("write failed: {e}")))?;

    let format_name = match format {
        ExportFormatArg::Audio => unreachable!("rejected above"),
        ExportFormatArg::Srt => "srt",
        ExportFormatArg::Vtt => "vtt",
        ExportFormatArg::Transcript => "transcript",
        ExportFormatArg::TranscriptPlain => "transcript-plain",
        ExportFormatArg::TranscriptJson => "transcript-json",
    };
    Ok(ExportSummary {
        project_name: project.name.clone(),
        format_name,
        words,
    })
}
