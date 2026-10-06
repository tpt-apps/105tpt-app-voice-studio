//! Long-silence detection between transcribed words (spec §8.1, §10).
//!
//! Silence is detected as gaps between consecutive words on each recording's
//! source timeline, plus optional leading/trailing quiet before the first and
//! after the last word. Threshold and retained padding are configurable
//! (spec §10: "configurable threshold and minimum retained gap").
//!
//! This is transcript-driven detection: it finds quiet spans *between
//! spoken words*. Audio-true silence analysis (room tone, breaths inside
//! words) refines this once the `tpt-dsp` foundation lands.

use std::time::Duration;

use tpt_app_voice_studio_core::time::TimeRange;
use tpt_app_voice_studio_core::id::RecordingId;
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::recording::Recording;
use tpt_app_voice_studio_model::transcript::Transcript;

/// Configuration for silence detection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SilenceConfig {
    /// Gaps at least this long are reported.
    pub min_gap: Duration,
    /// Padding kept on each side of a detected gap when producing the trim
    /// range, so speech is never clipped and natural pauses survive.
    pub retained_gap: Duration,
    /// Also report quiet before the first word and after the last word of a
    /// recording.
    pub include_leading_trailing: bool,
}

impl Default for SilenceConfig {
    fn default() -> Self {
        Self {
            min_gap: Duration::from_secs(1),
            retained_gap: Duration::from_millis(250),
            include_leading_trailing: true,
        }
    }
}

/// One detected quiet span with the trim that would shorten it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SilenceSuggestion {
    /// The recording the quiet span lies on.
    pub recording: RecordingId,
    /// The full extent of the gap between words.
    pub gap: TimeRange,
    /// The trim range that would shorten the gap to [`SilenceConfig::
    /// retained_gap`] of padding on each side. Always non-empty and within
    /// `gap`.
    pub trim: TimeRange,
}

impl SilenceSuggestion {
    /// The trim operation a review panel's "Trim" action applies.
    #[must_use]
    pub fn to_operation(&self) -> EditOperation {
        EditOperation::Trim {
            recording: self.recording,
            range: self.trim,
        }
    }
}

/// Scans every recording that has transcribed words for quiet spans.
///
/// A span is reported when it is at least [`SilenceConfig::min_gap`] long
/// *and* trimming it can keep the configured padding on both sides (i.e. the
/// gap is longer than twice the retained gap); shorter gaps are already at
/// or below the minimum kept silence and are left alone.
#[must_use]
pub fn detect_silences(
    transcript: &Transcript,
    recordings: &[Recording],
    config: &SilenceConfig,
) -> Vec<SilenceSuggestion> {
    let mut suggestions = Vec::new();
    for recording in recordings {
        let mut words: Vec<_> = transcript
            .words()
            .filter(|w| w.source_recording == recording.id)
            .map(|w| w.range())
            .collect();
        if words.is_empty() {
            continue;
        }
        words.sort_by_key(|r| r.start);

        if config.include_leading_trailing {
            let lead = TimeRange::new(Duration::ZERO, words[0].start).expect("ordered");
            push_if_trimmable(&mut suggestions, recording.id, lead, config);
        }

        for pair in words.windows(2) {
            let gap = TimeRange::new(pair[0].end, pair[1].start).expect("ordered words");
            push_if_trimmable(&mut suggestions, recording.id, gap, config);
        }

        if config.include_leading_trailing {
            let last = words[words.len() - 1];
            let tail = TimeRange::new(last.end, recording.duration).expect("ordered");
            push_if_trimmable(&mut suggestions, recording.id, tail, config);
        }
    }
    suggestions
}

