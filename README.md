# cleanlz4

A clean-room, memory-safe, zero-dependency Rust decoder for the LZ4 block and
frame formats and XXH32, written from the three documents in `docs/spec/` alone.

## What was built

- `crates/cleanlz4`: XXH32 (one-shot and streaming), the block decoder with
  dictionary-prefixed blocks, the frame decoder (every block maximum size,
  linked and independent blocks, block and content checksums, content size,
  concatenated, skippable and legacy frames, preset dictionaries with Dict-ID
  rules, an output limit), and a streaming decoder for every frame kind. No
  `unsafe`, no dependencies.
- `crates/oracle`: the reference side for tests: a wrapper of the C library, the
  conformance corpus generator and runner (including dictionary, skippable,
  legacy and concatenated suites), the malformed corpus, the differential compare
  module, the fuzz logic and runner, and the benchmarks.
- `docs/DESIGN.md`: every decision, the spec section it implements and the test
  that holds it. `docs/EVIDENCE.md`: the evidence index and what is not done.

## Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`
- all checks: `make check`
- corpus and conformance: `make corpus`, `make conformance TASK=<id> CATEGORIES=<list|all>`
- malformed corpus: `make malformed TASK=<id>`
- benchmarks: `make bench TASK=<id>`
- fuzz: `make fuzz-smoke`, `make fuzz-run TARGET=<target> TASK=<id>`; build with `cargo +nightly fuzz build`

See `docs/EVIDENCE.md` for results and for what remains.
