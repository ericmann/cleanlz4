# Review: cleanlz4 0.2.0 (build/2026-11-10-2)
Round: 2

Verdict: APPROVED

Reviewed `4ea19f9..9b59571` (26 tasks, all done, none blocked or skipped) against `docs/SPEC.md` 0.3 and `docs/PLAN.md`. Round 1 reviewed every task commit of the branch, P0-01 through P3-03, commit by commit. Its one finding became R1-01. This round reviews the fix round (`f190275..HEAD`): commit `1b9d5b0` (R1-01) and the controller and progress commits around it. `git diff f190275..HEAD` touches only `crates/oracle/tests/scratch.rs`, the `docs/DESIGN.md` line naming the tests, `docs/HANDOFF.md`, `docs/PROGRESS.md` and `.kukulu/` bookkeeping. Nothing under `crates/cleanlz4/`, `docs/spec/`, `docs/cleanroom.json`, `docs/kukulu.json`, `.gitignore`, `.cargo/`, `Makefile` or `scripts/` changed in this round. Across the whole branch, `docs/spec/`, `docs/cleanroom.json`, `docs/kukulu.json`, `.gitignore`, `.gitattributes` and `.cargo/` are untouched.

`kukulu_verify` (files: `crates/oracle/tests/scratch.rs`, `docs/DESIGN.md`) is green. All 19 constraint rules pass their own fixtures and report no hits. fmt, clippy, `cargo test --workspace`, the guard self-test (568 cases), both deps scripts and both spec-source scripts exit 0. The scratch suite runs 5 tests, all passing.

**PR checks:** `prChecks` reads `unknown: no checks reported on the 'build/2026-11-10-2' branch`. That is not a finding: the repository reports no CI checks for this branch.

## Findings

None.

**R1-01 is verified.** Every check below was done in this round:

- **Constraints (category 1).** `crates/oracle/tests/scratch.rs` gained a `LIVE` counter in the existing counting allocator. Its `alloc` adds `layout.size()`, `dealloc` subtracts it, and `realloc` adds `new_size` and subtracts the old size. All calls are still forwarded to `std::alloc::System`. This stays inside SPEC §3.7's carve-out (a test file under `crates/oracle/tests/`, calling no C function) and follows §6.1's counting rules. The new code adds no XXH32 digest literal and no config tunable written as a literal: `BIG_BLOCK` (1 MiB) is a test-data size, and the `word_salad` seed 11 follows the file's convention for fixed data. Neither is a `testdata/config.json` key. The legacy magic number `0x184C_2102` comes from the Frame format's "Legacy frame" section and is built by hand, as SPEC §6.5 allows.
- **Boundaries (category 2).** Nothing crosses a boundary. The task was tests-only, and `crates/cleanlz4/src/` is unchanged.
- **Tests (category 3).**
  - Both acceptance tests exist and pass: `streaming_legacy_scratch_is_freed_at_the_end_mark` and `streaming_legacy_scratch_is_freed_at_the_next_magic`.
  - Each test checks the mechanic directly. It compares the live heap after the legacy frame's last delivery with the live heap when `FrameDone` is reported, and requires a drop of at least half a block. It also checks the decoded bytes. It does not re-derive any formula.
  - `kukulu_mutate` (targeted `cargo`, filter `streaming_legacy_scratch`, `confirmKill: true`) on the two round-1 survivors:
    - Deleting `self.scratch = Vec::new();` in `end_frame` (`crates/cleanlz4/src/stream.rs:237`): **killed** by `streaming_legacy_scratch_is_freed_at_the_end_mark` ("12198964 live then, 12198964 at FrameDone"). The kill was confirmed: the unmutated re-run passes.
    - Deleting the same line in the `LegacySize` magic branch (`stream.rs:372`): **killed** by `streaming_legacy_scratch_is_freed_at_the_next_magic` ("15678341 live then, 15678341 at FrameDone"). Confirmed the same way.
    - A sanity mutant in the fix file itself removed `LIVE.fetch_sub` from `dealloc` in `scratch.rs`. Both new tests failed (**killed**, confirmed), so the counter carries the signal.
  - The three existing scratch tests and the four memory tests pass unchanged.
