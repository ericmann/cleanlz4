# Review: build/2026-11-10
Round: 4

## Verdict

APPROVED

This round reviews everything since the round 3 review (`daed491`):

- the owner decision under SPEC §9 Q2 that makes the `lz4` program the dictionary oracle (plan commit `9461374`);
- the 20 tasks that reopened with P1-10: P1-10, P2-08, P2-11 to P2-13, P3-02, P3-03, P3-06, P4-01 to P4-09, P4-12, R1-04, R1-05 and R1-07;
- R3-01 and R3-02.

All 65 tasks are done. None is blocked or skipped. Categories 1 to 3 are clean across the branch.

`kukulu_verify` passed every check on HEAD, both with no files and with `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/stream.rs` and `fuzz/fuzz_targets/diff_accept.rs`:

- all 14 constraints, with no fixture failures and no hits;
- the guard self-test (568 cases), fmt, clippy, `cargo test --workspace`, both deps scripts and the spec-source check;
- `cargo fmt` for `fuzz/` and `cargo +nightly fuzz build`;
- `make conformance-check`: 2349 of 2349.

`prChecks`: `unknown: no checks reported on the 'build/2026-11-10' branch`. There is no CI on the branch, so this is not a finding.

## Findings (most severe first)

None.

### Checks with no finding

- **Constraints (category 1).** The tool's scan is clean. I also read the diff for the rules the tool cannot check:
  - Library changes are confined to `frame.rs` and `stream.rs`. They contain no `assert!`, no slicing by index, and no unchecked arithmetic on input-derived values. Sizes go through `saturating_*`, `try_from` and `get`.
  - No config key is written as a literal. The new numbers are format constants (4 MiB block, 8 MiB legacy block, the 64 KiB window) or plan-specified test regimes.
  - No test contains an expected XXH32 digest. `0xDEAD_BEEF` is used only as a deliberately wrong checksum.
  - `docs/spec/`, `docs/cleanroom.json` and `.kukulu/` were not edited by hand.
  - `docs/kukulu.json` and PLAN.md changed only in controller and owner commits (`8c27b1f`, flight options plus reformatting; `9461374`, the owner decision), not in task commits.
  - `.gitignore`, `.gitattributes`, editor config and CI config are untouched.
  - Every result file was added in the commit of the task that produced it. The log has no Write, Edit or redirect into `testdata/results/`.
- **Clean room.** `kukulu_status` reports 6 denials before this review:
  - Seq 112, 396, 584, 646 and 657 were judged in earlier rounds.
  - Seq 820 is new. During P4-01, a Bash `sed -n` on `docs/kukulu.json` was refused as a protected path. No later call in P4-01 reached that file another way; the next logged call is `kukulu_task_done`. HANDOFF.md records the denial. Not a finding.
  - Seq 1053 is my own: a read-only `sed -n` on `docs/spec/lz4_Frame_format.md`, logged with `kukulu_feedback_log`. I read the section with the Read tool, the permitted route for a permitted input.
  - A scan of every logged target since `daed491` found no reads of `~/.cargo/registry`, `~/.cargo/git` or `target/**/build`, no LZ4 headers or source, no prior-art paths, no `cargo doc`, `cargo expand` or `cargo vendor`, no tracing or disassembly tools, and no network tools. The `lz4` program was used only as a black box.
- **Boundaries (category 2).**
  - `frame.rs` still never imports `stream`.
  - `stream.rs` reuses `FrameState::new`, `decode_legacy_block` and `magic_kind` from `frame.rs`, so the one-shot and streaming paths share one dictionary rule and one legacy block rule.
  - `cli.rs` runs `lz4` through `std::process::Command`, so `ffi.rs` is unchanged, as PLAN P1-10 requires.
