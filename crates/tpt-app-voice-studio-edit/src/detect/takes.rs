//! Multi-take detection (spec §8.2): finds the same line delivered in two
//! recordings, so the editor can pick the better take.
//!
//! Two segments are alternate takes when they come from different recordings
//! and their normalised word sequences are similar enough. Similarity is the
//! longest-common-subsequence length over the longer segment's word count, so
//! it is 1.0 for identical text and drops as words are reworded or dropped.
//!
//! Like the other detectors this only *suggests*: each pair converts into a
//! [`EditOperation::SelectTake`] that the editor applies explicitly (spec §3.5).

use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::transcript::{Segment, Transcript};

/// Configuration for take detection.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TakeConfig {
    /// Minimum LCS similarity, in `[0, 1]`, for two segments to be takes.
    pub min_similarity: f32,
    /// Segments with fewer normalised words than this are ignored: a short
    /// phrase such as "yeah" repeats by chance and is not a delivery of a line.
    pub min_words: usize,
}

impl Default for TakeConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.8,
            min_words: 3,
        }
    }
}

/// One side of a take pair: a segment and the recording it was spoken on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Take {
    /// The segment carrying this delivery.
    pub segment: SegmentId,
    /// The recording the delivery was spoken on.
    pub recording: RecordingId,
}

/// Two alternate deliveries of the same line.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TakePair {
    /// The take earlier in playback order.
    pub first: Take,
    /// The take later in playback order.
    pub second: Take,
    /// LCS similarity of the two segments' normalised words.
    pub similarity: f32,
}

impl TakePair {
    /// The operation that keeps `chosen` as the take for its segment.
    #[must_use]
    pub fn select(&self, chosen: Take) -> EditOperation {
        EditOperation::SelectTake {
            segment: chosen.segment,
            recording: chosen.recording,
        }
    }
}

/// Finds alternate takes across recordings.
///
/// Only pairs on *different* recordings are reported; repeats within one
/// recording are retakes, not alternate recordings of the line. Pairs come
/// back in playback order of their first take.
///
/// Note: this compares every cross-recording segment pair, which is quadratic
/// in segment count. Length-ratio pruning keeps it practical for interviews;
/// a windowed or hashed index is the next step for very long projects.
#[must_use]
pub fn detect_takes(transcript: &Transcript, config: &TakeConfig) -> Vec<TakePair> {
    let candidates: Vec<(Take, Vec<String>)> = transcript
        .segments
        .iter()
        .filter_map(|segment| candidate(segment, config))
        .collect();

    let mut pairs = Vec::new();
    for (i, (a, words_a)) in candidates.iter().enumerate() {
        for (b, words_b) in &candidates[i + 1..] {
            if a.recording == b.recording {
                continue;
            }
            if !length_compatible(words_a.len(), words_b.len(), config.min_similarity) {
                continue;
            }
            let similarity = similarity(words_a, words_b);
            if similarity >= config.min_similarity {
                pairs.push(TakePair {
                    first: *a,
                    second: *b,
                    similarity,
                });
            }
        }
    }
    pairs
}

/// The take a segment represents and its normalised words, if it is long
/// enough to compare. The recording is taken from the segment's first word.
fn candidate(segment: &Segment, config: &TakeConfig) -> Option<(Take, Vec<String>)> {
    let recording = segment.words.first()?.source_recording;
    let words: Vec<String> = segment
        .words
        .iter()
        .filter_map(|w| normalise(&w.text))
        .collect();
    (words.len() >= config.min_words).then_some((
        Take {
            segment: segment.id,
            recording,
        },
        words,
    ))
}

