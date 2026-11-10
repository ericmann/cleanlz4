# Summary: cleanlz4 0.2.0 (hardening release on 0.1.0)

**Merge:** `build/2026-11-10-2`, base `4ea19f9` (main) → head `0f71268`, 68 commits, 2 reviews (Round 1: CHANGES REQUESTED, Round 2: APPROVED). Final verdict: **APPROVED**. 26 tasks done, 0 blocked, 0 skipped.

## What was built

**Phase 0, baseline and lockdown (P0-01 to P0-04).** Checked that both corpora match `testdata/MANIFEST.json`, that `lz4` is v1.9.4 and the binding is 1.11.1+lz4-1.10.0, that `make check` is green and that the fuzz toolchain builds offline. Regenerated the conformance corpus and recorded a baseline run, `conformance-P0-03.json`: 2349 of 2349, identical to 0.1.0's. Clean-room lockdown has applied from P0-03 on.

**Phase 1, closing the 0.1.0 review notes (P1-01 to P1-08).**
- A dictionary differential test (`crates/oracle/tests/dict_corpus.rs`) runs all 72 dictionary entries through both the `lz4` program and cleanlz4.
- Two tests kill the 0.1.0 surviving mutants: `dict_dependent` in `tests/dict.rs` and the runner's dropped dictionary in `tests/results.rs`.
- `is_magic` is merged into `magic_kind`.
- The decoder now reuses one scratch buffer per frame instead of allocating a `Vec` per block. The change was benchmarked before (`bench-P1-05`) and after (`bench-P1-07`), and kept.

**Phase 2, re-verification (P2-01 to P2-10).**
- Full conformance: 2349 of 2349.
- Full malformed run: 144 of 144, 0 panics.
- Five one-hour fuzz runs (3601 s each, seed 1): 0 crashes and 0 divergences in every run.
- Triage: none needed.
- Final benchmark, `bench-P2-09`: aggregate 559.6 MB/s. There is no target.

**Phase 3, evidence and release prep (P3-01 to P3-03).**
- CHANGELOG entry for 0.2.0, and the version bumped in both crates and both lockfiles.
- `docs/EVIDENCE.md` and the README refreshed.
- Phase-end push.

**Review fix (R1-01).** Tests only. A live-bytes counter in `crates/oracle/tests/scratch.rs` and two tests check that the streaming decoder's legacy scratch buffer is freed when its frame ends.

## Decisions that shaped it

