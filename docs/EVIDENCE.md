# Evidence index

This file lists what the work produced and what it did not. The result figures
are read from the files, not typed from memory.

## Evidence files

```text
docs/DESIGN.md
docs/spikes/P1-06.md
docs/spikes/P2-09.md
testdata/MANIFEST.json
testdata/malformed/index.json
testdata/results/bench-P4-10.json
testdata/results/bench-P4-11.json
testdata/results/conformance-P1-12.json
testdata/results/conformance-P2-12.json
testdata/results/fuzz-diff_accept-P4-08.json
testdata/results/fuzz-diff_roundtrip-P4-07.json
testdata/results/fuzz-no_panic_block-P4-04.json
testdata/results/fuzz-no_panic_frame-P4-05.json
testdata/results/fuzz-no_panic_stream-P4-06.json
testdata/results/malformed-P2-13.json
```

`testdata/malformed/index.json` lists the 144 malformed files and the rule each breaks.

## Results

| File | Headline counts |
|---|---|
| `testdata/results/conformance-P1-12.json` | 156 of 156 passed, category `block` only; `corpus_sha256` b3d9df463fc95a8fe662fd023d9c39a7415277109a096f42599e43c836ae58c8 |
| `testdata/results/conformance-P2-12.json` | 2349 of 2349 passed across all ten categories; `corpus_sha256` fd6392a28a4b239780f073ad70015c93f589163e4d232a9dd11502c24c93ba05 |
| `testdata/results/fuzz-no_panic_block-P4-04.json` | target `no_panic_block`, 3,601 s, 20,681,767 runs, 0 crashes, 0 divergences, 1,249 corpus entries, seed 1 |
| `testdata/results/fuzz-no_panic_frame-P4-05.json` | target `no_panic_frame`, 3,601 s, 101,577,396 runs, 0 crashes, 0 divergences, 834 corpus entries, seed 1 |
| `testdata/results/fuzz-no_panic_stream-P4-06.json` | target `no_panic_stream`, 3,601 s, 21,051,494 runs, 0 crashes, 0 divergences, 895 corpus entries, seed 1 |
| `testdata/results/fuzz-diff_accept-P4-08.json` | target `diff_accept`, 3,601 s, 21,607,948 runs, 0 crashes, 100 divergences (the file cap; inputs in `testdata/results/divergences/`, untriaged), 820 corpus entries, seed 1 |
| `testdata/results/fuzz-diff_roundtrip-P4-07.json` | target `diff_roundtrip`, 3,601 s, 58,050 runs, 0 crashes, 0 divergences, 444 corpus entries, seed 1 |
| `testdata/results/malformed-P2-13.json` | 144 of 144 matched their expected error, 0 panics |
| `testdata/results/bench-P4-10.json` | 23 corpus files; cleanlz4 413.8 to 2,224.6 MB/s, oracle 188.0 to 1,957.9 MB/s; no target |
| `testdata/results/bench-P4-11.json` | 23 corpus files at the final `bench.repeats`; cleanlz4 437.6 to 2,048.7 MB/s, oracle 182.6 to 1,988.1 MB/s; no target |

The fuzz results so far are `fuzz-no_panic_block-P4-04.json`, `fuzz-no_panic_frame-P4-05.json` and `fuzz-no_panic_stream-P4-06.json` `fuzz-diff_roundtrip-P4-07.json` and `fuzz-diff_accept-P4-08.json`.

## Done checklist

SPEC section 1, "Done means":

1. **Every requirement in section 6 has its tests: not met.**
   Shown by `docs/DESIGN.md` (Spec rules implemented) and the tests under
   `crates/cleanlz4/tests/` and `crates/oracle/tests/`. Missing: dictionary
   support for frames (`DictionaryRequired`, `DictionaryMismatch`, a supplied
   dictionary applied to frames; task P2-08), dictionary differential tests
   (P1-10), the full conformance corpus run (P2-11, P2-12), the malformed corpus
   (P2-13), and streaming of skippable and legacy frames, dictionaries and the
   trailing-data policy (P3-02, P3-03).
2. **Every command in section 7 exits zero: owed to the summarize stage.**
   `make check` ran green on every committed task through `kukulu_verify`; the
   fuzz command was never run.
3. **The fuzz campaign has run for its configured duration with zero crashes and
   zero divergences: not met.**
   Five fuzz results exist (P4-04 to P4-08); `diff_accept` found 100 divergences awaiting triage (P4-09).
4. **`kukulu_similarity` and `kukulu_attest` have produced `docs/SIMILARITY.md`
   and a verifying bundle: owed to the summarize stage.**
5. **A draft PR is open: owed to the summarize stage** (`kukulu_run_finish`
   opens it).

### Blocked and skipped tasks

- P1-10 (blocked): the binding crate has no `LZ4_decompress_safe_usingDict`.
- Skipped because of it: P2-08, P2-11, P2-12, P2-13, P3-02, P3-03, P3-06, and
  P4-01 to P4-09 and P4-12.