- **Tests (category 3).** I sampled mutants with `kukulu_mutate`, one or more per changed module. Each kill's failure tail names the test the task assigned to that mechanic.
  - `frame.rs`, Dict-ID mismatch check inverted: killed by `different_ids_are_dictionary_mismatch`.
  - `frame.rs`, a linked frame not seeded from the dictionary: killed by `linked_frame_uses_the_dictionary_once_at_the_start`.
  - `frame.rs`, dictionary tail cut to 32 KiB: killed by `assembled_linked_dictionary_frames_decode` and `assembled_independent_dictionary_frames_decode`.
  - `stream.rs`, skippable arm at the boundary made `BadMagic` (R3-01's named mutant): killed by `skippable_frame_first_streams_like_one_shot`.
  - `stream.rs`, legacy zero word not ending the frame: killed by `legacy_frame_ends_at_end_of_input`.
  - `stream.rs`, `remainder()` emptied: killed by `return_remainder_leaves_a_trailing_magic_prefix_unconsumed`.
  - `stream.rs`, legacy size cap doubled: killed by `every_one_shot_test_frame_matches_when_streamed_bytewise`.
  - `diff.rs`, `end-of-block-room` arm deleted: killed by `every_indexed_regression_input_passes_its_target_logic`.
  - `cli.rs`, non-zero `lz4` exit mapped to `Ok`: killed by `wrong_dictionary_never_panics_and_agrees_with_the_oracle`.
  - `malformed.rs`, an expected variant changed: killed by `malformed_files_are_reproducible`.
  - Two oracle-side survivors are under Notes. Neither can hide a decoder defect, and this round (4) is past `review.mutationFixRounds` (2).
- **Results.**
  - `conformance-P2-12.json`: 2349 of 2349, across all ten categories.
  - `malformed-P2-13.json` and `malformed-R3-02.json`: 144 of 144 each, 0 panics.
  - Five fuzz runs of about 3,601 s each, all with 0 crashes. The `diff_accept` P4-08 run hit 100 divergences, all of one case. They are triaged row by row in DESIGN.md, and its P4-09 re-run of 3,615 s found 0 divergences.
- **Dictionary evidence.** Beyond what the tests check, I ran the `lz4` program as a black box (`lz4 -d -D <dict> -c`) on all 54 dictionary *frame* entries in the generated corpus. All 54 decode to exactly the input slice the manifest names, which is also what cleanlz4 produces in the conformance run.
- **Interpretation choices (category 6).** Every choice HANDOFF.md lists for round 3 is the reading most consistent with SPEC:
  - The `end-of-block-room` normalization is broad. It covers any `EndOfBlockViolation` against an oracle accept. But cleanlz4 raises that error only for end-of-block rules 1 and 2 (rule 3 is not enforced, per `docs/spikes/P1-06.md`), which the Block format lets a decoder enforce ("a conformant decoder is allowed to reject"). The SPEC §9 Q1 decision to enforce them is recorded in DESIGN.md. The conformance run compares cleanlz4's output with the original input, not through normalizations, so a false `EndOfBlockViolation` on a valid oracle block would still fail conformance.
  - R1-05's `Decoder::remainder` is one of the two mechanisms PLAN R1-05 allowed.
  - P4-12 left `fuzz.seconds_per_target` unchanged and recorded a recommendation, exactly as its criterion directs.
  - P3-06 raised `stream.chunking_cases` to 80,000 under its stated criterion.

## Blocked and skipped tasks

None. P1-10 was unblocked by the owner's plan change (`9461374`, SPEC §9 Q2: the `lz4` program is the dictionary oracle), and its chain has completed.

## Spec issues

- **SPEC §6.5 vs §9 Q2.** §6.5 still lists `LZ4_decompress_safe_usingDict` as part of the oracle surface. The owner's decision replaced it with the `lz4` program, but only in PLAN.md. SPEC should record that decision, so that §6.5 and the constraint rule `oracle-calls-only-the-permitted-c-api` describe what was built.
- **Carried from earlier rounds, still open:**
  - The oracle accepts zero offsets that the Block format calls invalid. SPEC should say that `docs/spec/` wins over the oracle.
  - The oracle protocol needs a carve-out for counting allocators in tests and examples.
  - "Bounded memory" has no stated accounting.
- **Known-gap candidates:** none. The one guard inconsistency this round is a refusal, not a gap: a read-only `sed -n` on `docs/spec/` is denied while `grep` on the same file is allowed.

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
  - Confirm `conformance-P2-12.json` shows 100% (2349 of 2349).
  - Confirm `malformed-P2-13.json` shows every file returning its expected variant with no panic (144 of 144, 0 panics).
  - Read `docs/spikes/P2-09.md` and DESIGN's Differential rules, and confirm each normalization cites a section.
- **Phase 3:**
  - Read `crates/cleanlz4/src/stream.rs` for ergonomics.
  - Run `cargo test -p oracle --test limits` and watch the bomb figures.
  - Read the chunking coverage table (P3-06) in DESIGN.md.
- **Phase 4:**
  - Zero crashes in all five runs. The 100 `diff_accept` divergences are triaged in DESIGN.md `## Divergence triage`, and the P4-09 re-run found 0.
  - Decide whether the `end-of-block-room` normalization is acceptable as written. It hides every `EndOfBlockViolation` against an oracle accept, not only the four-literal case the fuzzer found.
  - Decide whether `testdata/results/divergences/*.bin` stays committed alongside `testdata/regressions/`.
  - Act on P4-12's recommendation to lengthen `diff_roundtrip`'s run (and the matching timeouts in `docs/kukulu.json`).
  - Read `bench-P4-10.json` and `bench-P4-11.json` (actuals only).
- **Phase 5:**
  - After summarize, `docs/SIMILARITY.md` and the attestation bundle exist, and `verify.mjs` passes.
  - The owner runs the verifier from a clean checkout on a second machine.

## Notes

- **Advisory mutation survivors (oracle harness, past `review.mutationFixRounds`; not tasks):**
  - `crates/oracle/src/dict.rs`: `Ok(plain) => plain != data` changed to `Ok(_) => true` survives. A block that decodes correctly without the dictionary would then be labelled `dict_dependent`. This is metadata in the manifest and does not affect any decoder verdict. A test could assert that a block of data with no shared text is reported as not dependent.
  - `crates/oracle/src/runner.rs`: dropping the dictionary from frame entries (`contains_key("dict_file")`) survives `cargo test --workspace`. Only a conformance run would catch it, and it fails loudly rather than passing falsely: Dict-ID frames would return `DictionaryRequired`. `make conformance-check` is bound to `crates/cleanlz4/src/` only, so it does not run when only the runner changes.
- **Dictionary frames and the oracle.** PLAN P2-08 still says "the oracle cannot decode these frames". That was true before the owner decision; it is now stale, and the `dict_frame` doc comment in `crates/oracle/src/dict.rs` repeats it. My black-box check above shows the `lz4` program decodes all 54 dictionary frames as cleanlz4 does. A future task could add that comparison to `crates/oracle/tests/dict_frame.rs`, so that SPEC §1's byte-identity claim for those entries rests on a committed test.
- **Stale comments:**
  - `crates/oracle/src/cli.rs` says it is "Used only to make legacy frames", but it is now also the dictionary oracle.
  - `crates/oracle/src/fuzz_logic.rs` says "None of them calls the oracle", but `diff_roundtrip` and `diff_accept` do.
- **Duplication:** `crates/cleanlz4/src/frame.rs` has both `is_magic` and `magic_kind`, which test the same three magic numbers. The one-shot path could use `magic_kind(..).is_some()`.
- **R3-02** re-ran the full 144-file corpus, including the Dict-ID case its plan excluded, because P2-13 had already landed. `malformed-R3-02.json` therefore duplicates `malformed-P2-13.json`. This is harmless.
- **Performance (not a gate), carried from earlier rounds:** `decode_data_block` and `decode_legacy_block` allocate a fresh scratch `Vec` for each block.
