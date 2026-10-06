//! `tpt-voice-studio` — the TPT Voice Studio CLI.
//!
//! A first-class automation interface for studio-scale batch processing
//! (spec §13):
//!
//! - `transcribe` — transcribe a single WAV recording to a JSON transcript.
//! - `batch-transcribe` — transcribe every WAV in a directory.
//! - `export` — render SRT/VTT subtitles or a transcript (text/JSON) from a
//!   `.tptproj` project file; audio formats arrive with the codec
//!   integration.
//!
//! Transcription runs the `tpt-voice` engine over a local ASR model
//! directory (`--model-dir`); models are optional local downloads, so the
//! tool stays offline-first (spec §3.1) and simply requires configuration
//! before transcribing.
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

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use tpt_app_voice_studio_core::id::RecordingId;
use tpt_app_voice_studio_edit::EditSession;
use tpt_app_voice_studio_export::{
    render_subtitles, render_transcript, DisclaimerMode, SubtitleOptions, TranscriptExportOptions,
};
use tpt_app_voice_studio_model::export::{SubtitleFormat, TranscriptFormat};
use tpt_app_voice_studio_model::ProjectFile;
use tpt_app_voice_studio_transcribe::{
    decode_wav_file, TptVoiceEngine, TranscriptionEngine, TranscriptionOutput,
};

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
    /// Transcribe a single WAV recording (requires --model-dir).
    Transcribe {
        /// Recording to transcribe (WAV).
        #[arg(long)]
        input: PathBuf,
        /// Transcript output path (JSON).
        #[arg(long)]
        output: PathBuf,
        /// Directory holding the local ASR model (optional download; see
        /// docs/architecture.md).
        #[arg(long)]
        model_dir: Option<PathBuf>,
    },
    /// Transcribe every WAV in a directory (requires --model-dir).
    BatchTranscribe {
        /// Directory of WAV recordings.
        #[arg(long)]
        input: PathBuf,
        /// Directory for JSON transcript output (created if missing).
        #[arg(long)]
        output: PathBuf,
        /// Directory holding the local ASR model.
        #[arg(long)]
        model_dir: Option<PathBuf>,
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
        Commands::Transcribe {
            input,
            output,
            model_dir,
        } => run_transcribe(&input, &output, model_dir.as_deref(), cli.json),
        Commands::BatchTranscribe {
            input,
            output,
            model_dir,
        } => run_batch(&input, &output, model_dir.as_deref(), cli.json),
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

fn print_success(json: bool, value: &serde_json::Value, human: &str) {
    if json {
        println!("{value}");
    } else {
        println!("{human}");
    }
}

fn print_failure(json: bool, command: &str, failure: &Failure) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "command": command,
                "code": failure.code,
                "message": failure.message,
            })
        );
    } else {
        eprintln!("error: {}", failure.message);
    }
}

/// Loads the real engine from the configured model directory.
fn build_engine(model_dir: Option<&Path>) -> Result<TptVoiceEngine, Failure> {
    let Some(dir) = model_dir else {
        return Err(Failure::new(
            exit_code::CONFIGURATION_ERROR,
            "transcription requires --model-dir pointing at a local ASR model directory \
             (models are optional downloads; see docs/architecture.md)",
        ));
    };
    TptVoiceEngine::with_default_configs(dir).map_err(|e| {
        Failure::new(
            exit_code::TRANSCRIPTION_FAILED,
            format!("could not load the ASR model from {}: {e}", dir.display()),
        )
    })
}

/// Transcribes one WAV file and writes a JSON transcript.
fn transcribe_file(
    engine: &mut dyn TranscriptionEngine,
    input: &Path,
    output: &Path,
) -> Result<(usize, f32), Failure> {
    let audio =
        decode_wav_file(input).map_err(|e| Failure::new(exit_code::INPUT_ERROR, e.to_string()))?;
    let TranscriptionOutput {
        transcript,
        speakers,
    } = engine
        .transcribe(&audio.samples, audio.sample_rate, RecordingId::new(1))
        .map_err(|e| Failure::new(exit_code::TRANSCRIPTION_FAILED, e.to_string()))?;

    let words = transcript.word_count();
    let average_confidence = transcript.average_confidence().unwrap_or(0.0);
    let options = TranscriptExportOptions {
        format: TranscriptFormat::Json,
        include_speaker_labels: true,
        disclaimer: DisclaimerMode::None,
    };
    let json_text = render_transcript(&transcript, &speakers, &options);
    std::fs::write(output, json_text)
        .map_err(|e| Failure::new(exit_code::EXPORT_FAILED, format!("write failed: {e}")))?;
    Ok((words, average_confidence))
}

