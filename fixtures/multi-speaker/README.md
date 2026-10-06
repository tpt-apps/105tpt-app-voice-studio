# fixtures/multi-speaker

Golden fixtures with two or more speakers conversing (spec §18.2).

Each fixture consists of:

- `audio` — the source recording (rights-cleared or synthesized).
- `expected-transcript` — the reference transcript text.
- `expected-speakers` — the expected speaker segmentation (diarisation ground truth).
- `tolerance` — the permitted alignment deviation.

Also exercises multi-take scenarios (repeated deliveries of similar lines,
spec §8.2) and speaker rename/merge/split cases (spec §9).
