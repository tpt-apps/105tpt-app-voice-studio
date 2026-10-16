//! Forced alignment and confidence scoring pipeline stage for TPT Voice Studio.
//!
//! Owns the stages that turn a draft transcript into an editable, audio-bound
//! representation (spec §7):
//!
//! - Forced alignment of transcript text to word-level timestamps. The speech
//!   engine (`tpt-voice`) produces the alignment; this stage checks it, so
//!   every word is bound to a sample range inside its recording (spec §3.2).
//! - Per-word confidence scoring and tiering (high/medium/low) that must be
//!   retained for the life of the project (spec §7.1). Scores come from the
//!   engine and are kept as-is; tiers come from `tpt-app-voice-studio-core`.
//! - Multi-track alignment reconciliation across imperfectly synchronised
//!   microphone tracks, so edits map back to the correct source recording and
//!   sample range (spec §7.2). Not yet implemented.

use std::collections::HashMap;
use std::time::Duration;

use tpt_app_voice_studio_core::id::RecordingId;
use tpt_app_voice_studio_model::transcript::Transcript;

/// What [`normalize_timing`] changed. A non-zero count means the engine's
/// alignment was malformed and has been repaired, so callers can surface it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TimingReport {
    /// Words whose start or end was adjusted.
    pub adjusted_words: usize,
}

/// Repairs word timings so every recording's words are bound inside its
/// timeline and are ordered without overlap (spec §3.2, §7).
///
/// Per recording, in playback order:
/// - times are clamped to `[0, max_end]`, the recording's duration;
/// - `end` is never before `start`;
/// - `start` is never before the previous word's `end` on the same recording,
///   so words cannot overlap.
///
/// Words are never dropped, so no transcript text is lost; repaired words
/// may collapse to a zero-length span at a boundary.
///
/// `max_end` applies to every recording in `transcript`. Callers with several
/// recordings of different lengths should split the transcript per recording.
pub fn normalize_timing(transcript: &mut Transcript, max_end: Duration) -> TimingReport {
    let mut cursors: HashMap<RecordingId, Duration> = HashMap::new();
    let mut report = TimingReport::default();

    for word in transcript
        .segments
        .iter_mut()
        .flat_map(|segment| segment.words.iter_mut())
    {
        let original = (word.start, word.end);
        let cursor = cursors
            .get(&word.source_recording)
            .copied()
            .unwrap_or(Duration::ZERO);

        // The cursor is a previous `end`, so it never exceeds `max_end`; the
        // `max` chain therefore keeps `start <= end <= max_end` and `start >= cursor`.
        let start = word.start.min(max_end).max(cursor);
        let end = word.end.min(max_end).max(start);

        if (start, end) != original {
            report.adjusted_words += 1;
        }
        word.start = start;
        word.end = end;
        cursors.insert(word.source_recording, end);
    }
    report
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    use super::*;

    fn word(start_ms: u64, end_ms: u64, recording: u64) -> Word {
        Word {
            text: "w".to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(recording),
        }
    }

    fn transcript(words: Vec<Word>) -> Transcript {
        Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: None,
            words,
        }])
    }

    fn spans(transcript: &Transcript) -> Vec<(u128, u128)> {
        transcript
            .words()
            .map(|w| (w.start.as_millis(), w.end.as_millis()))
            .collect()
    }

    #[test]
    fn well_formed_alignment_is_left_untouched() {
        let mut t = transcript(vec![
            word(0, 500, 1),
            word(500, 900, 1),
            word(1_000, 1_500, 1),
        ]);
        let before = t.clone();
        let report = normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(report.adjusted_words, 0);
        assert_eq!(t, before);
    }

    #[test]
    fn inverted_spans_are_collapsed_not_dropped() {
        let mut t = transcript(vec![word(800, 300, 1)]);
        let report = normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(report.adjusted_words, 1);
        assert_eq!(spans(&t), vec![(800, 800)]);
        assert_eq!(t.word_count(), 1, "no word is lost");
    }

    #[test]
    fn overlapping_words_are_pushed_after_the_previous_word() {
        let mut t = transcript(vec![word(0, 600, 1), word(400, 900, 1)]);
        let report = normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(report.adjusted_words, 1);
        assert_eq!(spans(&t), vec![(0, 600), (600, 900)]);
        assert!(t.is_time_ordered());
    }

    #[test]
    fn times_beyond_the_recording_are_clamped_to_its_duration() {
        let mut t = transcript(vec![word(9_000, 12_000, 1)]);
        let report = normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(report.adjusted_words, 1);
        assert_eq!(spans(&t), vec![(9_000, 10_000)]);
    }

    #[test]
    fn words_past_the_end_collapse_to_the_boundary() {
        let mut t = transcript(vec![word(11_000, 12_000, 1)]);
        normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(spans(&t), vec![(10_000, 10_000)]);
    }

    #[test]
    fn ordering_is_tracked_per_recording() {
        // Recording 2 starting at 0 must not be pushed past recording 1's end.
        let mut t = transcript(vec![word(0, 2_000, 1), word(500, 900, 2)]);
        let report = normalize_timing(&mut t, Duration::from_secs(10));
        assert_eq!(report.adjusted_words, 0);
        assert_eq!(spans(&t), vec![(0, 2_000), (500, 900)]);
    }
}
