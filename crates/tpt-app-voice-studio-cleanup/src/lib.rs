//! Cleanup and enhancement operations for TPT Voice Studio.
//!
//! Owns the explicit, adjustable, reviewable cleanup operations (spec §10):
//!
//! - Voice isolation: remove background noise/room tone while preserving the
//!   primary voice.
//! - Parametric denoise with before/after preview.
//! - Loudness normalization to a target LUFS.
//! - Silence trimming with configurable threshold and minimum retained gap.
//!
//! Every operation is presented as a reviewable suggestion or adjustable
//! operation and must support an audible before/after comparison before being
//! committed to the edit history — cleanup is never silently baked in
//! (spec §3.5, §10). Per-speaker application is supported so multi-mic
//! sessions with uneven quality can be treated independently (spec §9).
//!
//! Signal processing is delegated to the `tpt-audio`/`tpt-dsp` foundation
//! (spec §5.1). Status: scaffold; `tpt-dsp` has no implementation repo yet
//! (design doc only), so integration lands in Phase 1 against `tpt-audio`
//! with `tpt-dsp` flagged as a dependency risk.

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
