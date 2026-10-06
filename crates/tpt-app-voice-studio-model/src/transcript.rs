//! Transcript: the editable, audio-bound representation of the session
//! (spec §6.3).
//!
//! The transcript is not a detachable artefact: every word carries its
//! timestamp range, confidence score, and the recording it was aligned
//! against (spec §3.2, §7.1). Segment order in this structure is the
//! *playback* order of the working transcript; within a freshly aligned
//! transcript it equals time order (see [`Transcript::is_time_ordered`]).

use std::time::Duration;

use tpt_app_voice_studio_core::confidence::Confidence;
use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
use tpt_app_voice_studio_core::time::TimeRange;

/// One aligned, spoken word (spec §6.3).
#[derive(Clone, PartialEq, Debug)]
pub struct Word {
    /// The transcribed text (usually one token; may be multi-word for
    /// detections like a filler phrase "you know").
    pub text: String,
    /// Start of the word's audio on the source recording's timeline.
    pub start: Duration,
    /// Exclusive end of the word's audio on the source recording's timeline.
    pub end: Duration,
    /// Recognition confidence, retained for the life of the project
    /// (spec §7.1, §17.1).
    pub confidence: Confidence,
    /// The recording this word was aligned against (spec §7.2).
    pub source_recording: RecordingId,
}

impl Word {
    /// The half-open audio range this word is bound to (spec §3.2).
    #[must_use]
    pub fn range(&self) -> TimeRange {
        TimeRange {
            start: self.start,
            end: self.end,
        }
    }
}

/// A run of consecutive words attributed to one speaker (spec §6.3).
#[derive(Clone, PartialEq, Debug)]
pub struct Segment {
    /// Unique id of this segment within the project.
    pub id: SegmentId,
    /// The speaker, once diarisation has assigned one.
    pub speaker: Option<SpeakerId>,
    /// The words of the segment, in playback order.
    pub words: Vec<Word>,
}

impl Segment {
    /// Number of words in the segment.
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// The time span from the first word's start to the last word's end,
    /// or `None` for an empty segment.
    #[must_use]
    pub fn span(&self) -> Option<TimeRange> {
        let first = self.words.first()?;
        let last = self.words.last()?;
        Some(TimeRange {
            start: first.start,
            end: last.end,
        })
    }
}

/// The full transcript of a project (spec §6.3).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Transcript {
    /// Segments in playback order.
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Creates a transcript from segments without reordering them.
    #[must_use]
    pub fn from_segments(segments: Vec<Segment>) -> Self {
        Self { segments }
    }

    /// Looks up a segment by id.
    #[must_use]
    pub fn segment(&self, id: SegmentId) -> Option<&Segment> {
        self.segments.iter().find(|s| s.id == id)
    }

    /// Mutable lookup of a segment by id.
    pub fn segment_mut(&mut self, id: SegmentId) -> Option<&mut Segment> {
        self.segments.iter_mut().find(|s| s.id == id)
    }

    /// Total number of words across all segments.
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.segments.iter().map(Segment::word_count).sum()
    }

    /// All words in playback order (segments in order, words within each).
    pub fn words(&self) -> impl Iterator<Item = &Word> {
        self.segments.iter().flat_map(|s| s.words.iter())
    }

    /// Average confidence across all words, or `None` for an empty
    /// transcript. This feeds the CLI's `average_confidence` result field
    /// (spec §13).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // word counts far below f32 precision limits in practice
    pub fn average_confidence(&self) -> Option<f32> {
        let count = self.word_count();
        if count == 0 {
            return None;
        }
        let sum: f32 = self.words().map(|w| w.confidence.value()).sum();
        Some(sum / count as f32)
    }

    /// True if segments are in non-decreasing start-time order and words
    /// within each segment are too (ignoring empty runs). Fresh alignment
    /// output must satisfy this; edited transcripts may deliberately violate
    /// it via reorder operations.
    #[must_use]
    pub fn is_time_ordered(&self) -> bool {
        let mut previous_end: Option<Duration> = None;
        for segment in &self.segments {
            for word in &segment.words {
                if word.end < word.start {
                    return false;
                }
                if let Some(end) = previous_end {
                    if word.start < end {
                        return false;
                    }
                }
                previous_end = Some(word.end);
            }
        }
        true
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn word(text: &str, start_secs: u64, end_secs: u64, confidence: f32) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_secs(start_secs),
            end: Duration::from_secs(end_secs),
            confidence: Confidence::new(confidence).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    fn sample_transcript() -> Transcript {
        Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![word("Welcome", 0, 1, 0.97), word("back", 1, 2, 0.95)],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![word("Thanks", 4, 5, 0.55), word("again", 5, 6, 0.42)],
            },
        ])
    }

    #[test]
    fn counts_words_and_segments() {
        let transcript = sample_transcript();
        assert_eq!(transcript.segments.len(), 2);
        assert_eq!(transcript.word_count(), 4);
    }

    #[test]
    fn looks_up_segments_by_id() {
        let transcript = sample_transcript();
        assert_eq!(
            transcript
                .segment(SegmentId::new(2))
                .expect("exists")
                .word_count(),
            2
        );
        assert!(transcript.segment(SegmentId::new(99)).is_none());
    }

    #[test]
    fn segment_span_covers_first_start_to_last_end() {
        let transcript = sample_transcript();
        let span = transcript
            .segment(SegmentId::new(1))
            .expect("exists")
            .span()
            .expect("non-empty");
        assert_eq!(span.start, Duration::ZERO);
        assert_eq!(span.end, Duration::from_secs(2));
        assert!(Transcript::default()
            .segments
            .first()
            .and_then(Segment::span)
            .is_none());
    }

    #[test]
    fn average_confidence_is_over_all_words() {
        let transcript = sample_transcript();
        let expected = (0.97 + 0.95 + 0.55 + 0.42) / 4.0;
        let average = transcript.average_confidence().expect("non-empty");
        assert!((average - expected).abs() < 1e-6, "{average} vs {expected}");
        assert!(Transcript::default().average_confidence().is_none());
    }

    #[test]
    fn time_ordering_holds_for_fresh_alignment_and_detects_violations() {
        assert!(sample_transcript().is_time_ordered());

        let mut out_of_order = sample_transcript();
        out_of_order.segments.swap(0, 1);
        assert!(!out_of_order.is_time_ordered());
    }

    #[test]
    fn word_range_binding_round_trips() {
        let transcript = sample_transcript();
        let word = &transcript.segments[0].words[1];
        assert_eq!(
            word.range(),
            TimeRange::new(Duration::from_secs(1), Duration::from_secs(2)).expect("valid")
        );
    }
}
