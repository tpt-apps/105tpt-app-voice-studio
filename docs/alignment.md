# Alignment

Status: skeleton — implemented in Phase 1 (spec §25 step 5; pipeline §7).

## Purpose

Describes forced alignment (word-level timestamps bound to the transcript),
per-word confidence scoring, and multi-track timing reconciliation.

## Pipeline position (spec §7)

```
Import → Decode (Cadence) → Diarisation (tpt-voice) → Transcription (tpt-voice)
       → Forced alignment (tpt-voice) → Confidence scoring per word
       → Editable transcript
```

## Confidence surfacing (spec §7.1)

- High confidence — rendered normally.
- Medium confidence — subtly flagged.
- Low confidence — clearly flagged, inviting manual correction.

Confidence is retained for the life of the project and drives the
"low-confidence words only" review filter (spec §17.2). The application never
presents automatically transcribed text as verified fact (spec §17.1).

## Multi-track alignment (spec §7.2)

When a project holds multiple microphone tracks of one session, alignment
reconciles timing across tracks — even when tracks were not perfectly
time-synchronised at capture — so a transcript edit maps back to the correct
source recording and sample range per speaker.

To be documented in Phase 1: alignment tolerances per fixture category
(spec §18.2), drift correction strategy for long recordings, confidence
score provenance and tiering thresholds, and round-trip re-alignment
tolerances after edits (spec §18.3).
