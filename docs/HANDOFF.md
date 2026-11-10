# Handoff: cleanlz4 0.2.0 (hardening release on 0.1.0)

- Branch: `build/2026-11-10-2`
- Base commit: `4ea19f9` (main)
- Head commit before this file: `b25ae5e`
- Tasks: 25 done, 0 blocked, 0 skipped, 0 open.
- Clean-room lockdown was in force from P0-03; no tool call needed a workaround. `kukulu_status` shows `denied: 2` in the guard log; neither denial affected a task.

## Blocked and skipped tasks

None.

## Interpretation choices

- P1-01: frames with a Dict-ID and no dictionary are checked against cleanlz4 only (`DictionaryRequired`), never sent to the `lz4` program, as the task said.
- P1-07: the statistic for "slower than before" is the aggregate throughput (total decoded bytes over the sum of each input's best time); the threshold is 7.28% from `docs/DESIGN.md` Measurements (`bench.repeats (P4-11)`). First after-run was 8.77% slower, the second 5.54%, so the scratch-buffer change was kept as the criterion says (either run at or under the threshold). Run-to-run noise in the aggregate is about 3 points.
- P2-08: nothing to triage (all five fuzz runs: 0 crashes, 0 divergences), so the entry is "none" and no `fuzz-*-P2-08` re-run exists.
- P3-01: the CHANGELOG date is what `date -u +%F` printed (2026-11-10); the scratch buffer is reported as kept.
- P3-02: Done checklist item 2 ("every section 7 command exits zero") is marked met, naming the task that ran each command; items 4 and 6 are owed to the summarize stage and the end of the run.
- EVIDENCE.md's fenced list is checked for sort order by `evidence_index_lists_every_result_file`; new files were inserted in sorted position, not appended.

## Assumption config keys

No key of `testdata/config.json` was added or changed in this flight. `fuzz.seconds_per_target`, `fuzz.seed`, `bench.repeats` and the others keep their 0.1.0 values; none was tuned.

## What a human must check by hand

- Phase 0: `testdata/results/conformance-P0-03.json` equals 0.1.0's `conformance-P2-12.json` in `corpus_sha256`, total, passed and every category; the P0-01 and P0-02 log entries show both corpora matching `testdata/MANIFEST.json`, `lz4` at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and `make check` green; `kukulu_status` reports `cleanroom.lockdown` true.
- Phase 1: read `crates/oracle/tests/dict_corpus.rs`, `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent` (`tests/dict.rs`) and `runner_passes_the_dictionary_to_frame_entries` (`tests/results.rs`); read `docs/DESIGN.md` Measurements, "Scratch-buffer reuse (P1-07)" and confirm the keep decision follows its table.
- Phase 2: `conformance-P2-01.json` 2349 of 2349, `malformed-P2-02.json` 144 of 144 with 0 panics, every `fuzz-*-P2-0[3-7].json` has `crashes` 0 and `seconds` at least `fuzz.seconds_per_target`, DESIGN.md "0.2.0 re-verification (P2-08)" says none, read `bench-P2-09.json` (no target).
- Phase 3: the owner reads `docs/SIMILARITY.md` (summarize stage) and runs the attestation `verify.mjs` from a clean checkout on a second machine.

## Evidence for the attestation

`docs/EVIDENCE.md`'s fenced list under `## Evidence files` is the evidence for the attestation.

## Note for the summarizer

Similarity: docs/kukulu.json holds one negative pair (rust-independent-vs-indep) and one positive pair (rust-derived-vs-indep), both Rust-vs-Rust, because kukulu 0.8.0 refuses a second pair of either role; SPEC 5.3's Rust-vs-C controls (independent-vs-c, derived-vs-c) are not scored in this run. The Rust-vs-Rust band applies to subject-vs-indep. It does not apply to subject-vs-c, whose Rust-vs-C reading is 0.1.0's docs/flights/cleanlz4/SIMILARITY.md. SUMMARY.md says which band applies to each test pair and repeats the tool's verdict verbatim (SPEC 5.3, 9 Q4).

## Codebase notes

- `evidence_index_lists_every_result_file` requires the fenced list in `docs/EVIDENCE.md` to be sorted by plain string order, so `conformance-P0-03` goes before `conformance-P1-12`; a new `evidence_results_table_names_every_result_file` also requires every result file name in the Results section.
- A fresh-allocation counter (`alloc` only, `realloc` and `dealloc` forwarded) is a sharp tool for "no allocation per block" claims: before the scratch-buffer change 16 blocks cost 18 to 22 allocations and 64 blocks 66 to 70; after, both cost the same (3 to 8). Pre-size the output `Vec` before counting, or its growth hides the signal.
- `decode_block` appends to its destination, so a reused scratch buffer must be cleared before each block.
- Benchmark aggregates swing about 3 points between runs on this machine; judge a change on the aggregate and repeat once before acting, as P1-07 did. Individual inputs swing more (7 of 23 inputs read more than 7.28% slower in the kept run).
- Each hour-long fuzz run (P2-03 to P2-07) goes through `kukulu_verify`, whose MCP call aborts at the 30-minute idle timeout while the command keeps running; the result file appears when the run ends and the task can be finished from it. Waiting loops that exceed the shell's foreground limit are moved to the background, so poll with bounded `timeout` loops.
- For the two `diff_*` runs the 100 committed divergence inputs were moved to `target/divergences-aside/` and restored; `git status` showed no deleted or new divergence file afterwards.
- `pgrep -f fuzz_run` matches its own shell; use `ps aux | grep "[f]uzz_run"` to check that no fuzz run is active before a benchmark.

## For a reviewer who has not seen this code

- The decoder changes are small: `frame.rs` (`is_magic` removed, `FrameState.scratch`, `decode_legacy_block` takes a scratch buffer) and `stream.rs` (`Decoder.scratch`, replaced where a legacy frame ends). `lib.rs`, the public API and `Error` are untouched. The rest of the flight is tests, result files, docs, the version bump (both crates and both lockfiles, version lines only) and the CHANGELOG.
- Every result file under `testdata/results/` was written by its tool in the commit of the work that produced it; none was typed by hand.
- Friction logged with `kukulu_feedback_log`: the MCP idle timeout during long fuzz runs (see above).
