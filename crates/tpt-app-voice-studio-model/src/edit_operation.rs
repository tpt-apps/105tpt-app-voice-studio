//! Edit operations: the entries of the reversible edit-decision-list
//! (spec §6.5).
//!
//! Every edit operation is appended to an ordered history rather than
//! mutating source audio; application semantics live in the `-edit` engine
//! crate.

use std::ops::Range;

use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
use tpt_app_voice_studio_core::time::TimeRange;

/// A single reversible, non-destructive edit (spec §6.5).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum EditOperation {
    /// Removes the words in `word_range` (half-open, into the segment's word
    /// list) from playback.
    DeleteRange {
        /// The segment whose words are addressed.
        segment: SegmentId,
        /// Half-open range of word indices to delete.
        word_range: Range<usize>,
    },
    /// Moves a segment to a new position in playback order.
    Reorder {
        /// The segment to move.
        segment: SegmentId,
        /// Target index in the (current) segment list; the segment is removed
        /// from its current position first, then inserted at this index.
        new_position: usize,
    },
    /// Removes `[range.start, range.end)` from the beginning/end region of a
    /// recording (e.g. leading/trailing silence).
    Trim {
        /// The recording to trim.
        recording: RecordingId,
        /// The time range to cut.
        range: TimeRange,
    },
    /// Silences `[range.start, range.end)` without deleting it (redactions).
    Mute {
        /// The recording to mute within.
        recording: RecordingId,
        /// The time range to mute.
        range: TimeRange,
    },
    /// Selects which recording's take supplies a segment's audio
    /// (multi-take selection, spec §8.2).
    SelectTake {
        /// The segment whose take is being chosen.
        segment: SegmentId,
        /// The recording providing the selected take.
        recording: RecordingId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn operations_compare_by_value() {
        let a = EditOperation::DeleteRange {
            segment: SegmentId::new(1),
            word_range: 2..5,
        };
        let b = a.clone();
        assert_eq!(a, b);

        let mute = EditOperation::Mute {
            recording: RecordingId::new(1),
            range: TimeRange::new(Duration::from_secs(1), Duration::from_secs(2)).expect("valid"),
        };
        assert_ne!(a, mute);
    }
}