- Phases: SPEC §8's four phases, run fully serially. There are no parallel streams because the corpora are untracked and benchmark and fuzz runs need the machine to themselves. (plan)
- Flight slug is `cleanlz4-0-2-0`, not `cleanlz4`, because `docs/flights/cleanlz4/` already holds 0.1.0. (plan)
- Task ids restart at P0-01, and the lockdown task is P0-03, the third of Phase 0's three checks. (plan, P0-03)
- P0-03 runs both `make conformance-check` and a full `make conformance` so that the baseline exists as a file. (P0-03)
- P0-02 checks the fuzz toolchain early, because Phase 2 needs five offline fuzz builds. (P0-02)
- §5.3 similarity: only the Rust-vs-Rust controls are configured (`rust-independent-vs-indep` negative, `rust-derived-vs-indep` positive), with tests `subject-vs-indep` and `subject-vs-c`. Kūkulu 0.8.0 refuses a second pair of either role. (plan)
- No tunable was tuned or added. SPEC 0.3 marks no key as an ASSUMPTION. (plan)
- Dictionary differential: frame entries go to `decompress_frame_with_dict`, and the 18 raw-block entries go to `decompress_block_with_dict` (a hand-built minimal frame). "Expected bytes" means `input_bytes` and `input_sha256` match, and the two outputs are equal. (P1-01)
- Frames with a Dict-ID and no dictionary are checked against cleanlz4 only (`DictionaryRequired`) and never sent to the `lz4` program. (P1-01)
- One task per 0.1.0 mutant. Each mutant was applied by hand and then restored. (P1-02, P1-03)
- `make conformance-check` is bound to `crates/cleanlz4/src/`, `runner.rs` and `dict.rs`. `make corpus` is not bound, so that a mutation run cannot rewrite the manifest. (plan)
- Scratch buffer: `FrameState` owns one buffer per regular frame, and `decode_legacy_block` takes one from its caller. At most one buffer is alive and none outlives its frame. (P1-06, held by tests since R1-01)
- Scratch-buffer criterion: the aggregate throughput, with a threshold of 7.28% (the 0.1.0 spread at `bench.repeats` = 10) and one confirming run before reverting. The first run was 8.77% slower and the second 5.54%, so the change was kept. Run-to-run noise is about 3 points. (P1-07)
- P2-08: nothing to triage, so the entry says "none" and there is no `-P2-08` re-run. (P2-08)
- CHANGELOG date is the UTC date `date -u +%F` printed (2026-11-10). A version-agnostic `tests/release.rs` checks that the crate versions and the newest changelog entry agree. (P3-01)
- Done checklist item 2 is marked met, naming the task that ran each command. Items 4 and 6 are left to the summarize stage. (P3-02)
- The fenced list in EVIDENCE.md is kept in sorted order, and new files are inserted in place rather than appended. (P3-02)
- New constraint rules: `oracle-unsafe-only-in-ffi`, `no-hardcoded-corpus-tunables`, `cargo-stays-offline`, `single-magic-classifier` and `no-new-dependencies`. (plan)
- Review rounds use the defaults: 3 non-converging, 6 hard. (plan)

## Assumptions still in play

None. No key of `testdata/config.json` is marked ⚠️ ASSUMPTION, and none was added, changed or tuned in this flight. All keys keep their 0.1.0 values.

## Spec issues

- **§5.3's six-pair similarity block cannot be written with Kūkulu 0.8.0.** The tool needs exactly one negative and one positive pair, so `independent-vs-c` and `derived-vs-c` were not scored, against "Score every pair; no pair is dropped". `subject-vs-c` must be read against 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`. Edit: say which band a single run keeps. The owner can also swap the control pairs and re-run `kukulu_similarity` by hand.
- **The slug from SPEC's title is taken** (`docs/flights/cleanlz4/`). Edit: put the version in the title, or name the slug.
- **`lockdown.after` is `P0-03`, but §8 Phase 0 describes one task.** Edit: name the lockdown task id.
- **§8 Phase 0's `make conformance-check` writes no file**, which conflicts with §3.6. Edit: require a full `make conformance` there.
- **`cargo test -p oracle --test manifest` passes when the corpora are absent.** Proposed: a strict mode in a later flight.
- **Task ids repeat across flights**, while result files are named by task id. Edit: add the flight to the file name.
- **Binding `make conformance-check` to `dict.rs` proves little by itself**, because it never runs the generator.
- **"Slower than before by more than the spread" names no statistic.** Edit: name the aggregate throughput.
- **"Decodes every `dictionary` entry with `decompress_frame_with_dict`"**, but 18 of the 72 entries are raw blocks.
- **§1 and the Phase 3 manual check name `docs/attestation/`**, but archiving moves the bundle to `docs/flights/<slug>/attestation/`.
- **SPEC does not say whether the PR stays a draft** until the owner's Phase 3 checks are done. Set `policies.finalize: "draft"` to hold it.
- **Hour-long verify calls run past the MCP idle timeout of 1800 s.**
- **§5.1 has a stray table header** (`| Key | Default | Meaning |` with no rows). Cosmetic.
- **§4 lets `xxh32.rs` import `error`**, while CLAUDE.md's module map says `core` and `std` only.
- **The prior-art trees of §5.3 were not checked by the planner**, because the guard denies them. The owner confirms them.

