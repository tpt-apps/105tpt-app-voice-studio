# Architecture

Status: skeleton — filled in during Phase 1 (spec §25 steps 2–19).

## Purpose

Describes how TPT Voice Studio is structured: the split between the
deterministic transcript/edit engine and the application shell, the pipeline
stages, and the relationship to the TPT foundation crates.

## Engine and shell are separate (spec §3.6)

```
                  Voice Studio Application
                          |
             +------------+-------------+
             |                          |
          Desktop (tauri)            CLI/Batch (cli)
             |                          |
             +------------+-------------+
                          |
                   Voice Studio Engine
                          |
        +------------------+------------------+
        |                  |                  |
   Transcription      Alignment/          Edit/Export
   + Diarisation       Speaker            (EDL, cleanup,
                        Model              rendering)
```

Crate responsibilities (mapping to `crates/`):

| Crate | Responsibility | Key spec sections |
|---|---|---|
| `-core` | ids, time ranges, confidence tiers, shared errors | §6, §7.1 |
| `-model` | domain model: project, recording, transcript, speaker, EDL entries, export jobs | §6 |
| `-transcribe` | decode → diarise → transcribe | §7 |
| `-align` | forced alignment, per-word confidence, multi-track reconciliation | §7.1–7.2 |
| `-edit` | edit-decision-list, transcript editing, filler/silence/take operations | §8 |
| `-cleanup` | isolation, denoise, loudness, silence trimming (reviewable ops) | §9–10 |
| `-export` | audio/clip/subtitle/transcript rendering from the EDL | §11 |
| `-cli` | `tpt-voice-studio` binary, stable exit codes, JSON results | §13 |
| `-tauri` | desktop shell and screens; optional localhost API | §12, §14 |
| `-test` | fixtures, golden/round-trip/property/fuzz support | §18 |

## Non-destructive render pipeline (spec §3.3)

```
Original recording
        |
        v
   Edit-decision-list  <-- transcript edits recorded here
        |
        v
  Rendered preview / export
```

To be documented in Phase 1: the render graph (EDL application, preview
playback path, export path), undo semantics, and how derived state (waveforms,
alignment caches) is stored outside the SQLite database (spec §15).

## Foundation dependencies (spec §5.1) — status as of 2026-10-07

The foundations are **public GitHub repos** (`github.com/tpt-solutions/*`,
branch `master`) consumed as git dependencies — no sibling checkouts needed
for CI or clean machines.

| Foundation | Provides | Status |
|---|---|---|
| `tpt-voice` | ASR, diarisation, forced alignment, isolation, TTS — pure Rust, one-crate facade (`SpeechPipeline`) | **Integrated** in the transcribe stage (`transcribe::engine`); real-engine runs need a local ASR model dir (optional download, spec §3.1) |
| `tpt-cadence` | codec import/export (WAV, MP3, AAC, FLAC) | WAV **decode integrated** (`transcribe::decode`); MP3/AAC/FLAC decode and all encode wired as integrations land |
| `tpt-audio` | audio engine facade (`tpt-av-audio`: timeline, mixer, I/O, cadence-backed decode) | Available; consumed when cleanup/render integration lands |
| `tpt-av-asset` | persistence, background jobs, waveform cache, watcher | Available; pending the library-store decision (see Persistence layers below) |
| `tpt-av-test` | fixtures, fuzzing, conformance | Available; consumed when the golden/fuzz suites land (spec §25 steps 20–22) |
| `tpt-dsp` | parametric denoise, loudness measurement, audio-level silence/breath detection | **Missing** — design doc only (`tpt-foundations/17-tpt-dsp.md`). Cleanup ships against `tpt-audio`/`tpt-voice` primitives in the interim; the word-gap silence detection in `edit::detect` does not need it |

### Path-dependency strategy and risks

Foundation crates are wired as **git dependencies**
(`git = "https://github.com/tpt-solutions/<repo>", branch = "master"`) in
`[workspace.dependencies]`. Consequences:

- CI and clean machines resolve the repos directly from GitHub; commits are
  pinned by `Cargo.lock`. Note that `Cargo.lock` is gitignored in this
  repository, so builds float on `master` until it is committed — revisit
  that choice when the dependency set stabilises.
- TPT machines with local sibling workspaces can redirect to them per-machine
  via a `[patch]` section in `~/.cargo/config.toml`:

  ```toml
  [patch."https://github.com/tpt-solutions/tpt-cadence"]
  tpt-av-cadence-core = { path = "C:/path/to/tpt-cadence/tpt-av-cadence-core" }
  tpt-av-cadence-wav = { path = "C:/path/to/tpt-cadence/tpt-av-cadence-wav" }
  ```

- Only crates a member actually consumes are wired; add the rest as their
  integrations land (see the commented list in the root `Cargo.toml`).
- Models: the ASR engine loads from a local model directory at runtime
  (`Transcriber::from_model_dir`). Model files are optional downloads and are
  never fetched automatically — consistent with offline-first (spec §3.1) and
  the no-automatic-asset-fetching rule (spec §16). Tests that need a model
  skip unless `TPT_VOICE_MODEL_DIR` is set.

To be documented in Phase 1: background job model via `tpt-av-asset` (spec
§25 step 12), failure isolation boundaries (spec §25 step 24), and the
security posture (offline operation, path validation, safe temp files,
fuzzing targets — spec §16).

## Persistence layers (spec §15) — status

Two distinct stores, per spec:

1. **Project file** (`.tptproj`, implemented in `model::project_file`) — the
   portable unit of work the spec §13 CLI operates on: a versioned JSON
   envelope around the full `Project` (recording metadata as paths plus
   fingerprints, transcript, speakers, and the edit-decision-list). Original
   audio is never embedded (spec §15). Opening a project replays the saved
   history through `EditSession::from_history`, which validates every
   operation — a tampered or mismatched history is rejected, not guessed at.
2. **SQLite library state** (not started) — project list, user preferences,
   and export job metadata (spec §15). Open design decision: the spec
   names SQLite, while the existing foundation crate `tpt-av-asset-db` is
   built on redb; pick one before wiring the library store. Waveform and
   alignment caches live outside this database either way (spec §15).
