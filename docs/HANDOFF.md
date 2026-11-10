# Handoff: initial build

- Branch: `build/2026-11-10`. Base commit: `0211bbf`. Head commit when this was written: `7db4d9a`.
- Tasks: 54 in all. 36 done, 1 blocked, 17 skipped, 0 open.
- Evidence for the attestation: the fenced list in `docs/EVIDENCE.md` (every tracked result file, `testdata/MANIFEST.json`, `docs/DESIGN.md`, `docs/spikes/*.md`). `docs/EVIDENCE.md` also states SPEC section 1's Done items.
- `kukulu_verify` with no files is green: guard self-test, fmt, clippy, tests, deps script, spec-source check, all constraints.

## What a reviewer must know first

The build is incomplete because one task blocked and the plan's dependency chain skipped what depended on it.

### Blocked

- **P1-10** (verbatim reason): "Tried: declared LZ4_decompress_safe_usingDict (SPEC §6.5, Block) via the binding crate in ffi.rs. Fails: the compiler reports E0425 "cannot find function LZ4_decompress_safe_usingDict in crate lz4_sys" (only a similarly named LZ4_decompress_safe_continue, which is not on the permitted surface, was suggested). The task says a missing §6.5 function blocks and names it. Fix: the owner supplies a binding that exports LZ4_decompress_safe_usingDict (or amends SPEC §6.5 to allow another route, e.g. declaring the C symbol directly in ffi.rs via an extern block, which links against the already-built liblz4); then re-run P1-10 and its dependents."

### Skipped (all follow from P1-10)

| Task | Waited on |
|---|---|
| P2-08 Preset dictionary and Dict-ID | P1-10 |
| P2-11 Conformance: dictionary, skippable, legacy, concatenated suites | P1-10 |
| P2-12 Full conformance run and results | P2-08 |
| P2-13 Malformed corpus and results | P2-08 |
| P3-02 Streaming: skippable, legacy, concatenated, dictionaries, trailing data | P2-08 |
| P3-03 Chunking-invariance property test | P3-02 |
| P3-06 Tune `stream.chunking_cases` | P3-03 |
| P4-01 to P4-03, P4-04 to P4-09 (fuzz targets, runner, five runs, triage) | P3-02 chain |
| P4-12 Tune `fuzz.seconds_per_target` | P4-03 |

Consequences in the code, for a reviewer who has not seen it:

- `cleanlz4` does not implement `DictionaryRequired`, `DictionaryMismatch` or the use of a supplied dictionary for frames. A frame with a Dict-ID decodes as if it had none. `DecodeOptions.dictionary` is stored and never read. The block-level dictionary decode (`decompress_block_with_dict`) exists and is unit tested, but never compared with the oracle.
- The streaming `Decoder` handles regular frames only. At a frame boundary a skippable or legacy magic is treated as not a frame (`BadMagic` or `TrailingData`), unlike the one-shot path.
- There is no fuzz target beyond the one-line placeholder `fuzz/fuzz_targets/no_panic_block.rs`, no fuzz result and no malformed corpus.
- The only conformance result is `testdata/results/conformance-P1-12.json` (category `block`, 156 of 156). No full-corpus run exists, so `make conformance-check` (wired to run on any change under `crates/cleanlz4/src/` from P2-12) is inactive.
- The frame and synthetic conformance suites are generated and recorded in `testdata/MANIFEST.json` (P2-10) but were never decoded as a whole.

## Interpretation choices (task ids)

- P0-03: manifest corpus names are `canterbury` and `silesia`; the Canterbury files live in `testdata/corpora/cantrbry`.
- P1-05: the low four bits of the final token are ignored; the output-limit check precedes the "literals present" check, so a huge claimed literal length with no data behind it is `OutputLimitExceeded`.
- P1-06/P1-07: observed in the oracle (`docs/spikes/P1-06.md`) and then set: end-of-block rule 1 enforced, rule 2 enforced, rule 3 not enforced, empty block rejected, `0x00` accepted.
- P1-07: the oracle accepts a zero match offset; cleanlz4 rejects it (`ZeroOffset`). Carried as the differential normalization `zero-offset`.
- P1-09: the oracle's high-compression level 12 faulted on an empty input through a dangling pointer, so `ffi::compress_hc` passes real memory for empty input (`ffi.rs` touched beyond the task's list).
- P1-11: entry `params` is a JSON object, entry file paths are relative to `testdata/conformance/`, and `Entry` has a flattened `extra` map for later suites.
- P2-02: the block size field is checked against the maximum before the data is requested; extra test `bytes_after_the_frame_are_trailing_data`.
- P2-05: content size is validated (the document permits skipping it); validation applies even when checksums are off.
- P2-06: one to three bytes that begin a known magic are `TruncatedInput` under both trailing-data policies.
- P2-07: legacy blocks are decoded as independent; the `lz4 -l` output of 20 MiB decoded without carrying history.
- P2-09: only `legacy-frame` and `zero-offset` are normalizations. Dict-ID without a dictionary is handled by a retry with an empty dictionary on `DictionaryRequired`. The probe test exempts the Dict-ID probe's fixed verdict because P2-08 was skipped.
- P2-10: combination order has the block size varying fastest, so `frame_levels[i % 4]` ties level to block size id; every level still meets both values of every other flag.
- P3-01: unknown boundary bytes are left unconsumed (not buffered) so `ReturnRemainder` reports them in full when they arrive whole.
- P3-04: a content size above the output limit fails with `OutputLimitExceeded` at the header, so a declared `u64::MAX` no longer reaches `ContentSizeMismatch`; the existing test in `crates/cleanlz4/tests/frame.rs` was changed to match.
- P4-11: the three counts compared were 3, 10 and 30.

