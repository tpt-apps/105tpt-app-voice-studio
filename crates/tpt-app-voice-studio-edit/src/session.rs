//! The edit session: original transcript + reversible operation history
//! (spec §3.3, §6.5, §8).

use std::collections::BTreeMap;

use tpt_app_voice_studio_core::error::{
    RangeExceedsDurationError, UnknownEntityError, WordRangeOutOfBoundsError,
};
use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
use tpt_app_voice_studio_core::time::TimeRange;
use tpt_app_voice_studio_model::edit_operation::EditOperation;
use tpt_app_voice_studio_model::recording::Recording;
use tpt_app_voice_studio_model::transcript::Transcript;

use crate::error::EditError;

/// A non-destructive editing session over one aligned transcript.
///
/// The session holds the original transcript (never mutated), the recordings
/// the transcript is bound to, and the ordered [`EditOperation`] history.
/// `working` is always the replay of the history against the original, so:
///
/// - every applied operation is undoable ([`EditSession::undo`]) and
///   redoable ([`EditSession::redo`]),
/// - the original audio binding survives any number of edits (spec §3.2),
/// - the original recording file is never touched (spec §3.3).
pub struct EditSession {
    original: Transcript,
    recordings: BTreeMap<RecordingId, Recording>,
    history: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    working: Transcript,
}

