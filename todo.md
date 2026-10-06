# tpt-app-voice-studio — Project Todo

Tracks all work for **TPT Voice Studio**, the offline-first, transcript-driven audio editor (TPT Solutions).
License: dual **MIT OR Apache-2.0**.

---

## Phase 0: Project Setup & Foundation

- [x] Verify foundation crates are reachable from this workspace (path dependency or registry): `tpt-voice` (transcription/diarisation/alignment/isolation), `tpt-audio`/`tpt-dsp` (denoise/normalization/silence detection), `tpt-cadence` (codecs), `tpt-av-asset` (project/job persistence), `tpt-av-test` (fixtures/conformance) — spec §5
  - Status 2026-10-07: `tpt-cadence`, `tpt-audio` (`tpt-av-audio*` members), `tpt-av-asset`, and `tpt-av-test` all exist as sibling workspaces and are path-wired in the root `Cargo.toml` (entries only; members switch over in Phase 1). `tpt-cadence` is now resolvable — the earlier concern is resolved.
  - **Dependency risk (open):** `tpt-voice` (the primary speech engine) does not exist yet anywhere reachable, and `tpt-dsp` exists only as a design doc (`tpt-foundations/17-tpt-dsp.md`). See docs/architecture.md; engine-facing crates stay trait-based until these land.
- [x] Initialize git repository, add `.gitignore` (Rust/Cargo template)
- [x] Create Cargo workspace `Cargo.toml` (members per spec §4: `-core`, `-model`, `-transcribe`, `-align`, `-edit`, `-cleanup`, `-export`, `-cli`, `-tauri`, `-test`)
- [x] Create `LICENSE-MIT` and `LICENSE-APACHE` (dual license, copyright TPT Solutions)
- [x] Set `license = "MIT OR Apache-2.0"` in workspace `Cargo.toml`
- [x] Create `deny.toml` (cargo-deny license/advisory enforcement)
- [x] Scaffold `tpt-app-voice-studio-core` crate
- [x] Scaffold `tpt-app-voice-studio-model` crate (domain model per spec §6)
- [x] Scaffold `tpt-app-voice-studio-transcribe` crate
- [x] Scaffold `tpt-app-voice-studio-align` crate
- [x] Scaffold `tpt-app-voice-studio-edit` crate
- [x] Scaffold `tpt-app-voice-studio-cleanup` crate
- [x] Scaffold `tpt-app-voice-studio-export` crate
- [x] Scaffold `tpt-app-voice-studio-cli` crate
- [x] Scaffold `tpt-app-voice-studio-tauri` crate
- [x] Scaffold `tpt-app-voice-studio-test` crate
- [x] Create `README.md` (project overview, positioning, quickstart)
- [x] Create `CONTRIBUTING.md`
- [x] Create `CHANGELOG.md`
- [x] Create `docs/` skeleton: `architecture.md`, `transcript-model.md`, `edit-model.md`, `alignment.md`, `export-formats.md`, `accuracy.md`
- [x] Set up CI (GitHub Actions): build, test, clippy, fmt check
- [x] Add `cargo-deny check` to CI
- [x] Scaffold `fixtures/` directories: `single-speaker/`, `multi-speaker/`, `noisy/`, `multi-track/`, `accents/` — spec §18.2
- [x] Scaffold `tests/` directories: `integration/`, `golden/`, `round-trip/`

---

## Phase 1: MVP Build

Goal: deliver the full MVP per spec §19 and Definition of Done per spec §24, following the recommended implementation order in spec §25.

### Ingestion & Codecs
- [ ] Integrate `tpt-cadence`; enumerate supported import/export formats (WAV, MP3, AAC, FLAC) — spec §25 step 2
- [ ] Single-file import — spec §19
- [ ] Multi-track/multi-mic session import — spec §19
- [ ] Drag-and-drop import — spec §19

### Domain Model
- [ ] Implement `Project`, `Recording`, `TrackRole` types — spec §6.1–6.2
- [ ] Implement `Transcript`, `Segment`, `Word` types with confidence field — spec §6.3
- [ ] Implement `Speaker` type — spec §6.4
- [ ] Implement `EditOperation` enum (`DeleteRange`, `Reorder`, `Trim`, `Mute`, `SelectTake`) — spec §6.5
- [ ] Implement `ExportJob`, `ExportFormat` types — spec §6.6

### Transcription, Diarisation & Alignment
- [ ] Integrate `tpt-voice` for transcription and diarisation — spec §25 step 3
- [ ] Implement forced alignment integration and per-word confidence scoring — spec §25 step 5
- [ ] Confidence surfacing: high/medium/low visual tiers, retained for life of project — spec §7.1
- [ ] Multi-track alignment reconciliation across imperfectly synchronised tracks — spec §7.2

### Non-Destructive Editing
- [ ] Implement the non-destructive edit-decision-list (EDL) model — spec §25 step 6
- [ ] Implement transcript-based delete/reorder editing against the EDL — spec §25 step 7
- [ ] Implement trim leading/trailing silence operation — spec §8
- [ ] Implement mute-range-without-deleting operation (redactions) — spec §8
- [ ] Implement filler-word and silence detection (configurable per language) with manual review — spec §25 step 8, §8.1
- [ ] Implement multi-take detection/selection — spec §25 step 9, §8.2

