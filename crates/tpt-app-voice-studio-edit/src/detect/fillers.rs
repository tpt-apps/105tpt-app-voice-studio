//! Filler-word detection against a configurable per-language list
//! (spec §8.1).
//!
//! Detection is advisory: a [`FillerSuggestion`] maps to the
//! [`EditOperation::DeleteRange`] that would remove it, and nothing is
//! applied until the editor accepts it.

use std::collections::HashSet;

use tpt_app_voice_studio_core::id::SegmentId;
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::transcript::Transcript;

/// A case-insensitive set of filler words/phrases for one language.
///
/// Entries are normalized (lowercased, surrounding punctuation stripped) at
/// construction, so "Um," in a transcript matches the entry `um`. Multi-word
/// entries such as `you know` match across consecutive transcript words.
#[derive(Clone, Debug)]
pub struct FillerList {
    entries: HashSet<String>,
    max_entry_words: usize,
}

impl FillerList {
    /// Builds a list from per-language entries.
    #[must_use]
    pub fn from_entries<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut normalized = HashSet::new();
        let mut max_entry_words = 1;
        for entry in entries {
            let words: Vec<String> = entry
                .as_ref()
                .split_whitespace()
                .map(normalize_token)
                .collect();
            if words.is_empty() {
                continue;
            }
            max_entry_words = max_entry_words.max(words.len());
            normalized.insert(words.join(" "));
        }
        Self {
            entries: normalized,
            max_entry_words,
        }
    }

    /// The default English filler list.
    ///
    /// Deliberately conservative: interjections and hedge phrases only.
    /// Content words that can be legitimate ("like", "so", "actually") are
    /// left out of the default and can be added by the user per project.
    #[must_use]
    pub fn english() -> Self {
        Self::from_entries([
            "um", "uh", "er", "ah", "erm", "hmm", "mm", "mmm", "you know", "i mean",
        ])
    }

    /// True if `phrase` (one or more normalized words joined by spaces) is a
    /// filler.
    #[must_use]
    pub fn contains(&self, phrase: &str) -> bool {
        let words: Vec<String> = phrase
            .split_whitespace()
            .map(normalize_token)
            .collect();
        self.entries.contains(&words.join(" "))
    }

    /// Length (in words) of the longest entry, bounding match windows.
    #[must_use]
    pub fn max_entry_words(&self) -> usize {
        self.max_entry_words
    }
}

/// One detected filler run, addressable for removal.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FillerSuggestion {
    /// The segment containing the filler.
    pub segment: SegmentId,
    /// Half-open range of word indices within the segment.
    pub words: std::ops::Range<usize>,
    /// The matched words as they appear in the transcript (joined by spaces,
    /// original casing/punctuation).
    pub text: String,
}

impl FillerSuggestion {
    /// The non-destructive delete that removes this filler — the operation a
    /// review panel's "Remove" action applies (spec §8.1).
    #[must_use]
    pub fn to_operation(&self) -> EditOperation {
        EditOperation::DeleteRange {
            segment: self.segment,
            word_range: self.words.clone(),
        }
    }
}

/// Scans a transcript for filler runs, longest-match-first, non-overlapping.
///
/// Matches are case-insensitive and ignore surrounding punctuation, but are
/// word-boundary exact ("um" does not match inside "ultimate").
#[must_use]
pub fn detect_fillers(transcript: &Transcript, fillers: &FillerList) -> Vec<FillerSuggestion> {
    let mut suggestions = Vec::new();
    for segment in &transcript.segments {
        let words: Vec<&str> = segment.words.iter().map(|w| w.text.as_str()).collect();
        let mut index = 0;
        while index < words.len() {
            let mut matched = 0;
            for window in (1..=fillers.max_entry_words()).rev() {
                if index + window > words.len() {
                    continue;
                }
                let candidate: Vec<String> = words[index..index + window]
                    .iter()
                    .map(|w| normalize_token(w))
                    .collect();
                if candidate.iter().any(String::is_empty) {
                    continue;
                }
                if fillers.entries.contains(&candidate.join(" ")) {
                    matched = window;
                    break;
                }
            }
            if matched > 0 {
                suggestions.push(FillerSuggestion {
                    segment: segment.id,
                    words: index..index + matched,
                    text: words[index..index + matched].join(" "),
                });
                index += matched;
            } else {
                index += 1;
            }
        }
    }
    suggestions
}

