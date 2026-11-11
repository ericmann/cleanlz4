# cleanlz4 — Specification

Version: 0.4 Status: draft for owner review. Drives the **0.2.1 flight**, on top of the 0.2.0 codebase. Flight slug: `cleanlz4-0-2-1`.

**Changes in 0.4 (2026-11-11):** this version scopes the third flight, a patch release that changes no decoder code. It settles the spec issues the 0.2.0 summary raised (`docs/flights/cleanlz4-0-2-0/SUMMARY.md`, Spec issues). The planner reads that summary for context, not as work to repeat.
- **Header:** names the flight slug, because the title-derived slug is taken by the 0.1.0 archive.
- **§1:** the attestation bundle is read at its archived path, and the PR stays a draft.
- **§4:** `xxh32.rs` imports nothing crate-internal, matching the code and `CLAUDE.md`.
- **§5.1:** drops the stray table header and names the statistic for any before/after measurement.
- **§5.3:** says what to write when Kūkulu takes only one control pair per role.
- **§5.4:** result files carry the flight's version in their task id, and the writer never overwrites one.
- **§8 is rewritten** for this flight. The work it schedules:
  - a no-overwrite guard on the results writer;
  - a strict corpus-manifest check;
  - an assertion that kills the `block.rs` zero-offset mutant without relying on a hang;
  - a re-verification without hour-long fuzz runs, since no decoder code changes;
  - the `0.2.1` changelog entry.
- **§9** resolves Q4 from the 0.2.0 run.
- **Policies:** this flight's `docs/kukulu.json` sets `"policies": { "finalize": "draft" }`, so the summarize stage leaves the PR a draft for the owner's checks and never asks for auto-merge.

**Changes in 0.3 (2026-11-10):** this version scopes the second flight, a hardening release. It builds on 0.1.0 and does not rebuild it.
- **§5.3** adds same-language similarity controls. 0.1.0's "independence not supported" verdict came only from Rust-vs-Rust n-gram metrics, which were judged against Rust-vs-C controls.
- **Spec gaps settled:** it writes in the gaps the first flight had to settle by itself (§3, §5.1, §5.2, §6.1, §6.2, §6.4, §6.5, §7).
- **§8 is rewritten** to list only this flight's phases. The work it schedules:
  - the dictionary-frame differential test;
  - the two surviving mutants;
  - small decoder cleanups;
  - a full re-verification;
  - a `CHANGELOG.md` entry, so the release tool works unaided.
- **§9** marks Q1 and Q3 resolved and adds Q4.

**Changes in 0.2 (2026-11-10):** §6.5 records the owner's §9 Q2 decision from the first flight. The binding crate lacks `LZ4_decompress_safe_usingDict`, so the `lz4` command-line program, run as a black box, is the oracle for dictionary decoding (clean-room record seq 681). That function is removed from the permitted C API, which now has fifteen functions; the oracle never called it. §9 Q2 is marked resolved.

The Kūkulu planner reads this file in full and derives the build plan from it. §1 to §7 describe the whole product, as built in 0.1.0 and 0.2.0 and settled since. §8 lists only the work of this flight. The earlier flights' plans, reviews and summaries are archived in `docs/flights/cleanlz4/` (0.1.0) and `docs/flights/cleanlz4-0-2-0/` (0.2.0), and the planner reads those summaries for context, not as work to repeat. This is a clean-room flight: `docs/cleanroom.json` is committed on the base branch before the first `/kukulu:go-flight`, and every stage runs under it.

## 1. Overview

cleanlz4 is a memory-safe, zero-dependency Rust decoder for the LZ4 block and frame formats, with the XXH32 checksum the frame format uses. It is built clean-room from the three documents in `docs/spec/` alone. It is the second public clean-room rebuild made with Kūkulu, after cleanflate (zlib inflate). It is the first one whose whole provenance lives in a Kūkulu 0.8.x signed bundle that anyone can verify offline.

The artifact's credibility rests on three properties, in this order:

