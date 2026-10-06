//! Transcript delivery formats (spec §11, §17.3).
//!
//! Transcript text and timestamps are taken from the rendered output
//! timeline, so an exported transcript reads exactly as the final edited
//! audio plays — the same alignment data drives editing and export (spec
//! §3.2, §11).
//!
//! For formal or legal use, the disclaimer of spec §17.3 can be attached on
//! opt-in; the application never claims verbatim accuracy on its own
//! (spec §17.1).

use std::collections::HashMap;
use std::time::Duration;

use tpt_app_voice_studio_core::id::{SegmentId, SpeakerId};
use tpt_app_voice_studio_edit::timeline::render_timeline;
use tpt_app_voice_studio_model::export::TranscriptFormat;
use tpt_app_voice_studio_model::speaker::Speaker;
use tpt_app_voice_studio_model::transcript::Transcript;

/// The formal-use disclaimer text of spec §17.3.
pub const TRANSCRIPT_DISCLAIMER: &str = "This transcript was generated using automated speech \
recognition and forced alignment. It has not been independently verified for verbatim accuracy \
unless explicitly marked as reviewed.";

/// Appended when the reviewer has explicitly marked the transcript reviewed.
pub const REVIEWED_NOTE: &str = "This transcript has been explicitly marked as reviewed.";

/// Disclaimer attachment for an export (spec §17.3, opt-in).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DisclaimerMode {
    /// No disclaimer (default; suitable for working copies).
    #[default]
    None,
    /// Attach [`TRANSCRIPT_DISCLAIMER`] as-is.
    Unverified,
    /// Attach [`TRANSCRIPT_DISCLAIMER`] plus [`REVIEWED_NOTE`].
    Reviewed,
}

/// Options for transcript rendering.
#[derive(Clone, Debug)]
pub struct TranscriptExportOptions {
    /// Output format.
    pub format: TranscriptFormat,
    /// Prefix segments with the speaker's label.
    pub include_speaker_labels: bool,
    /// Disclaimer attachment (spec §17.3).
    pub disclaimer: DisclaimerMode,
}

impl Default for TranscriptExportOptions {
    fn default() -> Self {
        Self {
            format: TranscriptFormat::Timestamped,
            include_speaker_labels: true,
            disclaimer: DisclaimerMode::None,
        }
    }
}

/// A segment positioned on the rendered output timeline.
struct RenderedSegment<'a> {
    speaker: Option<SpeakerId>,
    start: Duration,
    end: Duration,
    text: String,
    words: Vec<&'a tpt_app_voice_studio_model::transcript::Word>,
}

