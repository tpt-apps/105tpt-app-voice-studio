# Accuracy and Confidence Handling

Status: skeleton — refined during Phase 1 (spec §17, §18.2).

## Purpose

Documents how Voice Studio communicates transcription uncertainty and keeps
it trustworthy over the life of a project. The application must never present
automatically transcribed text as a verified verbatim record without
qualifying confidence information.

## Principles

- **No silent overconfidence** (§17.1): every word carries a confidence score;
  low-confidence words are visually distinguished in the editor.
- **Review workflow** (§17.2): the transcript editor supports filtering to
  "low-confidence words only", so a legal or corporate reviewer audits exactly
  the portions needing human verification instead of re-reading everything.
- **Explicit disclaimers on export** (§17.3): formal/legal transcript exports
  can carry the automated-recognition disclaimer on opt-in (see
  [export-formats.md](export-formats.md)).
- **Confidence persistence** (§7.1): scores stay attached to words for the
  life of the project, so review filters work at any time, not only right
  after import.

## Golden fixtures and tolerances (spec §18.2)

The controlled fixture set (`fixtures/single-speaker/`, `multi-speaker/`,
`noisy/`, `multi-track/`, `accents/`) carries, per fixture: an expected
transcript, expected speaker segmentation, and an alignment tolerance.
Golden tests compare engine output against these within tolerance.

To be documented in Phase 1: concrete tier thresholds (high/medium/low),
how confidence aggregates to segments/projects (e.g. the CLI's
`average_confidence` result field, spec §13), measured accuracy per fixture
category once `tpt-voice` lands, and the regression fixture process for
accuracy bugs (spec §18.6).