1. **Provenance.** The signed log and bundle show that no existing LZ4 or xxHash implementation was read.
2. **Correctness.** Output is byte-identical to the reference C implementation across the conformance corpus and differential fuzzing, and accept/reject decisions agree on malformed input.
3. **Safety.** `#![forbid(unsafe_code)]`, no reachable panics, bounded memory.

Performance is measured and reported. It is not a gate.

Done means:

- every requirement in §6 has its tests;
- every command in §7 exits zero;
- the fuzz campaign has run for its configured duration with zero crashes, and every divergence is triaged in `DESIGN.md`;
- `kukulu_similarity` has produced `docs/SIMILARITY.md` with the controls in §5.3, and `kukulu_attest` has produced a bundle that passes `verify.mjs`. The bundle is written under `docs/attestation/`, and archiving moves it to `docs/flights/<slug>/attestation/`, where the owner verifies it;
- `CHANGELOG.md` has a numbered entry for the release, and the crate versions match it;
- a draft PR is open, and it stays a draft until the owner's manual checks are done (`policies.finalize: "draft"`).

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
    - the oracle's C function names and preferences fields, exactly as listed in §6.5 (§6.5 lists no signatures: the Rust parameter types are learned from compiler diagnostics alone);
    - the `lz4` command-line program, run as a black box;
    - the corpora in §5.2 (fetched in 0.1.0), as opaque bytes.

    Everything else about LZ4 or xxHash is forbidden. That includes:
    - any source or implementation write-up of LZ4, LZ4 HC or xxHash, in any language;
    - `lz4_flex`, `lz4-sys` internals, `twox-hash`, `xxhash-rust`, `lz-fear` and any other Rust crate in this space;
    - the C headers;
    - articles on how a named implementation decodes fast.

    `docs/cleanroom.json` enforces what a tool-call guard can. The guard is a tripwire, not a sandbox (Kūkulu `docs/cleanroom.md`), so the rule binds even where the guard cannot see.
2. **No unsafe, no dependencies.** `crates/cleanlz4` has `#![forbid(unsafe_code)]` at the crate root, the token `unsafe` appears nowhere under it, and its `[dependencies]` table is empty. Dev-dependencies belong only to `crates/oracle` and `fuzz/`.
3. **No reachable panics.** Library code carries `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing))]`, so unit tests are exempt. No `allow` or `expect` attribute under `crates/cleanlz4/src/` names those lints.
4. **Design cites the spec.** Every non-trivial decision is recorded in `docs/DESIGN.md` with the section of the `docs/spec/` document it implements (for example, "Frame format, Block checksum"). An optimization cites cleanlz4's own profile evidence, never another implementation.
5. **Every tunable is a key.** Every number in §5.1, and every other key of `testdata/config.json`, is a config key with a default, never a bare literal. A tunable a new task introduces is added to `testdata/config.json` and marked ⚠️ ASSUMPTION until it is measured.
6. **Results are committed as data.** Every task that runs conformance or fuzzing writes its counts to the result files in §5.4 in the same commit. The demo's timeline is rebuilt from these files and the chain, so a result that is only printed to a terminal does not exist.
7. **The oracle is a black box.** `crates/oracle` calls the C library only through the functions in §6.5, and only `crates/oracle/src/ffi.rs` declares or calls them. No disassembly, profiling or tracing of the oracle or of `liblz4` (`commandPatterns` in the policy).

    One carve-out: a counting global allocator (`unsafe impl GlobalAlloc` that wraps `std::alloc::System`) may live in `crates/oracle/tests/` and `crates/oracle/examples/` to measure memory. It calls no C function. The "only `ffi.rs` contains `unsafe`" rule applies to everything else.

## 4. Architecture

