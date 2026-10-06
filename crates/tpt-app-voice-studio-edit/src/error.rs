//! Errors returned by the edit engine.

use tpt_app_voice_studio_core::error::{
    RangeExceedsDurationError, UnknownEntityError, WordRangeOutOfBoundsError,
};
use tpt_app_voice_studio_core::id::{RecordingId, SegmentId};
use tpt_app_voice_studio_core::time::TimeRange;

/// Validation or application failure for an edit operation.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum EditError {
    /// The operation referenced a segment or recording that does not exist.
    #[error(transparent)]
    UnknownEntity(#[from] UnknownEntityError),

    /// A `DeleteRange` addressed words beyond the end of the segment.
    #[error(transparent)]
    WordRangeOutOfBounds(#[from] WordRangeOutOfBoundsError),

    /// A `DeleteRange` selected no words (`start == end`).
    #[error("delete range on segment {segment} selects no words")]
    EmptyWordRange {
        /// The segment the empty range was addressed to.
        segment: SegmentId,
    },

    /// A `Reorder` target position does not exist in the segment list.
    #[error("reorder of segment {segment} targets position {position}, but only {segment_count} segments exist")]
    PositionOutOfBounds {
        /// The segment being moved.
        segment: SegmentId,
        /// The rejected target position.
        position: usize,
        /// The number of segments in the current list.
        segment_count: usize,
    },

    /// A `Trim`/`Mute` range lies (partly) beyond the recording's duration.
    #[error(transparent)]
    RangeExceedsDuration(#[from] RangeExceedsDurationError),

    /// A `Trim`/`Mute` range selects no time.
    #[error("operation on recording {recording} uses an empty time range")]
    EmptyTimeRange {
        /// The recording the empty range was addressed to.
        recording: RecordingId,
        /// The empty range.
        range: TimeRange,
    },

    /// The recorded history failed to replay (an internal invariant).
    #[error("edit history replay failed: {0}")]
    ReplayInvariant(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;
    use std::time::Duration;

    #[test]
    fn messages_carry_operation_context() {
        let err = EditError::EmptyWordRange {
            segment: SegmentId::new(4),
        };
        assert_eq!(
            err.to_string(),
            "delete range on segment SegmentId(4) selects no words"
        );

        let err = EditError::PositionOutOfBounds {
            segment: SegmentId::new(4),
            position: 9,
            segment_count: 3,
        };
        assert_eq!(
            err.to_string(),
            "reorder of segment SegmentId(4) targets position 9, but only 3 segments exist"
        );

        let err = EditError::EmptyTimeRange {
            recording: RecordingId::new(2),
            range: TimeRange::new(Duration::from_secs(1), Duration::from_secs(1)).expect("valid"),
        };
        assert!(err.to_string().contains("empty time range"));
    }

    #[test]
    fn transparent_errors_delegate_display() {
        let err = EditError::UnknownEntity(UnknownEntityError::Recording(RecordingId::new(7)));
        assert_eq!(err.to_string(), "unknown recording RecordingId(7)");

        let err = EditError::WordRangeOutOfBounds(WordRangeOutOfBoundsError {
            segment: SegmentId::new(1),
            word_count: 3,
            range: Range { start: 0, end: 9 },
        });
        assert!(err.to_string().contains("has 3 words"));
    }
}
