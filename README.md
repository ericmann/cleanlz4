# cleanlz4

A clean-room, memory-safe, zero-dependency Rust decoder for the LZ4 block and
frame formats and XXH32, written from the three documents in `docs/spec/` alone.

## What was built

- `crates/cleanlz4`: XXH32 (one-shot and streaming), the block decoder with
  dictionary-prefixed blocks, the frame decoder (every block maximum size,
  linked and independent blocks, block and content checksums, content size,
  concatenated, skippable and legacy frames, an output limit), and a streaming
  decoder for regular frames. No `unsafe`, no dependencies.
- `crates/oracle`: the reference side for tests: a wrapper of the C library, the
  conformance corpus generator and runner, the differential compare module and
  the benchmarks.
- `docs/DESIGN.md`: every decision, the spec section it implements and the test
  that holds it. `docs/EVIDENCE.md`: the evidence index and what is not done.

## Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`
- all checks: `make check`
- corpus and conformance: `make corpus`, `make conformance TASK=<id> CATEGORIES=<list|all>`
- benchmarks: `make bench TASK=<id>`
- fuzz build: `cargo +nightly fuzz build`

See `docs/EVIDENCE.md` for results and for what remains.
