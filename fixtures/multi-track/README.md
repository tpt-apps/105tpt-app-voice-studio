# fixtures/multi-track

Golden fixtures of multi-mic sessions: several recordings of the same
session, one per participant, imperfectly time-synchronised (spec §7.2, §18.2).

Each fixture consists of:

- `audio` — one file per track (matching filenames per session).
- `expected-transcript` — the reference transcript with per-speaker attribution.
- `expected-speakers` — the expected speaker-to-track mapping.
- `tolerance` — the permitted cross-track alignment deviation.

Used by the multi-track alignment reconciliation tests and by round-trip
tests that verify transcript edits map back to the correct source recording
and sample range (spec §18.3).