fn run_transcribe(input: &Path, output: &Path, model_dir: Option<&Path>, json: bool) -> u8 {
    let outcome =
        build_engine(model_dir).and_then(|mut engine| transcribe_file(&mut engine, input, output));
    match outcome {
        Ok((words, average_confidence)) => {
            let result = serde_json::json!({
                "status": "success",
                "command": "transcribe",
                "input": input.display().to_string(),
                "output": output.display().to_string(),
                "words_transcribed": words,
                "average_confidence": average_confidence,
            });
            print_success(
                json,
                &result,
                &format!(
                    "transcribed {}: {words} words, average confidence {average_confidence:.2} → {}",
                    input.display(),
                    output.display()
                ),
            );
            exit_code::SUCCESS
        }
        Err(failure) => {
            print_failure(json, "transcribe", &failure);
            failure.code
        }
    }
}

/// Lists the WAV recordings in `input_dir`, sorted by path.
fn list_wav_files(input_dir: &Path) -> Result<Vec<PathBuf>, Failure> {
    if !input_dir.is_dir() {
        return Err(Failure::new(
            exit_code::INPUT_ERROR,
            format!("input directory not found: {}", input_dir.display()),
        ));
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(input_dir)
        .map_err(|e| Failure::new(exit_code::INPUT_ERROR, e.to_string()))?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(Failure::new(
            exit_code::INPUT_ERROR,
            format!("no WAV files found in {}", input_dir.display()),
        ));
    }
    Ok(paths)
}

/// Output base name for a recording's transcript.
fn transcript_stem(input: &Path) -> String {
    input.file_stem().map_or_else(
        || "recording".to_string(),
        |s| s.to_string_lossy().into_owned(),
    )
}

/// Transcribes every file into `output_dir`, reporting per-file results.
fn transcribe_batch_files(
    engine: &mut dyn TranscriptionEngine,
    files: &[PathBuf],
    output_dir: &Path,
    json: bool,
) -> (usize, usize, Vec<serde_json::Value>) {
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;
    for input in files {
        let stem = transcript_stem(input);
        let output = output_dir.join(format!("{stem}.json"));
        match transcribe_file(engine, input, &output) {
            Ok((words, average_confidence)) => {
                succeeded += 1;
                if !json {
                    println!(
                        "ok: {} → {words} words, confidence {average_confidence:.2}",
                        input.display()
                    );
                }
                results.push(serde_json::json!({
                    "input": input.display().to_string(),
                    "output": output.display().to_string(),
                    "status": "success",
                    "words_transcribed": words,
                    "average_confidence": average_confidence,
                }));
            }
            Err(failure) => {
                failed += 1;
                if !json {
                    eprintln!("failed: {} ({})", input.display(), failure.message);
                }
                results.push(serde_json::json!({
                    "input": input.display().to_string(),
                    "status": "error",
                    "code": failure.code,
                    "message": failure.message,
                }));
            }
        }
    }
    (succeeded, failed, results)
}

fn run_batch(input_dir: &Path, output_dir: &Path, model_dir: Option<&Path>, json: bool) -> u8 {
    if !input_dir.is_dir() {
        let failure = Failure::new(
            exit_code::INPUT_ERROR,
            format!("input directory not found: {}", input_dir.display()),
        );
        print_failure(json, "batch-transcribe", &failure);
        return failure.code;
    }
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        let failure = Failure::new(
            exit_code::EXPORT_FAILED,
            format!("cannot create output directory: {e}"),
        );
        print_failure(json, "batch-transcribe", &failure);
        return failure.code;
    }

    let mut engine = match build_engine(model_dir) {
        Ok(engine) => engine,
        Err(failure) => {
            print_failure(json, "batch-transcribe", &failure);
            return failure.code;
        }
    };

    let files = match list_wav_files(input_dir) {
        Ok(files) => files,
        Err(failure) => {
            print_failure(json, "batch-transcribe", &failure);
            return failure.code;
        }
    };

    let (succeeded, failed, results) =
        transcribe_batch_files(&mut engine, &files, output_dir, json);

    let status = if failed == 0 {
        "success"
    } else if succeeded == 0 {
        "error"
    } else {
        "partial_success"
    };
    let code = if failed == 0 {
        exit_code::SUCCESS
    } else if succeeded == 0 {
        exit_code::TRANSCRIPTION_FAILED
    } else {
        exit_code::PARTIAL_SUCCESS
    };
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": status,
                "command": "batch-transcribe",
                "succeeded": succeeded,
                "failed": failed,
                "total": files.len(),
                "results": results,
            })
        );
    } else {
        println!(
            "{succeeded} of {} recordings transcribed ({} failed)",
            files.len(),
            failed
        );
    }
    code
}

fn run_export(
    project_path: &Path,
    format: ExportFormatArg,
    output_path: &Path,
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
            let result = serde_json::json!({
                "status": "success",
                "command": "export",
                "project": summary.project_name,
                "format": summary.format_name,
                "output": output_path.display().to_string(),
                "words": summary.words,
            });
            print_success(
                json,
                &result,
                &format!(
                    "wrote {} ({}, {} words)",
                    output_path.display(),
                    summary.format_name,
                    summary.words
                ),
            );
            exit_code::SUCCESS
        }
        Err(failure) => {
            print_failure(json, "export", &failure);
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
    project_path: &Path,
    format: ExportFormatArg,
    output_path: &Path,
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
