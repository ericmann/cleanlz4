# Review: cleanlz4 0.2.0 (build/2026-11-10-2)
Round: 1

Verdict: CHANGES REQUESTED

Reviewed `4ea19f9..bfb30d7` (25 tasks, all done, none blocked or skipped) against `docs/SPEC.md` 0.3 and `docs/PLAN.md`, commit by commit. `kukulu_verify` (no files) is green: every one of the 19 constraint rules passes its own fixtures and has no hits; fmt, clippy, `cargo test --workspace`, the guard self-test (568 cases), both deps scripts and both spec-source scripts exit 0. `prChecks` reads `unknown: no checks reported on the 'build/2026-11-10-2' branch` (not a finding in round 1).

## Findings

### 1. Category 3 (tests): the streaming decoder's legacy scratch buffer can outlive its frame and no test notices (P1-06)

- **Where:** `crates/cleanlz4/src/stream.rs:237` (`self.scratch = Vec::new();` in `end_frame`) and `crates/cleanlz4/src/stream.rs:372` (`self.scratch = Vec::new();` in the `LegacySize` branch where a known magic number ends a legacy frame).
- **What is wrong:** P1-06 (PLAN: "replaced by `Vec::new()` at both places a legacy frame ends there ... So at most one scratch buffer is alive, none outlives its frame") and its `docs/DESIGN.md` entry "One scratch buffer per frame" ("none outlives its frame ... so the section 6.1 bound is unchanged. Held by ...") claim a property that no test holds. `kukulu_mutate` deleted each of the two lines in turn and ran the full `cargo test --workspace`: **both mutants survived**. The three `tests/scratch.rs` tests count fresh allocations within one frame only; the four memory tests never stream a legacy frame followed by another frame.
- **What would break:** a regression that keeps the legacy frame's buffer (up to one 8 MiB legacy block) alive after the frame ends would pass the suite. A following regular frame then holds its own `FrameState` scratch and the pending buffer next to it, so the decoder can hold two block-sized scratch buffers at once, which is what SPEC §6.1's bound (limit + one block maximum size + 65,536 bytes) and the DESIGN claim rule out.
- **Sweep of neighbouring mechanics in `stream.rs`** (round-1 rule): passing a fresh `Vec` instead of `self.scratch` to `decode_legacy_block` was killed (`legacy_fresh_allocations_do_not_grow_with_the_block_count`, streaming 22 vs 70); deleting `self.frame_done_pending = true;` in the magic branch was killed (`legacy_frame_then_regular_frame`). So the survivors are exactly the two "scratch dies with its frame" lines.
- **Minimal fix:** tests only, no decoder change. In `crates/oracle/tests/scratch.rs`, extend the counting allocator with a live-bytes counter (alloc adds the size, dealloc subtracts it, realloc adds the change; SPEC §3.7 carve-out) and add two streaming tests over a hand-built legacy frame with blocks large enough to dominate (for example 1 MiB of `word_salad` per block): one where the frame ends at a zero end mark (the `end_frame` path) and one where it is followed by another frame's magic number (the `LegacySize` path). Each asserts that the live bytes held when the legacy frame's `FrameDone` is reported are lower, by at least half a block, than while its last block was being delivered, and that the decoded bytes are right. Name both tests in the DESIGN entry.
- **Task:** P1-06 (fix task R1-01).

No other finding. Details of what was checked follow in the notes.

## Spec issues

- **SPEC §5.3's six-pair similarity block cannot be written with Kūkulu 0.8.0** (already recorded in PLAN's Spec issues): `docs/kukulu.json` keeps only the Rust-vs-Rust controls, so SPEC's "Score every pair; no pair is dropped" is not met for `independent-vs-c` and `derived-vs-c`. `subject-vs-c` must be read against 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`, as the HANDOFF's summarizer note says. SPEC should name which band a single run keeps, or wait for a tool that takes a band per language pairing.
- **SPEC §5.1 stray table header** (`| Key | Default | Meaning |` with no rows): cosmetic, already in PLAN.
- Known-gap candidates (guard): none. Both denials in the log (seq 43 and 66) are plan-stage false positives: a `sed -n 471,483p` range read as a path under the plugin cache, and a heredoc that wrote the SPEC-given prior-art path strings into a scratch config builder. Neither read anything, and neither was worked around by reaching the same target another way.

