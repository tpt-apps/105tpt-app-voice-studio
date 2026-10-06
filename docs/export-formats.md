# Export Formats

Status: skeleton — implemented in Phase 1 (spec §25 step 13; export §11).

## Purpose

Describes every deliverable Voice Studio renders, all derived from the same
aligned, edited transcript and the edit-decision-list — so exports remain in
sync with the final edited audio without a manual re-timing pass.

## Formats (spec §11)

- **Audio** — WAV, MP3, AAC, FLAC (via `tpt-cadence`), rendered from the EDL.
- **Clips/highlights** — selected transcript ranges exported as standalone
  audio files.
- **Subtitles/captions** — SRT/VTT generated directly from the aligned,
  edited transcript.
- **Transcript** — plain text, timestamped, speaker-labeled, suitable for
  client delivery or legal/corporate record-keeping:

  ```
  [00:00:12] Host: Welcome back to the show.
  [00:00:15] Guest: Thanks for having me.
  ```

## Formal-use disclaimer (spec §17.3)

Transcript exports intended for formal or legal use can include, on opt-in:

> This transcript was generated using automated speech recognition and forced
> alignment. It has not been independently verified for verbatim accuracy
> unless explicitly marked as reviewed.

To be documented in Phase 1: exact SRT/VTT timing derivation from word ranges,
encoding/container defaults per format, clip export naming/metadata, JSON
transcript schema (matching the CLI machine-readable output, spec §13), and
export job status semantics (spec §6.6).
