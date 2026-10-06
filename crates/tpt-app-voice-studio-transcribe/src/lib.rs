//! Transcription and diarisation pipeline stage for TPT Voice Studio.
//!
//! Owns the decode → diarise → transcribe portion of the pipeline (spec §7):
//!
//! - Decoding imported audio via the codec foundation (`tpt-cadence`).
//! - Speaker diarisation/segmentation via the speech engine (`tpt-voice`).
//! - Batch transcription, producing draft segments that the alignment crate
//!   binds to word-level timings.
//!
//! Engine access must sit behind traits owned by this crate so the deterministic
//! engine stays testable and the desktop shell, CLI, and batch mode all drive
//! the same pipeline (spec §3.6).
//!
//! Status: scaffold. `tpt-voice` is not yet resolvable (dependency risk,
//! spec §5.1); integration lands in Phase 1 once it exists.

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
