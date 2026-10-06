# Export Formats

Status: text-timeline exports implemented in `tpt-app-voice-studio-export`
(subtitles, transcripts, clip selection); audio export lands with the
`tpt-cadence` integration (spec §25 step 13).

## Implemented behaviour

All text exports are positioned on the **rendered output timeline**
(`edit::timeline`), so subtitles, transcripts, and clips stay in sync with
the final edited audio without a re-timing pass (spec §11).

- **Subtitles (SRT/VTT)** — `export::subtitles` packs the rendered timeline
  into cues bounded by configurable duration (default 5 s) and line budget
  (default 2 × 42 chars); cues never span segments; optional speaker-label
  prefixes on segment-starting cues. SRT uses `HH:MM:SS,mmm`, VTT uses dots
  and a `WEBVTT` header.
- **Transcripts** — `export::transcript` renders plain text, timestamped
  speaker-labeled lines in the spec §11 layout (`[00:00:12] Host: ...`), or
  JSON (`format_version`, per-segment rendered timing/text plus per-word
  timing and confidence; confidences serialize rounded to 4 decimal places).
  Speaker labels come from the project speaker registry (spec §9).
- **Formal-use disclaimer (spec §17.3)** — `DisclaimerMode::{None,
  Unverified, Reviewed}` attaches the canonical disclaimer text (and the
  reviewed note) on opt-in only.
- **Clips/highlights** — `export::select_clip` turns a rendered-time
  selection into the exact cutting instruction: included words with source
  ranges clamped proportionally at selection boundaries, rebased rendered
  positions, and total duration. The audio file render through `tpt-cadence`
  consumes this directly.

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

To be documented in Phase 1: audio container/codec defaults per format, clip
file naming/metadata, export job status semantics (spec §6.6), and the CLI
result envelope for exports (spec §13).
