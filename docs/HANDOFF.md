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
