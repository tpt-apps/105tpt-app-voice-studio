//! Clip/highlight selection against the rendered timeline (spec §11).
//!
//! A clip is a rendered-time selection over the edited transcript. This
//! module computes exactly which words it contains and — for words cut at
//! the selection boundary — which portion of each word's *source* audio the
//! renderer must include. The result is the complete cutting instruction for
//! a standalone clip file; the audio render itself lands with the codec
//! integration (`tpt-cadence`).

use std::time::Duration;

use tpt_app_voice_studio_core::time::TimeRange;
use tpt_app_voice_studio_edit::timeline::render_timeline;
use tpt_app_voice_studio_model::transcript::{Transcript, Word};

/// One word of a clip, with its source and rendered ranges clipped to the
/// selection.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ClipWord<'a> {
    /// The transcript word (its text, confidence, and source binding).
    pub word: &'a Word,
    /// The portion of the word's source audio included in the clip (clamped
    /// proportionally when the selection boundary falls inside the word).
    pub source: TimeRange,
    /// Where this portion lands inside the produced clip (starts at zero).
    pub rendered: TimeRange,
}

/// The complete cutting instruction for one clip.
#[derive(Clone, PartialEq, Debug)]
pub struct ClipSelection<'a> {
    /// The clip's words in playback order; `rendered` ranges are contiguous
    /// from zero.
    pub words: Vec<ClipWord<'a>>,
    /// Total rendered duration of the clip.
    pub duration: Duration,
}

/// Selects the part of the edited transcript whose rendered time intersects
/// `range`, rebased so the clip starts at zero.
///
/// Words partially covered by the selection are included with their source
/// range clamped proportionally, so the clip boundary falls inside the word
/// at the same relative position in source and output.
#[must_use]
pub fn select_clip(transcript: &Transcript, range: TimeRange) -> ClipSelection<'_> {
    let mut words = Vec::new();
    for entry in render_timeline(transcript) {
        let Some(overlap) = entry.rendered.intersection(range) else {
            continue;
        };
        let source = clamp_source_to_rendered(entry.word, entry.rendered, overlap);
        // `overlap ⊆ range` by construction, so these never underflow;
        // saturating_sub keeps the invariant explicit.
        words.push(ClipWord {
            word: entry.word,
            source,
            rendered: TimeRange {
                start: overlap.start.saturating_sub(range.start),
                end: overlap.end.saturating_sub(range.start),
            },
        });
    }
    let duration = words.last().map(|w| w.rendered.end).unwrap_or_default();
    ClipSelection { words, duration }
}

/// Maps a rendered-time sub-range of a word back onto the word's source
/// range, proportionally.
fn clamp_source_to_rendered(word: &Word, rendered: TimeRange, overlap: TimeRange) -> TimeRange {
    let source = word.range();
    let span = rendered.duration();
    if span.is_zero() {
        return source;
    }
    let fraction_start = overlap.start.saturating_sub(rendered.start);
    let fraction_end = overlap.end.saturating_sub(rendered.start);
    let source_start = source.start + scale(source.duration(), fraction_start, span);
    let source_end = source.start + scale(source.duration(), fraction_end, span);
    TimeRange {
        start: source_start,
        end: source_end,
    }
}

