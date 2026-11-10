# cleanlz4 — Specification

Version: 0.2 Status: draft for owner review

**Changes in 0.2 (2026-11-10):** §6.5 records the owner's §9 Q2 decision from the first flight. The binding crate lacks `LZ4_decompress_safe_usingDict`, so the `lz4` command-line program, run as a black box, is the oracle for dictionary decoding (clean-room record seq 681). That function is removed from the permitted C API, which now has fifteen functions; the oracle never called it. §9 Q2 is marked resolved.

The Kūkulu planner reads this file in full and derives the build plan from it. This is a clean-room flight: `docs/cleanroom.json` is committed on the base branch before the first `/kukulu:go-flight`, and every stage runs under it.

## 1. Overview

cleanlz4 is a memory-safe, zero-dependency Rust decoder for the LZ4 block and frame formats, with the XXH32 checksum the frame format uses. It is built clean-room from the three documents in `docs/spec/` alone. It is the second public clean-room rebuild made with Kūkulu, after cleanflate (zlib inflate). It is the first one whose whole provenance lives in a Kūkulu 0.7.x signed bundle that anyone can verify offline.

The artifact's credibility rests on three properties, in this order:

1. **Provenance.** The signed log and bundle show that no existing LZ4 or xxHash implementation was read.
2. **Correctness.** Output is byte-identical to the reference C implementation across the conformance corpus and differential fuzzing, and accept/reject decisions agree on malformed input.
3. **Safety.** `#![forbid(unsafe_code)]`, no reachable panics, bounded memory.

Performance is measured and reported. It is not a gate.

Done means:

- every requirement in §6 has its tests;
- every command in §7 exits zero;
- the fuzz campaign in Phase 4 has run for its configured duration with zero crashes and zero divergences;
- `kukulu_similarity` and `kukulu_attest` have produced `docs/SIMILARITY.md` and a bundle under `docs/attestation/` that passes `verify.mjs`;
- a draft PR is open.

## 2. Goals and non-goals

- Goal: decode LZ4 blocks per `docs/spec/lz4_Block_format.md`, including its end-of-block rules.
- Goal: decode LZ4 frames per `docs/spec/lz4_Frame_format.md`. That covers:
    - the frame descriptor and its flags;
    - every block maximum size;
    - linked and independent blocks;
    - uncompressed blocks;
    - block checksums;
    - the content checksum;
    - content size;
    - dictionary ID, with a caller-supplied dictionary;
    - concatenated frames;
    - skippable frames;
    - the legacy frame.
- Goal: XXH32 per `docs/spec/xxhash_spec.md`, used for the header checksum, block checksums and content checksum.
- Goal: a one-shot API and a streaming API that give identical output however input and output are chunked.
- Goal: a configurable output limit that stops decompression bombs with a typed error and bounded memory.
- Goal: a provenance record good enough to publish: the Kūkulu chain, the bundle, a similarity score with calibrated controls, and the demo artifacts in §5.4.
- Non-goal: compression. An encoder is a separate future flight under the same rules.
- Non-goal: XXH64 and XXH3.
- Non-goal: LZ4 HC-specific behavior. HC output is ordinary LZ4 and must decode, but nothing is built for HC.
- Non-goal: a C ABI, `no_std`, SIMD or multithreading.
- Non-goal: matching the reference implementation's error messages. Only accept/reject agreement is required (§6.4).

## 3. Engineering principles

Each is checkable by grep, by a test, or by the guard. The planner turns the mechanical ones into `constraints` in `docs/kukulu.json`.

1. **Clean room.** The permitted inputs are:
    - this spec and the files under `docs/spec/`;
    - Rust standard library documentation and general Rust and computer-science knowledge;
    - the oracle's C API signatures, exactly as listed in §6.5;
    - the `lz4` command-line program, run as a black box;
    - the corpora fetched in P0-03, as opaque bytes.

    Everything else about LZ4 or xxHash is forbidden. That includes:
    - any source or implementation write-up of LZ4, LZ4 HC or xxHash, in any language;
    - `lz4_flex`, `lz4-sys` internals, `twox-hash`, `xxhash-rust`, `lz-fear` and any other Rust crate in this space;
    - the C headers;
    - articles on how a named implementation decodes fast.

    `docs/cleanroom.json` enforces what a tool-call guard can. The guard is a tripwire, not a sandbox (Kūkulu `docs/cleanroom.md`), so the rule binds even where the guard cannot see.
