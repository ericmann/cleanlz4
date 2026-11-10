# Review: build/2026-11-10
Round: 2

## Verdict

CHANGES REQUESTED

Round 1's four fix tasks that ran (R1-01, R1-02, R1-03, R1-06) are sound and do what their findings asked. `kukulu_verify` passed every check on HEAD:

- all 14 constraints, with no fixture failures and no hits;
- fmt, clippy and `cargo test --workspace`;
- both deps scripts, spec sources, and the guard self-test (568 cases).

Every mutant I applied to the round's mechanics was killed, except two. The verdict is not an approval for two reasons. P1-10 is still blocked, so 20 tasks are still skipped. Two of the round's mechanics are untested: the legacy frame's output-capacity cap and the frame-combination generator.

`prChecks`: `unknown: no checks reported on the 'build/2026-11-10' branch`. There is no CI on the branch, so this is not a finding.

## Findings (most severe first)

No category 1 or 2 findings. I read the round's diffs against CLAUDE.md's reviewer-checked constraints:

- the decoder changes use `get`, `saturating_*` and `checked_add`, with no `assert!`, index slicing or panicking call;
- no config value is spelled as a literal;
- `docs/spec/`, `docs/kukulu.json`, `docs/cleanroom.json` and `.gitignore` are untouched.

`.kukulu/` changes only through the tools.

### F1. Category 3: the legacy frame's output capacity is not held to the limit by any test (R2-01)

- **Where:** `crates/cleanlz4/src/frame.rs:467`, `append_capped(out, &block, *remaining)?;` in `decode_legacy_frame`.
- **Mutant:** replaced with `out.extend_from_slice(&block);`, which restores the amortized doubling R1-02 removed. It **survived**. Every R1-02 test uses regular frames: `one_shot_output_capacity_never_exceeds_the_limit` and both `bomb_peak_*` tests.
- **What breaks:** SPEC §1 property 3 and §2 ("a configurable output limit that stops decompression bombs with a typed error and bounded memory") are untested for legacy frames. Their blocks are 8 MiB. A future edit could let three full legacy blocks plus a short one reserve 32 MiB of capacity under a 24 MiB + 1 limit, and no test would fail. DESIGN.md's bound says "at any limit" for the one-shot path, and the legacy frame is part of that path.
- **Minimal fix:** a test only (see R2-01). The code is right today.

### F2. Category 3: the frame-combination generator is tested only through the committed manifest (R2-02)

- **Where:** `crates/oracle/src/corpus.rs:154-167`, `frame_combos`.
- **Mutants**, both of which **survived**:
  - `block_size_id: 4 + ((hi + lo) % 4) as u8` changed to `4 + (hi % 4) as u8`, which ties block size to two flag bits and leaves duplicate combinations;
  - `linked: hi & 8 != 0` changed to `hi & 4 != 0`.
- **Why they survive:** R1-06's `every_frame_level_meets_every_block_size` reads the committed `testdata/MANIFEST.json`, and so do `frame_suites_cover_all_64_flag_combinations` and `every_frame_level_and_block_size_appears`. `cargo test` never regenerates the manifest. The only direct check on `frame_combos()` is its length.
- **What breaks:** an edit to the generator passes `kukulu_verify` until someone runs `make corpus`. A stale manifest is never noticed either: nothing compares it with the generator's current order. That is SPEC §6.3's "every combination is recorded in MANIFEST.json".
- **Minimal fix:** tests only (see R2-02).

## Blocked and skipped tasks

- **P1-10 (blocked again): left blocked.** Only an owner decision unblocks it (see Spec issues).
  - Round 1 unblocked it with a narrowed scope: build `dict_block` without `LZ4_decompress_safe_usingDict`, and use the oracle's decode of the linked `[dict, data]` frame as its dictionary oracle. The implementer blocked it again, citing the same missing function.
  - On re-reading, the re-block is the reading most consistent with SPEC. SPEC §6.5 says "If the binding crate lacks one of these, the task blocks and names it". §9 Q2 says what resolves it: "The owner either swaps the binding crate or permits the `lz4` CLI as the oracle for that case."
  - The linked-frame route was a third option that neither SPEC nor the owner has sanctioned. Round 1's own Spec issues said the owner's decision was still owed. CLAUDE.md's oracle protocol also says to block. SPEC wins over a review note.
  - Unblocking P1-10 a second time would ask the implementer to depart from SPEC, and the round shows it will not. So this is not a finding against the implementer.
  - The implementer's attempt to declare the function in `ffi.rs` was a compile probe of the binding, as the oracle protocol allows. It read no binding source (log seq 471 to 475; the work was reverted at seq 475).
