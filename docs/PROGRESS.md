# cleanlz4 build progress

Branch: build/2026-11-10-2
Started: 2026-11-10T15:48:48.480Z

## Tasks

- [x] P0-01 Confirm the corpora, the tool versions and the spec sources
- [x] P0-02 Confirm make check and the fuzz toolchain
- [x] P0-03 Regenerate the corpus, record the baseline conformance run and lock down
- [x] P0-04 Phase 0 end: push and record the manual check
- [x] P1-01 Dictionary differential test: every dictionary entry against the lz4 program
- [x] P1-02 Kill the dict_dependent mutant in the oracle's dict module
- [x] P1-03 Kill the dropped-dictionary mutant in the conformance runner
- [x] P1-04 Merge is_magic into magic_kind
- [x] P1-05 Benchmark before the scratch-buffer change
- [x] P1-06 Reuse one scratch buffer across blocks
- [x] P1-07 Measure the scratch-buffer change and decide
- [x] P1-08 Phase 1 end: push and record the manual check
- [x] P2-01 The full conformance run
- [x] P2-02 The full malformed run
- [x] P2-03 Run no_panic_block for the configured duration
- [x] P2-04 Run no_panic_frame for the configured duration
- [ ] P2-05 Run no_panic_stream for the configured duration
- [ ] P2-06 Run diff_roundtrip for the configured duration
- [ ] P2-07 Run diff_accept for the configured duration
- [ ] P2-08 Triage crashes and divergences
- [ ] P2-09 The final benchmark
- [ ] P2-10 Phase 2 end: push and record the manual check
- [ ] P3-01 The CHANGELOG entry for 0.2.0 and the crate versions
- [ ] P3-02 Refresh EVIDENCE.md and README.md
- [ ] P3-03 Phase 3 end: push and record the manual check

## Log

(one entry per task, appended by implement)

### P0-01 — 70eab32

Checks: cantrbry 11 files, silesia 12 files. manifest tests 3 passed, no "absent" notice. lz4 -V: "*** LZ4 command line interface 64-bits v1.9.4, by Yann Collet ***"; lz4-sys 1.11.1+lz4-1.10.0 (match). rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1. spec sources: ok (3 files). kukulu_verify all green. Empty commit (no files touched).

### P0-02 — 4d9be0d

make check exit 0; guard self-test: ok (568 cases); spec sources: ok (3 files); fuzz build ok (cargo +nightly fuzz build, 50s, offline). Smoke lines: fuzz_run no_panic_block smoke: 55088 runs, crashes 0, divergences 0; no_panic_frame 1552066 runs 0/0; no_panic_stream 556128 runs 0/0; diff_roundtrip 1197 runs 0/0; diff_accept 83802 runs 0/0. No files touched; empty commit.

### P0-03 — baa94de

Lockdown: P0-03 done, no URL is allowed from here. make corpus: MANIFEST.json unchanged. conformance-check and conformance-P0-03.json: 2349 of 2349 passed, 0 failed, ten categories; identical to P2-12 apart from task. corpus_sha256 fd6392a28a4b239780f073ad70015c93f589163e4d232a9dd11502c24c93ba05. EVIDENCE.md: fenced list is sorted-checked, so the new path sits before P1-12; results table row added.

### P0-04 — 52ccc3d

Push: done (origin build/2026-11-10-2). Manual check: NOT VERIFIED (human): testdata/results/conformance-P0-03.json equals 0.1.0's conformance-P2-12.json in corpus_sha256, total, passed and every category; the log entries of P0-01 and P0-02 show both corpora matching testdata/MANIFEST.json, the lz4 program at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and make check green; kukulu_status reports cleanroom.lockdown true

### P1-01 — 4e039a9

Added crates/oracle/tests/dict_corpus.rs (3 tests, all pass first run; the decoder already agreed). Compared 72 dictionary entries: 54 frame (28 with dict_id... 36 with, 18 without per size breakdown: with-id 18+10+8, without 9+5+4) and 18 block-dict; sizes 4096/65536/131072 all covered. Comment fixes in dict.rs, cli.rs, fuzz_logic.rs; DESIGN.md: new decision, two stale statements corrected, spec-rules row replaced. make corpus left MANIFEST.json unchanged; conformance-check 2349/2349.

### P1-02 — 3df514c

Added a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent in tests/dict.rs. Mutant (Ok(_) => true) fails it: "size 4096: labelled dict_dependent" at tests/dict.rs:99; undone, git diff --exit-code on dict.rs clean. dict.rs unchanged.