Known-gap candidates (guard): none.
  - `kukulu_status` shows `denied: 2`. Both are plan-stage false positives (seq 43 and 66), assessed in round 1. No new denial was logged by the implement or fix stages.
  - One call in this review was denied. It was a read-only `sed -n 480p` on `.kukulu/cleanroom/log.jsonl`, treated as a protected-path write. That is a false positive (over-strict, not a gap). It was not worked around and was logged with `kukulu_feedback_log`.

## Manual checks owed

- **Phase 0:**
  - `testdata/results/conformance-P0-03.json` equals 0.1.0's `conformance-P2-12.json` in `corpus_sha256`, total, passed and every category.
  - The P0-01 and P0-02 log entries show both corpora matching `testdata/MANIFEST.json`, `lz4` at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and `make check` green.
  - `kukulu_status` reports `cleanroom.lockdown` true.
- **Phase 1:**
  - Read `crates/oracle/tests/dict_corpus.rs`, `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent` (`tests/dict.rs`) and `runner_passes_the_dictionary_to_frame_entries` (`tests/results.rs`).
  - Read `docs/DESIGN.md` Measurements, "Scratch-buffer reuse (P1-07)", and confirm the keep decision follows its table. Mind the noise of about 3 points.
- **Phase 2:**
  - `conformance-P2-01.json` shows 2349 of 2349.
  - `malformed-P2-02.json` shows 144 of 144 with 0 panics.
  - Every `fuzz-*-P2-0[3-7].json` has `crashes` 0 and `seconds` of at least `fuzz.seconds_per_target`.
  - DESIGN.md "0.2.0 re-verification (P2-08)" says none.
  - Read `bench-P2-09.json` (there is no target).
- **Phase 3:**
  - Read `docs/SIMILARITY.md`. The Rust-vs-Rust band applies to `subject-vs-indep` only. For `subject-vs-c`, use 0.1.0's reading.
  - Run the attestation `verify.mjs` from a clean checkout on a second machine. After archiving, the bundle is under `docs/flights/cleanlz4-0-2-0/attestation/`.

## Review history

- **Round 1: CHANGES REQUESTED.** 1 finding, 1 fix task (R1-01; sources: review 0, mutation 1, blocked 0), not non-converging. The streaming legacy scratch buffer could outlive its frame without any test noticing: two surviving mutants in `stream.rs`.
- **Round 2: APPROVED.** 0 findings, 0 fix tasks. No finding recurred.
  - Both mutants are now killed and confirmed.
  - The approval carries a `## Notes` section, so items were flagged but not queued as work: a timing assumption in the new tests (no action needed), benchmark noise of about 3 points in P1-07, and round 1's `block.rs` zero-offset mutant being caught by a timeout rather than an assertion.

## Pipeline friction

