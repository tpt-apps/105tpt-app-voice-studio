//! Per-word transcription confidence and its review tiers.
//!
//! The application must never present automatically transcribed text as
//! verified fact (spec §17.1): every word carries a confidence score, and
//! scores are retained for the life of the project so reviewers can filter to
//! the words needing human verification at any time (spec §7.1, §17.2).

use std::fmt;

/// Visual/review tier derived from a word's confidence score (spec §7.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ConfidenceTier {
    /// Rendered normally; no review attention required.
    High,
    /// Subtly flagged; worth a second look.
    Medium,
    /// Clearly flagged, inviting manual correction.
    Low,
}

/// A confidence score in `0.0..=1.0` attached to a transcribed word.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Confidence(f32);

impl Confidence {
    /// A word at or above this score is [`ConfidenceTier::High`].
    pub const HIGH_THRESHOLD: f32 = 0.9;
    /// A word at or above this score (and below [`Self::HIGH_THRESHOLD`]) is
    /// [`ConfidenceTier::Medium`]; anything below is [`ConfidenceTier::Low`].
    pub const MEDIUM_THRESHOLD: f32 = 0.6;

    /// Creates a confidence score.
    ///
    /// # Errors
    /// Returns [`ConfidenceError::OutOfRange`] if `value` is not a finite
    /// number in `0.0..=1.0`.
    pub fn new(value: f32) -> Result<Self, ConfidenceError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ConfidenceError::OutOfRange(value));
        }
        Ok(Self(value))
    }

    /// The raw score in `0.0..=1.0`.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }

    /// The review tier this score maps to.
    #[must_use]
    pub fn tier(self) -> ConfidenceTier {
        if self.0 >= Self::HIGH_THRESHOLD {
            ConfidenceTier::High
        } else if self.0 >= Self::MEDIUM_THRESHOLD {
            ConfidenceTier::Medium
        } else {
            ConfidenceTier::Low
        }
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

/// Error returned when a confidence score is out of range or not a number.
#[derive(Clone, Copy, PartialEq, Debug, thiserror::Error)]
pub enum ConfidenceError {
    /// The value was negative, above one, or not finite.
    #[error("confidence must be a finite value in 0.0..=1.0, got {0}")]
    OutOfRange(f32),
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn accepts_boundary_values() {
        assert_eq!(Confidence::new(0.0).expect("valid").value(), 0.0);
        assert_eq!(Confidence::new(1.0).expect("valid").value(), 1.0);
    }

    #[test]
    fn rejects_out_of_range_and_nan() {
        for bad in [-0.1_f32, 1.01, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                matches!(Confidence::new(bad), Err(ConfidenceError::OutOfRange(_))),
                "expected rejection for {bad}"
            );
        }
    }

    #[test]
    fn tiers_map_at_thresholds() {
        assert_eq!(
            Confidence::new(1.0).expect("valid").tier(),
            ConfidenceTier::High
        );
        assert_eq!(
            Confidence::new(0.9).expect("valid").tier(),
            ConfidenceTier::High
        );
        assert_eq!(
            Confidence::new(0.89).expect("valid").tier(),
            ConfidenceTier::Medium
        );
        assert_eq!(
            Confidence::new(0.6).expect("valid").tier(),
            ConfidenceTier::Medium
        );
        assert_eq!(
            Confidence::new(0.59).expect("valid").tier(),
            ConfidenceTier::Low
        );
        assert_eq!(
            Confidence::new(0.0).expect("valid").tier(),
            ConfidenceTier::Low
        );
    }
}