### P1-03 — e099856

Added runner_passes_the_dictionary_to_frame_entries in tests/results.rs. Mutant (condition -> false) fails it: assertion `left == right` failed: failures: [("dictionary/canterbury/alice29.txt/d4096/indep-id", "the frame needs a dictionary and none was supplied"), ...]; undone, runner.rs diff clean. kukulu_verify with runner.rs ran make conformance-check: 2349 of 2349 passed.

### P1-04 — a4dc3ed

Deleted is_magic; decode_legacy_frame uses magic_kind(word).is_some(). Tests: frame.rs magic_kind_names_the_three_kinds_and_nothing_else, tests/frame.rs legacy_frame_ends_before_a_second_legacy_frame. Mutant (legacy arm of magic_kind removed) fails both; undone. DESIGN entry added. single-magic-classifier constraint active and green; conformance-check 2349/2349.

### P1-05 — 679e56f

bench-P1-05.json written by make bench; no code changed. Slowest cleanlz4: canterbury/xargs.1 388.5 MB/s; fastest silesia/x-ray 1761.7 MB/s. Aggregate (total bytes / sum of best times) = 532.1 MB/s. EVIDENCE.md list (sorted) and results table updated; results tests pass.

### P1-06 — 0c003d3

Tests first on unchanged decoder failed: legacy one-shot 16 blocks 18 fresh allocs vs 64 blocks 66; one-shot (independent) 19 vs 67; streaming 22 vs 70. After change: legacy 3/3 one-shot, 7/7 streaming; one-shot independent 4/4, linked 5/5; streaming 7/7 and 8/8. FrameState.scratch, decode_legacy_block takes scratch, Decoder.scratch replaced at both legacy frame ends. Memory tests (oracle limits, cleanlz4 limits) pass unchanged; conformance-check 2349/2349; fuzz build ok; make fuzz-smoke exit 0, all five targets crashes 0 divergences 0. DESIGN entry added, "per-block scratch buffer" reworded.

### P1-07 — 9a9eb31

A(before)=532.1 MB/s. After run 1: 485.4 (slowdown 8.77%, above 7.28%); run 2: 502.7 (slowdown 5.54%). Criterion met on second run: scratch change KEPT, no code changes. 7 of 23 inputs individually slower than 7.28% (info). DESIGN Measurements "Scratch-buffer reuse (P1-07)" table added; bench-P1-07.json (second run) and EVIDENCE.md updated. Note for reviewer: run-to-run noise is large (~3 points of aggregate).

### P1-08 — 321ad27

Push: done. Manual check: NOT VERIFIED (human): read crates/oracle/tests/dict_corpus.rs (the dictionary differential test) and the two mutant-killing tests, a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent in crates/oracle/tests/dict.rs and runner_passes_the_dictionary_to_frame_entries in crates/oracle/tests/results.rs; read DESIGN.md Measurements, Scratch-buffer reuse (P1-07), and confirm the keep-or-revert decision follows its table

### P2-01 — d406151

conformance-P2-01.json: passed 2349, total 2349, failed 0; corpus_sha256 fd6392a28a4b239780f073ad70015c93f589163e4d232a9dd11502c24c93ba05; identical to P2-12 apart from task; MANIFEST.json unchanged by make corpus. Per category: block 156, checksums 301, concatenated 6, content-size 401, dictionary 72, frame-independent 45, frame-linked 53, legacy 14, skippable 21, synthetic 1280. DESIGN Measurements section and EVIDENCE (sorted list + table) updated.

### P2-02 — c507c2b

malformed-P2-02.json: total 144, matched_expected_error 144, panics 0. testdata/malformed untouched. EVIDENCE.md list and table updated.

### P2-03 — e4c08b4

fuzz-no_panic_block-P2-03.json: seconds 3601, runs 37841791, crashes 0, divergences 0, corpus_entries 1194, seed 1. Crashes: 0 Divergences: 0. Friction: the kukulu_verify call hit the MCP 1800s idle timeout and returned an error while the make fuzz-run process (75 min) kept running; I waited for the result file rather than restart, then committed it by hand. EVIDENCE.md updated (sorted).

### P2-04 — b9e29b0

fuzz-no_panic_frame-P2-04.json: seconds 3601, runs 142651039, crashes 0, divergences 0, corpus_entries 928, seed 1. Crashes: 0 Divergences: 0. No new regressions files. (kukulu_verify again aborted at the 1800s MCP idle timeout while the run continued; waited for the result file.)