/// Scales `offset` (a position within `span`) proportionally onto `length`.
fn scale(length: Duration, offset: Duration, span: Duration) -> Duration {
    if span.is_zero() {
        return Duration::ZERO;
    }
    let scaled = length.as_secs_f64() * (offset.as_secs_f64() / span.as_secs_f64());
    Duration::from_secs_f64(scaled)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::RecordingId;
    use tpt_app_voice_studio_core::id::SegmentId;
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    use super::*;

    fn word(text: &str, start_ms: u64, end_ms: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_millis(start_ms),
            end: Duration::from_millis(end_ms),
            confidence: Confidence::new(0.9).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    /// Rendered timeline: "a" [0,1s) "b" [1,3s) "c" [3,4s);
    /// sources carry long original gaps.
    fn transcript() -> Transcript {
        Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: None,
            words: vec![
                word("a", 0, 1_000),
                word("b", 5_000, 7_000),
                word("c", 20_000, 21_000),
            ],
        }])
    }

    #[test]
    fn full_range_clip_covers_everything_rebased_to_zero() {
        let transcript_local = transcript();
        let full = TimeRange::new(Duration::ZERO, Duration::from_secs(4)).expect("valid");
        let clip = select_clip(&transcript_local, full);
        assert_eq!(clip.words.len(), 3);
        assert_eq!(clip.duration, Duration::from_secs(4));
        assert_eq!(clip.words[0].rendered.start, Duration::ZERO);
        assert_eq!(clip.words[2].rendered.end, Duration::from_secs(4));
        // Unclipped words keep their full source ranges.
        assert_eq!(
            clip.words[1].source,
            TimeRange::new(Duration::from_secs(5), Duration::from_secs(7)).expect("v")
        );
    }

    #[test]
    fn boundary_words_are_clamped_proportionally() {
        let selection = TimeRange::new(Duration::from_millis(500), Duration::from_millis(3_500))
            .expect("valid");
        let transcript_local = transcript();
        let clip = select_clip(&transcript_local, selection);

        let texts: Vec<&str> = clip.words.iter().map(|w| w.word.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "b", "c"]);
        assert_eq!(clip.duration, Duration::from_secs(3));

        // "a" is cut in half: source [0.5s, 1s).
        assert_eq!(
            clip.words[0].source,
            TimeRange::new(Duration::from_millis(500), Duration::from_millis(1_000))
                .expect("valid")
        );
        // "b" is fully inside: untouched.
        assert_eq!(
            clip.words[1].source,
            TimeRange::new(Duration::from_secs(5), Duration::from_secs(7)).expect("valid")
        );
        // "c" is cut in half: source [20s, 20.5s).
        assert_eq!(
            clip.words[2].source,
            TimeRange::new(Duration::from_secs(20), Duration::from_millis(20_500)).expect("valid")
        );

        // Rendered positions are contiguous and rebased to the selection.
        assert_eq!(
            clip.words[0].rendered,
            TimeRange::new(Duration::ZERO, Duration::from_millis(500)).expect("v")
        );
        assert_eq!(
            clip.words[1].rendered,
            TimeRange::new(Duration::from_millis(500), Duration::from_millis(2_500)).expect("v")
        );
        assert_eq!(
            clip.words[2].rendered,
            TimeRange::new(Duration::from_millis(2_500), Duration::from_secs(3)).expect("v")
        );
    }

    #[test]
    fn empty_and_out_of_range_selections_produce_empty_clips() {
        let transcript_local = transcript();
        let empty = select_clip(&transcript_local, TimeRange::default());
        assert!(empty.words.is_empty());
        assert_eq!(empty.duration, Duration::ZERO);

        let beyond =
            TimeRange::new(Duration::from_secs(100), Duration::from_secs(200)).expect("valid");
        let transcript_local = transcript();
        let clip = select_clip(&transcript_local, beyond);
        assert!(clip.words.is_empty());
    }

    #[test]
    fn clip_selection_tracks_the_edited_transcript_not_the_original() {
        // Deleting "b" from the session shifts the rendered timeline; a clip
        // over the same rendered range now covers different source audio.
        let mut session = tpt_app_voice_studio_edit::EditSession::new(Vec::new(), transcript());
        session
            .apply(tpt_app_voice_studio_model::EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 1..2,
            })
            .expect("valid");
        let selection = TimeRange::new(Duration::ZERO, Duration::from_secs(2)).expect("valid");
        let clip = select_clip(session.working(), selection);
        let texts: Vec<&str> = clip.words.iter().map(|w| w.word.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "c"]);
        // "c" now renders at [1,2) and its source is fully included.
        assert_eq!(
            clip.words[1].source,
            TimeRange::new(Duration::from_secs(20), Duration::from_secs(21)).expect("v")
        );
        assert_eq!(clip.words[1].rendered.start, Duration::from_secs(1));
    }
}