impl EditSession {
    /// Creates a session over a freshly aligned transcript.
    #[must_use]
    pub fn new(recordings: Vec<Recording>, original: Transcript) -> Self {
        let recordings: BTreeMap<RecordingId, Recording> =
            recordings.into_iter().map(|r| (r.id, r)).collect();
        Self {
            working: original.clone(),
            original,
            recordings,
            history: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Reconstructs a session with a saved edit history (e.g. loaded from a
    /// project file): every operation is validated and replayed against the
    /// original transcript.
    ///
    /// # Errors
    /// Returns the first [`EditError`] encountered while validating or
    /// replaying the history; the history in the file does not match the
    /// transcript it is applied to.
    pub fn from_history(
        recordings: Vec<Recording>,
        original: Transcript,
        history: Vec<EditOperation>,
    ) -> Result<Self, EditError> {
        let mut session = Self::new(recordings, original);
        for operation in history {
            session.apply(operation)?;
        }
        Ok(session)
    }

    /// The transcript reflecting every applied (not undone) operation.
    #[must_use]
    pub fn working(&self) -> &Transcript {
        &self.working
    }

    /// The original aligned transcript, unaffected by any edit.
    #[must_use]
    pub fn original(&self) -> &Transcript {
        &self.original
    }

    /// The applied, in-order edit history (the edit-decision-list).
    #[must_use]
    pub fn history(&self) -> &[EditOperation] {
        &self.history
    }

    /// The recording metadata the session validates against.
    #[must_use]
    pub fn recording(&self, id: RecordingId) -> Option<&Recording> {
        self.recordings.get(&id)
    }

    /// Validates `operation` against the current working state without
    /// applying it.
    ///
    /// # Errors
    /// Returns the specific [`EditError`] for unknown entities, out-of-bounds
    /// word/position indices, or ranges that are empty or exceed the
    /// recording duration.
    pub fn validate(&self, operation: &EditOperation) -> Result<(), EditError> {
        match operation {
            EditOperation::DeleteRange {
                segment,
                word_range,
            } => self.validate_delete_range(*segment, word_range.clone()),
            EditOperation::Reorder {
                segment,
                new_position,
            } => self.validate_reorder(*segment, *new_position),
            EditOperation::Trim { recording, range } | EditOperation::Mute { recording, range } => {
                self.validate_time_range(*recording, *range)
            }
            EditOperation::SelectTake { segment, recording } => {
                self.require_segment(*segment)?;
                self.require_recording(*recording)?;
                Ok(())
            }
        }
    }

    /// Validates and applies `operation`, appending it to the history.
    ///
    /// Any pending redo history is discarded (standard editor semantics).
    /// The working transcript is recomputed by replaying the extended
    /// history, so apply/undo/redo share one code path and can never
    /// disagree.
    ///
    /// # Errors
    /// See [`EditSession::validate`]. The session is unchanged on error.
    pub fn apply(&mut self, operation: EditOperation) -> Result<(), EditError> {
        self.validate(&operation)?;
        let mut extended = Vec::with_capacity(self.history.len() + 1);
        extended.extend(self.history.iter().cloned());
        extended.push(operation.clone());
        self.working = self
            .replay(&extended)
            .expect("validated operations must replay cleanly");
        self.history.push(operation);
        self.redo_stack.clear();
        Ok(())
    }

    /// Undoes the most recent operation, if any. Returns whether an
    /// operation was undone.
    pub fn undo(&mut self) -> bool {
        let Some(undone) = self.history.pop() else {
            return false;
        };
        self.redo_stack.push(undone);
        self.recompute_working();
        true
    }

    /// Re-applies the most recently undone operation, if any. Returns
    /// whether an operation was redone.
    pub fn redo(&mut self) -> bool {
        let Some(redone) = self.redo_stack.pop() else {
            return false;
        };
        self.working = self
            .replay(&{
                let mut ops = self.history.clone();
                ops.push(redone.clone());
                ops
            })
            .expect("previously validated operations must replay cleanly");
        self.history.push(redone);
        true
    }

    /// The time ranges muted against `recording` by the current history
    /// (spec §8: mute without deleting). The renderer/export stage applies
    /// these at playback/render time.
    #[must_use]
    pub fn muted_ranges(&self, recording: RecordingId) -> Vec<TimeRange> {
        self.history
            .iter()
            .filter_map(|op| match op {
                EditOperation::Mute {
                    recording: r,
                    range,
                } if *r == recording => Some(*range),
                _ => None,
            })
            .collect()
    }

    // --- validation helpers ---

    fn require_segment(&self, id: SegmentId) -> Result<usize, EditError> {
        self.working
            .segments
            .iter()
            .position(|s| s.id == id)
            .ok_or(EditError::UnknownEntity(UnknownEntityError::Segment(id)))
    }

    fn require_recording(&self, id: RecordingId) -> Result<&Recording, EditError> {
        self.recordings
            .get(&id)
            .ok_or(EditError::UnknownEntity(UnknownEntityError::Recording(id)))
    }

    fn validate_delete_range(
        &self,
        segment: SegmentId,
        word_range: std::ops::Range<usize>,
    ) -> Result<(), EditError> {
        let index = self.require_segment(segment)?;
        let word_count = self.working.segments[index].word_count();
        if word_range.is_empty() {
            return Err(EditError::EmptyWordRange { segment });
        }
        if word_range.end > word_count {
            return Err(EditError::WordRangeOutOfBounds(WordRangeOutOfBoundsError {
                segment,
                word_count,
                range: word_range,
            }));
        }
        Ok(())
    }

    fn validate_reorder(&self, segment: SegmentId, new_position: usize) -> Result<(), EditError> {
        self.require_segment(segment)?;
        let segment_count = self.working.segments.len();
        if new_position >= segment_count {
            return Err(EditError::PositionOutOfBounds {
                segment,
                position: new_position,
                segment_count,
            });
        }
        Ok(())
    }

    fn validate_time_range(
        &self,
        recording: RecordingId,
        range: TimeRange,
    ) -> Result<(), EditError> {
        let rec = self.require_recording(recording)?;
        if range.is_empty() {
            return Err(EditError::EmptyTimeRange { recording, range });
        }
        if range.end > rec.duration {
            return Err(EditError::RangeExceedsDuration(RangeExceedsDurationError {
                recording,
                duration: rec.duration,
                range,
            }));
        }
        Ok(())
    }

    // --- history replay ---

    fn recompute_working(&mut self) {
        self.working = self
            .replay(&self.history)
            .expect("validated operations must replay cleanly");
    }

    /// Replays `operations` against the original transcript, producing the
    /// working transcript for that history prefix.
    fn replay(&self, operations: &[EditOperation]) -> Result<Transcript, EditError> {
        let mut transcript = self.original.clone();
        for op in operations {
            match op {
                EditOperation::DeleteRange {
                    segment,
                    word_range,
                } => {
                    let seg = transcript.segment_mut(*segment).ok_or_else(|| {
                        EditError::ReplayInvariant(format!("missing segment {segment}"))
                    })?;
                    if word_range.end > seg.words.len() || word_range.is_empty() {
                        return Err(EditError::ReplayInvariant(format!(
                            "invalid word range {word_range:?} for segment {segment}"
                        )));
                    }
                    seg.words.drain(word_range.clone());
                }
                EditOperation::Reorder {
                    segment,
                    new_position,
                } => {
                    let index = transcript
                        .segments
                        .iter()
                        .position(|s| s.id == *segment)
                        .ok_or_else(|| {
                            EditError::ReplayInvariant(format!("missing segment {segment}"))
                        })?;
                    if *new_position >= transcript.segments.len() {
                        return Err(EditError::ReplayInvariant(format!(
                            "invalid reorder position {new_position} for segment {segment}"
                        )));
                    }
                    let seg = transcript.segments.remove(index);
                    transcript.segments.insert(*new_position, seg);
                }
                EditOperation::Trim { recording, range } => {
                    trim_words(&mut transcript, *recording, *range);
                }
                EditOperation::Mute { .. } | EditOperation::SelectTake { .. } => {
                    // Render-time concerns: no transcript-level effect.
                }
            }
        }
        Ok(transcript)
    }
}

/// Applies a trim: words fully inside the cut range are removed; partially
/// overlapping words are clamped to the cut boundaries (and dropped if that
/// empties them); a word spanning the whole cut range is kept intact, since
/// words are atomic in this model. Only words bound to `recording` are
/// affected.
fn trim_words(transcript: &mut Transcript, recording: RecordingId, range: TimeRange) {
    for segment in &mut transcript.segments {
        segment.words = segment
            .words
            .iter()
            .filter_map(|word| {
                if word.source_recording != recording {
                    return Some(word.clone());
                }
                if word.end <= range.start || word.start >= range.end {
                    return Some(word.clone()); // entirely outside the cut
                }
                let start = if word.start < range.start {
                    word.start
                } else {
                    range.end
                };
                let end = if word.end > range.end {
                    word.end
                } else {
                    range.start
                };
                if start < end {
                    let mut kept = word.clone();
                    kept.start = start;
                    kept.end = end;
                    Some(kept)
                } else {
                    None // word consisted only of the trimmed region
                }
            })
            .collect();
    }
}

/// Test helpers shared by the edit crate's unit tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::time::Duration;

    use tpt_app_voice_studio_core::confidence::Confidence;
    use tpt_app_voice_studio_core::id::{RecordingId, SegmentId, SpeakerId};
    use tpt_app_voice_studio_model::recording::{Recording, RecordingFingerprint, TrackRole};
    use tpt_app_voice_studio_model::transcript::{Segment, Transcript, Word};

    /// A one-hour recording so trim/mute validation never trips on duration.
    #[must_use]
    pub fn studio_recording(id: RecordingId) -> Recording {
        Recording {
            id,
            path: PathBuf::from(format!("{}.wav", id.get())),
            fingerprint: RecordingFingerprint(id.get()),
            track_role: TrackRole::Mixed,
            duration: Duration::from_secs(3600),
            sample_rate: 48_000,
            channels: 1,
        }
    }

    /// A word bound to recording 1 at `[start, end)` seconds.
    #[must_use]
    pub fn word(text: &str, start: u64, end: u64) -> Word {
        Word {
            text: text.to_string(),
            start: Duration::from_secs(start),
            end: Duration::from_secs(end),
            confidence: Confidence::new(0.95).expect("valid"),
            source_recording: RecordingId::new(1),
        }
    }

    /// A two-segment transcript:
    /// segment 1 ("hello world", words at 0-1s, 1-2s),
    /// segment 2 ("goodbye now", words at 4-5s, 5-6s).
    #[must_use]
    pub fn sample_transcript() -> Transcript {
        Transcript::from_segments(vec![
            Segment {
                id: SegmentId::new(1),
                speaker: Some(SpeakerId::new(1)),
                words: vec![word("hello", 0, 1), word("world", 1, 2)],
            },
            Segment {
                id: SegmentId::new(2),
                speaker: Some(SpeakerId::new(2)),
                words: vec![word("goodbye", 4, 5), word("now", 5, 6)],
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::test_support::{sample_transcript, studio_recording, word};
    use super::*;
    use tpt_app_voice_studio_model::transcript::Segment;

    fn session() -> EditSession {
        EditSession::new(
            vec![studio_recording(RecordingId::new(1))],
            sample_transcript(),
        )
    }

    fn working_text(session: &EditSession) -> Vec<String> {
        session.working().words().map(|w| w.text.clone()).collect()
    }

    #[test]
    fn delete_removes_words_and_undo_redo_restores_them() {
        let mut session = session();
        session
            .apply(EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 1..2,
            })
            .expect("valid");
        assert_eq!(working_text(&session), vec!["hello", "goodbye", "now"]);

        assert!(session.undo());
        assert_eq!(
            working_text(&session),
            vec!["hello", "world", "goodbye", "now"]
        );

        assert!(session.redo());
        assert_eq!(working_text(&session), vec!["hello", "goodbye", "now"]);
    }

    #[test]
    fn delete_can_empty_a_segment_but_segment_identity_survives() {
        let mut session = session();
        session
            .apply(EditOperation::DeleteRange {
                segment: SegmentId::new(2),
                word_range: 0..2,
            })
            .expect("valid");
        let seg = session.working().segment(SegmentId::new(2)).expect("kept");
        assert_eq!(seg.word_count(), 0);
        assert!(seg.span().is_none());
    }

    #[test]
    fn reorder_moves_segment_with_target_index_after_removal() {
        let mut session = session();
        session
            .apply(EditOperation::Reorder {
                segment: SegmentId::new(1),
                new_position: 1,
            })
            .expect("valid");
        let order: Vec<SegmentId> = session.working().segments.iter().map(|s| s.id).collect();
        assert_eq!(order, vec![SegmentId::new(2), SegmentId::new(1)]);

        assert!(session.undo());
        let order: Vec<SegmentId> = session.working().segments.iter().map(|s| s.id).collect();
        assert_eq!(order, vec![SegmentId::new(1), SegmentId::new(2)]);
    }

    #[test]
    fn trim_cuts_interior_words_and_clamps_partial_overlap() {
        let mut session = session();
        // Cut 0.5s..5.5s: "hello" (0-1) clamps to 0-0.5, "world" (1-2),
        // "goodbye" (4-5) are fully inside and removed, "now" (5-6) clamps
        // to 5.5-6.
        session
            .apply(EditOperation::Trim {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_millis(500), Duration::from_millis(5_500))
                    .expect("valid"),
            })
            .expect("valid");
        let texts: Vec<&str> = session.working().words().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, vec!["hello", "now"]);
        let hello = &session.working().segments[0].words[0];
        assert_eq!(hello.start, Duration::from_millis(0));
        assert_eq!(hello.end, Duration::from_millis(500));
        let now = &session.working().segments[1].words[0];
        assert_eq!(now.start, Duration::from_millis(5_500));
    }

    #[test]
    fn trim_ignores_words_from_other_recordings() {
        let mut transcript = sample_transcript();
        // "world" is re-bound to another recording; the trim must not touch it.
        transcript.segments[0].words[1].source_recording = RecordingId::new(2);
        let mut session = EditSession::new(
            vec![
                studio_recording(RecordingId::new(1)),
                studio_recording(RecordingId::new(2)),
            ],
            transcript,
        );
        session
            .apply(EditOperation::Trim {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_secs(0), Duration::from_secs(6))
                    .expect("valid"),
            })
            .expect("valid");
        let texts: Vec<&str> = session.working().words().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, vec!["world"]);
    }

    #[test]
    fn mute_leaves_transcript_untouched_and_is_queryable() {
        let mut session = session();
        let range = TimeRange::new(Duration::from_secs(4), Duration::from_secs(6)).expect("valid");
        session
            .apply(EditOperation::Mute {
                recording: RecordingId::new(1),
                range,
            })
            .expect("valid");
        assert_eq!(session.working(), session.original());
        assert_eq!(session.muted_ranges(RecordingId::new(1)), vec![range]);
        assert!(session.muted_ranges(RecordingId::new(9)).is_empty());
    }

    #[test]
    fn select_take_records_history_without_transcript_change() {
        let mut session = session();
        session
            .apply(EditOperation::SelectTake {
                segment: SegmentId::new(2),
                recording: RecordingId::new(1),
            })
            .expect("valid");
        assert_eq!(session.working(), session.original());
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn validation_rejects_bad_operations_without_touching_state() {
        let mut session = session();

        let cases = [
            EditOperation::DeleteRange {
                segment: SegmentId::new(99),
                word_range: 0..1,
            },
            EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 1..1,
            },
            EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 0..5,
            },
            EditOperation::Reorder {
                segment: SegmentId::new(1),
                new_position: 2,
            },
            EditOperation::Trim {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_secs(3599), Duration::from_secs(3601))
                    .expect("valid"),
            },
            EditOperation::Mute {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_secs(2), Duration::from_secs(2))
                    .expect("valid"),
            },
            EditOperation::SelectTake {
                segment: SegmentId::new(1),
                recording: RecordingId::new(77),
            },
        ];
        let before = session.working().clone();
        for op in &cases {
            let err = session.validate(op).expect_err("must be rejected");
            assert!(matches!(
                err,
                EditError::UnknownEntity(_)
                    | EditError::EmptyWordRange { .. }
                    | EditError::WordRangeOutOfBounds(_)
                    | EditError::PositionOutOfBounds { .. }
                    | EditError::RangeExceedsDuration(_)
                    | EditError::EmptyTimeRange { .. }
            ));
            assert!(session.apply(op.clone()).is_err());
        }
        assert_eq!(session.working(), &before);
        assert!(session.history().is_empty());
    }

    #[test]
    fn applying_after_undo_discards_redo_history() {
        let mut session = session();
        session
            .apply(EditOperation::DeleteRange {
                segment: SegmentId::new(1),
                word_range: 0..1,
            })
            .expect("valid");
        assert!(session.undo());
        session
            .apply(EditOperation::Mute {
                recording: RecordingId::new(1),
                range: TimeRange::new(Duration::from_secs(0), Duration::from_secs(1))
                    .expect("valid"),
            })
            .expect("valid");
        assert!(!session.redo(), "redo must be cleared by a new edit");
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn undo_to_empty_returns_false_and_original_survives_every_edit() {
        let mut session = session();
        for word_range in [0..1, 0..1] {
            session
                .apply(EditOperation::DeleteRange {
                    segment: SegmentId::new(1),
                    word_range: word_range.clone(),
                })
                .expect("valid");
        }
        session
            .apply(EditOperation::Reorder {
                segment: SegmentId::new(2),
                new_position: 0,
            })
            .expect("valid");

        while session.undo() {}

        // The original binding survives every edit (spec §3.2, §3.3).
        assert_eq!(session.working(), session.original());
        assert_eq!(
            working_text(&session),
            vec!["hello", "world", "goodbye", "now"]
        );
        assert!(!session.undo());
    }

    #[test]
    fn new_session_matches_original_and_has_empty_history() {
        let mut session = session();
        assert_eq!(session.working(), session.original());
        assert!(session.history().is_empty());
        assert!(!session.undo());
        assert!(!session.redo());
    }

    #[test]
    fn session_with_unaligned_speaker_assignment_keeps_data() {
        // Speakers arrive from diarisation; the edit engine must not require
        // or alter them (spec §9).
        let transcript = Transcript::from_segments(vec![Segment {
            id: SegmentId::new(1),
            speaker: None,
            words: vec![word("um", 10, 11)],
        }]);
        let session = EditSession::new(vec![studio_recording(RecordingId::new(1))], transcript);
        assert_eq!(working_text(&session), vec!["um"]);
        assert!(session.working().segments[0].speaker.is_none());
    }
}
