//! `tpt-voice-studio` — the TPT Voice Studio CLI.
//!
//! A first-class automation interface for studio-scale batch processing
//! (spec §13):
//!
//! - `transcribe` — transcribe a single recording to a transcript file.
//! - `batch-transcribe` — transcribe every recording in a directory.
//! - `export` — render audio, subtitle, or transcript output from a project.
//!
//! Results are machine-readable (JSON) and the exit-code contract is stable
//! (spec §13):
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
//! remain stable.
//!
//! The CLI drives the same engine crates as the desktop application; it must
//! never grow a second implementation of any pipeline stage (spec §3.6).
//!
//! Status: scaffold. Argument parsing and the commands above land in Phase 1.

use std::process::ExitCode;

/// Stable CLI exit codes (spec §13). Public so the desktop shell and tests can
/// reference the contract instead of hard-coding integers.
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

fn main() -> ExitCode {
    println!(
        "tpt-voice-studio {} (scaffold — commands land in Phase 1)",
        env!("CARGO_PKG_VERSION")
    );
    ExitCode::from(exit_code::SUCCESS)
}
