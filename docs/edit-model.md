# Edit Model

Status: engine implemented in `tpt-app-voice-studio-edit` (spec §25 steps
6–7). Playback/export rendering against this model lands with the export
crate; review panels and take wiring land with the UI and `tpt-voice`
integration.

## Purpose

Describes the non-destructive edit-decision-list (EDL): how transcript edits
become reversible audio operations, and how the same source recording can
support multiple divergent edits (e.g. a long-form and a short-form cut).

## Core model

```
Original recording
        |
        v
   Edit-decision-list  <-- transcript edits recorded here
        |
        v
  Rendered preview / export
```

Every edit operation is appended to an ordered, reversible edit history
rather than mutating source audio (spec §3.3, §6.5):

- `DeleteRange { segment, word_range }`
- `Reorder { segment, new_position }`
- `Trim { recording, range }`
- `Mute { recording, range }`
- `SelectTake { segment, recording }`

Editing operations surfaced to the user (spec §8): delete selected
words/sentences; reorder segments; trim leading/trailing silence; mute a
range without deleting it (redactions); select between alternate takes of the
same line.

## Review-first cleanup (spec §3.5, §8.1)

Filler words, long silences, and breaths are detected and offered as
suggestions in a review panel (accept individually, accept all, ignore) —
never silently removed. Filler lists are configurable per language.

## Multi-take selection (spec §8.2)

Where a speaker recorded multiple attempts at the same line, aligned takes
are shown and the editor chooses which appears in the final edit without
losing the others.

## Implemented engine semantics

`EditSession` (in `tpt-app-voice-studio-edit`) holds the original aligned
transcript, the recording metadata, and the ordered operation history. The
working transcript is always the replay of the history against the original,
so apply, undo, and redo share one code path and the original audio binding
survives every edit (spec §3.2).

Per-operation application rules (all covered by unit tests):

- **DeleteRange** removes the addressed half-open word range; segments
  emptied by deletion are retained (stable identity for undo/history).
- **Reorder** moves a segment; the target index is interpreted in the
  segment list *after* the segment is removed.
- **Trim** cuts a time range from one recording: words fully inside are
  removed; partially overlapping words are clamped to the cut boundaries and
  dropped if the clamp empties them; a word spanning the whole cut is kept
  intact (words are atomic). Only words bound to the trimmed recording are
  affected.
- **Mute** records a silenced range for render/export (`EditSession::
  muted_ranges`); the working transcript is unchanged.
- **SelectTake** records the take choice; the transcript-level take swap is
  wired when take alignment lands (`tpt-voice` integration).

Validation rejects unknown segments/recordings, out-of-bounds word ranges
and reorder positions, and empty or out-of-duration time ranges — leaving
the session untouched.

Rendered timing policy (`edit::timeline`): each kept word occupies its
original duration on the output timeline, concatenated without gaps in
playback order, so deleting a span removes the audio between its neighbours
(a physical cut). Word source ranges are never remapped; deliberate pause
retention belongs to silence-trim configuration (spec §10).

To be documented in Phase 1: UI undo/redo affordances, EDL persistence
format (spec §15), and per-speaker cleanup interaction with the EDL
(spec §9–10).