2. **No unsafe, no dependencies.** `crates/cleanlz4` has `#![forbid(unsafe_code)]` at the crate root, the token `unsafe` appears nowhere under it, and its `[dependencies]` table is empty. Dev-dependencies belong only to `crates/oracle` and `fuzz/`.
3. **No reachable panics.** Library code carries `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]`. Tests are exempt.
4. **Design cites the spec.** Every non-trivial decision is recorded in `docs/DESIGN.md` with the section of the `docs/spec/` document it implements (for example, "Frame format, Block checksum"). An optimization cites cleanlz4's own profile evidence, never another implementation.
5. **Every tunable is a key.** Numbers marked ⚠️ ASSUMPTION in §5 are config keys with defaults, never bare literals.
6. **Results are committed as data.** Every task that runs conformance or fuzzing writes its counts to the result files in §5.4 in the same commit. The demo's timeline is rebuilt from these files and the chain, so a result that is only printed to a terminal does not exist.
7. **The oracle is a black box.** `crates/oracle` calls the C library only through the signatures in §6.5. No disassembly, profiling or tracing of the oracle or of `liblz4` (`commandPatterns` in the policy).

## 4. Architecture

| Path | Responsibility | May import |
|---|---|---|
| `crates/cleanlz4/src/xxh32.rs` | XXH32, one-shot and streaming | `core`, `std` |
| `crates/cleanlz4/src/block.rs` | Block decoding into a caller-provided window | `core`, `std` |
| `crates/cleanlz4/src/frame.rs` | Frame parsing, block sequencing, checksums, skippable and legacy frames | `block`, `xxh32` |
| `crates/cleanlz4/src/stream.rs` | Streaming decoder state machine | `frame`, `block` |
| `crates/cleanlz4/src/error.rs` | The typed error enum | — |
| `crates/cleanlz4/src/lib.rs` | Public API, limits, configuration | all of the above |
| `crates/oracle/` | FFI to the C library, corpus generation, the reference side of differential tests | `cleanlz4`, the binding crate |
| `fuzz/` | `cargo-fuzz` targets | `cleanlz4`, `oracle` |
| `testdata/` | Corpora, the generated conformance corpus, the malformed corpus, result files | — |

`crates/cleanlz4` never depends on `crates/oracle`. Nothing under `crates/cleanlz4/` mentions the oracle's binding crate.

## 5. Data and configuration

### 5.1 Tunables

| Key | Default | Meaning |
|---|---|---|
| `limits.max_output_bytes` | 1 GiB ⚠️ ASSUMPTION | Default output cap for the one-shot API |
| `fuzz.seconds_per_target` | 3600 ⚠️ ASSUMPTION | Fuzz duration per target in Phase 4 |
| `fuzz.seed` | 1 | Seed for generated differential inputs |
| `bench.repeats` | 10 ⚠️ ASSUMPTION | Repetitions per benchmark input |

### 5.2 Corpora (fetched in P0-03, then lockdown)

- Canterbury: `https://corpus.canterbury.ac.nz/resources/cantrbry.tar.gz`
- Silesia: `https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip`

⚠️ ASSUMPTION: these URLs still resolve. If one does not, P0-03 blocks and the owner supplies the file by hand, recorded as an intervention. Each download is recorded in `testdata/MANIFEST.json` with URL, SHA-256, size and time.

### 5.3 Similarity controls

The owner prepares the prior art **outside the repository** before the flight:

- the C reference: `lib/` of the `lz4` repository at tag v1.11.0, commit `c72c7c0ec132ad2ce2fe9fad7b0c728d9faee2fc`;
- `lz4_flex` 0.14.0 `src/`, commit `1bffdcbbf906a234b913937cb2f57c0245915038` (independent Rust);
- a `c2rust` 0.22.1 translation of lz4 v1.11.0 (`lz4.c`, `lz4frame.c`, `lz4hc.c`, `xxhash.c`), unedited (derived Rust; the same-language positive control).

The planner writes this block into `docs/kukulu.json` as given, with the owner's absolute paths:

```json
"cleanroom": {
  "similarity": {
    "subject": ["crates/cleanlz4/src"],
    "pairs": [
      { "id": "independent-vs-c", "role": "negative", "left": "/home/ericmann/Projects/prior-art/lz4_flex/src", "right": "/home/ericmann/Projects/prior-art/lz4/lib" },
      { "id": "derived-vs-c",     "role": "positive", "left": "/home/ericmann/Projects/prior-art/c2rust-lz4/src", "right": "/home/ericmann/Projects/prior-art/lz4/lib" },
      { "id": "subject-vs-c",     "role": "test",     "left": "subject", "right": "/home/ericmann/Projects/prior-art/lz4/lib" },
      { "id": "subject-vs-indep", "role": "test",     "left": "subject", "right": "/home/ericmann/Projects/prior-art/lz4_flex/src" }
    ]
  }
}
```

If the `c2rust` translation cannot be produced, the owner says so before the flight. The positive pair is then dropped, and `SIMILARITY.md` states that this run has no same-language positive control.

### 5.4 Result files (the demo's data)

Committed in the same commit as the work that produced them, under `testdata/results/`:

