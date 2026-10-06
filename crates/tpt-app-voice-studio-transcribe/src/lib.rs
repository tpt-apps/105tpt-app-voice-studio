//! Transcription and diarisation pipeline stage for TPT Voice Studio.
//!
//! Owns the decode → diarise → transcribe portion of the pipeline (spec §7):
//!
//! - [`decode`]: file import on top of the `tpt-cadence` codec foundation,
//!   producing mono PCM the engine consumes. Supported today: WAV (PCM and
//!   IEEE float, any channel count). MP3/AAC/FLAC decode lands with the
//!   remaining codec crates.
//! - [`engine`]: the [`engine::TranscriptionEngine`] port plus two
//!   implementations — [`engine::TptVoiceEngine`] (the real engine, backed
//!   by `tpt-voice`'s `SpeechPipeline`: transcription, diarisation, and
//!   forced alignment in one pass) and [`engine::FakeTranscriptionEngine`]
//!   (deterministic, for tests and UI development without model files).
//!
//! Engine output is converted into the project domain model with the
//! bindings that make transcript editing possible (spec §3.2): every word
//! carries its timing, confidence, and source recording, and diarised
//! speakers become registry entries (spec §7.1, §9).
//!
//! The engine sits behind the port trait so the deterministic engine stays
//! testable and the desktop shell, CLI, and batch mode all drive the same
//! pipeline (spec §3.6). Running the real engine requires a local ASR model
//! directory; models are optional downloads, consistent with the offline-
//! first policy (spec §3.1) — the app ships and works without them.

pub mod decode;
pub mod engine;
pub mod error;

pub use decode::{decode_wav_file, DecodedAudio, SUPPORTED_IMPORT_FORMATS};
pub use engine::{
    FakeTranscriptionEngine, TptVoiceEngine, TranscriptionEngine, TranscriptionOutput,
};
pub use error::TranscribeError;
