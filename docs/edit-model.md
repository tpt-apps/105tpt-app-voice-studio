# Edit Model

Status: skeleton — implemented in Phase 1 (spec §25 steps 6–9; editing §8).

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

To be documented in Phase 1: EDL application semantics (order, idempotence,
composition), timestamp remapping for playback/export after each operation,
undo/redo mechanics, and how operations interact across multi-track projects.
