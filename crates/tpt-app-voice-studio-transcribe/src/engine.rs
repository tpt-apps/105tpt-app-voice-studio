//! The transcription engine port and its implementations (spec §3.6, §25
//! step 3).
//!
//! - [`TranscriptionEngine`] is the port the rest of Voice Studio programs
//!   against; it never sees `tpt-voice` types.
//! - [`TptVoiceEngine`] is the real engine: `tpt-voice`'s `SpeechPipeline`
//!   (ASR → diarisation → speaker annotation → forced alignment) in one
//!   call, converted into the project domain model.
//! - [`FakeTranscriptionEngine`] returns deterministic output with no model
//!   files, so tests and UI development run anywhere.
//!
//! Conversion invariants (spec §3.2, §7.1, §9): every word carries timing,
//! confidence, and its source recording; segment/speaker relationships are
//! preserved; per-word confidence is retained verbatim from the engine.

use std::collections::HashMap;
use std::time::Duration;

use tpt_app_voice_studio_core::confidence::Confidence;
use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
use tpt_app_voice_studio_model::speaker::Speaker;
use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};
use tpt_voice::align::AlignerConfig;
use tpt_voice::diarize::DiarizerConfig;
use tpt_voice::utils::SpeakerLabel;

use crate::error::TranscribeError;

/// Result of one transcription pass over one recording.
#[derive(Clone, PartialEq, Debug)]
pub struct TranscriptionOutput {
    /// The aligned, speaker-annotated transcript bound to `recording`.
    pub transcript: Transcript,
    /// The speaker registry entries referenced by the transcript.
    pub speakers: Vec<Speaker>,
}

impl TranscriptionOutput {
    /// Mean per-word confidence across the transcript (0.0 for empty).
    ///
    /// This feeds the CLI's `average_confidence` result field (spec §13).
    #[must_use]
    pub fn average_confidence(&self) -> f32 {
        self.transcript.average_confidence().unwrap_or(0.0)
    }
}

/// The transcription/diarisation port (spec §3.6).
///
/// `&mut self` reflects the real engine: inference state is not shared
/// across concurrent runs; callers serialise access (a session processes
/// one recording at a time).
pub trait TranscriptionEngine {
    /// Transcribes mono PCM, diarises speakers, and word-aligns the result
    /// against `recording`.
    ///
    /// # Errors
    /// Returns [`TranscribeError::Engine`] for engine failures and
    /// [`TranscribeError::EmptyAudio`] for silent input.
    fn transcribe(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        recording: RecordingId,
    ) -> Result<TranscriptionOutput, TranscribeError>;
}

/// The real engine, backed by `tpt-voice`'s `SpeechPipeline`.
///
/// Constructing one loads the ASR model from a local model directory
/// (`Transcriber::from_model_dir`); the classical diariser and energy
/// aligner are weight-free. Model directories are optional local downloads
/// (spec §3.1): without one, transcription is simply not offered.
pub struct TptVoiceEngine {
    pipeline: tpt_voice::SpeechPipeline,
}

impl TptVoiceEngine {
    /// Loads the ASR model from `model_dir` and builds the pipeline with
    /// the given diariser/aligner configuration.
    ///
    /// # Errors
    /// Returns [`TranscribeError::Engine`] if the model cannot be loaded.
    pub fn from_model_dir(
        model_dir: impl AsRef<std::path::Path>,
        diarizer: DiarizerConfig,
        aligner: AlignerConfig,
    ) -> Result<Self, TranscribeError> {
        let pipeline = tpt_voice::SpeechPipeline::from_model_dir(model_dir, diarizer, aligner)?;
        Ok(Self { pipeline })
    }

    /// Loads the ASR model with default diariser/aligner configuration —
    /// the common path for shells and CLIs.
    ///
    /// # Errors
    /// See [`Self::from_model_dir`].
    pub fn with_default_configs(
        model_dir: impl AsRef<std::path::Path>,
    ) -> Result<Self, TranscribeError> {
        Self::from_model_dir(
            model_dir,
            DiarizerConfig::default(),
            AlignerConfig::default(),
        )
    }

    /// Access to the pipeline for advanced configuration (decode budget
    /// etc.).
    #[must_use]
    pub fn pipeline(&self) -> &tpt_voice::SpeechPipeline {
        &self.pipeline
    }
}

impl TranscriptionEngine for TptVoiceEngine {
    fn transcribe(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        recording: RecordingId,
    ) -> Result<TranscriptionOutput, TranscribeError> {
        if samples.is_empty() {
            return Err(TranscribeError::EmptyAudio);
        }
        let output = self.pipeline.run(samples, sample_rate)?;
        Ok(convert_engine_transcript(&output.transcript, recording))
    }
}

/// A deterministic engine for tests and UI work: emits one segment per
/// `words` entry with evenly spaced timing and fixed confidence.
#[derive(Clone, Debug)]
pub struct FakeTranscriptionEngine {
    /// Confidence attached to every word (0..1).
    pub confidence: f32,
    /// The words to emit, one segment each.
    pub words: Vec<String>,
}