## Manual checks still owed

Copied from `docs/HANDOFF.md`:

- Phase 0: `testdata/results/conformance-P0-03.json` equals 0.1.0's `conformance-P2-12.json` in `corpus_sha256`, total, passed and every category; the P0-01 and P0-02 log entries show both corpora matching `testdata/MANIFEST.json`, `lz4` at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and `make check` green; `kukulu_status` reports `cleanroom.lockdown` true.
- Phase 1: read `crates/oracle/tests/dict_corpus.rs`, `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent` (`tests/dict.rs`) and `runner_passes_the_dictionary_to_frame_entries` (`tests/results.rs`); read `docs/DESIGN.md` Measurements, "Scratch-buffer reuse (P1-07)" and confirm the keep decision follows its table.
- Phase 2: `conformance-P2-01.json` 2349 of 2349, `malformed-P2-02.json` 144 of 144 with 0 panics, every `fuzz-*-P2-0[3-7].json` has `crashes` 0 and `seconds` at least `fuzz.seconds_per_target`, DESIGN.md "0.2.0 re-verification (P2-08)" says none, read `bench-P2-09.json` (no target).
- Phase 3: the owner reads `docs/SIMILARITY.md` (summarize stage) and runs the attestation `verify.mjs` from a clean checkout on a second machine.

## Notes

- **Constraints (category 1):** all rules green with valid fixtures. By reading: no `unsafe`, `assert!`, unchecked arithmetic or index slicing was added under `crates/cleanlz4/src/` (the decoder diff is `is_magic` removed, `FrameState.scratch`, `decode_legacy_block`'s new parameter and `Decoder.scratch`); the only new `unsafe` is the counting allocator in `crates/oracle/tests/scratch.rs`, inside SPEC §3.7's carve-out; no XXH32 digest literal; `docs/spec/`, `docs/cleanroom.json`, `docs/kukulu.json`, `.gitignore`, `.cargo/`, `Makefile` and `scripts/` are untouched on the branch; the lockfile diffs are the four version lines only; no implement-stage tool call names a path outside the repository, a registry, a build directory, a header, or a disassembler or tracer.
- **Result files:** the five fuzz result files agree with `fuzz/logs/<target>-P2-0n.log` (`number_of_executed_units` and 3601 s in each), so they were written by the runner; every new result file is in `docs/EVIDENCE.md`'s sorted list and its Results table.
- **Interpretation choices** in HANDOFF are each the reading most consistent with SPEC. P1-07's keep decision follows the plan's criterion (8.77% then 5.54% against 7.28%); the run-to-run noise the implementer reports (about 3 points) is worth the owner's attention when reading that table, but it is not a deviation.
- **Mutation sample** (all `killed` unless said): `frame.rs` scratch put-back removed (`one_shot_`/`streaming_fresh_allocations_...`), scratch `clear()` removed (`compressed_block_frame_decodes` and two dictionary tests), legacy arm of `magic_kind` removed (`legacy_frame_ends_before_a_second_legacy_frame`), legacy scratch replaced per block (`legacy_fresh_allocations_...`); `stream.rs` as in finding 1; `dict.rs` `Ok(_) => true` (`a_block_sharing_nothing_...`) and `runner.rs` dictionary hand-over disabled (`runner_passes_the_dictionary_to_frame_entries`), so both 0.1.0 survivors are now dead; `xxh32.rs` round rotation 13 to 12 (four oracle XXH32 tests); `lib.rs` default limit doubled (`limits_default_matches_library_constant`); `block.rs` zero-offset check disabled: the mutant makes `block::tests::zero_offset_is_rejected` loop until the timeout rather than fail, which counts as caught but is worth knowing.
- `crates/oracle/tests/scratch.rs` seeds `word_salad` with literals 7 and 9; that follows the existing tests' convention for fixed test data and is not `fuzz.seed`.
