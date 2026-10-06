//! Detection of reviewable cleanup suggestions (spec §8.1).
//!
//! The application detects, but never silently removes (spec §3.5):
//!
//! - filler words ([`fillers`]), matched against a configurable per-language
//!   list,
//! - long silences ([`silence`]), detected as gaps between transcribed words
//!   on each recording's timeline.
//!
//! Both detectors return *suggestions* that convert one-to-one into
//! [`EditOperation`]s, so a review panel's "Remove"/"Trim" action is an
//! ordinary, undoable edit against the EDL — the engine never applies them
//! on its own.

pub mod fillers;
pub mod silence;