## ⚠️ ASSUMPTION config keys

| Key | Default | Tuned? |
|---|---|---|
| `limits.max_output_bytes` | 1073741824 | measured in P3-05, kept |
| `bench.repeats` | 10 | measured in P4-11, kept (7.28% spread at 10, 7.36% at 30) |
| `stream.chunking_cases` | 10000 | not tuned (P3-06 skipped) |
| `fuzz.seconds_per_target` | 3600 | not tuned (P4-12 skipped) |
| `fuzz.smoke_seconds` | 20 | unused (no fuzz task ran) |
| `fuzz.seed` | 1 | used as the seed for all generation |
| `dict.sizes_bytes` | [4096, 65536, 131072] | unused (P1-10, P2-11 did not run) |
| other `conformance.*`, `diff.max_output_bytes`, `fuzz.max_*` | see `testdata/config.json` | not tuned |

## What a human must check by hand, per phase

- Phase 0: `make check` is green; the guard self-test passes; `testdata/MANIFEST.json` names both corpora, the binding crate version (1.11.1, library 1.10.0) and the `lz4` CLI version (1.9.4); `.cargo/config.toml` sets `net.offline`.
- Phase 1: read `docs/DESIGN.md`'s rule table against the block format and the XXH32 section; read `docs/spikes/P1-06.md` and confirm each end-of-block decision follows its table; `conformance-P1-12.json` shows `passed == total`.
- Phase 2: not completable as written: `conformance-P2-12.json` and `malformed-P2-13.json` do not exist. Read `docs/spikes/P2-09.md` and DESIGN's Differential rules and confirm each normalization cites a Frame format or Block format section.
- Phase 3: read `crates/cleanlz4/src/stream.rs` for ergonomics; run `cargo test -p oracle --test limits` and watch the bomb figure. The chunking coverage table does not exist.
- Phase 4: no fuzz results. Read `bench-P4-10.json` and `bench-P4-11.json` (actuals only, no target).
- Phase 5: after the summarize stage, `docs/SIMILARITY.md` and the attestation bundle must exist and `verify.mjs` pass.

## Codebase notes

- The binding crate's names and signatures were learned from compiler output only; the guesses `LZ4_compress_default`, `LZ4_compress_HC`, `LZ4_compressBound`, `LZ4_decompress_safe`, the `LZ4F_*` context, update, end and decompress functions, `LZ4F_VERSION` and the preferences struct (including block checksum) all compiled first time. `LZ4_decompress_safe_usingDict` is absent.
- The oracle's frame decoder rejects a legacy frame ("unknown frame type") and accepts a Dict-ID frame without a dictionary.
- The oracle's block decoder needs slack output capacity: it rejects some valid blocks at exact capacity (`docs/spikes/P1-06.md`). Differential block tests give it 65,536 extra bytes.
- clippy's `chunks_exact_to_as_chunks` lint rejects `chunks_exact` with a constant size under `-D warnings`; use `slice::as_chunks::<N>()`. `manual_repeat_n` wants `std::iter::repeat_n`.
- Editing with a script that matches rustfmt-reformatted text fails silently; run clippy before `kukulu_verify`.
- `copy_match` must not compute the in-buffer start when nothing is left after the dictionary part: `dst.len() - offset` underflows.
- Test binaries and an example in `crates/oracle` use `unsafe impl GlobalAlloc` (counting allocators in `tests/limits.rs` and `examples/limit_probe.rs`), as the tasks required. The oracle protocol's "only `ffi.rs` contains `unsafe`" applies to the C calls; these are test and example code only.
- Memory: P3-05 measured a peak above the limit of 16,777,206 bytes (10 bytes under the criterion) for 4 MiB blocks, larger than the bound P3-04 wrote down (limit + one block + 65,536). Likely cause: the linked-history `Vec` keeps a 4 MiB-block capacity after being trimmed, plus the scratch block and output growth. A decoder change (trim the history to its tail before extending it) would remove most of it.
- `make corpus` takes about 90 seconds and leaves about 1 GB under `testdata/conformance/` (gitignored); `testdata/MANIFEST.json` is 1.7 MB and regenerates byte for byte.
- One clean-room guard denial occurred: a `sed` on `docs/spec/lz4_Frame_format.md` through Bash was refused (protected path). The spec was read with the Read tool instead; nothing was worked around.
- Every commit carries `Kukulu-Stage` and `Kukulu-Task` trailers. The branch was pushed at each phase end.

