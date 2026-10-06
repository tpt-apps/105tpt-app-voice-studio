//! Rendered-timeline derivation: mapping the working transcript onto the
//! output timeline a player/exporter cuts against.
//!
//! Policy (deterministic, covered by tests): each kept word occupies its
//! original duration on the output timeline, concatenated without gaps in
//! playback order. Deleting a span therefore removes the audio between its
//! neighbours — a physical cut. Words' *source* ranges stay untouched
//! (spec §3.2); only the output mapping is derived here. Deliberate pause
//! retention belongs to silence-trim configuration (spec §10).

use tpt_app_voice_studio_core::id::SegmentId;
use tpt_app_voice_studio_core::time::TimeRange;
use tpt_app_voice_studio_model::transcript::{Transcript, Word};

/// One kept word positioned on the output timeline.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TimelineEntry<'a> {
    /// The segment the word belongs to.
    pub segment: SegmentId,
    /// The word's index within its segment.
    pub word_index: usize,
    /// The word itself, with its untouched source-range binding.
    pub word: &'a Word,
    /// Where this word's audio lands on the rendered output timeline.
    pub rendered: TimeRange,
}

/// Derives the rendered output timeline for a working transcript.
///
/// Entries appear in playback order; their `rendered` ranges are contiguous
/// (each starts where the previous ends) and zero-length for zero-duration
/// words.
#[must_use]
pub fn render_timeline(transcript: &Transcript) -> Vec<TimelineEntry<'_>> {
    let mut cursor = std::time::Duration::ZERO;
    let mut entries = Vec::new();
    for segment in &transcript.segments {
        for (word_index, word) in segment.words.iter().enumerate() {
            let duration = word.end.saturating_sub(word.start);
            let rendered = TimeRange {
                start: cursor,
                end: cursor + duration,
            };
            cursor = rendered.end;
            entries.push(TimelineEntry {
                segment: segment.id,
                word_index,
                word,
                rendered,
            });
        }
    }
    entries
}

/// Finds the timeline entry active at `instant` on the output timeline.
#[must_use]
pub fn entry_at(
    transcript: &Transcript,
    instant: std::time::Duration,
) -> Option<TimelineEntry<'_>> {
    render_timeline(transcript)
        .into_iter()
        .find(|entry| entry.rendered.contains_instant(instant))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{entry_at, render_timeline};
    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    fn word(text: &str, start: u64, end: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_secs(start),
            end: Duration::from_secs(end),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    fn transcript() -> Transcript {
        // Playback order: "a b" (gapped in source), then "c" in a later
        // segment. Source gaps must collapse on the output timeline.
        Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![word("a", 0, 1), word("b", 5, 7)],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![word("c", 20, 21)],
            },
        ])
    }

    #[test]
    fn rendered_ranges_are_contiguous_and_collapse_source_gaps() {
        let transcript = transcript();
        let entries = render_timeline(&transcript);
        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].rendered.start, Duration::ZERO);
        assert_eq!(entries[0].rendered.end, Duration::from_secs(1));
        assert_eq!(entries[1].rendered.start, Duration::from_secs(1));
        assert_eq!(entries[1].rendered.end, Duration::from_secs(3));
        assert_eq!(entries[2].rendered.start, Duration::from_secs(3));
        assert_eq!(entries[2].rendered.end, Duration::from_secs(4));

        // Contiguity across every entry.
        for pair in entries.windows(2) {
            assert_eq!(pair[0].rendered.end, pair[1].rendered.start);
        }
    }

    #[test]
    fn entries_carry_segment_and_index_positions() {
        let transcript = transcript();
        let entries = render_timeline(&transcript);
        assert_eq!(entries[0].segment, SegmentId::new(1));
        assert_eq!(entries[0].word_index, 0);
        assert_eq!(entries[1].segment, SegmentId::new(1));
        assert_eq!(entries[1].word_index, 1);
        assert_eq!(entries[2].segment, SegmentId::new(2));
        assert_eq!(entries[2].word_index, 0);
        assert_eq!(entries[1].word.text, "b");
    }

    #[test]
    fn source_binding_is_untouched_by_rendering() {
        let transcript = transcript();
        let entries = render_timeline(&transcript);
        assert_eq!(entries[1].word.start, Duration::from_secs(5));
        assert_eq!(entries[1].word.end, Duration::from_secs(7));
    }

    #[test]
    fn entry_at_finds_the_active_word() {
        let transcript = transcript();
        let entry = entry_at(&transcript, Duration::from_secs(2)).expect("inside 'b'");
        assert_eq!(entry.word.text, "b");
        assert!(entry_at(&transcript, Duration::from_secs(99)).is_none());
        // Zero-length boundary: end is exclusive, so t=1s lands in "b".
        assert_eq!(
            entry_at(&transcript, Duration::from_secs(1))
                .expect("inside 'b'")
                .word
                .text,
            "b"
        );
    }

    #[test]
    fn empty_and_deleted_segments_produce_no_entries() {
        let empty = Transcript::default();
        assert!(render_timeline(&empty).is_empty());
        assert!(entry_at(&empty, Duration::ZERO).is_none());
    }
}
