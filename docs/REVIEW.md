# Review: build/2026-11-10
Round: 3

## Verdict

CHANGES REQUESTED

Round 2's two fix tasks did what their findings asked:

- R2-01 adds a capacity test for legacy frames.
- R2-02 adds tests that check the frame-combination generator directly and check the committed manifest against it.

Neither commit touches library code, and no new finding came out of this round. `kukulu_verify` passed every check on HEAD:

- all 14 constraints, with no fixture failures and no hits;
- fmt, clippy and `cargo test --workspace`;
- both deps scripts, spec sources, and the guard self-test (568 cases).

I cannot approve, because P1-10 is still blocked. The review rules allow approval only when no task is blocked. Only the owner can unblock P1-10, through SPEC §6.5 or §9 Q2 (see Spec issues), and while it stays blocked 20 tasks stay skipped.

`kukulu_review_submit` refuses CHANGES REQUESTED with no task, so I re-read the skipped work for parts that need no dictionary oracle. Two such parts are SPEC requirements the plan routed through P2-08 for no reason, and this round queues them as R3-01 and R3-02 (findings F1 and F2). Everything that does need the dictionary still waits on the owner.

`prChecks`: `unknown: no checks reported on the 'build/2026-11-10' branch`. There is no CI on the branch, so this is not a finding.

## Findings (most severe first)

Round 2's work has no findings. The two findings below come from skipped tasks whose blocker does not apply to all of their work.

### F1. Category 5: the streaming and one-shot APIs disagree on skippable, legacy and concatenated input (R3-01)

- **Where:** `crates/cleanlz4/src/stream.rs:309-340`, `step_boundary`. At a frame boundary it accepts only a regular frame's magic. A skippable or legacy magic is `BadMagic` for the first frame and `TrailingData` after it.
- **What SPEC says:**
  - §2: "a one-shot API and a streaming API that give identical output however input and output are chunked".
  - §6.1: the one-shot `decompress_frame` "decodes concatenated frames and skips skippable frames". The streaming decoder must therefore do the same.
  - The Frame format sections "Skippable Frames" and "Legacy frame".
- **What breaks:** any stream that starts with a skippable frame, or contains a legacy frame, decodes one-shot but fails when streamed. HANDOFF.md admits this. The one-shot path already has these rules (P2-06, P2-07), and none of them needs a dictionary.
- **Why it is skipped:** P3-02 depends on P2-08 only for its two dictionary tests.
- **Minimal fix:** R3-01, which is P3-02 without dictionaries. When P1-10 is unblocked and P3-02 reopens, P3-02 adds only the dictionary part.

### F2. Category 3: the malformed corpus SPEC §6.4 requires does not exist (R3-02)

- **Where:** `testdata/malformed/` and `testdata/results/malformed-*.json` are both absent.
- **What SPEC says:** §6.4 and §8 Phase 2 require every malformed file to return its expected variant without a panic. Of §6.4's nine case kinds, none needs a dictionary.
- **Why it is skipped:** P2-13 depends on P2-08 for one case only: "a Dict-ID with no dictionary".
- **What breaks:** the expected-variant and no-panic checks the decoder's error paths depend on never run. Neither does the Phase 2 manual check.
- **Minimal fix:** R3-02, which is P2-13 without the Dict-ID case. P2-13 adds that case when it reopens.

### Checks with no finding

- **Constraints (category 1):** the `kukulu_verify` constraint scan is clean. I also read both diffs for the reviewer-checked rules:
  - Both diffs are test code only: `crates/cleanlz4/tests/limits.rs` and `crates/oracle/tests/corpus.rs`.
  - `docs/DESIGN.md` changes only in the "Held by" list of the Output limit entry, as R2-01 asked.
  - No config value is spelled out. R2-02 reads the levels from `Config`. The `64` in its tests is the size of the combination space (16 flag settings times 4 block sizes), not a config key.
  - `docs/spec/`, `docs/kukulu.json`, `docs/cleanroom.json`, `.gitignore`, `.gitattributes` and CI config are untouched. `.kukulu/` changed only through the tools.
- **Clean room:** `kukulu_status` reports 3 denials for the whole flight: seq 112, 396 and 584. All three were judged in earlier rounds. The log has no denial after seq 592, which is where it stood at `lastReviewCommit` d56b903. So round 2's implement stage had no denials, and nothing was worked around.
- **Tests (category 3):** I applied each mutant with `kukulu_mutate`, using the targeted cargo command and `confirmKill`. Every one was killed by the test its task names, and each failure tail names that test:
  - `crates/cleanlz4/src/frame.rs:467`: `append_capped(out, &block, *remaining)?;` changed to `out.extend_from_slice(&block);`. Killed by `legacy_output_capacity_never_exceeds_the_limit`, which reports "capacity 33554432".
  - `crates/oracle/src/corpus.rs:159`: the block size id changed from `4 + ((hi + lo) % 4)` to `4 + (hi % 4)`. Killed by `frame_combos_are_distinct_and_every_level_meets_every_block_size`, which finds duplicate combinations.
  - `crates/oracle/src/corpus.rs:160`: the linked flag changed from `hi & 8` to `hi & 4`. Killed by both `manifest_frame_entries_match_the_generator` (32 distinct keys, not 64) and `frame_combos_are_distinct_and_every_level_meets_every_block_size`.
  - `crates/oracle/src/corpus.rs:172`: `frame_level` changed to index by `(i / 16)` instead of `i`. This is a neighbouring mutant, not one of R2-02's own. It was killed by `manifest_frame_entries_match_the_generator`, which shows that a stale manifest is now caught.