- `conformance-<task-id>.json`:
    - `{ "task", "corpus_sha256", "total", "passed", "failed", "by_category": { "<category>": { "total", "passed" } } }`
    - Categories: `block`, `frame-independent`, `frame-linked`, `checksums`, `content-size`, `dictionary`, `skippable`, `legacy`, `concatenated`, `synthetic`.
- `malformed-<task-id>.json`:
    - `{ "task", "total", "matched_expected_error", "panics" }`.
- `fuzz-<target>-<task-id>.json`:
    - `{ "task", "target", "seconds", "runs", "crashes", "divergences", "corpus_entries", "seed" }`.
- `bench-<task-id>.json`:
    - `{ "task", "inputs": [ { "name", "bytes", "cleanlz4_mb_s", "c_mb_s" } ] }`.

`docs/DESIGN.md` and these files are the only places results live. `SUMMARY.md` quotes them.

## 6. Interfaces

### 6.1 API

- **One-shot:**
    - `decompress_frame(input) -> Result<Vec<u8>>`, which decodes concatenated frames and skips skippable frames;
    - `decompress_block(input, max_output) -> Result<Vec<u8>>`;
    - `decompress_block_with_dict(input, dict, max_output)`.
- **Streaming:** a decoder object that accepts input chunks of any size (1 byte included) and writes into caller-provided buffers. It reports bytes consumed, bytes produced and a status of `NeedsInput`, `OutputFull`, `FrameDone` or `Done`.
- **Options:**
    - an output limit;
    - a preset dictionary for frames that carry a dictionary ID;
    - the trailing-data policy (error or return the remainder);
    - whether to verify checksums (on by default; turning it off is reported in the result).
- **Introspection:** the parsed frame descriptor (block size, independence, flags, content size, dictionary ID) is exposed.

### 6.2 Errors

A typed enum with distinct variants:

- bad magic;
- unsupported version;
- reserved bit set;
- bad header checksum;
- invalid block maximum size;
- truncated input;
- offset of zero;
- offset beyond the window or before the start of output;
- an end-of-block rule violated;
- block checksum mismatch;
- content checksum mismatch;
- content size mismatch;
- dictionary required or mismatched;
- output limit exceeded;
- trailing data.

No stringly-typed errors.

### 6.3 Conformance corpus

Generated by the oracle, deterministically from `fuzz.seed`. Every combination is recorded in `testdata/MANIFEST.json`:

- compression levels across the fast and HC ranges;
- each block maximum size;
- linked and independent blocks;
- block checksum on and off;
- content checksum on and off;
- content size present and absent;
- with and without a dictionary;
- the legacy frame and skippable frames;
- concatenated frames;
- the corpus inputs;
- synthetic inputs: empty, 1 byte, incompressible, all-zeros, highly repetitive, matches at window-boundary distances, minimum and long match lengths, and literal-only blocks.

### 6.4 Malformed corpus and differential rules

Hand-built from the spec's rules:

- a bad magic;
- reserved bits;
- a bad header checksum;
- truncation at every offset of a small valid frame;
- zero offsets;
- offsets past the window or before the output start;
- end-of-block violations;
- wrong checksums;
- a wrong content size.

Each file records its expected error variant. On arbitrary bytes, cleanlz4 and the oracle must agree on accept or reject. Where the spec permits a choice, `DESIGN.md` records the decision and the differential test encodes it.

### 6.5 Oracle C API (the only permitted surface)

From the reference library's public API, called through a Rust binding crate in `crates/oracle` only:

- **Block:** `LZ4_compress_default`, `LZ4_compress_HC`, `LZ4_compressBound`, `LZ4_decompress_safe`.
- **Frame compression:** `LZ4F_createCompressionContext`, `LZ4F_compressBegin`, `LZ4F_compressUpdate`, `LZ4F_compressEnd`, `LZ4F_compressBound`, `LZ4F_freeCompressionContext`.
- **Frame decompression:** `LZ4F_createDecompressionContext`, `LZ4F_decompress`, `LZ4F_freeDecompressionContext`.
- **Errors:** `LZ4F_isError`, `LZ4F_getErrorName`.
- **Preferences:** `LZ4F_preferences_t` with `frameInfo` (`blockSizeID`, `blockMode`, `contentChecksumFlag`, `frameType`, `contentSize`, `dictID`, `blockChecksumFlag`), `compressionLevel` and `autoFlush`.

If the binding crate lacks one of these, the task blocks and names it. It never reads the binding crate's source. Legacy and skippable frames are produced by the `lz4` command-line program (`-l` for legacy) or built by hand from the spec.

**Dictionary oracle:** the `lz4` command-line program, run as a black box (`lz4 -d -D <dict>`), is the oracle for dictionary decoding. To check a raw dictionary block, wrap it in a minimal frame built by hand from the spec. Only `crates/oracle` runs the program, and its version is recorded in `testdata/MANIFEST.json`.

