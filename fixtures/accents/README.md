# fixtures/accents

Golden fixtures covering accent and dialect variation, non-native speech, and
regional pronunciation (spec §18.2).

Each fixture consists of:

- `audio` — the source recording (rights-cleared or synthesized).
- `expected-transcript` — the reference transcript text.
- `expected-speakers` — the expected speaker segmentation.
- `tolerance` — the permitted alignment deviation.

These fixtures are the accuracy benchmark for the confidence system: they
document expected word error rate and confidence behaviour for diverse
speech, feeding docs/accuracy.md once the speech engine integration lands.
