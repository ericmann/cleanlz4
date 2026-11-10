# Evidence index

This file lists what the work produced and what it did not. The result figures
are read from the files, not typed from memory.

## Evidence files

```text
docs/DESIGN.md
docs/spikes/P1-06.md
docs/spikes/P2-09.md
testdata/MANIFEST.json
testdata/results/bench-P4-10.json
testdata/results/bench-P4-11.json
testdata/results/conformance-P1-12.json
```

`testdata/malformed/index.json` is not listed: it was never produced (see
the checklist).

## Results

| File | Headline counts |
|---|---|
| `testdata/results/conformance-P1-12.json` | 156 of 156 passed, category `block` only; `corpus_sha256` b3d9df463fc95a8fe662fd023d9c39a7415277109a096f42599e43c836ae58c8 |
| `testdata/results/bench-P4-10.json` | 23 corpus files; cleanlz4 413.8 to 2,224.6 MB/s, oracle 188.0 to 1,957.9 MB/s; no target |
| `testdata/results/bench-P4-11.json` | 23 corpus files at the final `bench.repeats`; cleanlz4 437.6 to 2,048.7 MB/s, oracle 182.6 to 1,988.1 MB/s; no target |

No `conformance-P2-12.json`, `malformed-*.json` or `fuzz-*.json` exists.

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
   No fuzz target was built and no `fuzz-*.json` exists (P4-01 to P4-09).
4. **`kukulu_similarity` and `kukulu_attest` have produced `docs/SIMILARITY.md`
   and a verifying bundle: owed to the summarize stage.**
5. **A draft PR is open: owed to the summarize stage** (`kukulu_run_finish`
   opens it).

### Blocked and skipped tasks

- P1-10 (blocked): the binding crate has no `LZ4_decompress_safe_usingDict`.
- Skipped because of it: P2-08, P2-11, P2-12, P2-13, P3-02, P3-03, P3-06, and
  P4-01 to P4-09 and P4-12.