- **P2-08, P2-11, P2-12, P2-13, P3-02, P3-03, P3-06, P4-01 to P4-09, P4-12, R1-04, R1-05, R1-07 (skipped):** each waits on the P1-10 chain. They reopen when P1-10 does.
  - P2-13, P3-02's skippable, legacy and concatenated streaming, and the fuzz tasks need no oracle dictionary. They are skipped only because the plan routes them through P2-08.
  - Re-queuing them as fix tasks would duplicate twenty tasks that reopen anyway, so I have not.
  - The owner's decision is the critical path for SPEC §1's Done list: full conformance, the malformed corpus, chunking invariance and the fuzz campaign.

## Spec issues

- **SPEC §6.5 and §9 Q2: the owner's decision on the dictionary oracle is now blocking most of the build.** The binding crate (`lz4-sys` 1.11.1) has no `LZ4_decompress_safe_usingDict` (compiler E0425, twice). SPEC routes this case to the owner, so a reviewer cannot. The owner has three options, each of which unblocks P1-10 and its 20 dependents:
  1. Permit the `lz4` CLI as the dictionary oracle, as §9 Q2 already offers. `lz4 -D <dict>` decodes dictionary frames, and the CLI is already a permitted black box (§3.1). This also gives P2-08 an oracle for frames, which the plan said was impossible.
  2. Amend §6.5 to accept the linked-frame method for blocks: compress `[dict, data]` as a linked frame and decode it whole with `LZ4F_decompress`, a function already on the surface. Then state that the oracle cannot judge a wrong dictionary.
  3. Swap the binding crate for one that exports the function, and record its version in the manifest.

  Whichever is chosen, PLAN P1-10's line "a missing `LZ4_decompress_safe_usingDict` blocks, naming it" must change with it. That line is what the implementer followed.
- **Carried from round 1, still open:**
  - The oracle accepts zero offsets that the Block format calls "invalid (corrupted)". SPEC should say that `docs/spec/` wins over the oracle.
  - The oracle protocol needs a carve-out for test and example counting allocators.
  - "Bounded memory" has no stated accounting. R1-02 settled it for this build: a reallocation counts its size change only. SPEC or DESIGN should state it.

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
  - Owed once P1-10's chain runs: `conformance-P2-12.json` (all ten categories, 100%) and `malformed-P2-13.json` (`matched_expected_error == total`, `panics == 0`).
  - Read `docs/spikes/P2-09.md` and DESIGN's Differential rules, and confirm each normalization cites a section.
- **Phase 3:**
  - Read `crates/cleanlz4/src/stream.rs` for ergonomics.
  - Run `cargo test -p oracle --test limits` and watch the bomb figures. R1-02 measured the excess over the limit at 4,259,840 bytes at every limit probed.
  - The chunking coverage table is owed (P3-03, P3-06).
- **Phase 4:**
  - Every `fuzz-*.json` shows zero crashes and zero divergences, or DESIGN triages each one.
  - Every run's `seconds` is at least `fuzz.seconds_per_target`.
  - Read `bench-P4-10.json` and `bench-P4-11.json` (actuals only).
- **Phase 5:**
  - After summarize, `docs/SIMILARITY.md` and the attestation bundle exist, and `verify.mjs` passes.
  - The owner runs the verifier from a clean checkout on a second machine.

## Notes

- **Round 1 findings, resolved:**
  - F1 (R1-01): `probe_frame` reads `diff.max_output_bytes`. The new source-scan test killed a mutant that spelled `67_108_864`.
  - F2 (R1-02): output, scratch and history capacities are capped. Three mutants were killed, by the capacity test and by `bomb_peak_allocation_is_bounded_past_several_blocks`: the cap on output growth removed, the cap on scratch growth removed, and the history extended before it is trimmed. A correctness mutant on the history trim was killed by `history_reaches_back_65535_bytes_across_blocks`.
  - F3 (R1-03): all three round-1 survivors in `diff.rs` are now killed, each by the test the task named: the `zero-offset` arm, the `OverCap` arm, and the Legacy-kind guard.
  - F5 (R1-06): the order is fixed and the manifest regenerated. The manifest diff touches only conformance entries, but see F2.
  - F4 (R1-05) waits on P3-02.
- **Clean room:**
  - No denials during round 1's implement stage. The flight's two earlier denials (seq 112, seq 396) were judged in round 1.
  - I scanned every log entry since round 1's review for registry, build-directory, header, prior-art, disassembly, tracing and network targets, and found none.
  - R1-03's implementer applied its three mutants with `sed` and restored them, because it had no mutate tool. The tree was clean afterwards, and that is not a finding.
- **My own denial:** a read-only `sed -n 2910,2990p mcp/server.mjs` in the Kūkulu plugin directory was refused. The guard read `2910,2990p` as a path. I wanted to see whether `kukulu_task_next` shows an unblock reason to the implementer. I did not retry another way, and I logged it with `kukulu_feedback_log`.
- **Pipeline note (logged):** round 1's unblock reason lived only in PROGRESS.md, while PLAN.md's task body still told the implementer to block. One round was spent learning that.
- **Performance (not a gate):** `decode_data_block` and `decode_legacy_frame` still allocate a fresh scratch `Vec` per block and copy it into the output. It is now capped, but it is not reused.