impl FakeTranscriptionEngine {
    /// A fake saying the given words.
    #[must_use]
    pub fn saying(words: &[&str]) -> Self {
        Self {
            confidence: 0.9,
            words: words.iter().map(ToString::to_string).collect(),
        }
    }
}

impl TranscriptionEngine for FakeTranscriptionEngine {
    fn transcribe(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        recording: RecordingId,
    ) -> Result<TranscriptionOutput, TranscribeError> {
        if samples.is_empty() {
            return Err(TranscribeError::EmptyAudio);
        }
        #[allow(clippy::cast_precision_loss)] // sample counts stay far below f64 precision limits
        let secs = samples.len() as f64 / f64::from(sample_rate);
        let max_end = Duration::from_secs_f64(secs);
        let segments = self
            .words
            .iter()
            .enumerate()
            .map(|(i, text)| Segment {
                id: SegmentId::new(i as u64 + 1),
                speaker: None,
                words: vec![Word {
                    text: text.clone(),
                    start: Duration::from_millis(i as u64 * 500),
                    end: Duration::from_millis(i as u64 * 500 + 500).min(max_end),
                    confidence: Confidence::new(self.confidence).expect("valid"),
                    source_recording: recording,
                }],
            })
            .collect();
        Ok(TranscriptionOutput {
            transcript: Transcript::from_segments(segments),
            speakers: Vec::new(),
        })
    }
}

/// Converts an engine transcript (from a `tpt-voice` pipeline run) into
/// the project domain model.
///
/// - Engine `SegmentId`s carry over unchanged.
/// - Speaker labels become registry entries in first-appearance order;
///   labels use the display name when the engine has one, else the raw id
///   (`SPEAKER_00`).
/// - Segments without word timestamps (possible when alignment finds no
///   speech) fall back to a single word spanning the segment, so the
///   segment remains editable rather than silently vanishing.
#[must_use]
pub fn convert_engine_transcript(
    transcript: &tpt_voice::utils::Transcript,
    recording: RecordingId,
) -> TranscriptionOutput {
    let mut speaker_ids: HashMap<String, SpeakerId> = HashMap::new();
    let mut speakers: Vec<Speaker> = Vec::new();

    let speaker_id_for = |label: &SpeakerLabel,
                          speakers: &mut Vec<Speaker>,
                          ids: &mut HashMap<String, SpeakerId>|
     -> SpeakerId {
        let next_index = ids.len();
        *ids.entry(label.id.clone()).or_insert_with(|| {
            let id = SpeakerId::new(next_index as u64 + 1);
            let display = label.name.clone().unwrap_or_else(|| label.id.clone());
            let mut speaker = Speaker::new(id, display);
            speaker.recordings.insert(recording);
            speakers.push(speaker);
            id
        })
    };

    let segments = transcript
        .segments
        .iter()
        .map(|seg| {
            let speaker = seg
                .speaker
                .as_ref()
                .map(|label| speaker_id_for(label, &mut speakers, &mut speaker_ids));
            let words: Vec<Word> = match &seg.words {
                Some(word_list) if !word_list.is_empty() => word_list
                    .iter()
                    .map(|w| Word {
                        text: w.word.clone(),
                        start: secs_to_duration(w.start_time),
                        end: secs_to_duration(w.end_time),
                        confidence: Confidence::new(w.confidence.clamp(0.0, 1.0))
                            .unwrap_or(Confidence::new(0.0).expect("0.0 is valid")),
                        source_recording: recording,
                    })
                    .collect(),
                _ => vec![Word {
                    text: seg.text.clone(),
                    start: secs_to_duration(seg.start_time),
                    end: secs_to_duration(seg.end_time),
                    confidence: Confidence::new(seg.confidence.clamp(0.0, 1.0))
                        .unwrap_or(Confidence::new(0.0).expect("0.0 is valid")),
                    source_recording: recording,
                }],
            };
            Segment {
                id: SegmentId::new(seg.id.0),
                speaker,
                words,
            }
        })
        .collect();

    TranscriptionOutput {
        transcript: Transcript::from_segments(segments),
        speakers,
    }
}

