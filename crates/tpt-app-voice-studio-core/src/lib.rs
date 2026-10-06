//! Shared primitives for TPT Voice Studio.
//!
//! This crate holds the small, dependency-free types that every other Voice
//! Studio crate builds on: strongly typed identifiers, time ranges, transcript
//! confidence tiers, and the shared error type.
//!
//! It is part of the deterministic transcript/edit engine (spec §3.6) and must
//! never depend on the application shell, the CLI, or any GUI concern.
//!
//! Spec references: `spec.txt` §6 (domain model), §7.1 (confidence tiers),
//! §13 (CLI exit codes are defined in the CLI crate, not here).

#[cfg(test)]
mod tests {
    /// Placeholder so `cargo test` exercises the crate before real modules
    /// land in Phase 1. Remove once the crate has genuine coverage.
    #[test]
    fn crate_links() {
        assert_eq!(1 + 1, 2);
    }
}