/// Records a gap when it passes the threshold and leaves a non-empty trim.
fn push_if_trimmable(
    suggestions: &mut Vec<SilenceSuggestion>,
    recording: RecordingId,
    gap: TimeRange,
    config: &SilenceConfig,
) {
    if gap.duration() < config.min_gap {
        return;
    }
    let trim_start = gap.start + config.retained_gap;
    let trim_end = gap.end.saturating_sub(config.retained_gap);
    let Ok(trim) = TimeRange::new(trim_start, trim_end) else {
        return; // padding does not fit: gap is already near the minimum kept
    };
    if trim.is_empty() {
        return;
    }
    suggestions.push(SilenceSuggestion {
        recording,
        gap,
        trim,
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
    use tpt_app_voice_studio_model::recording::{Recording, RecordingFingerprint, TrackRole};
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};
    use std::path::PathBuf;

    use super::*;

    fn recording(id: u64, duration_secs: u64) -> Recording {
        Recording {
            id: RecordingId::new(id),
            path: PathBuf::from(format!("{id}.wav")),
            fingerprint: RecordingFingerprint(id),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(duration_secs),
            sample_rate: 48_000,
            channels: 1,
        }
    }

    fn word(recording: u64, start: u64, end: u64) -> Word {
        Word {
            text: "w".to_string(),
            start: Duration::from_secs(start),
            end: Duration::from_secs(end),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(recording),
        }
    }

    fn transcript_of(words: Vec<Word>) -> Transcript {
        Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: None,
            words,
        }])
    }

    #[test]
    fn detects_interior_leading_and_trailing_silence() {
        let transcript = transcript_of(vec![
            word(1, 2, 3),   // leading quiet: 0-2s
            word(1, 7, 8),   // interior gap: 3-7s
            word(1, 9, 10),  // trailing quiet: 10-14s
        ]);
        let recs = vec![recording(1, 14)];
        let suggestions = detect_silences(&transcript, &recs, &SilenceConfig::default());

        let gaps: Vec<(u64, u64)> = suggestions
            .iter()
            .map(|s| (s.gap.start.as_secs(), s.gap.end.as_secs()))
            .collect();
        assert_eq!(gaps, vec![(0, 2), (3, 7), (10, 14)]);
    }

    #[test]
    fn trim_ranges_keep_retained_padding() {
        let transcript = transcript_of(vec![word(1, 0, 1), word(1, 7, 8)]);
        let recs = vec![recording(1, 8)];
        let config = SilenceConfig {
            min_gap: Duration::from_secs(2),
            retained_gap: Duration::from_millis(500),
            include_leading_trailing: false,
        };
        let suggestions = detect_silences(&transcript, &recs, &config);
        assert_eq!(suggestions.len(), 1);
        let s = &suggestions[0];
        assert_eq!(s.gap, TimeRange::new(Duration::from_secs(1), Duration::from_secs(7)).expect("v"));
        assert_eq!(
            s.trim,
            TimeRange::new(
                Duration::from_millis(1_500),
                Duration::from_millis(6_500),
            )
            .expect("v")
        );
        // The suggestion converts into a real trim operation.
        assert_eq!(
            s.to_operation(),
            EditOperation::Trim {
                recording: RecordingId::new(1),
                range: s.trim,
            }
        );
    }

    #[test]
    fn gaps_below_threshold_or_without_room_for_padding_are_skipped() {
        // gap1 = 1.5s: above min_gap (1s) but below 2×retained (1.5s), so no
        // trim fits and it is left alone. gap2 = 2.5s: reported.
        let transcript = transcript_of(vec![
            word(1, 0, 1),
            word(1, 2_500, 3_500), // 1.5s gap before this word
            word(1, 6_000, 7_000), // 2.5s gap before this word
        ]);
        let recs = vec![recording(1, 7)];
        let config = SilenceConfig {
            min_gap: Duration::from_secs(1),
            retained_gap: Duration::from_millis(750),
            include_leading_trailing: false,
        };
        let suggestions = detect_silences(&transcript, &recs, &config);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].gap,
            TimeRange::new(Duration::from_millis(3_500), Duration::from_millis(6_000))
                .expect("valid")
        );
    }

    #[test]
    fn trim_suggestions_apply_cleanly_to_a_session() {
        let transcript = transcript_of(vec![word(1, 0, 1), word(1, 8, 9)]);
        let recs = vec![recording(1, 9)];
        let suggestions = detect_silences(&transcript, &recs, &SilenceConfig::default());

        let mut session = crate::session::EditSession::new(recs, transcript);
        for suggestion in &suggestions {
            session
                .apply(suggestion.to_operation())
                .expect("validated trim");
        }
        // All words survive a trim that only removes quiet between them.
        assert_eq!(session.working().word_count(), 2);
    }

    #[test]
    fn recordings_without_words_are_skipped_and_tracks_are_separate() {
        let mut transcript = transcript_of(vec![word(1, 0, 1), word(1, 5, 6)]);
        // A second recording's word interleaves on recording 1's timeline;
        // per-recording grouping must keep its gap analysis independent.
        transcript.segments[0].words.push(word(2, 2, 3));

        let recs = vec![recording(1, 10), recording(2, 10)];
        let suggestions = detect_silences(&transcript, &recs, &SilenceConfig::default());

        let on_rec1: Vec<_> = suggestions
            .iter()
            .filter(|s| s.recording == RecordingId::new(1))
            .collect();
        let on_rec2: Vec<_> = suggestions
            .iter()
            .filter(|s| s.recording == RecordingId::new(2))
            .collect();
        // Recording 1: interior gap 1-5s and trailing 6-10s. The word at
        // 2-3s belongs to recording 2 and does not close recording 1's gap.
        assert_eq!(on_rec1.len(), 2);
        assert_eq!(on_rec1[0].gap.start, Duration::from_secs(1));
        assert_eq!(on_rec1[1].gap.start, Duration::from_secs(6));
        // Recording 2: leading 0-2s and trailing 3-10s around its one word.
        assert_eq!(on_rec2.len(), 2);
        assert_eq!(on_rec2[0].gap.end, Duration::from_secs(2));
        assert_eq!(on_rec2[1].gap.start, Duration::from_secs(3));
    }

    #[test]
    fn include_leading_trailing_can_be_disabled() {
        let transcript = transcript_of(vec![word(1, 3, 4), word(1, 8, 9)]);
        let recs = vec![recording(1, 12)];
        let config = SilenceConfig {
            include_leading_trailing: false,
            ..SilenceConfig::default()
        };
        let suggestions = detect_silences(&transcript, &recs, &config);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].gap.start, Duration::from_secs(4));
    }
}
