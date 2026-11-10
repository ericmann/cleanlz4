# Changelog

All notable changes to this project are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/ericmann/cleanlz4/releases/tag/v0.1.0