## Round 1

Head before this section: 08a3987. Counts: 40 done, 1 blocked, 20 skipped, 61 total.

Done: R1-01 (probe_frame reads `diff.max_output_bytes`; new test `large_config_values_are_not_spelled_in_oracle_sources`), R1-02 (output, scratch and history capacity held to limit + one block + 65,536 bytes; `block::reserve_capped`, `frame::append_capped`; probe excess now 4,259,840 bytes at every limit; DESIGN.md table replaced), R1-03 (three diff.rs tests, each killed its mutant when applied by hand), R1-06 (frame combination order changed, MANIFEST.json regenerated, `corpus_sha256` now 110a2d51...).

Blocked: P1-10 came up as open at the start of the round and was blocked again, per the oracle protocol: the binding crate has no `LZ4_decompress_safe_usingDict` (compiler diagnostic: not found in crate). A human must supply it or amend SPEC 6.5. This leaves P2-08, P2-11 and everything after them skipped, and R1-04, R1-05, R1-07 skipped with them.

Interpretation: R1-02 maps an allocation failure to `OutputLimitExceeded` (no allocation error variant exists). R1-06 order: combination i has hi = i / 4, lo = i % 4; flags from the bits of hi, block size id 4 + (hi + lo) % 4. R1-03 mutants were applied by sed and restored, since no mutate tool was available to the implementer.

Reviewer notes: no conformance-P2-12 result exists, so R1-06 wrote no fresh conformance file; `make conformance-check` passes 2236 of 2236. The compiler diagnostic for the missing function printed a path under `~/.cargo/registry/`; the file was not opened.

## Round 2

Head before this section: 611f2fd. Counts: 42 done, 1 blocked, 20 skipped, 63 total, 0 open.

Done: R2-01 (`legacy_output_capacity_never_exceeds_the_limit` in crates/cleanlz4/tests/limits.rs; a hand-built legacy frame of three 8 MiB blocks and a 5-byte block; DESIGN.md Output limit "Held by" list updated; frame.rs unchanged), R2-02 (`frame_combos_are_distinct_and_every_level_meets_every_block_size` and `manifest_frame_entries_match_the_generator` in crates/oracle/tests/corpus.rs; corpus.rs and MANIFEST.json unchanged).

Mutants, applied by hand with sed and restored (no mutate tool was available): R2-01 `out.extend_from_slice(&block)` fails the new test (capacity 33,554,432). R2-02 both corpus.rs mutants fail both new tests.

Blocked: P1-10 is still blocked (binding crate lacks `LZ4_decompress_safe_usingDict`); P2-08, P2-11 and later tasks stay skipped.

Interpretation: R2-02 groups combinations by level value from Config. Clippy required `repeat_n` in the R2-01 test.

## Round 3

Head before this section: 393fbf2. Base commit `0211bbf`. Counts: 65 done, 0 blocked, 0 skipped, 65 total, 0 open. `kukulu_verify` with no files is green (guard self-test, fmt, clippy, tests, deps scripts, spec-source check, all 14 constraints).

Done, in order: P1-10 (the `lz4` program is the dictionary oracle, owner decision under SPEC section 9 Q2: `oracle::cli::decompress_frame_with_dict`, `oracle::dict::dict_block`; `ffi.rs` unchanged), P2-08 (`FrameState::new` takes the dictionary: `DictionaryRequired`, `DictionaryMismatch`, preset prefix or initial history; `oracle::dict::dict_frame`), P2-11 (dictionary, skippable, legacy and concatenated suites, 113 new entries, `make corpus` 108 s and byte-stable), P2-12 (`conformance-P2-12.json`: 2349 of 2349 across all ten categories), P2-13 (144 malformed files, `malformed-P2-13.json`: 144 of 144, 0 panics), P3-02 (streaming skippable, legacy, concatenated, dictionaries, trailing data), P3-03 (chunking invariance over `stream.chunking_cases` cases), P3-06 (tuned), P4-01 and P4-02 (five fuzz targets), P4-03 (runner, `make fuzz-run`, `make fuzz-smoke`), P4-04 to P4-08 (one hour each), P4-09 (triage and a `diff_accept` re-run), P4-12 (tuning record), R1-04, R1-05, R1-07, R3-01, R3-02.

