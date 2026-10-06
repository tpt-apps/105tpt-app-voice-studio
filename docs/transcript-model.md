# Transcript Model

Status: skeleton — implemented in Phase 1 (spec §25 step 4; domain model §6).

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

## Planned types (spec §6.3–6.4)

- `Transcript { segments: Vec<Segment> }`
- `Segment { id, speaker: Option<SpeakerId>, words: Vec<Word> }`
- `Word { text, start, end, confidence, source_recording }`
- `Speaker { id, label, recordings }`

To be documented in Phase 1: identifier scheme and stability across edits,
segment ordering guarantees, confidence tier thresholds (high/medium/low),
speaker rename/merge/split semantics (spec §9), and serialization for SQLite
persistence (spec §15).
