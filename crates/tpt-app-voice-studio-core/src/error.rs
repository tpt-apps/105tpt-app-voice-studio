//! Shared error types used across engine crates.
//!
//! Each engine crate also defines its own specific error enums (for example
//! the edit engine's operation errors); the types here are the ones several
//! crates need identically.

use crate::id::{RecordingId, SegmentId};
use crate::time::TimeRange;

/// An entity reference in an edit operation did not resolve.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum UnknownEntityError {
    /// The edit referenced a segment id that does not exist.
    #[error("unknown segment {0}")]
    Segment(SegmentId),
    /// The edit referenced a recording id that does not exist.
    #[error("unknown recording {0}")]
    Recording(RecordingId),
}

/// The referenced segment does not contain the requested word range.
#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
#[error("segment {segment} has {word_count} words; requested word range {range:?}")]
pub struct WordRangeOutOfBoundsError {
    /// The segment whose words were addressed.
    pub segment: SegmentId,
    /// The number of words actually present.
    pub word_count: usize,
    /// The rejected (half-open) word index range.
    pub range: std::ops::Range<usize>,
}

/// A time range that must lie within a recording's duration did not.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[error("range {range:?} exceeds duration {duration:?} of recording {recording}")]
pub struct RangeExceedsDurationError {
    /// The recording whose duration was exceeded.
    pub recording: RecordingId,
    /// The recording's duration.
    pub duration: std::time::Duration,
    /// The rejected range.
    pub range: TimeRange,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn error_messages_name_the_entity() {
        let err = UnknownEntityError::Segment(SegmentId::new(3));
        assert_eq!(err.to_string(), "unknown segment SegmentId(3)");
        let err = UnknownEntityError::Recording(RecordingId::new(5));
        assert_eq!(err.to_string(), "unknown recording RecordingId(5)");
    }

    #[test]
    fn word_range_error_message_carries_context() {
        let err = WordRangeOutOfBoundsError {
            segment: SegmentId::new(1),
            word_count: 4,
            range: 2..9,
        };
        assert_eq!(
            err.to_string(),
            "segment SegmentId(1) has 4 words; requested word range 2..9"
        );
    }

    #[test]
    fn duration_error_message_carries_context() {
        let err = RangeExceedsDurationError {
            recording: RecordingId::new(2),
            duration: Duration::from_secs(10),
            range: crate::time::TimeRange::new(Duration::from_secs(8), Duration::from_secs(12))
                .expect("valid"),
        };
        assert!(err.to_string().contains("exceeds duration 10s"));
    }
}