/// Renders the transcript in the configured format.
#[must_use]
pub fn render_transcript(
    transcript: &Transcript,
    speakers: &[Speaker],
    options: &TranscriptExportOptions,
) -> String {
    let rendered_segments = rendered_segments(transcript);
    let label_of = |speaker: Option<SpeakerId>| -> Option<&str> {
        speaker.and_then(|id| {
            speakers
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.label.as_str())
        })
    };
    let disclaimer_text = match options.disclaimer {
        DisclaimerMode::None => None,
        DisclaimerMode::Unverified => Some(TRANSCRIPT_DISCLAIMER.to_string()),
        DisclaimerMode::Reviewed => Some(format!("{TRANSCRIPT_DISCLAIMER}\n{REVIEWED_NOTE}")),
    };

    match options.format {
        TranscriptFormat::PlainText => {
            let mut out = String::new();
            for seg in &rendered_segments {
                let text = labeled_text(seg, label_of(seg.speaker), options.include_speaker_labels);
                out.push_str(&text);
                out.push_str("\n\n");
            }
            if let Some(disclaimer) = disclaimer_text {
                out.push_str("---\n");
                out.push_str(&disclaimer);
                out.push('\n');
            }
            out
        }
        TranscriptFormat::Timestamped => {
            use std::fmt::Write as _;
            let mut out = String::new();
            for seg in &rendered_segments {
                let text = labeled_text(seg, label_of(seg.speaker), options.include_speaker_labels);
                let _ = writeln!(out, "[{}] {text}", format_clock(seg.start));
            }
            if let Some(disclaimer) = disclaimer_text {
                out.push_str("\n---\n");
                out.push_str(&disclaimer);
                out.push('\n');
            }
            out
        }
        TranscriptFormat::Json => {
            let segments: Vec<serde_json::Value> = rendered_segments
                .iter()
                .map(|seg| {
                    let label = label_of(seg.speaker);
                    serde_json::json!({
                        "speaker": label,
                        "start": seg.start.as_secs_f64(),
                        "end": seg.end.as_secs_f64(),
                        "text": seg.text,
                        "words": seg.words.iter().map(|w| serde_json::json!({
                            "text": w.text,
                            "start": w.start.as_secs_f64(),
                            "end": w.end.as_secs_f64(),
                            // f32 confidences serialize rounded to 4 decimal
                            // places so the JSON shows the intended value
                            // rather than the f64 widening artefact.
                            "confidence": round4(f64::from(w.confidence.value())),
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            let doc = serde_json::json!({
                "format_version": 1,
                "timeline": "rendered",
                "segments": segments,
                "disclaimer": disclaimer_text,
            });
            serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// Groups the rendered timeline into per-segment runs, preserving playback
/// order. Reordered segments appear in their edited order; timings are the
/// rendered (output) positions. Segments without words are skipped.
fn rendered_segments(transcript: &Transcript) -> Vec<RenderedSegment<'_>> {
    let mut rendered: Vec<Option<RenderedSegment>> = transcript
        .segments
        .iter()
        .map(|s| {
            Some(RenderedSegment {
                speaker: s.speaker,
                start: Duration::ZERO,
                end: Duration::ZERO,
                text: String::new(),
                words: Vec::new(),
            })
        })
        .collect();
    let index_of: HashMap<SegmentId, usize> = transcript
        .segments
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, i))
        .collect();

    for entry in render_timeline(transcript) {
        let slot = rendered[index_of[&entry.segment]]
            .as_mut()
            .expect("present");
        if slot.words.is_empty() {
            slot.start = entry.rendered.start;
        }
        slot.end = entry.rendered.end;
        slot.text.push_str(&entry.word.text);
        slot.text.push(' ');
        slot.words.push(entry.word);
    }

    rendered
        .into_iter()
        .flatten()
        .filter(|seg| !seg.words.is_empty())
        .map(|mut seg| {
            seg.text = seg.text.trim_end().to_string();
            seg
        })
        .collect()
}

fn labeled_text(
    seg: &RenderedSegment<'_>,
    label: Option<&str>,
    include_speaker_labels: bool,
) -> String {
    match (include_speaker_labels, label) {
        (true, Some(label)) => format!("{label}: {}", seg.text),
        _ => seg.text.clone(),
    }
}

/// `[HH:MM:SS]`-style clock used by the timestamped format (spec §11
/// example).
fn format_clock(t: Duration) -> String {
    let total_secs = t.as_secs();
    let (hours, rem) = (total_secs / 3600, total_secs % 3600);
    let (minutes, seconds) = (rem / 60, rem % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Rounds to four decimal places for JSON serialization of confidences.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_model::export::TranscriptFormat;
    use tpt_app_voice_studio_model::speaker::Speaker;
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    use super::*;

    fn word(text: &str, start_ms: u64, end_ms: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            confidence: Confidence::new(0.42).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    /// Segment 1: "Welcome back" (rendered 0-2s), segment 2: "Thanks" (2-3s).
    fn transcript() -> Transcript {
        Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![word("Welcome", 0, 1_000), word("back", 1_000, 2_000)],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![word("Thanks", 20_000, 21_000)],
            },
        ])
    }

    fn speakers() -> Vec<Speaker> {
        vec![
            Speaker::new(SpeakerId::new(1), "Host"),
            Speaker::new(SpeakerId::new(2), "Guest"),
        ]
    }

    #[test]
    fn timestamped_output_matches_spec_layout() {
        let options = TranscriptExportOptions::default();
        let out = render_transcript(&transcript(), &speakers(), &options);
        // Rendered times, not source times: "Thanks" renders at 2s, not 20s.
        assert_eq!(
            out,
            "[00:00:00] Host: Welcome back\n[00:00:02] Guest: Thanks\n"
        );
    }

    #[test]
    fn plain_text_prose_without_timestamps() {
        let options = TranscriptExportOptions {
            format: TranscriptFormat::PlainText,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&transcript(), &speakers(), &options);
        assert_eq!(out, "Host: Welcome back\n\nGuest: Thanks\n\n");
    }

    #[test]
    fn labels_can_be_omitted() {
        let options = TranscriptExportOptions {
            include_speaker_labels: false,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&transcript(), &speakers(), &options);
        assert_eq!(out, "[00:00:00] Welcome back\n[00:00:02] Thanks\n");
    }

    #[test]
    fn disclaimer_modes_attach_spec_text() {
        let base = TranscriptExportOptions {
            disclaimer: DisclaimerMode::Unverified,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&transcript(), &speakers(), &base);
        assert!(out.contains(TRANSCRIPT_DISCLAIMER));
        assert!(!out.contains(REVIEWED_NOTE));

        let reviewed = TranscriptExportOptions {
            disclaimer: DisclaimerMode::Reviewed,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&transcript(), &speakers(), &reviewed);
        assert!(out.contains(TRANSCRIPT_DISCLAIMER));
        assert!(out.contains(REVIEWED_NOTE));

        let none = TranscriptExportOptions::default();
        let out = render_transcript(&transcript(), &speakers(), &none);
        assert!(!out.contains("generated using automated"));
    }

    #[test]
    fn json_output_carries_rendered_timing_and_confidence() {
        let options = TranscriptExportOptions {
            format: TranscriptFormat::Json,
            disclaimer: DisclaimerMode::Unverified,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&transcript(), &speakers(), &options);
        let doc: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(doc["format_version"], 1);
        assert_eq!(doc["timeline"], "rendered");
        let segments = doc["segments"].as_array().expect("segments");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0]["speaker"], "Host");
        assert_eq!(segments[1]["start"], 2.0);
        assert_eq!(segments[0]["words"][0]["confidence"], 0.42);
        assert!(doc["disclaimer"]
            .as_str()
            .expect("disclaimer")
            .contains("automated speech recognition"));
    }

    #[test]
    fn json_is_valid_for_empty_transcript() {
        let options = TranscriptExportOptions {
            format: TranscriptFormat::Json,
            ..TranscriptExportOptions::default()
        };
        let out = render_transcript(&Transcript::default(), &[], &options);
        let doc: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(doc["segments"].as_array().expect("segments").len(), 0);
    }
}