/// `f64` seconds to `Duration`, clamping negatives/NaN to zero (engine
/// timestamps are non-negative but defensive conversion keeps the boundary
/// malformed-input rule of spec §18.1).
fn secs_to_duration(secs: f64) -> Duration {
    if !secs.is_finite() || secs <= 0.0 {
        return Duration::ZERO;
    }
    Duration::try_from_secs_f64(secs).unwrap_or(Duration::MAX)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use tpt_voice::utils::transcript::{Transcript as EngineTranscript, TranscriptSegment};
    use tpt_voice::utils::WordTimestamp;

    fn engine_segment(
        id: u64,
        start: f64,
        end: f64,
        text: &str,
        speaker: Option<SpeakerLabel>,
        words: Option<Vec<WordTimestamp>>,
    ) -> TranscriptSegment {
        TranscriptSegment {
            id: tpt_voice::utils::transcript::SegmentId(id),
            start_time: start,
            end_time: end,
            text: text.to_string(),
            speaker,
            confidence: 0.8,
            words,
        }
    }

    fn engine_transcript(segments: Vec<TranscriptSegment>) -> tpt_voice::utils::Transcript {
        let mut transcript = EngineTranscript::new("en");
        for seg in segments {
            transcript.push_segment(seg);
        }
        transcript
    }

    const REC: RecordingId = RecordingId::new(1);

    #[test]
    fn conversion_binds_words_speakers_and_confidence() {
        let transcript = engine_transcript(vec![
            engine_segment(
                1,
                0.0,
                1.0,
                "hello world",
                Some(SpeakerLabel::new(0)),
                Some(vec![
                    WordTimestamp::new("hello", 0.0, 0.5, 0.97),
                    WordTimestamp::new("world", 0.5, 1.0, 0.42),
                ]),
            ),
            engine_segment(
                2,
                2.0,
                3.0,
                "guest line",
                Some(SpeakerLabel::named("SPEAKER_01", "Guest")),
                Some(vec![WordTimestamp::new("guest", 2.0, 2.5, 0.8)]),
            ),
        ]);

        let converted = convert_engine_transcript(&transcript, REC);
        assert_eq!(converted.transcript.word_count(), 3);
        let first = &converted.transcript.segments[0];
        assert_eq!(first.speaker, Some(SpeakerId::new(1)));
        assert_eq!(first.words[1].confidence.value(), 0.42);
        assert_eq!(first.words[0].source_recording, REC);
        assert_eq!(
            first.words[1].end,
            Duration::try_from_secs_f64(1.0).expect("valid")
        );

        // Registry: first-appearance order, display names preferred.
        assert_eq!(converted.speakers.len(), 2);
        assert_eq!(converted.speakers[0].label, "SPEAKER_00");
        assert_eq!(converted.speakers[1].label, "Guest");
        assert!(converted.speakers[0].recordings.contains(&REC));

        // Average confidence feeds the CLI result (spec §13).
        assert!(converted.average_confidence() > 0.0 && converted.average_confidence() < 1.0);
    }

    #[test]
    fn segments_without_word_timestamps_fall_back_to_whole_segment_word() {
        let transcript = engine_transcript(vec![engine_segment(
            5,
            1.0,
            2.5,
            "no alignment here",
            None,
            None,
        )]);
        let converted = convert_engine_transcript(&transcript, REC);
        let seg = &converted.transcript.segments[0];
        assert_eq!(seg.word_count(), 1);
        assert_eq!(seg.words[0].text, "no alignment here");
        assert_eq!(seg.words[0].start, Duration::from_secs(1));
        assert_eq!(seg.speaker, None);
    }

    #[test]
    fn out_of_range_confidences_are_clamped_not_rejected() {
        let mut seg = engine_segment(1, 0.0, 1.0, "hey", None, None);
        seg.confidence = 1.5;
        let converted = convert_engine_transcript(&engine_transcript(vec![seg]), REC);
        assert_eq!(
            converted.transcript.segments[0].words[0].confidence.value(),
            1.0
        );
    }

    #[test]
    fn fake_engine_produces_deterministic_bound_words() {
        let mut engine = FakeTranscriptionEngine::saying(&["um", "hello", "world"]);
        let samples = vec![0.0_f32; 16_000]; // one second
        let out = engine
            .transcribe(&samples, 16_000, RecordingId::new(7))
            .expect("works");
        assert_eq!(out.transcript.word_count(), 3);
        assert_eq!(
            out.transcript.segments[0].words[0].source_recording,
            RecordingId::new(7)
        );

        let err = engine
            .transcribe(&[], 16_000, RecordingId::new(7))
            .expect_err("empty rejected");
        assert!(matches!(err, TranscribeError::EmptyAudio));
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // tone-generator sample index, not financial data
    fn real_engine_runs_end_to_end_when_a_model_is_available() {
        let Some(model_dir) = std::env::var_os("TPT_VOICE_MODEL_DIR") else {
            eprintln!("skipping: TPT_VOICE_MODEL_DIR is not set (no local ASR model)");
            return;
        };
        let mut engine = TptVoiceEngine::from_model_dir(
            model_dir,
            DiarizerConfig::default(),
            AlignerConfig::default(),
        )
        .expect("model loads");

        // One second of a 220 Hz tone: no speech, so the pipeline must
        // complete (not panic) and yield a structurally valid result.
        let samples: Vec<f32> = (0..16_000)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / 16_000.0).sin())
            .collect();
        let out = engine
            .transcribe(&samples, 16_000, RecordingId::new(1))
            .expect("pipeline runs");
        for seg in &out.transcript.segments {
            for word in &seg.words {
                assert!(word.end >= word.start);
                assert_eq!(word.source_recording, RecordingId::new(1));
            }
        }
    }
}
