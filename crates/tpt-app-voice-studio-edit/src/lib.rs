//! Non-destructive transcript-driven editing engine for TPT Voice Studio.
//!
//! Owns the edit-decision-list (EDL) model and the transcript editing
//! operations that drive it (spec §8):
//!
//! - The ordered, reversible EDL: transcript edits are recorded as
//!   `EditOperation` entries and applied against source audio only at
//!   playback and export time — the original recording is never modified
//!   (spec §3.3, §6.5).
//! - Delete/reorder of words, sentences, and segments driven directly from
//!   transcript selections.
//! - Trim of leading/trailing silence and mute-range-without-deleting
//!   (redactions).
//! - Filler-word and silence detection with manual review semantics — detect,
//!   suggest, never silently apply (spec §3.5, §8.1).
//! - Multi-take detection and selection between alternate deliveries of the
//!   same line (spec §8.2).
//!
//! This crate is pure engine code: deterministic, I/O-free rendering decisions,
//! usable identically from the desktop UI and the CLI (spec §3.6).

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
