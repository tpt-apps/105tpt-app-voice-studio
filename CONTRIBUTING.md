# Contributing to TPT Voice Studio

Thank you for contributing! This document covers the development setup, the
quality bar, and the contribution terms.

## Development setup

1. Install a Rust stable toolchain via [rustup](https://rustup.rs). The
   minimum supported version is the workspace `rust-version` in `Cargo.toml`.
2. Install the supplementary tools used in CI:
   ```sh
   cargo install cargo-deny
   rustup component add clippy rustfmt
   ```
3. From the repository root:
   ```sh
   cargo build
   cargo test
   ```

Windows is the primary development/target platform; Linux is secondary.
Platform-specific code must be isolated and documented.

## Architecture rules

- The deterministic transcript/edit engine must never depend on the desktop
  shell, and the shell must never grow engine behaviour (spec §3.6). If a
  change feels at home in both, it belongs in an engine crate.
- The domain model (`-model`) is plain data plus invariants: no I/O, no codecs,
  no speech-engine linkage.
- Non-destructive editing is a hard invariant: nothing outside export/render
  may write to an original recording, and every user-visible edit must be an
  undoable edit-decision-list entry (spec §3.3).
- Cleanup (filler removal, silence trimming, denoise) must surface as
  reviewable, adjustable operations — never silently applied (spec §3.5).
- Every transcribed word keeps its confidence score for the life of the
  project (spec §7.1).

## Quality bar (enforced in CI)

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo deny check` (licenses, advisories, bans)

## Testing expectations (spec §18)

Every pipeline stage change should come with tests covering: valid input,
invalid input, boundary cases, malformed input, and the expected result.
Bug fixes must include a permanent regression fixture. Round-trip guarantees
(edit → EDL → render → re-import/re-align consistency) are verified by the
suites under `tests/`.

## Commit and pull requests

- Keep commits focused; write messages that explain *why*, not just *what*.
- CI must be green before merge (fmt, clippy, tests, cargo-deny).
- Describe user-visible changes in `CHANGELOG.md` under `[Unreleased]`.

## License terms for contributions

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work is dual-licensed as `MIT OR Apache-2.0`,
as defined in the Apache-2.0 license text, and TPT Solutions may exercise
either license. Do not submit code you are not entitled to dual-license.
