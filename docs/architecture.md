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

| Foundation | Provides | Status |
|---|---|---|
| `tpt-cadence` | codec import/export (WAV, MP3, AAC, FLAC) | Present as sibling workspace; path entries wired in root `Cargo.toml`; member integration in Phase 1 step 2 |
| `tpt-audio` | waveform/audio primitives (`tpt-av-audio*`) | Present as sibling workspace; path entries wired; member integration in Phase 1 step 10 |
| `tpt-av-asset` | persistence, background jobs, waveform cache, watcher | Present as sibling workspace; path entries wired; member integration in Phase 1 steps 11–12 |
| `tpt-av-test` | fixtures, fuzzing, conformance | Present as sibling workspace; path entries wired; integration in Phase 1 steps 20–22 |
| `tpt-voice` | transcription, diarisation, forced alignment, voice isolation | **Missing — dependency risk.** Repo does not exist yet anywhere reachable; this is the primary speech engine (spec §5.1). Engine-facing crates (`-transcribe`, `-align`) therefore define trait boundaries; implementations land when the crate exists |
| `tpt-dsp` | denoise, loudness measurement, silence/breath detection | **Missing — dependency risk.** Design doc only (`tpt-foundations/17-tpt-dsp.md`); `-cleanup` ships against `tpt-audio` primitives in the interim |

### Path-dependency strategy and risks

Foundation crates are wired as **path dependencies into sibling workspaces**
(`../tpt-cadence/...` etc.) via `[workspace.dependencies]`. This is exact for
TPT-internal development machines, but:

- A clean machine or GitHub Actions runner without the sibling repositories
  cannot resolve these paths. Members must not reference them until a
  distribution story exists (vendoring, a private registry, or git
  dependencies/submodules). Until then, workspace builds carry **zero**
  foundation dependencies and CI stays green anywhere.
- Cross-workspace path dependencies create lockstep version coupling; when
  members start consuming foundations in Phase 1, prefer depending on the
  narrowest member crates (e.g. `tpt-av-cadence-core`, not the whole codec set).

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
