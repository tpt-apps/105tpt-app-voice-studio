# fixtures/noisy

Golden fixtures with deliberately degraded audio: background noise, room
tone, music beds, and low recording quality (spec §18.2).

Each fixture consists of:

- `audio` — the degraded source recording.
- `expected-transcript` — the reference transcript text.
- `expected-speakers` — the expected speaker segmentation.
- `tolerance` — the permitted alignment deviation (expected to be wider than
  the clean categories; document the value per fixture).

These fixtures drive the confidence-tiering tests (low-confidence words must
be flagged, spec §7.1) and the voice-isolation/denoise cleanup tests
(spec §10).
