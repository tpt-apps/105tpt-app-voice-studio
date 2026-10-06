//! Test support for TPT Voice Studio.
//!
//! Central home for the testing strategy of spec §18:
//!
//! - Golden audio/transcript fixtures across the controlled categories
//!   (`single-speaker`, `multi-speaker`, `noisy`, `multi-track`, `accents`),
//!   each with an expected transcript, expected speaker segmentation, and an
//!   alignment tolerance (§18.2).
//! - Round-trip harnesses: transcript edit → EDL → rendered audio →
//!   re-import/re-align must remain consistent within tolerance (§18.3).
//! - Property-test helpers for timestamp/word-range mapping, EDL ordering and
//!   application, subtitle timing, and confidence aggregation (§18.4).
//! - Fuzz harness wiring for audio/container parsers, transcript parsers,
//!   project file parsing, and CLI arguments, reusing `tpt-av-test` where
//!   possible (§18.5).
//!
//! Regression policy (§18.6): every production bug produces a permanent
//! regression fixture here.

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
