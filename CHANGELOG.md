# Changelog

All notable changes to TPT Voice Studio are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Core primitives (`tpt-app-voice-studio-core`): strongly typed ids
  (`ProjectId`, `RecordingId`, `SegmentId`, `SpeakerId`, `ExportJobId`),
  half-open `TimeRange` with validated geometry helpers, `Confidence` with
  high/medium/low review tiers (spec §7.1), and shared engine error types.
- Domain model (`tpt-app-voice-studio-model`) per spec §6: `Project`,
  `Recording`/`TrackRole`/`RecordingFingerprint`, `Transcript`/`Segment`/
  `Word` (word-level timing + confidence + source-recording binding),
  `Speaker`, `EditOperation`, and `ExportJob`/`ExportFormat` types, with
  query helpers and unit tests.
- Edit engine (`tpt-app-voice-studio-edit`): the non-destructive
  edit-decision-list (spec §8, §25 steps 6–7). `EditSession` validates and
  applies `DeleteRange`/`Reorder`/`Trim`/`Mute`/`SelectTake` against the
  working transcript, with undo/redo computed by history replay so the
  original alignment binding always survives. `edit::timeline` derives the
  rendered output timeline (gap-less cut semantics) for playback/export.
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
