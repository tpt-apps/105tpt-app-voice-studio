//! Non-destructive transcript-driven editing engine for TPT Voice Studio.
//!
//! Owns the edit-decision-list (EDL) model and the transcript editing
//! operations that drive it (spec §8):
//!
//! - [`EditSession`] holds the original aligned transcript plus an ordered,
//!   reversible history of [`EditOperation`]s (spec §3.3, §6.5). The working
//!   transcript is always derived by replaying the history against the
//!   original, so every edit is undoable and the original binding is never
//!   lost (spec §3.2).
//! - [`timeline`] derives the rendered (output) timeline from the working
//!   transcript: the mapping a playback/export renderer uses to cut source
//!   audio without ever modifying it.
//!
//! Application policies (deterministic, documented, and covered by tests):
//!
//! - **Delete** removes the addressed words; segments emptied by deletion are
//!   retained (stable identity for undo/history) and simply play no audio.
//! - **Reorder** moves a segment; the target index is interpreted in the
//!   segment list after the segment is removed.
//! - **Trim** cuts a time range from one recording: words fully inside the
//!   range are removed; partially overlapping words are clamped to the cut
//!   boundaries and dropped if the clamp empties them; a word spanning the
//!   whole cut range is kept intact (words are atomic).
//! - **Mute** records a silenced range for the render/export stage; the
//!   working transcript is unchanged.
//! - **SelectTake** records the take choice; the transcript-level swap is
//!   wired when take alignment lands (spec §8.2, `tpt-voice` integration).
//!
//! Rendered timing policy: each kept word occupies its original duration on
//! the output timeline, concatenated without gaps — deleting a span removes
//! the audio between its neighbours, as a physical cut would. Deliberate
//! pause preservation is the job of silence-trim configuration (spec §10).

pub mod error;
pub mod session;
pub mod timeline;

pub use error::EditError;
pub use session::EditSession;
