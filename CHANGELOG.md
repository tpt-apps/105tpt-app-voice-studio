# Changelog

All notable changes to TPT Voice Studio are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Speech engine integration (`transcribe` crate, spec §25 step 3, §7): the
  `tpt-voice` foundation is now a git dependency (public GitHub repos) and
  drives the pipeline. `TranscriptionEngine` port with the real
  `TptVoiceEngine` (`SpeechPipeline`: ASR → diarisation → speaker
  annotation → word alignment) and a deterministic `FakeTranscriptionEngine`
  for model-free tests; engine output converts into the domain model with
  per-word confidence and a speaker registry.
- WAV import (`transcribe::decode`) on the `tpt-cadence` foundation:
  8/16/24/32-bit PCM and 32/64-bit float, any channel count, mono-downmixed
  for the engine, with the supported-format list centralised.
- Functional CLI transcription (spec §13): `transcribe` and
  `batch-transcribe` run the real engine over WAV input with `--model-dir`,
  emit the `words_transcribed`/`average_confidence` result fields, and
  implement batch partial-success semantics (exit 0/1/2). Missing model
  configuration is a `CONFIGURATION_ERROR` (5) with guidance; tests needing
  a real model skip unless `TPT_VOICE_MODEL_DIR` is set.
- Project file persistence (`tpt-app-voice-studio-model::project_file`): the
  versioned `.tptproj` JSON envelope referenced by the spec §13 CLI — full
  project including recordings metadata, transcript, speakers, and the EDL;
  round-trip, version-rejection, and disk I/O tested. Serde added to the
  core and model types (ids transparent, confidence as a validated number).
- `EditSession::from_history`: reconstructs an edited working transcript by
  validating and replaying a saved edit history — the path used by project
  files, the desktop app, and the CLI alike.
- CLI (`tpt-voice-studio`, spec §13): `export` is end-to-end functional for
  SRT/VTT subtitles and plain/timestamped/JSON transcripts from project
  files, with `--disclaimer` (spec §17.3) and speaker-label options;
  `transcribe`/`batch-transcribe` are wired with the contract exit codes and
  explicit unavailable-engine messages until `tpt-voice` is resolvable.
  `--json` emits the machine-readable result envelope; the exit-code
  contract (0–7) is enforced and integration-tested against the compiled
  binary.
- Reviewable cleanup detection (`tpt-app-voice-studio-edit::detect`,
  spec §8.1, §25 step 8): filler-word detection against configurable
  per-language lists (conservative English default, multi-word phrases,
  punctuation/case-insensitive) and long-silence detection from word gaps
  with configurable threshold and retained gap. Suggestions convert
  one-to-one into EDL operations and are never applied automatically
  (spec §3.5).
- Subtitle export (`tpt-app-voice-studio-export::subtitles`): SRT/VTT cue
  packing from the rendered output timeline with configurable duration and
  line limits, optional speaker labels (spec §11).
- Transcript export (`...::transcript`): plain-text, timestamped
  speaker-labeled (spec §11 example layout), and machine-readable JSON
  formats, all positioned on the rendered timeline so exports stay in sync
  with the edited audio, with the spec §17.3 formal-use disclaimer on
  opt-in (unverified/reviewed modes).
- Clip selection (`...::clip`): computes the exact source ranges a renderer
  must cut for any rendered-time selection over the edited transcript — the
  cutting instruction for clip/highlight export (audio render pending the
  codec integration).
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
- Changed the foundation dependency strategy: `tpt-voice`, `tpt-cadence` are
  consumed as git dependencies from `github.com/tpt-solutions/*` (public
  repos), replacing sibling path dependencies — CI and clean machines build
  anywhere; local-workspace overrides documented in docs/architecture.md.
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
