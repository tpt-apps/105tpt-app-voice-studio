//! Domain model for TPT Voice Studio.
//!
//! Defines the persistent, engine-level representation of a Voice Studio
//! project per spec §6:
//!
//! - `Project` and `Recording` (§6.1–6.2), including multi-track
//!   multi-mic sessions via `TrackRole`.
//! - `Transcript`, `Segment`, `Word` (§6.3), where every word carries a
//!   timestamp range, a confidence score, and the source recording it was
//!   aligned against — the binding that makes "edit the transcript, edit the
//!   audio" possible (spec §3.2).
//! - `Speaker` (§6.4) with rename/merge/split support (spec §9).
//! - `EditOperation` (§6.5): the reversible, ordered edit-decision-list
//!   entries (`DeleteRange`, `Reorder`, `Trim`, `Mute`, `SelectTake`).
//! - `ExportJob` and `ExportFormat` (§6.6).
//!
//! The model is plain data plus invariants. It must not perform I/O, depend on
//! codecs, or link against the speech engine; pipeline crates own behaviour.
//!
//! Status: scaffold. The types named above are implemented in Phase 1
//! (todo.md, "Domain model").

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