/// Lower-cases a word and strips punctuation, so "Go," and "go" match.
/// A token that is only punctuation yields nothing.
fn normalise(text: &str) -> Option<String> {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// Upper bound on similarity from lengths alone: LCS ≤ the shorter length, so
/// similarity ≤ shorter / longer. Pruning on it skips the LCS entirely.
fn length_compatible(a: usize, b: usize, min_similarity: f32) -> bool {
    let (short, long) = if a <= b { (a, b) } else { (b, a) };
    #[allow(clippy::cast_precision_loss)] // word counts are far below f32 precision limits
    let bound = short as f32 / long as f32;
    bound >= min_similarity
}

/// LCS length over the longer sequence's length, in `[0, 1]`.
fn similarity(a: &[String], b: &[String]) -> f32 {
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0.0;
    }
    let mut previous = vec![0_usize; b.len() + 1];
    let mut current = vec![0_usize; b.len() + 1];
    for word_a in a {
        for (j, word_b) in b.iter().enumerate() {
            current[j + 1] = if word_a == word_b {
                previous[j] + 1
            } else {
                previous[j + 1].max(current[j])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    #[allow(clippy::cast_precision_loss)] // word counts are far below f32 precision limits
    {
        previous[b.len()] as f32 / longest as f32
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_model::recording::{Recording, RecordingFingerprint, TrackRole};
    use tpt_app_voice_studio_model::transcript::Word;

    use crate::session::EditSession;

    use super::*;

    fn word(text: &str, recording: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::ZERO,
            end: Duration::from_millis(100),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(recording),
        }
    }

    fn segment(id: u64, text: &str, recording: u64) -> Segment {
        Segment {
            id: SegmentId::new(id),
            speaker: None,
            words: text
                .split_whitespace()
                .map(|t| word(t, recording))
                .collect(),
        }
    }

    fn recording(id: u64) -> Recording {
        Recording {
            id: RecordingId::new(id),
            path: PathBuf::from(format!("take-{id}.wav")),
            fingerprint: RecordingFingerprint(id),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(60),
            sample_rate: 48_000,
            channels: 1,
        }
    }

    #[test]
    fn same_line_on_two_recordings_is_a_take_pair_despite_punctuation() {
        let transcript = Transcript::from_segments(vec![
            segment(1, "We should go now.", 1),
            segment(2, "we should go, now", 2),
        ]);
        let pairs = detect_takes(&transcript, &TakeConfig::default());
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].first.segment, SegmentId::new(1));
        assert_eq!(pairs[0].second.segment, SegmentId::new(2));
        assert!((pairs[0].similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn reworded_lines_below_threshold_are_not_takes() {
        // LCS("we should go now", "we should leave now") = 3 of 4 = 0.75 < 0.8.
        let transcript = Transcript::from_segments(vec![
            segment(1, "we should go now", 1),
            segment(2, "we should leave now", 2),
        ]);
        assert!(detect_takes(&transcript, &TakeConfig::default()).is_empty());
    }

    #[test]
    fn repeats_within_one_recording_are_retakes_not_alternates() {
        let transcript = Transcript::from_segments(vec![
            segment(1, "we should go now", 1),
            segment(2, "we should go now", 1),
        ]);
        assert!(detect_takes(&transcript, &TakeConfig::default()).is_empty());
    }

    #[test]
    fn very_short_phrases_are_ignored() {
        let transcript =
            Transcript::from_segments(vec![segment(1, "yeah okay", 1), segment(2, "yeah okay", 2)]);
        assert!(detect_takes(&transcript, &TakeConfig::default()).is_empty());
    }

    #[test]
    fn selecting_a_take_applies_cleanly_to_a_session() {
        let transcript = Transcript::from_segments(vec![
            segment(1, "this is the line", 1),
            segment(2, "this is the line", 2),
        ]);
        let recordings = vec![recording(1), recording(2)];
        let pairs = detect_takes(&transcript, &TakeConfig::default());
        let pair = pairs[0];

        let mut session = EditSession::new(recordings, transcript);
        session
            .apply(pair.select(pair.second))
            .expect("selecting a detected take is a valid edit");
        assert_eq!(session.history().len(), 1);
        assert_eq!(session.history()[0], pair.select(pair.second));
    }
}
