//! Half-open time ranges over recording/word timelines.
//!
//! `TimeRange` is the currency of the alignment binding (spec §3.2): every
//! word carries the exact audio range it represents, and edit operations
//! (`Trim`, `Mute`, export range selection) all operate on the same type.
//! Ranges are half-open `[start, end)` on the `std::time::Duration` timeline
//! of a specific recording.

use std::time::Duration;

/// Error returned when a time range would be invalid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum TimeRangeError {
    /// The range's end was before its start.
    #[error("time range end {end:?} is before start {start:?}")]
    EndBeforeStart {
        /// The rejected start time.
        start: Duration,
        /// The rejected end time.
        end: Duration,
    },
}

/// A half-open time range `[start, end)` within one recording.
///
/// Invariant: `start <= end`. An empty range (`start == end`) is legal and
/// selects nothing.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub struct TimeRange {
    /// Inclusive start of the range.
    pub start: Duration,
    /// Exclusive end of the range.
    pub end: Duration,
}

impl TimeRange {
    /// Creates a range, validating that `end` does not precede `start`.
    ///
    /// # Errors
    /// Returns [`TimeRangeError::EndBeforeStart`] if `end < start`.
    pub fn new(start: Duration, end: Duration) -> Result<Self, TimeRangeError> {
        if end < start {
            return Err(TimeRangeError::EndBeforeStart { start, end });
        }
        Ok(Self { start, end })
    }

    /// Length of the range; zero for an empty range.
    #[must_use]
    pub fn duration(self) -> Duration {
        debug_assert!(
            self.start <= self.end,
            "range invariant enforced at construction"
        );
        self.end.checked_sub(self.start).unwrap_or_default()
    }

    /// True if the range selects no time.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// True if `instant` falls within `[start, end)`.
    #[must_use]
    pub fn contains_instant(self, instant: Duration) -> bool {
        self.start <= instant && instant < self.end
    }

    /// True if `other` intersects this range at all (touching endpoints do
    /// not count: ranges are half-open).
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// True if `other` lies entirely within this range.
    #[must_use]
    pub fn contains_range(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// The part of `self` that also lies within `bounds`, if any.
    ///
    /// Returns `None` when the ranges do not overlap, including when they
    /// merely touch at an endpoint (ranges are half-open) and when either
    /// range is empty.
    pub fn intersection(self, bounds: Self) -> Option<Self> {
        let start = self.start.max(bounds.start);
        let end = self.end.min(bounds.end);
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Clamps this range into `bounds`: parts of the range beyond the bounds
    /// are cut away. The result may be empty.
    ///
    /// # Errors
    /// Returns [`TimeRangeError::EndBeforeStart`] if `bounds` is invalid.
    pub fn clamp_to(self, bounds: Self) -> Result<Self, TimeRangeError> {
        let start = self.start.clamp(bounds.start, bounds.end);
        let end = self.end.clamp(bounds.start, bounds.end);
        Self::new(start, end)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use std::time::Duration;

    const S: fn(u64) -> Duration = |s| Duration::from_secs(s);

    #[test]
    fn accepts_and_reports_basic_geometry() {
        let range = TimeRange::new(S(1), S(4)).expect("valid range");
        assert_eq!(range.duration(), S(3));
        assert!(!range.is_empty());
    }

    #[test]
    fn rejects_end_before_start() {
        let err = TimeRange::new(S(5), S(2)).expect_err("invalid range");
        assert!(matches!(err, TimeRangeError::EndBeforeStart { .. }));
    }

    #[test]
    fn empty_range_selects_nothing() {
        let range = TimeRange::new(S(2), S(2)).expect("empty range is legal");
        assert!(range.is_empty());
        assert!(!range.contains_instant(S(2)));
        assert_eq!(range.duration(), Duration::ZERO);
    }

    #[test]
    fn contains_instant_is_half_open() {
        let range = TimeRange::new(S(1), S(4)).expect("valid");
        assert!(range.contains_instant(S(1)));
        assert!(range.contains_instant(S(3)));
        assert!(!range.contains_instant(S(4)));
    }

    #[test]
    fn intersection_and_overlap_semantics() {
        let a = TimeRange::new(S(1), S(4)).expect("valid");
        let b = TimeRange::new(S(3), S(6)).expect("valid");
        assert!(a.intersects(b));
        assert_eq!(
            a.intersection(b).expect("overlap"),
            TimeRange::new(S(3), S(4)).expect("valid")
        );
        // Touching endpoints do not intersect (half-open).
        let c = TimeRange::new(S(4), S(6)).expect("valid");
        assert!(!a.intersects(c));
        assert!(a.intersection(c).is_none());
        // Containment.
        let inside = TimeRange::new(S(2), S(3)).expect("valid");
        assert!(a.contains_range(inside));
        assert!(!inside.contains_range(a));
    }

    #[test]
    fn clamp_cuts_range_at_bounds_and_may_empty_it() {
        let bounds = TimeRange::new(S(2), S(5)).expect("valid");
        let wide = TimeRange::new(S(1), S(7)).expect("valid");
        assert_eq!(
            wide.clamp_to(bounds).expect("valid"),
            TimeRange::new(S(2), S(5)).expect("valid")
        );
        let outside = TimeRange::new(S(6), S(8)).expect("valid");
        assert!(outside.clamp_to(bounds).expect("valid").is_empty());
    }
}