## Blocked and skipped tasks

- **P1-10 (blocked): left blocked.** The blocker is real. The binding crate (`lz4-sys` 1.11.1) does not export `LZ4_decompress_safe_usingDict` (compiler E0425, seen in three rounds).
  - SPEC §6.5 says "If the binding crate lacks one of these, the task blocks and names it". §9 Q2 gives the owner the remedy.
  - Round 2's review explained why a reviewer cannot unblock it a second time. That reasoning still holds, and nothing in round 2 changed it.
  - What unblocks it: a spec change or a binding swap by the owner (see Spec issues), followed by a plan change to P1-10's line "a missing `LZ4_decompress_safe_usingDict` blocks, naming it".
- **Skipped, all waiting on the P1-10 chain:** P2-08, P2-11, P2-12, P2-13, P3-02, P3-03, P3-06, P4-01 to P4-09, P4-12, R1-04, R1-05 and R1-07. They reopen with P1-10.
  - The non-dictionary parts of P2-13 and P3-02 are queued now as R3-02 and R3-01.
  - P3-03 (chunking invariance) and the fuzz tasks also need no dictionary oracle. I did not queue them, because doing so would duplicate whole tasks that reopen anyway. The owner may want to re-plan them so they depend on R3-01 instead of P3-02.

## Spec issues

- **SPEC §6.5 and §9 Q2: the owner must decide how dictionary decoding is checked against the oracle.** This decision blocks the flight and owes SPEC §1's Done list: full conformance, the malformed corpus, chunking invariance and the fuzz campaign. The options, carried from round 2, are:
  1. Permit the `lz4` CLI (`lz4 -D <dict>`) as the dictionary oracle, as §9 Q2 already offers.
  2. Amend §6.5 to accept the linked-frame method for blocks: compress `[dict, data]` as a linked frame and decode it with `LZ4F_decompress`. SPEC would then also have to say that this method cannot judge a wrong dictionary.
  3. Swap the binding crate for one that exports the function, and record its version in the manifest.
- **Carried from earlier rounds, still open:**
  - The oracle accepts zero offsets that the Block format calls invalid. SPEC should say that `docs/spec/` wins over the oracle.
  - The oracle protocol needs a carve-out for counting allocators in tests and examples.
  - "Bounded memory" has no stated accounting. DESIGN records the rule this build used: a reallocation counts only its size change.

## Manual checks still owed (from HANDOFF.md)

- **Phase 0:**
  - `make check` is green, and the guard self-test passes.
  - `testdata/MANIFEST.json` names both corpora, the binding crate's version (1.11.1, library 1.10.0) and the `lz4` CLI's version (1.9.4).
  - `.cargo/config.toml` sets `net.offline`.
- **Phase 1:**
  - Read `docs/DESIGN.md`'s rule table against the Block format and the XXH32 section.
  - Read `docs/spikes/P1-06.md` and confirm each end-of-block decision follows its table.
  - Confirm `conformance-P1-12.json` shows `passed == total`.
- **Phase 2:**
  - Owed once P1-10's chain runs: `conformance-P2-12.json` (100%) and `malformed-P2-13.json` (every file returns its expected variant, with no panics).
  - Read `docs/spikes/P2-09.md` and DESIGN's Differential rules, and confirm each normalization cites a section.
- **Phase 3:**
  - Read `crates/cleanlz4/src/stream.rs` for ergonomics.
  - Run `cargo test -p oracle --test limits` and watch the bomb figures.
  - The chunking coverage table is owed (P3-03, P3-06).
- **Phase 4:**
  - No fuzz results exist yet. When they do, check for zero crashes and zero divergences, or a triage of each in DESIGN.
  - Read `bench-P4-10.json` and `bench-P4-11.json` (actuals only).
- **Phase 5:**
  - After summarize, `docs/SIMILARITY.md` and the attestation bundle exist, and `verify.mjs` passes.
  - The owner runs the verifier from a clean checkout on a second machine.

## Notes

- **The implementer still applies mutants by hand.** R2-01 and R2-02 applied their mutants with `sed` and restored them, because the implementer has no mutate tool. The tree was clean afterwards, and the kills match what I saw with `kukulu_mutate`.
- **Possible test gap:** R2-02's manifest check covers the `synthetic` and `frames-cantrbry` suites, as its task asked. It does not cover the sampled `frames-silesia` combinations, which PLAN put out of scope.
- **Performance (not a gate), carried from round 2:** `decode_data_block` and `decode_legacy_frame` allocate a fresh scratch `Vec` for each block and do not reuse it.
- **My own tool friction (all logged with `kukulu_feedback_log`):**
  - The guard refused a read-only `sed` on `.kukulu/cleanroom/log.jsonl`, though a `grep` on the same file had been allowed. I did not retry another way. For the denial check I relied on `kukulu_status` and on that earlier `grep`.
  - A `sed` on the plugin's `mcp/server.mjs` was refused in the same way.
  - A list `filter` passed to the targeted cargo mutation command expands to two test names, which cargo rejects. That run came back inconclusive, and I re-ran it with one filter.
