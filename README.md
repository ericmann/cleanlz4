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

## 0.2.0

0.2.0 is a hardening release on 0.1.0, with no change to the public API, the
`Error` variants or the configuration:

- a dictionary differential test: every dictionary entry of the conformance
  corpus is decoded by the `lz4` program and by cleanlz4 and compared
  (`crates/oracle/tests/dict_corpus.rs`);
- two tests that kill mutants 0.1.0's review found surviving;
- decoder cleanups: one magic-number classifier, and one scratch buffer per
  frame, measured before and after;
- a re-verification: the full conformance and malformed runs, a fuzz run of
  every target and a benchmark, all on the 0.2.0 code.

See `CHANGELOG.md` for the entry and `docs/EVIDENCE.md` for the result files.
0.1.0's plan, review and summary are under `docs/flights/cleanlz4/`.

## Attestation

The 0.2.0 flight's clean-room attestation bundle is published in
`attestation/cleanlz4-0-2-0/`, with a `SHA256SUMS` and a `VERIFY.txt`, and as a
tarball on the v0.2.0 release. It was signed by the key with this fingerprint:

```text
sha256:29eb2de703efe67ab50bf992d6c66c1e75d824010c727b1a76b0b336a5ed9f75
```

The same fingerprint is published on <https://kukulu.tech>. To verify the
bundle offline (needs Node 22 or later, nothing else):

```text
git clone --depth 1 https://github.com/ericmann/cleanlz4 && node cleanlz4/attestation/cleanlz4-0-2-0/verify.mjs cleanlz4/attestation/cleanlz4-0-2-0 --key sha256:29eb2de703efe67ab50bf992d6c66c1e75d824010c727b1a76b0b336a5ed9f75
```

The last line it prints is `verify-attestation: PASS`, and it exits 0.

## Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`
- all checks: `make check`, which also runs the clean-room guard self-test
  (`node <kukulu plugin>/scripts/cleanroom-guard.mjs --self-test docs/cleanroom.json`),
  the dependency check (`bash scripts/check-deps.sh`) and the spec-source check
  (`node scripts/verify-spec-sources.mjs`)
- corpus and conformance: `make corpus`, `make conformance TASK=<id> CATEGORIES=<list|all>`, `make conformance-check`
- malformed corpus: `make malformed TASK=<id>`
- benchmarks: `make bench TASK=<id>`
- fuzz: `make fuzz-smoke`, `make fuzz-run TARGET=<target> TASK=<id>`; build with `cargo +nightly fuzz build`

See `docs/EVIDENCE.md` for results and for what remains.
