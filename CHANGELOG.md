# Changelog

All notable changes to TPT Voice Studio are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with the ten crates from spec §4: `-core`, `-model`,
  `-transcribe`, `-align`, `-edit`, `-cleanup`, `-export`, `-cli`
  (binary `tpt-voice-studio`), `-tauri` (desktop shell placeholder), and
  `-test`.
- Dual licensing under `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`).
- `deny.toml` and a CI job enforcing license/advisory/ban checks.
- GitHub Actions CI: format check, clippy (`-D warnings`), build + test on
  Windows and Linux, and `cargo-deny`.
- Documentation skeleton: `architecture.md`, `transcript-model.md`,
  `edit-model.md`, `alignment.md`, `export-formats.md`, `accuracy.md`.
- Fixture directory scaffolding for the golden test categories:
  `single-speaker/`, `multi-speaker/`, `noisy/`, `multi-track/`, `accents/`.
- Test suite directory scaffolding: `tests/integration/`, `tests/golden/`,
  `tests/round-trip/`.
- Foundation dependency status survey: `tpt-cadence`, `tpt-audio`,
  `tpt-av-asset`, and `tpt-av-test` are present as sibling workspaces and
  path-wired in the root manifest; `tpt-voice` and `tpt-dsp` are not yet
  resolvable and are flagged as dependency risks (spec §5.1).