### Speaker & Multi-Track Handling
- [ ] Diarisation-driven speaker labeling with rename/merge/split — spec §9
- [ ] Per-speaker independent cleanup application — spec §9

### Cleanup & Enhancement
- [ ] Integrate `tpt-audio`/`tpt-dsp` for isolation, denoise, and normalization — spec §25 step 10
- [ ] Voice isolation with before/after preview — spec §10
- [ ] Denoise (parametric) with before/after preview — spec §10
- [ ] Loudness normalization (target LUFS) with before/after preview — spec §10
- [ ] Silence trimming (configurable threshold/min retained gap) with before/after preview — spec §10

### Persistence & Background Jobs
- [ ] Implement SQLite persistence for projects, transcripts, and edit history — spec §25 step 11, §15
- [ ] Implement background job handling via `tpt-av-asset` for long transcription/alignment jobs — spec §25 step 12
- [ ] Waveform caching and resumable processing for long recordings — spec §5.1 (`tpt-av-asset`)

### Export
- [ ] Audio export (WAV/MP3/AAC/FLAC) rendered from the EDL — spec §25 step 13, §11
- [ ] Clip/highlight export from selected transcript ranges — spec §11
- [ ] SRT/VTT subtitle export from aligned, edited transcript — spec §11
- [ ] Timestamped, speaker-labeled transcript export — spec §11
- [ ] Optional legal/formal-use disclaimer on transcript export — spec §17.3

### CLI
- [ ] Implement CLI: `transcribe`, `batch-transcribe`, `export` — spec §25 step 14, §13
- [ ] Implement stable exit-code contract (0–7) — spec §13
- [ ] Machine-readable (JSON) result output — spec §13

### Desktop UI
- [ ] Implement desktop transcript editor UI with linked waveform — spec §25 step 15, §12.1
- [ ] Implement speaker track view — spec §25 step 16, §12.2
- [ ] Implement filler/silence review panel (batch accept/reject) — spec §25 step 17, §12.3, §8.1
- [ ] Implement cleanup panel with before/after preview — spec §25 step 18, §12.4
- [ ] Implement export panel (format/range selection, job progress) — spec §25 step 19, §12.5
- [ ] Implement project/asset browser (recordings, project list, job status) — spec §12.6
- [ ] Low-confidence-only filter view for review workflow — spec §17.2

### Local API (optional)
- [ ] Localhost-only API, disabled by default, bound to 127.0.0.1 — spec §14

### Security & Privacy
- [ ] Safe handling of malformed/corrupt audio files — spec §16
- [ ] Strict path validation and safe temporary-file handling — spec §16
- [ ] No automatic external asset fetching; no mandatory network access — spec §16

### Testing
- [ ] Unit tests per pipeline stage (decode, diarisation, transcription, alignment, edit application, export): valid/invalid/boundary/malformed cases — spec §18.1
- [ ] Golden audio/transcript test suite across fixture categories — spec §25 step 20, §18.2
- [ ] Round-trip edit/export tests (edit → EDL → render → re-import/re-align consistency) — spec §25 step 21, §18.3
- [ ] Property tests: timestamp/word-range mapping, EDL ordering/application, subtitle timing, confidence aggregation — spec §18.4
- [ ] Fuzzing for audio/container parsers, transcript import/export parsers, project file parsing, CLI arguments — spec §25 step 22, §18.5
- [ ] Regression fixture process: every production bug gets a permanent regression test — spec §18.6

### Hardening, Packaging & Beta
- [ ] Benchmark and profile against long-form (60+ minute) multi-track recordings — spec §25 step 23, §24
- [ ] Harden error handling and failure isolation — spec §25 step 24
- [ ] Package Windows release; verify clean-machine installation without dev tooling — spec §24
- [ ] Run private beta with real podcast/interview producers — spec §25

---

## Phase 2: Post-MVP Expansion

- [ ] Text-to-speech for gap-filling and guide narration (via `tpt-voice` TTS) — spec §20
- [ ] Multi-language transcription/alignment support — spec §20
- [ ] Live/streaming transcription during recording (beyond post-recording import) — spec §20
- [ ] Richer subtitle styling/templating — spec §20
- [ ] Watch-folder batch ingestion — spec §20

---

## Phase 3: Team & Platform Features

- [ ] Collaboration/review workflows (reviewer comments, per-segment approval status) — spec §20
- [ ] Plugin SDK for custom cleanup/export operations — spec §20
- [ ] Vertical template: legal verbatim mode — spec §20
- [ ] Vertical template: e-learning chaptering/module export — spec §20
- [ ] Vertical template: documentary rough-cut assembly from transcript selections across many source recordings — spec §20
- [ ] Studio-tier: multi-machine licensing, local network/file-based team project sharing, advanced export templating, commercial support — spec §22 (build only once customer demand is evidenced — spec §22, §23)