## 7. Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`
- test one: `cargo test --workspace {filter}`
- guard: `node /home/ericmann/.claude/plugins/cache/displacetech/kukulu/0.8.0/scripts/cleanroom-guard.mjs --self-test docs/cleanroom.json`
- deps: a script that fails if `crates/cleanlz4/Cargo.toml` has a non-empty `[dependencies]` or any `unsafe` token appears under `crates/cleanlz4/`
- fuzz (Phase 4 only): `cargo +nightly fuzz run <target> -- -max_total_time=<fuzz.seconds_per_target>`

## 8. Phases

### Phase 0 — Scaffolding and inputs

- **P0-01:** Cargo workspace (`crates/cleanlz4`, `crates/oracle`, `fuzz/`, `testdata/`), CI-equivalent `make check` running §7, and the deps script.
- **P0-02:** Verify every file in `docs/spec/` against `docs/spec/SOURCES.json` (SHA-256 and size). A mismatch blocks.
- **P0-03:** Fetch the corpora in §5.2 into `testdata/corpora/` and record them in `testdata/MANIFEST.json`. **This is the lockdown task** (`lockdown.after` in the policy). No network after it.
- **Manual check:** `make check` is green on empty crates, and the guard self-test passes.

### Phase 1 — XXH32 and the block decoder

- XXH32 against the spec's description, with test vectors produced by the oracle path (frame checksums of known inputs), never copied from an implementation.
- Block decoding, including overlapping matches, the end-of-block rules, and dictionary-prefixed decoding.
- Conformance results for category `block` in `testdata/results/`.
- **Manual check:** a short report in `DESIGN.md` of every spec rule implemented, with its section.

### Phase 2 — Frames

- Frame descriptor parsing, every block size, linked and independent blocks, uncompressed blocks, all checksums, content size, dictionary ID, concatenated, skippable and legacy frames.
- The full conformance corpus (§6.3) and the malformed corpus (§6.4), with their results files.
- **Manual check:** 100% conformance, and every malformed file returns its expected variant without a panic.

### Phase 3 — Streaming and limits

- The streaming decoder and a chunking-invariance property test: random input and output chunk sizes, 1 included, give output identical to one-shot over at least 10,000 cases ⚠️ ASSUMPTION.
- The output limit, checked with a small frame that expands past the limit: it fails fast with bounded memory.

### Phase 4 — Fuzzing and measurement

- Fuzz targets:
    - (a) arbitrary bytes into each entry point, which must never panic;
    - (b) differential: oracle-compressed random input with random preferences, decoded by both sides, which must give identical output;
    - (c) differential accept/reject on arbitrary bytes, with divergences written to `testdata/results/` for triage.
- Run each for `fuzz.seconds_per_target` and write the results files.
- Benchmarks against the oracle on the corpora, written to `bench-<task-id>.json`. Report actuals; there is no target.
- **Manual check:** zero crashes and zero divergences, or every divergence triaged in `DESIGN.md` with its resolution.

### Phase 5 — Evidence and hand-off

- `kukulu_similarity` with the controls in §5.3.
- `kukulu_attest` produces `docs/attestation/`, and `node docs/attestation/verify.mjs docs/attestation --key sha256:<owner's fingerprint>` passes.
- `SUMMARY.md` quotes the results files, the denial counts and verdicts, the interventions, and the similarity scores.
- **Manual check:** the owner runs the verifier from a clean checkout on a second machine.

## 9. Open questions

1. **End-of-block rules on decode.** Should violations be rejected, or accepted where the oracle accepts them? The planner decides from the oracle's observed behavior, without reading its source, and records the decision in `DESIGN.md`.
2. **The binding crate's surface.** If it lacks a §6.5 function, the task blocks. The owner either swaps the binding crate or permits the `lz4` CLI as the oracle for that case. *Resolved (0.2):* the binding crate lacked `LZ4_decompress_safe_usingDict`, and the owner permitted the `lz4` CLI as the dictionary oracle (see §6.5).
3. **Dictionary test inputs.** The oracle generates dictionaries from corpus slices. The planner fixes their sizes as config keys.

## Operator notes (read before starting the flight)

- Say `record intervention: <what you did>` for anything done by hand, and `record verdict: #<seq>: <prior-art attempt | false positive | offline test> — <why>` for each denial, as soon as it happens.
- Do not stage demo denials in this repository. The video's guard demo is recorded in a throwaway copy.
- See `DEMO-CAPTURE.md` (kept outside this repository) for what to preserve during and after the flight.
- Phase 0 records the oracle library's version and the lz4 CLI's version (`lz4 -V`) in `testdata/MANIFEST.json`.