| Path | Responsibility | May import |
|---|---|---|
| `crates/cleanlz4/src/xxh32.rs` | XXH32, one-shot and streaming | `core`, `std` |
| `crates/cleanlz4/src/block.rs` | Block decoding into a caller-provided window | `core`, `std`, `error` |
| `crates/cleanlz4/src/frame.rs` | Frame parsing, block sequencing, checksums, skippable and legacy frames, the one-shot driver | `block`, `xxh32`, `error` (never `stream`) |
| `crates/cleanlz4/src/stream.rs` | Streaming decoder state machine, built on `frame`'s primitives | `frame`, `block`, `xxh32`, `error` |
| `crates/cleanlz4/src/error.rs` | The typed error enum | — |
| `crates/cleanlz4/src/lib.rs` | Public API, limits, configuration | all of the above |
| `crates/oracle/` | FFI to the C library, corpus generation, the reference side of differential tests | `cleanlz4`, the binding crate |
| `fuzz/` | `cargo-fuzz` targets | `cleanlz4`, `oracle` |
| `testdata/` | Corpora, the generated conformance corpus, the malformed corpus, result files | — |

`crates/cleanlz4` never depends on `crates/oracle`. Nothing under `crates/cleanlz4/` mentions the oracle's binding crate.

## 5. Data and configuration

### 5.1 Tunables

`testdata/config.json` holds every key, and its number is written nowhere else. The defaults below are the values 0.1.0 shipped, and 0.2.0 kept them. This flight changes none of them, and adds only the key named in §8 Phase 1. Keys under `conformance.*` and `diff.*` are generator and harness settings, and `testdata/config.json` holds their values.