1. plan / config-refusal: SPEC 5.3 asks for two negative and two positive similarity pairs, but cfg() refuses them ("cleanroom.similarity.pairs needs exactly one pair with role negative (found 2)") and every tool, kukulu_status included, then fails. The planner had to drop SPEC's two Rust-vs-C control pairs to get a loadable config. Separately, the title-derived flight slug (cleanlz4) collided with the existing docs/flights/cleanlz4 archive, so the skill's slug rule had to be overridden by hand.
2. plan / tool-refusal: two guard denials while planning. A read-only `sed -n 471,483p docs/mcp-tools.md` in the plugin directory was refused as a protected path (sed's script argument was read as a path). A Bash heredoc that built the draft docs/kukulu.json was refused because the similarity block's prior-art paths appeared in the command text. The planner must write SPEC's prior-art paths into kukulu.json, so that block can only go in through the Write or Edit tool, and the skill does not say so.
3. implement / stall: kukulu_verify running the 75-minute fuzz extraVerify sent no progress for 1800 s, so the MCP client aborted the tool call with an idle-timeout error although the command kept running. The long-run protocol cannot work without a progress heartbeat or CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT above 1800 s.
4. review / tool-refusal: kukulu_mutate's targeted filter accepts a list of up to 10 names but substitutes them space-separated into `cargo test --workspace {filter}`, and cargo test takes only one TESTNAME. Every list filter fails with "unexpected argument" and is reported inconclusive. This cost one mutation run.
5. review / stall: a kukulu_mutate call (block.rs zero-offset mutant, targeted filter 'zero') sent no progress for 1800 s and the MCP client aborted it. The targeted command's own timeout is also 1800000 ms, so the result never came back and 30 minutes were lost.
6. review / tool-refusal: the clean-room guard denied a read-only `sed -n 480p .kukulu/cleanroom/log.jsonl` as a protected-path write, while `grep` and `tail` on the same file were allowed. The reviewer had to drop the read.

## Suggested CLAUDE.md additions

- `evidence_index_lists_every_result_file` requires the fenced list in `docs/EVIDENCE.md` to be in plain string order (so `conformance-P0-03` goes before `conformance-P1-12`). `evidence_results_table_names_every_result_file` also requires every result file name in the Results section.
- A fresh-allocation counter (`alloc` only, with `realloc` and `dealloc` forwarded) is a sharp tool for "no allocation per block" claims. Pre-size the output `Vec` before counting, or its growth hides the signal.
- `decode_block` appends to its destination, so a reused scratch buffer must be cleared before each block.
- Benchmark aggregates swing about 3 points between runs on this machine. Judge a change on the aggregate and repeat once before acting. Individual inputs swing more.
- Each hour-long fuzz run through `kukulu_verify` hits the 30-minute MCP idle timeout while the command keeps running. The result file appears when the run ends. Poll with bounded `timeout` loops in the background.
- Before the `diff_*` fuzz runs, move the committed divergence inputs aside (`target/divergences-aside/`), then restore them and check `git status`.
- `pgrep -f fuzz_run` matches its own shell. Use `ps aux | grep "[f]uzz_run"` before a benchmark.

## Advisory mutation survivors

None. No survivor was placed outside the round's diff. Round 1's `block.rs` zero-offset mutant was caught by a timeout rather than an assertion; it is advisory only.

## Breaking call

Breaking: no — CHANGELOG.md's newest entry ([0.2.0]) has no `### Breaking changes` section. The public API, the `Error` variants and every config key are unchanged.

## Contamination score

The verdict is: **independence supported on every separable metric**

The band comes from the Rust-vs-Rust controls only, and it applies to `subject-vs-indep`. It does not apply to `subject-vs-c`; for that pair, use the Rust-vs-C reading in 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`. The Rust-vs-C controls were not scored in this run (SPEC 5.3, 9 Q4).

- subject-vs-indep, ngram_jaccard@5: 0.2245 (c = -0.0010, below negative)
- subject-vs-indep, ngram_containment@5: 0.4817 (c = -0.0206, below negative)
- subject-vs-indep, ngram_jaccard@10: 0.0529 (c = 0.0016, at or near negative)
- subject-vs-indep, ngram_containment@10: 0.1328 (c = -0.0356, below negative)
- subject-vs-indep, ident_jaccard: 0.0792 (c = 0.0506, at or near negative)
- subject-vs-indep, ident_match_rate_left: 0.1918 (c = 0.1288, at or near negative)
- subject-vs-indep, comment_jaccard: 0.0007 (c = n/a, not separated)
- subject-vs-c, ngram_jaccard@5: 0.0349 (c = -0.2516, below negative)
- subject-vs-c, ngram_containment@5: 0.1125 (c = -0.7605, below negative)
- subject-vs-c, ngram_jaccard@10: 0.0006 (c = -0.0553, below negative)
- subject-vs-c, ngram_containment@10: 0.0027 (c = -0.1935, below negative)
- subject-vs-c, ident_jaccard: 0.0128 (c = -0.0670, below negative)
- subject-vs-c, ident_match_rate_left: 0.0685 (c = -0.0041, below negative)
- subject-vs-c, comment_jaccard: 0.0001 (c = n/a, not separated)

Controls: rust-independent-vs-indep (negative) and rust-derived-vs-indep (positive). Raw scores and limits are in `docs/SIMILARITY.md`.
