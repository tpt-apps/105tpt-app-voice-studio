//! TPT Voice Studio desktop application shell.
//!
//! Native Tauri application hosting the primary UX surfaces (spec §12):
//!
//! - Transcript-first editor with linked waveform (§12.1)
//! - Speaker track view (§12.2)
//! - Filler/silence review panel with batch accept/reject (§12.3)
//! - Cleanup panel with before/after preview (§12.4)
//! - Export panel backed by the shared export engine (§12.5)
//! - Project/asset browser with background job status (§12.6)
//!
//! The shell must stay thin: all transcript, alignment, editing, cleanup, and
//! export behaviour lives in the engine crates so the CLI and desktop app
//! cannot drift (spec §3.6). The shell also owns the optional localhost-only
//! API (spec §14), disabled by default and bound to 127.0.0.1 when enabled.
//!
//! Status: scaffold. Tauri v2 integration, the frontend, and the screens above
//! land in Phase 1; this binary is a placeholder so the workspace builds.

fn main() {
    println!(
        "tpt-app-voice-studio-tauri {} (scaffold — Tauri shell lands in Phase 1)",
        env!("CARGO_PKG_VERSION")
    );
}