/// Lowercases and strips leading/trailing punctuation, keeping interior
/// characters (so `mm-hmm` stays intact).
fn normalize_token(token: &str) -> String {
    token
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    use super::*;

    fn word(text: &str) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::ZERO,
            end: Duration::from_secs(1),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    fn transcript_of(words: &[&str]) -> Transcript {
        Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: Some(SpeakerId::new(1)),
            words: words.iter().map(|t| word(t)).collect(),
        }])
    }

    fn texts(suggestions: &[FillerSuggestion]) -> Vec<String> {
        suggestions.iter().map(|s| s.text.clone()).collect()
    }

    #[test]
    fn default_list_matches_interjections_and_phrases() {
        let list = FillerList::english();
        assert!(list.contains("um"));
        assert!(list.contains("Um,"));
        assert!(list.contains("you know"));
        assert!(list.contains("You know,"));
        assert!(!list.contains("ultimate"));
        assert!(!list.contains("know"));
    }

    #[test]
    fn detects_longest_match_first_and_skips_ahead() {
        // "you know" must match as one phrase, not as "you" + "know".
        let transcript = transcript_of(&["Well", "you", "know", "it's", "fine"]);
        let list = FillerList::english();
        let suggestions = detect_fillers(&transcript, &list);
        assert_eq!(texts(&suggestions), vec!["you know"]);
        assert_eq!(suggestions[0].words, 1..3);
    }

    #[test]
    fn punctuation_and_case_are_ignored_but_words_are_exact() {
        let transcript = transcript_of(&["Um,", "(uh)", "understand"]);
        let suggestions = detect_fillers(&transcript, &FillerList::english());
        assert_eq!(texts(&suggestions), vec!["Um,", "(uh)"]);
    }

    #[test]
    fn suggestions_span_segments_and_map_to_delete_operations() {
        let mut transcript = transcript_of(&["um"]);
        transcript.segments.push(Segment {
            id: SegmentId::new(2),
            speaker: None,
            words: vec![word("like"), word("so")],
        });
        let suggestions = detect_fillers(&transcript, &FillerList::english());
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].segment, SegmentId::new(1));

        let op = suggestions[0].to_operation();
        assert_eq!(
            op,
            EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 0..1,
            }
        );
    }

    #[test]
    fn detected_suggestions_apply_cleanly_to_a_session() {
        let transcript = transcript_of(&["um", "hello", "you", "know", "world"]);
        let suggestions = detect_fillers(&transcript, &FillerList::english());
        assert_eq!(suggestions.len(), 2);

        // Apply each suggestion through the real EDL engine, in reverse
        // index order so earlier indices stay valid.
        let mut session = crate::session::EditSession::new(Vec::new(), transcript);
        for suggestion in suggestions.iter().rev() {
            session.apply(suggestion.to_operation()).expect("valid op");
        }
        let remaining: Vec<&str> = session.working().words().map(|w| w.text.as_str()).collect();
        assert_eq!(remaining, vec!["hello", "world"]);
    }

    #[test]
    fn empty_tokens_never_match() {
        let transcript = transcript_of(&["—"]);
        let suggestions = detect_fillers(&transcript, &FillerList::english());
        assert!(suggestions.is_empty());
    }

    #[test]
    fn custom_lists_are_configurable_per_language() {
        let list = FillerList::from_entries(["ähm", "sozusagen"]);
        let transcript = transcript_of(&["Ähm,", "sozusagen", "ja"]);
        let suggestions = detect_fillers(&transcript, &list);
        assert_eq!(texts(&suggestions), vec!["Ähm,", "sozusagen"]);
    }
}
