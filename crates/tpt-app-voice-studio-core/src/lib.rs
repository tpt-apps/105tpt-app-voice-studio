//! Shared primitives for TPT Voice Studio.
//!
//! This crate holds the small, dependency-light types that every other Voice
//! Studio crate builds on: strongly typed identifiers, time ranges, transcript
//! confidence tiers, and the shared error types.
//!
//! It is part of the deterministic transcript/edit engine (spec §3.6) and must
//! never depend on the application shell, the CLI, or any GUI concern.
//!
//! Spec references: `spec.txt` §6 (domain model), §7.1 (confidence tiers).

pub mod confidence;
pub mod error;
pub mod id;
pub mod time;