| Key | Default | Meaning | Status after 0.1.0 (task ids are 0.1.0's) |
|---|---|---|---|
| `limits.max_output_bytes` | 1 GiB | Default output cap for the one-shot API | Measured (P3-05); kept |
| `fuzz.seconds_per_target` | 3600 | Fuzz duration per target | Measured (P4-12); kept. `diff_roundtrip` was still finding coverage at 93% of its run, and the owner accepts that |
| `fuzz.smoke_seconds` | 20 | Per-target duration of `make fuzz-smoke` | Not tuned |
| `fuzz.seed` | 1 | Seed for generated differential inputs and the conformance corpus | Fixed |
| `fuzz.max_output_bytes` | 16 MiB | Output cap inside fuzz targets | Not tuned |
| `fuzz.max_divergence_files` | 100 | Divergence inputs written per run | Not tuned. Empty the directory before a re-run, or the cap hides new divergences |
| `bench.repeats` | 10 | Repetitions per benchmark input | Measured (P4-11); kept |
| `stream.chunking_cases` | 80,000 | Cases in the chunking-invariance property test | Tuned (P3-06): 392 of 392 shapes hit |
| `dict.sizes_bytes` | [4096, 65536, 131072] | Dictionary sizes in the conformance corpus | A plan choice |

**Measuring a change.** Any before/after benchmark comparison judges the aggregate throughput over all inputs, and its threshold is the run-to-run spread recorded under `DESIGN.md`'s Measurements. A change over the threshold gets one confirming run before any revert. 0.2.0 measured about 3 points of noise between runs on the owner's machine.

### 5.2 Corpora (fetched in 0.1.0; never committed)

- Canterbury: `https://corpus.canterbury.ac.nz/resources/cantrbry.tar.gz`, unpacked into `testdata/corpora/cantrbry/`
- Silesia: `https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip`, unpacked into `testdata/corpora/silesia/`

Both archives and their contents are recorded in `testdata/MANIFEST.json` with URL, SHA-256, size and time. `testdata/corpora/` and the generated `testdata/conformance/` are gitignored. The manifest's hashes stand in for them, and `make corpus` regenerates the conformance corpus deterministically from `fuzz.seed`.

This flight downloads nothing. Its first task checks that the corpora on disk match the manifest (`cargo test -p oracle --test manifest`). If they are missing or do not match, the task blocks, and the owner restores them by hand and records an intervention.

### 5.3 Similarity controls

The owner prepares the prior art **outside the repository** before the flight:

- the C reference: `lib/` of the `lz4` repository at tag v1.11.0, commit `c72c7c0ec132ad2ce2fe9fad7b0c728d9faee2fc`;
- `lz4_flex` 0.14.0 `src/`, commit `1bffdcbbf906a234b913937cb2f57c0245915038` (independent Rust);
- a `c2rust` 0.22.1 translation of lz4 v1.11.0 (`lz4.c`, `lz4frame.c`, `lz4hc.c`, `xxhash.c`), unedited (derived Rust; the positive control for `subject-vs-c`, which is cross-language like that pair).

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

**Why 0.3 adds controls.** Until 0.3, `derived-vs-c` was labelled the same-language positive control, but it compares Rust with C, and so does `independent-vs-c`. Both controls are cross-language, so they calibrate only `subject-vs-c`. `subject-vs-indep` compares Rust with Rust, which shares far more token n-grams. In 0.1.0 its four n-gram metrics scored above the cross-language positive band, while its identifier and comment metrics sat at the negative control. The verdict was "independence not supported". Without a Rust-vs-Rust band, that verdict cannot be read either way.

**Same-language controls (new in 0.3).** The owner prepares two more trees **outside the repository** before the flight:

- **Independent Rust, for the negative control:** `crates/cleanflate/src` of the owner's cleanflate project. It is a clean-room Rust inflate decoder built with the same pipeline and conventions, and shares no code with `lz4_flex`. It shares cleanlz4's idioms, which is the overlap a negative control must absorb. Because it implements a different format, it shares less domain vocabulary than an LZ4 decoder would. If the owner can name a second Rust LZ4 decoder, independent of `lz4_flex`, at a pinned commit, it replaces cleanflate here.
- **Derived Rust, for the positive control:** a copy of `lz4_flex` 0.14.0 `src/` at the commit above with every comment removed, every local variable and private function consistently renamed, and `rustfmt` applied. It is otherwise unedited. It stands for a lightly disguised copy.

Add these pairs to the block above:

```json
{ "id": "rust-independent-vs-indep", "role": "negative", "left": "/home/ericmann/Projects/cleanflate/crates/cleanflate/src", "right": "/home/ericmann/Projects/prior-art/lz4_flex/src" },
{ "id": "rust-derived-vs-indep",     "role": "positive", "left": "/home/ericmann/Projects/prior-art/lz4_flex-disguised/src", "right": "/home/ericmann/Projects/prior-art/lz4_flex/src" }
```

**Reading the result.** Judge `subject-vs-indep` against the two Rust-vs-Rust controls, and `subject-vs-c` against the two Rust-vs-C controls. `SUMMARY.md` says which control band applies to each test pair, and repeats the tool's own verdict verbatim. The verdict is never reworded or overridden.

**When the tool takes one band (new in 0.4).** Kūkulu 0.8.0 refuses a `pairs` list with more than one `negative` or more than one `positive` pair, and every tool then fails to load the config (§9 Q4). If the running Kūkulu still refuses, the planner writes the four pairs 0.2.0 used, and no others:
- `rust-independent-vs-indep` (negative) and `rust-derived-vs-indep` (positive), from the block above;
- `subject-vs-indep` and `subject-vs-c` (test).

`independent-vs-c` and `derived-vs-c` are then left out. `SUMMARY.md` says so, applies the band to `subject-vs-indep` only, and reads `subject-vs-c` against 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`. If the running Kūkulu accepts several pairs per role, the planner writes all six and every pair is scored.

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

**Task ids in result files (new in 0.4).** Task ids restart at `P0-01` every flight, and review-fix ids repeat too (`malformed-R3-02.json` is 0.1.0's). From this flight on, the `TASK` passed to every runner is the release version, a hyphen and the task id, for example `make conformance TASK=0.2.1-P2-01 CATEGORIES=all`, which writes `conformance-0.2.1-P2-01.json` with `"task": "0.2.1-P2-01"`. Earlier flights' files keep their names. The results writer refuses to overwrite an existing file: it exits non-zero and names the path.

## 6. Interfaces

### 6.1 API

- **One-shot:**
    - `decompress_frame(input) -> Result<Vec<u8>>`, which decodes concatenated frames and skips skippable frames;
    - `decompress_block(input, max_output) -> Result<Vec<u8>>`;
    - `decompress_block_with_dict(input, dict, max_output)`.
- **Streaming:** a decoder object that accepts input chunks of any size (1 byte included) and writes into caller-provided buffers. It reports bytes consumed, bytes produced and a status of `NeedsInput`, `OutputFull`, `FrameDone` or `Done`.
- **Options:**
    - an output limit;
    - a preset dictionary, given as `{ bytes, id: Option<u32> }`:
        - A frame whose Dict-ID matches `id` uses it.
        - A dictionary with no `id` is used for any regular frame, because the Frame format allows a dictionary agreed off-band.
        - A Dict-ID with no dictionary is `DictionaryRequired`, and two different ids are `DictionaryMismatch`.
        - Legacy frames never use a dictionary.
    - the trailing-data policy (error or return the remainder);
    - whether to verify checksums (on by default; turning it off is reported in the result).
- **Introspection:** the parsed frame descriptor (block size, independence, flags, content size, dictionary ID) is exposed.
- **Bounded memory:** with output limit `L`, peak heap held by the decoder is at most `L` plus one block maximum size plus 65,536 bytes of history. Memory is measured with the counting allocator in §3.7, as follows:
    - a fresh allocation counts its full size;
    - a reallocation counts only its change in size;
    - a free subtracts.
- **Failure modes of the limit:**
    - A frame whose declared content size exceeds the limit fails at the header with `OutputLimitExceeded`.
    - An allocation failure is also reported as `OutputLimitExceeded`.

### 6.2 Errors

A typed enum with distinct variants:

- bad magic;
- unsupported version;
- reserved bit set;
- bad header checksum;
- invalid block maximum size;
- a block larger than the frame's block maximum size;
- truncated input, including 1 to 3 trailing bytes that begin a known magic number;
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

**`docs/spec/` wins over the oracle.** Where `docs/spec/` says an input is invalid ("must", "shall", "invalid") and the oracle accepts it anyway, cleanlz4 rejects it. A zero offset is one example, and an end-of-block rule the oracle enforces only when it lacks output slack is another. The difference is carried as a named normalization in `crates/oracle/src/diff.rs`, recorded under `DESIGN.md`'s Differential rules with the section it cites. A normalization never makes cleanlz4 accept what the spec forbids.

The normalizations shipped in 0.1.0 are `legacy-frame`, `zero-offset` and `end-of-block-room`. The owner accepts `end-of-block-room` at its current breadth: it covers any `EndOfBlockViolation` that the oracle accepts.

Spec facts the documents leave open, settled in 0.1.0 and kept:
- Legacy-frame blocks are independent. Each is up to 8 MiB, and a zero block size ends the frame. "All blocks full except the last" is not enforced.
- The `lz4` program is v1.9.4. The Frame format describes v1.10.1 behavior for the legacy zero-size end marker, and cleanlz4 follows the document.

### 6.5 Oracle C API (the only permitted surface)

From the reference library's public API, called through a Rust binding crate in `crates/oracle` only:

- **Block:** `LZ4_compress_default`, `LZ4_compress_HC`, `LZ4_compressBound`, `LZ4_decompress_safe`.
- **Frame compression:** `LZ4F_createCompressionContext`, `LZ4F_compressBegin`, `LZ4F_compressUpdate`, `LZ4F_compressEnd`, `LZ4F_compressBound`, `LZ4F_freeCompressionContext`.
- **Frame decompression:** `LZ4F_createDecompressionContext`, `LZ4F_decompress`, `LZ4F_freeDecompressionContext`.
- **Errors:** `LZ4F_isError`, `LZ4F_getErrorName`.
- **Preferences:** `LZ4F_preferences_t` with `frameInfo` (`blockSizeID`, `blockMode`, `contentChecksumFlag`, `frameType`, `contentSize`, `dictID`, `blockChecksumFlag`), `compressionLevel` and `autoFlush`.

If the binding crate lacks one of these, the task blocks and names it. It never reads the binding crate's source. Legacy and skippable frames are produced by the `lz4` command-line program (`-l` for legacy) or built by hand from the spec.

**Dictionary oracle:** the `lz4` command-line program, run as a black box (`lz4 -d -D <dict>`), is the oracle for dictionary decoding. To check a raw dictionary block, wrap it in a minimal frame built by hand from the spec. Only `crates/oracle` runs the program, and its version is recorded in `testdata/MANIFEST.json`.

**What the oracle cannot do**, so it is not asked:
- **No dictionary compressor in the C API.** A dictionary block is made by compressing `[dict, data]` as linked blocks and keeping the second block (0.1.0's P1-10). A frame that uses a dictionary is assembled in `crates/oracle/src/dict.rs` from such blocks (independent) or from one linked compression of `[dict, chunks…]` (linked), under a header built by hand from the spec.
- **No XXH32 function.** XXH32 vectors come only from the oracle's frame content checksums, so seed 0 is the only seed oracle-checked. Other seeds are tested only against the spec's own description.
- **Its frame decoder rejects legacy frames** ("unknown frame type"). The `lz4` program is the oracle for legacy frames.

## 7. Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`
- test one: `cargo test --workspace {filter}`
- guard: `node /home/ericmann/.claude/plugins/cache/displacetech/kukulu/0.8.0/scripts/cleanroom-guard.mjs --self-test docs/cleanroom.json`
- deps: a script that fails if `crates/cleanlz4/Cargo.toml` has a non-empty `[dependencies]` or any `unsafe` token appears under `crates/cleanlz4/`
- fuzz: `make fuzz-run TARGET=<target> TASK=<id>`. This runs `cargo +nightly fuzz run <target>` with `-max_total_time` from `fuzz.seconds_per_target`, plus the runner's own `-seed` (from `fuzz.seed`), `-print_final_stats=1`, a corpus directory and the results writer. `make fuzz-smoke` runs every target for `fuzz.smoke_seconds`.
- data: `make corpus`, `make conformance TASK=<id> CATEGORIES=<list|all>`, `make conformance-check`, `make malformed TASK=<id>`, `make bench TASK=<id>`
- all checks: `make check`

## 8. Phases (the 0.2.1 flight)

**Already built in 0.1.0 and 0.2.0, not to be rebuilt:**
- the workspace, `make check` and the deps and spec-source scripts;
- XXH32, the block, frame and streaming decoders, the output limit, and 0.2.0's one scratch buffer per frame;
- the conformance and malformed corpora and their runners, and the dictionary differential test;
- the five fuzz targets and their runner, and the benchmarks;
- `docs/EVIDENCE.md`.

The summaries and every decision are in `docs/flights/cleanlz4/SUMMARY.md`, `docs/flights/cleanlz4-0-2-0/SUMMARY.md` and `docs/DESIGN.md`.

**This flight changes nothing under `crates/cleanlz4/src/`.** The decoder, the public API and every error variant stay as 0.2.0 shipped them, so the release is `0.2.1`, a patch with no breaking change. New tests may go under `crates/cleanlz4/tests/`. If a task finds that a decoder change is needed, it blocks and names the change; it is never made in this flight. The planner makes this a constraint rule.

Every `TASK` passed to a runner in this flight is `0.2.1-<task-id>` (§5.4).

### Phase 0 — Baseline and lockdown

Phase 0 has three tasks, and **`P0-03` is the lockdown task**: `lockdown.after` in `docs/cleanroom.json` is `P0-03`, and that file is never edited. This flight adds no dependency, so nothing needs the network before or after it. A task that seems to need a new crate blocks instead.

- **P0-01:**
    - check that `testdata/corpora/cantrbry/` and `testdata/corpora/silesia/` exist before running `cargo test -p oracle --test manifest`, because that test passes when the directory is absent;
    - check that the test shows both corpora matching `testdata/MANIFEST.json`;
    - check that `lz4 -V` and the binding crate's version match those the manifest records.
- **P0-02:** `make check` is green, `node scripts/verify-spec-sources.mjs` passes, and `cargo +nightly fuzz build` succeeds offline.
- **P0-03:** run `make corpus`, then `make conformance TASK=0.2.1-P0-03 CATEGORIES=all`. The result must be 2349 of 2349 with the same `corpus_sha256` as 0.2.0's `conformance-P2-01.json`. A result that is only printed does not exist (§3.6), so `make conformance-check` alone is not enough here.
- **Manual check:** `conformance-0.2.1-P0-03.json` matches 0.2.0's `conformance-P2-01.json` in `corpus_sha256`, total, passed and every category.

### Phase 1 — Close the 0.2.0 notes

- **The results writer never overwrites.** The shared writer in `crates/oracle` that every runner uses refuses to write a result file that already exists: it exits non-zero and names the path. Add a test that writes one file into a scratch directory, then shows the second write fails and leaves the first file's bytes unchanged.
- **A strict corpus check.** Add an `#[ignore]` test to `crates/oracle/tests/manifest.rs` that fails when `testdata/corpora/` or either corpus directory is absent, and otherwise checks every file against `testdata/MANIFEST.json` as the existing test does. The existing test keeps passing when the corpora are absent, so `cargo test --workspace` still runs on a machine without them. `CLAUDE.md`'s Commands gain the strict form, `cargo test -p oracle --test manifest -- --ignored`, and later flights' Phase 0 runs that instead of the manual directory check.
- **Kill the zero-offset mutant by assertion.** In 0.2.0's review, deleting the zero-offset check in `crates/cleanlz4/src/block.rs` was caught only because a test hung until its timeout. Add a test under `crates/cleanlz4/tests/` that decodes a minimal block whose first match has offset 0 and asserts the zero-offset error variant.
    - Apply the mutant by hand, see the new test fail, and restore the file. `git diff --exit-code -- crates/cleanlz4/src/` must be clean afterwards.
    - If the mutated decoder never returns, the test must still fail by assertion, not by hanging. It runs the decode on a thread and fails when the thread has not returned within `test.hang_seconds`. That is a new key in `testdata/config.json`, default 10, marked ⚠️ ASSUMPTION.
    - Record the decision in `DESIGN.md` with "Block format, Offset" as its section.
- **Docs agree with the code.** The `xxh32.rs` row of `CLAUDE.md`'s module map and §4 both say `core`, `std`. `docs/EVIDENCE.md` and `README.md` give the attestation bundle's archived path, `docs/flights/<slug>/attestation/`, wherever they name the bundle.
- **Manual check:** read the three new tests, and confirm that the zero-offset test failed by assertion when the mutant was applied (the task's log quotes the failure line).

### Phase 2 — Re-verify

No decoder code changes, so this flight runs no hour-long fuzz campaign; 0.2.0's five one-hour runs stand for the decoder.
- A full conformance run (`conformance-0.2.1-<task-id>.json`, every category). It must give 2349 of 2349.
- A full malformed run (`malformed-0.2.1-<task-id>.json`). It must give 144 of 144 with 0 panics.
- `make fuzz-smoke`. Every target must finish its `fuzz.smoke_seconds` with no crash. It writes no result file, so the task's log quotes each target's final line.
- No benchmark: the decoder is unchanged, and 0.2.0's `bench-P2-09.json` stands.
- **Manual check:** the two result files show the counts above, and the smoke log shows no crash.

### Phase 3 — Evidence and release prep

- Add the `## [0.2.1] - <date>` entry to `CHANGELOG.md`, covering everything this flight changed. Set `version = "0.2.1"` in `crates/cleanlz4/Cargo.toml` and `crates/oracle/Cargo.toml`. The entry must be on the build branch before its PR is merged, because `kukulu_release` reads it at the merge commit.
- Then refresh `docs/EVIDENCE.md`, so it lists every result file the flight added, and refresh `README.md`.
- The summarize stage, not an implement task, runs `kukulu_similarity` with the pairs §5.3 says to write and `kukulu_attest`, and writes `SUMMARY.md`. The planner schedules no implement task for them.
- **Manual check:**
    - the owner reads `docs/SIMILARITY.md` as §5.3 directs;
    - the owner runs `node docs/flights/cleanlz4-0-2-1/attestation/verify.mjs docs/flights/cleanlz4-0-2-1/attestation --key sha256:<owner's fingerprint>` from a clean checkout on a second machine;
    - the owner marks the draft PR ready and merges it.

## 9. Open questions

1. **End-of-block rules on decode.** Should violations be rejected, or accepted where the oracle accepts them? The planner decides from the oracle's observed behavior, without reading its source, and records the decision in `DESIGN.md`. *Resolved (0.1.0, spike P1-06):*
    - cleanlz4 enforces a rule exactly when the oracle rejects that rule's probes;
    - rules 1 and 2 are enforced, rule 3 is not;
    - an empty block is rejected and a block of `0x00` is accepted;
    - §6.4's "`docs/spec/` wins" governs the rest.
2. **The binding crate's surface.** If it lacks a §6.5 function, the task blocks. The owner either swaps the binding crate or permits the `lz4` CLI as the oracle for that case. *Resolved (0.2):* the binding crate lacked `LZ4_decompress_safe_usingDict`, and the owner permitted the `lz4` CLI as the dictionary oracle (see §6.5).
3. **Dictionary test inputs.** The oracle generates dictionaries from corpus slices. The planner fixes their sizes as config keys. *Resolved (0.1.0):* `dict.sizes_bytes`, see §5.1.
4. **How `kukulu_similarity` pairs controls with tests.** Does it calibrate each test pair against all controls, or only against the controls of the same language pairing? The summarizer reads the answer off `docs/SIMILARITY.md`, without reading the tool's source, and reports per §5.3. If the tool pools every control into one band, the owner raises it as Kūkulu feedback, and this flight carries on. *Resolved (0.2.0):* Kūkulu 0.8.0 takes exactly one negative and one positive pair, so a run has one band. 0.2.0 used the Rust-vs-Rust band, and §5.3 says what to write while that holds. It is logged as Kūkulu feedback.

## Operator notes (read before starting the flight)

- Say `record intervention: <what you did>` for anything done by hand, and `record verdict: #<seq>: <prior-art attempt | false positive | offline test> — <why>` for each denial, as soon as it happens.
- Do not stage demo denials in this repository. The video's guard demo is recorded in a throwaway copy.
- See `DEMO-CAPTURE.md` (kept outside this repository) for what to preserve during and after the flight.
- Phase 0 of 0.1.0 recorded the oracle library's version and the lz4 CLI's version (`lz4 -V`) in `testdata/MANIFEST.json`. This flight must run with the same versions. A mismatch blocks Phase 0.

**Before starting the 0.2.1 flight:**
- Merge this spec's PR, then run `/kukulu:go-flight` from a fresh session on an up-to-date `main` (`git switch main && git pull --ff-only`).
- Confirm the §5.3 trees still exist at the paths given. The planner cannot check them, because the guard denies those paths, and `kukulu_similarity` refuses a missing directory.
- Make sure `testdata/corpora/` is present. Never run `git clean -x` here: it deletes the gitignored corpora and the conformance corpus, and this flight cannot download them.
- This flight runs no hour-long fuzz campaign, so the MCP idle timeout (1,800 s) should not be hit. The longest call is a full conformance or malformed run.
- Clean-room log stashes from the ends of earlier flights may be in `git stash list` (`clean-room log (session)`, and two from the 0.2.0 release). Leave them; nothing in this flight needs them.

**After the flight:** the clean-room guard blocks `gh` and `git push` for agents, so the owner merges the PR and pushes `main` by hand. Before asking for `release it`, check out `main` and fast-forward it to `origin/main`, so the release tool tags the right merge commit.
