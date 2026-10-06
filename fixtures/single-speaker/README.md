# fixtures/single-speaker

Golden fixtures with one speaker and clean audio (spec §18.2).

Each fixture consists of:

- `audio` — the source recording (short, rights-cleared or synthesized).
- `expected-transcript` — the reference transcript text.
- `expected-speakers` — the expected speaker segmentation (here: a single speaker).
- `tolerance` — the permitted alignment deviation (word start/end times).

Used by the golden test suite under `tests/golden/`.
