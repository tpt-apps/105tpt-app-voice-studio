# TPT Voice Studio

**Edit spoken-word audio by editing its transcript.**

TPT Voice Studio is a native, offline-first desktop application (with a
first-class CLI) for transcript-driven editing of podcasts, interviews,
documentaries, e-learning narration, and legal/corporate recordings. It
transcribes, diarises, and forcibly aligns recorded speech to word-level
timing, then treats the transcript as an editable, non-destructive
representation of the audio itself: delete or reorder text and the
corresponding audio edit is applied automatically — reviewable, undoable, and
exportable without ever touching the original file.

Part of the TPT Apps family, built by TPT Solutions on the open TPT
voice/audio foundation.

## Why this exists

Transcript-based editing is established (Descript, Adobe text-based editing),
but existing options are subscription cloud services or waveform-first DAWs
with transcription bolted on. Voice Studio's differentiators:

- **Offline first** — no cloud transcription, no account, no telemetry; core
  workflow needs no internet at all.
- **No subscription** — perpetual licence as the default commercial model.
- **One deterministic local pipeline** — diarisation, forced alignment,
  isolation/denoise, and loudness normalization in a single native Rust engine.
- **Visible confidence** — every word carries a confidence score; low-confidence
  words are flagged for review, never silently trusted.
- **Multi-track/multi-mic sessions** — professional interview setups, not just
  single-file uploads.
- **CLI and batch mode** — production teams process many recordings per run,
  driven by the same engine as the GUI.

## Repository layout

```
crates/
  tpt-app-voice-studio-core         shared primitives (ids, ranges, errors)
  tpt-app-voice-studio-model        domain model (spec §6)
  tpt-app-voice-studio-transcribe   decode → diarise → transcribe stage
  tpt-app-voice-studio-align        forced alignment + confidence stage
  tpt-app-voice-studio-edit         non-destructive edit-decision-list engine
  tpt-app-voice-studio-cleanup      isolation / denoise / loudness / silence
  tpt-app-voice-studio-export       audio, clip, subtitle, transcript export
  tpt-app-voice-studio-cli          `tpt-voice-studio` CLI (transcribe/batch/export)
  tpt-app-voice-studio-tauri        desktop shell (Tauri)
  tpt-app-voice-studio-test         fixtures, golden data, round-trip/fuzz harnesses
docs/                               architecture and model documentation
fixtures/                           golden audio fixtures (per test category)
tests/                              integration, golden, and round-trip suites
```

The architectural boundary that matters: the deterministic transcript/edit
engine (everything except the Tauri shell) is usable without any GUI, so
studios can batch-process recordings from the CLI.

## Quickstart

Requires a stable Rust toolchain (see `rust-version` in `Cargo.toml`).

```sh
# Build the workspace
cargo build

# Run the CLI (scaffold — commands land in Phase 1)
cargo run -p tpt-app-voice-studio-cli

# Test everything
cargo test
```

## Status

Phase 0 (workspace scaffold) is complete; the MVP build per `spec.txt` §19 is
tracked in [todo.md](todo.md). See [docs/architecture.md](docs/architecture.md)
for the engine/shell split and the current state of the TPT foundation
dependencies (`tpt-voice`, `tpt-dsp` are not yet resolvable and are flagged as
dependency risks).

## Documentation

- [Architecture](docs/architecture.md) — crate graph, pipeline, foundation status
- [Transcript model](docs/transcript-model.md) — words, segments, confidence
- [Edit model](docs/edit-model.md) — the non-destructive edit-decision-list
- [Alignment](docs/alignment.md) — word timing and multi-track reconciliation
- [Export formats](docs/export-formats.md) — audio, clips, subtitles, transcripts
- [Accuracy](docs/accuracy.md) — confidence handling and review workflow

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution terms.