Fuzz results (seconds, runs, crashes, divergences, corpus entries): `no_panic_block` 3,601 / 20,681,767 / 0 / 0 / 1,249; `no_panic_frame` 3,601 / 101,577,396 / 0 / 0 / 834; `no_panic_stream` 3,601 / 21,051,494 / 0 / 0 / 895; `diff_roundtrip` 3,601 / 58,050 / 0 / 0 / 444; `diff_accept` 3,601 / 21,607,948 / 0 / 100 / 820, and its P4-09 re-run 3,615 / 16,184,756 / 0 / 0 / 1,012. The 100 divergences were one case: a block ending with four final literals after a match of 19 or more bytes (end-of-block rule 2). cleanlz4 returns `EndOfBlockViolation`; the oracle accepts it only when its output buffer has spare room. Resolution: normalization `end-of-block-room` in `diff.rs` (a permitted choice, Block format "End of block conditions"), no decoder change; the inputs are in `testdata/regressions/` (indexed) and `testdata/results/divergences/`.

Interpretation choices this round: P1-10 `dict_block` uses linked blocks of `[dict, data]` and keeps the second block; P2-08 applies the dictionary's last 64 KiB as the prefix of every independent block or as the initial history of a linked frame, and the streaming decoder shares `FrameState::new`; P2-11 writes each dictionary under `testdata/conformance/dictionary/` and records `dict_file`, `dict_dependent` (the raw block's flag, for all four entries of a pair), `dict_id` only for frames that carry one; P2-13 expected variants come from the rules, not from running the decoder, and the corpus keeps the Dict-ID-without-dictionary case (R3-02 re-ran the same 144 files under its own id); P3-06 raised `stream.chunking_cases` from 10,000 to 80,000 (343 of 392 shapes hit at the old value, 392 of 392 at the new, 68 s for the full test); P4-01 `no_panic_block` splits dictionary and block by the third byte (`len * byte / 256`); P4-02 `diff_roundtrip` packs the level index and chunk count into byte 1; R1-05 exposes the held-back partial-magic bytes through `Decoder::remainder` instead of leaving them unconsumed (a fixed-window one-byte caller would never advance otherwise); R3-01 was already satisfied by P3-02, only its missing named test and the DESIGN entry were added.

⚠️ ASSUMPTION keys: `stream.chunking_cases` tuned (10,000 to 80,000, P3-06). `fuzz.seconds_per_target` was measured (P4-12) and not changed: `diff_roundtrip` still had new coverage at 3,351 s (93% of the run), so the recommendation, recorded in DESIGN.md, is to raise it, together with the timeouts in the protected `docs/kukulu.json`.

Friction and deviations: `kukulu_verify` on a `testdata/results/fuzz-*` path hit the MCP idle timeout of 1,800 s every time (logged with `kukulu_feedback_log`); the `make fuzz-run` process it started kept running to the end and wrote the result file, which I then read and committed after running `cargo test -p oracle --test results`. For P4-09 I moved the 100 divergence inputs aside during the re-run (the divergence file cap would have hidden a new divergence) and restored them afterwards. One Bash call that also named `docs/kukulu.json` was denied by the clean-room guard (protected path) and not retried. No mutation tool was available, so no mutant was run for R3-01.

What a human must check by hand: that `testdata/results/divergences/*.bin` can stay committed alongside `testdata/regressions/` (the evidence index test required listing them); that the `end-of-block-room` normalization is acceptable (it hides every `EndOfBlockViolation` against an oracle accept, not only the four-literal case the fuzzer found); that the recommendation to lengthen `diff_roundtrip`'s run is acted on.

## Codebase notes (round 3)

- The oracle's end-of-block acceptance depends on spare output capacity, not only on the block: a final run of four literals after a match of 19 or more bytes is accepted with 65,536 bytes of slack and rejected at exact capacity.
- `record_divergence` stops writing at `fuzz.max_divergence_files`, so a later fuzz run can hide new divergences unless the directory is emptied first.
- rustfmt reformats test source, so scripted string patches of test files fail silently; check the file after patching.
- Bytewise property tests over frames of 100 KB or more take minutes at every offset; sample large frames.
- clippy in this toolchain wants `is_multiple_of` and `explicit_counter_loop` fixes; run clippy before `kukulu_verify`.
- `kukulu_feedback_log` commits and moves HEAD, so call it after `kukulu_task_done`, not before.
