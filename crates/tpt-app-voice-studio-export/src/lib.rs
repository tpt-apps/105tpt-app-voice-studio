//! Export pipeline for TPT Voice Studio.
//!
//! Renders deliverables from the edit-decision-list and the aligned, edited
//! transcript (spec §11):
//!
//! - Audio export (WAV, MP3, AAC, FLAC via `tpt-cadence`) rendered from the
//!   EDL, so exports always match the edited project.
//! - Clip/highlight export from selected transcript ranges.
//! - SRT/VTT subtitle generation derived from the same aligned data used for
//!   editing — subtitles stay in sync with the final edited audio without a
//!   manual re-timing pass.
//! - Timestamped, speaker-labeled transcript export suitable for client
//!   delivery or legal/corporate record-keeping, with the optional formal-use
//!   disclaimer on opt-in (spec §17.3).
//!
//! The same export engine backs both the desktop export panel and the CLI
//! `export` command (spec §12.5, §13).

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
