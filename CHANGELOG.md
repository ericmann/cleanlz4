# Changelog

All notable changes to this project are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-10

A hardening release: no change to the public API, to the `Error` variants or to any key of `testdata/config.json`.

### Added

- **Dictionary differential test:** `crates/oracle/tests/dict_corpus.rs` decodes every `dictionary` entry of the conformance corpus (72 entries: 54 frames and 18 dictionary blocks, at every configured dictionary size) with the `lz4` program and with cleanlz4, and requires equal bytes that match the recorded input. A frame with a Dict-ID and no dictionary must be `DictionaryRequired`.
- **Two mutant-killing tests:** `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent` (`crates/oracle/tests/dict.rs`) and `runner_passes_the_dictionary_to_frame_entries` (`crates/oracle/tests/results.rs`) fail when the dictionary label or the runner's dictionary hand-over is mutated, as 0.1.0's review found they would not.
- **Allocation tests:** `crates/oracle/tests/scratch.rs` shows that fresh allocations while decoding do not grow with the block count, for one-shot, streaming and legacy frames.
- **Release test:** `crates/oracle/tests/release.rs` checks that both crate versions agree and that the newest changelog entry and its link line name that version.
- **Legacy test:** two legacy frames back to back decode in order (`legacy_frame_ends_before_a_second_legacy_frame`), and a unit test names the three kinds of magic number.

### Changed

- **One magic classifier:** `is_magic` is merged into `magic_kind`; the constraint `single-magic-classifier` keeps the duplicate from returning.
- **One scratch buffer per frame:** compressed blocks decode into one reused buffer instead of a new `Vec` per block. The memory bound is unchanged. Measured (`testdata/results/bench-P1-05.json` before, `bench-P1-07.json` after): aggregate 532.1 MB/s before, 502.7 MB/s after (5.54% slower, within the 7.28% spread measured at 10 repeats; a first run gave 8.77%), so the change was kept.
- **Comments and design notes:** stale statements about the dictionary oracle in `crates/oracle/src/` and `docs/DESIGN.md` are corrected.
- **Conformance check:** `make conformance-check` also runs for changes to the runner and the dictionary module.

### Verified

- **Baseline:** `testdata/results/conformance-P0-03.json`, 2349 of 2349 passed, identical to 0.1.0's full run (`corpus_sha256` fd6392a2...ba05).
- **Conformance:** `testdata/results/conformance-P2-01.json`, 2349 of 2349 passed across ten categories.
- **Malformed:** `testdata/results/malformed-P2-02.json`, 144 of 144 matched, 0 panics.
- **Fuzzing**, 3,601 s per target, seed 1, 0 crashes and 0 divergences in every run: `fuzz-no_panic_block-P2-03.json` (37,841,791 runs), `fuzz-no_panic_frame-P2-04.json` (142,651,039), `fuzz-no_panic_stream-P2-05.json` (30,891,817), `fuzz-diff_roundtrip-P2-06.json` (61,410), `fuzz-diff_accept-P2-07.json` (14,818,411). Nothing to triage (`docs/DESIGN.md`, Divergence triage).
- **Benchmark:** `testdata/results/bench-P2-09.json`, cleanlz4 410.9 to 2,275 MB/s and aggregate 559.6 MB/s, the oracle 185.8 to 1,983.4 MB/s; there is no target.

### Notes

- The oracle library (`lz4-sys` 1.11.1+lz4-1.10.0) and the `lz4` program (v1.9.4) are 0.1.0's.
- No task of this release was blocked or skipped.

## [0.1.0] - 2026-11-10

The first release. cleanlz4 is a clean-room, memory-safe, zero-dependency Rust decoder for the LZ4 block and frame formats and XXH32, written from the three documents in `docs/spec/` alone.

### Added

- **XXH32:** one-shot and streaming.
- **Block decoder:** decodes into a caller-provided buffer, with overlapping match copies and a dictionary prefix. It enforces the end-of-block rules that the reference implementation also enforces (spike P1-06).
- **Frame decoder:**
  - frame descriptor parsing, with independent and linked blocks;
  - block and content checksums, and content size;
  - concatenated, skippable and legacy frames;
  - a trailing-data policy;
  - preset dictionaries with Dict-ID.
- **Streaming `Decoder`:** a state machine for every frame kind, dictionaries and trailing data. `Decoder::remainder()` returns bytes left over at the end of the input.
- **Output limit:** `DEFAULT_MAX_OUTPUT_BYTES` is 1 GiB, and memory is bounded by the limit plus one block plus 64 KiB.
- **Typed errors:** a typed `Error` enum with 17 variants. Library code has no reachable panic, no `unsafe` and no dependencies.
- **Test harness:** a differential harness against the reference C library and the `lz4` program, run as black boxes. It includes:
  - a conformance corpus, with 2349 of 2349 passing across ten categories;
  - a hand-built malformed corpus, with 144 of 144 rejected as expected and 0 panics;
  - five `cargo-fuzz` targets, each run for an hour with 0 crashes;
  - benchmarks against the reference library.
- **Evidence:** an evidence index (`docs/EVIDENCE.md`), design decisions with their spec citations (`docs/DESIGN.md`), and a signed clean-room attestation (`docs/attestation/`).

### Notes

- SPEC 0.2 records the owner's decision to use the `lz4` program as the dictionary oracle. Frames that use a dictionary are checked against their known original data. Raw dictionary blocks are checked against the `lz4` program.
- The differential rule `end-of-block-room` treats every `EndOfBlockViolation` that the reference library accepts as a match. This applies to tests only, and the decoder stays strict.
- The similarity check in `docs/SIMILARITY.md` reports "independence not supported". The verdict rests only on n-gram metrics against a same-language comparison tree, judged against cross-language controls. Every metric against the C reference sits at or below the negative control.

[0.2.0]: https://github.com/ericmann/cleanlz4/releases/tag/v0.2.0
[0.1.0]: https://github.com/ericmann/cleanlz4/releases/tag/v0.1.0