- **Spec drift (category 5).** `docs/DESIGN.md`'s "One scratch buffer per frame" entry now names both tests. Its claim that "none outlives its frame" is now backed by tests, which is what SPEC §6.1's bound needs.
- **Process.**
  - The PLAN's verification step required the implementer to apply each mutant by hand. The implementer did so with `sed -i`, at clean-room log seq 480, and restored the file. `git diff f190275..HEAD -- crates/cleanlz4` is empty, so the restore is complete.
  - Both mutants' failure messages are quoted in the PROGRESS log.
  - The commit message follows the template.

## Spec issues

- **SPEC §5.3's six-pair similarity block cannot be written with Kūkulu 0.8.0.** This is already recorded in PLAN's Spec issues and in round 1. `docs/kukulu.json` keeps only the Rust-vs-Rust controls, so SPEC's "Score every pair; no pair is dropped" is not met for `independent-vs-c` and `derived-vs-c`. `subject-vs-c` must be read against 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`, as the HANDOFF's note for the summarizer says.
- **SPEC §5.1 has a stray table header** (`| Key | Default | Meaning |` with no rows). This is cosmetic and already in PLAN.
- **Known-gap candidates (guard):** none.
  - `kukulu_status` shows `denied: 2`. Both are plan-stage false positives (seq 43 and 66), assessed in round 1. No new denial was logged by the implement or fix stages.
  - One call in this review was denied. It was a read-only `sed -n 480p` on `.kukulu/cleanroom/log.jsonl`, treated as a protected-path write. That is a false positive (over-strict, not a gap). It was not worked around and was logged with `kukulu_feedback_log`.

## Manual checks still owed

Copied from `docs/HANDOFF.md`:

- **Phase 0:**
  - `testdata/results/conformance-P0-03.json` equals 0.1.0's `conformance-P2-12.json` in `corpus_sha256`, total, passed and every category.
  - The P0-01 and P0-02 log entries show both corpora matching `testdata/MANIFEST.json`, `lz4` at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and `make check` green.
  - `kukulu_status` reports `cleanroom.lockdown` true.
- **Phase 1:**
  - Read `crates/oracle/tests/dict_corpus.rs`, `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent` (`tests/dict.rs`) and `runner_passes_the_dictionary_to_frame_entries` (`tests/results.rs`).
  - Read `docs/DESIGN.md` Measurements, "Scratch-buffer reuse (P1-07)", and confirm the keep decision follows its table.
- **Phase 2:**
  - `conformance-P2-01.json` shows 2349 of 2349, and `malformed-P2-02.json` shows 144 of 144 with 0 panics.
  - Every `fuzz-*-P2-0[3-7].json` has `crashes` 0 and `seconds` of at least `fuzz.seconds_per_target`.
  - DESIGN.md "0.2.0 re-verification (P2-08)" says none.
  - Read `bench-P2-09.json` (no target).
- **Phase 3:**
  - The owner reads `docs/SIMILARITY.md` (summarize stage).
  - The owner runs the attestation `verify.mjs` from a clean checkout on a second machine.

## Notes

- **Timing assumption in the new tests.** The tests take the live-bytes reading at `FrameDone` from the call that reports it. If a future decoder change delivered a legacy frame's last block in the same call that reports `FrameDone`, `last_delivery` would fall back to the block before it. That reading is still higher than the one at `FrameDone`, so the assertion stays meaningful. This needs no action.
- **Benchmark noise.** Round 1's remark on P1-07's run-to-run noise (about 3 points of the aggregate) still stands as something for the owner to note when reading the Measurements table.
- **Earlier timeout kill.** `block.rs`'s zero-offset mutant from round 1 was caught by a timeout rather than an assertion. That is unchanged and advisory only.
