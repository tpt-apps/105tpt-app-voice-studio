# Transcript Model

Status: core model implemented in `tpt-app-voice-studio-model` (spec §6.3–6.4);
serialization for SQLite persistence lands with persistence (spec §15).

## Purpose

Defines the transcript as the primary, editable representation of the
recording: segments, words, speakers, and the per-word confidence and timing
data that bind text to audio.

## Key invariants

- **Transcript and audio are bound by alignment** (spec §3.2): every word
  carries a timestamp range and a link back to the exact audio samples it
  represents, established through forced alignment. The binding must remain
  accurate after every edit operation.
- **Confidence is never discarded** (spec §7.1): each word's confidence score
  stays attached for the life of the project so reviewers can filter to
  "words needing review" at any time (spec §17.2).
- **Words know their source**: each word records which recording it was
  aligned against, which is how multi-track sessions map edits back to the
  right microphone track (spec §6.3, §7.2).

## Implemented types (spec §6.3–6.4)

- `Transcript { segments: Vec<Segment> }` — segments in playback order.
- `Segment { id, speaker: Option<SpeakerId>, words: Vec<Word> }`
- `Word { text, start, end, confidence, source_recording }` — `start`/`end`
  are `Duration`s on the source recording's timeline; ranges are half-open
  (`[start, end)`), matching `core::TimeRange`.
- `Speaker { id, label, recordings }` — labels are human-editable
  (rename/merge/split semantics arrive with speaker handling, spec §9).

Helper semantics already covered by tests: `Transcript::word_count()`,
`Transcript::words()` (playback-order iteration), `Transcript::segment(id)`
lookups, `Segment::span()`, `Word::range()`, and
`Transcript::average_confidence()` (feeds the CLI's `average_confidence`
field, spec §13). `Transcript::is_time_ordered()` validates fresh alignment
output; edited transcripts may deliberately violate it via reorder
operations.

Confidence tiers live in `tpt-app-voice-studio-core::confidence`:
`Confidence` validates `0.0..=1.0` and maps to `ConfidenceTier`
(high ≥ 0.9, medium ≥ 0.6, low below — thresholds are constants on the type
and are product decisions to revisit with real engine data).

To be documented in Phase 1: identifier scheme and stability across edits,
speaker rename/merge/split semantics (spec §9), and serialization for SQLite
persistence (spec §15).
