# cleanlz4 build plan

Derived from docs/SPEC.md v0.1 on 2026-11-09. SPEC.md wins over this file.

## Decisions

- Phases → SPEC §8's six phases (0 to 5), in order, none merged. Inside a phase the order is: oracle wrappers, then the decoder piece, then its oracle tests, then its result file.
- Parallel streams → none; the plan is fully serial. The corpora and the generated conformance corpus are untracked, so a stream worktree could not verify, and after P0-03 nothing may be downloaded into one.
- Flight slug → `cleanlz4` (from SPEC's title). `baseBranch` → `main`, the branch the plan is committed on.
- §9 Q1 (end-of-block rules) → cleanlz4 follows the oracle rule by rule: a rule is enforced exactly when the oracle's `LZ4_decompress_safe` rejects that rule's probes. SPEC §1 property 2 and §6.4 require accept/reject agreement, and the Block format document allows either choice ("a conformant decoder is allowed to reject"). The observation is the bounded spike P1-06 (the planner cannot run the oracle before it is built); P1-05 ships strict (all three rules enforced) and P1-07 aligns it with the spike's table.
- §9 Q2 (binding crate) → `lz4-sys`, the crate the policy's own fixtures name and `deny.packages` leaves out. Its Rust names are learned from compiler diagnostics only. A missing SPEC §6.5 function blocks the task that needs it, naming it (P1-01, P1-02, P1-10).
- §9 Q3 (dictionary sizes) → config key `dict.sizes_bytes`, default `[4096, 65536, 131072]` (below, at and above the 64 KB window).
- Tunables → one file, `testdata/config.json`, read by `crates/oracle/src/config.rs`. Keys: `limits.max_output_bytes` 1073741824, `fuzz.seconds_per_target` 3600, `fuzz.seed` 1, `fuzz.smoke_seconds` 20, `fuzz.max_output_bytes` 16777216, `fuzz.max_divergence_files` 100, `bench.repeats` 10, `stream.chunking_cases` 10000, `dict.sizes_bytes`, `conformance.hc_levels` `[4, 9, 12]`, `conformance.frame_levels` `[0, 4, 9, 12]`, `conformance.silesia_combos_per_file` 8, `conformance.large_input_bytes` 8388608, `conformance.random_block_cases` 2000, `diff.max_output_bytes` 67108864. The library's own default is `cleanlz4::DEFAULT_MAX_OUTPUT_BYTES` in `lib.rs`; a test holds it equal to `limits.max_output_bytes`.
- Phase 3's "at least 10,000 cases ⚠️ ASSUMPTION" → key `stream.chunking_cases`, tuned in P3-06 (it may only go up; 10,000 is SPEC's floor).
- No network after P0-03 → every crate (the binding crate, `sha2`, `serde`, `serde_json`, `libfuzzer-sys`) is declared and downloaded in P0-01; P0-03 ends by writing `.cargo/config.toml` with `[net] offline = true`. The policy fixes the lockdown task id as P0-03, so nothing that downloads can come later.
- Oracle crate dependencies → `lz4-sys`, `sha2`, `serde` (derive), `serde_json`. Benchmarks and property tests are hand-rolled (`std::time::Instant`, a small PRNG in `crates/oracle/src/prng.rs`), so no other crate is needed.
- Corpora and the generated conformance corpus are not committed → `testdata/corpora/` and `testdata/conformance/` are gitignored (Silesia is over 200 MB). `testdata/MANIFEST.json` records every download and every generated entry with its SHA-256, generation is deterministic from `fuzz.seed`, and `corpus_sha256` in a result file is the hash over the manifest's entry hashes. The malformed corpus is small and is committed.
- Where tests live → `crates/cleanlz4` has no dev-dependencies, so its own tests use byte vectors built by hand from the spec text; every test that needs the oracle, SHA-256 or JSON lives in `crates/oracle/tests/`.
- XXH32 vectors → the oracle path only: the content checksum the oracle writes at the end of a frame is `XXH32(content, 0)`. No digest literal appears in any test. Seeds other than 0 cannot be oracle-checked; DESIGN.md says so.
- Error enum → the fifteen bullets of SPEC §6.2 become sixteen variants (`DictionaryRequired` and `DictionaryMismatch` are separate), plus `BlockSizeExceeded` (see Spec issues): `BadMagic`, `UnsupportedVersion`, `ReservedBitSet`, `BadHeaderChecksum`, `InvalidBlockMaxSize`, `TruncatedInput`, `ZeroOffset`, `OffsetOutOfRange`, `EndOfBlockViolation`, `BlockChecksumMismatch`, `ContentChecksumMismatch`, `ContentSizeMismatch`, `DictionaryRequired`, `DictionaryMismatch`, `OutputLimitExceeded`, `TrailingData`, `BlockSizeExceeded`. All are unit variants.
- Frame boundary rule → with 0 bytes left the stream is done (empty input decodes to empty output). With 1 to 3 bytes left that are a prefix of a known magic number: `TruncatedInput`. With bytes that are not a magic number: `BadMagic` when no frame has been decoded yet, else the trailing-data policy (`TrailingData`, or stop and report the remainder).
- Dictionary semantics → a supplied dictionary is `{ bytes, id: Option<u32> }`. Frame has a Dict-ID and no dictionary was supplied: `DictionaryRequired`. Both ids present and different: `DictionaryMismatch`. A dictionary without an id is used for any frame, and a dictionary is used for a frame without a Dict-ID too (Frame format, Dictionary ID: "can also be passed off-band"). Legacy frames never use one.
- `verify_checksums = false` → skips block and content checksums and is reported as `checksums_verified: false`; the one-byte header checksum is always verified, because it guards the layout the decoder is about to trust.
- Legacy frame → blocks are decoded as independent, each up to 8 MiB; a block size of zero ends the frame (the documented Linux Kernel extension is accepted); "all blocks full except the last" is an encoder rule and is not enforced. P2-07 names the one observation that would change the first choice.
- One-shot and streaming → two drivers over the same primitives in `frame.rs` (SPEC §4 lets `stream` import `frame`, not the reverse). The one-shot path arrives in Phase 2 and the streaming state machine in Phase 3, as SPEC's phases say; P3-03 holds them equal.
- Differential rules → `crates/oracle/src/diff.rs` (P2-09) compares cleanlz4 with the oracle and names every normalization; each one is a DESIGN.md entry backed by the spike `docs/spikes/P2-09.md`. cleanlz4 stays strict wherever the Frame format says "must" or "shall".
- Fuzz targets → five: `no_panic_block`, `no_panic_frame`, `no_panic_stream` (SPEC's (a), one per entry point), `diff_roundtrip` (b) and `diff_accept` (c). Each runs for `fuzz.seconds_per_target` as its own task through `kukulu_verify`, because an hour does not fit a foreground shell call.
- Phase 5 → `kukulu_similarity`, `kukulu_attest` and `SUMMARY.md` belong to the summarize stage (the tools refuse while an implement run is armed). Phase 5 here prepares the evidence index they read.
- Conformance categories → one category per entry, by the suite that made it; for oracle-compressed frames of corpus inputs the precedence is `content-size`, then `checksums`, then `frame-linked` or `frame-independent`.

## Conventions

- Commit template: `CLAUDE.md` `## Commit template`, title `<ID>: <title>`, written to a file and committed with `git commit -F <file>`.
- Every task's verification starts with `kukulu_verify` on its Files touched. Shared protocols are in `CLAUDE.md` `## Task protocols`; a task names the ones it follows.
- Spec documents are cited as "Block format, <section>", "Frame format, <section>" and "xxHash spec, <section>"; SPEC sections as "SPEC §n".
- "The invariants" in a task means the `## Constraints` section of `CLAUDE.md`, which binds every file the task touches.

## Phase 0 — Scaffolding and inputs

### P0-01: Cargo workspace, make check, the deps script and the config file

**Goal:** Create the workspace, the fuzz crate, `make check`, the deps script and `testdata/config.json`, with every dependency the flight will ever need downloaded.

**Files touched:** `Cargo.toml`, `Cargo.lock`, `.gitignore`, `Makefile`, `scripts/check-deps.sh`, `scripts/test-check-deps.sh`, `crates/cleanlz4/Cargo.toml`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/src/error.rs`, `crates/cleanlz4/src/xxh32.rs`, `crates/cleanlz4/src/block.rs`, `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/stream.rs`, `crates/oracle/Cargo.toml`, `crates/oracle/src/lib.rs`, `crates/oracle/src/ffi.rs`, `crates/oracle/src/config.rs`, `crates/oracle/src/prng.rs`, `crates/oracle/tests/config.rs`, `testdata/config.json`, `testdata/results/.gitkeep`, `fuzz/Cargo.toml`, `fuzz/Cargo.lock`, `fuzz/fuzz_targets/no_panic_block.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §3.2, §3.3, §3.5, §4, §5.1, §7. The invariants apply.
- Root `Cargo.toml`: a workspace with members `crates/cleanlz4` and `crates/oracle`, `exclude = ["fuzz"]`, `resolver = "2"`, `[profile.dev] opt-level = 1` and `[profile.test] opt-level = 2` (overflow checks stay on).
- `crates/cleanlz4/Cargo.toml`: package `cleanlz4`, edition 2021, an empty `[dependencies]` table and no other dependency table. `lib.rs` starts with `#![forbid(unsafe_code)]` and `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing))]` (tests are exempt, SPEC §3.3), declares the five modules, and defines `pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 1 << 30;` documented as config key `limits.max_output_bytes`. The five module files hold one `//!` line each.
- `crates/oracle/Cargo.toml`: package `oracle`, `publish = false`, dependencies `cleanlz4` (path), `lz4-sys = "1"`, `sha2 = "0.10"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`. `ffi.rs` holds `pub const BINDING_CRATE: &str = "lz4-sys";` and is the only file that may name the binding crate (constraint `binding-crate-only-in-ffi-module`).
- `testdata/config.json` holds exactly the keys and defaults listed under Decisions, "Tunables". `config.rs` deserializes it with `serde` (`deny_unknown_fields`) into a `Config` struct, found through `env!("CARGO_MANIFEST_DIR")`, and exposes `Config::load()` and `repo_root()`. No tunable's value is written anywhere else.
- `prng.rs`: a small deterministic generator (`Prng::new(seed)`, `next_u64`, `below(n)`, `fill(&mut [u8])`); any well-known 64-bit mixing generator will do.
- `fuzz/Cargo.toml`: its own `[workspace]`, `[package.metadata] cargo-fuzz = true`, dependencies `libfuzzer-sys = "0.4"`, `cleanlz4` and `oracle` by path, one `[[bin]]` `no_panic_block` whose body is `fuzz_target!(|data: &[u8]| { let _ = data; });`.
- `scripts/check-deps.sh [root]` exits non-zero when `crates/cleanlz4/Cargo.toml` has any entry in `[dependencies]` or any other dependency table, when the word `unsafe` (`grep -rnw`) appears under `crates/cleanlz4/`, or when `lib.rs` lacks `#![forbid(unsafe_code)]`. `scripts/test-check-deps.sh` builds four temporary trees (clean, a dependency line, an `unsafe` token, a missing forbid) and checks the script's exit status on each.
- `Makefile` target `check` runs SPEC §7's fmt, lint, test, guard and deps commands in that order, then `bash scripts/test-check-deps.sh`.
- `.gitignore` gains `testdata/corpora/`, `testdata/conformance/` and `fuzz/logs/`.
- `docs/DESIGN.md` is created with the headings `## Decisions`, `## Spec rules implemented`, `## Differential rules`, `## Divergence triage` and `## Measurements`, and one Decisions entry for the lint attribute's `cfg_attr` form.
- Run `cargo build --workspace` and `cargo +nightly fuzz build` so both lockfiles exist and every crate is in the local cache. Never run `cargo add` for a crate in the policy's `deny.packages`. If the binding crate cannot be resolved or built, block and say so.

**Acceptance tests:** `crates/oracle/tests/config.rs`: `config_file_parses_with_every_key`, `limits_default_matches_library_constant`, `tunable_values_are_positive` (asserts presence and positivity, never a default's value); `prng.rs` unit test `same_seed_gives_same_sequence`; `scripts/test-check-deps.sh` exits zero.

**Out of scope:** any decoder or oracle logic; downloading the corpora; `.cargo/config.toml`.

**Verification:** `kukulu_verify` with the Files touched (this runs the fuzz build too); `make check` exits zero.

**Depends on:** none

### P0-02: Verify docs/spec against SOURCES.json

**Goal:** Prove every file in `docs/spec/` is the pinned byte copy, and keep proving it on every later task.

**Files touched:** `scripts/verify-spec-sources.mjs`, `scripts/test-verify-spec-sources.mjs`, `Makefile`

**Design constraints:**

- SPEC §8 Phase 0 (P0-02), §3.1. `docs/spec/` is protected: read it, never write it.
- `node scripts/verify-spec-sources.mjs [--root <dir>]` reads `docs/spec/SOURCES.json` and, for each entry, checks that the file exists, that its size equals `bytes` and that its SHA-256 equals `sha256` (Node's `crypto`, no packages). It also fails when a file other than `README.md` and `SOURCES.json` sits in `docs/spec/` without an entry. It prints one line per failure and exits 1, or `spec sources: ok (<n> files)` and exits 0.
- The test script copies `docs/spec/` to a temporary directory, checks the clean copy passes, flips one byte and checks the exit status is 1, then adds a stray file and checks again.
- `Makefile`'s `check` gains both scripts. A real mismatch on the repository's own files blocks this task: call `kukulu_task_block` naming the file and both hashes. Do not repair the file.

**Acceptance tests:** `node scripts/test-verify-spec-sources.mjs` exits zero and prints the three cases (`clean`, `tampered byte`, `stray file`).

**Out of scope:** fetching anything; changing `SOURCES.json`.

**Verification:** `kukulu_verify` (from this task on it runs `node scripts/verify-spec-sources.mjs`); `make check`.

**Depends on:** P0-01

### P0-03: Fetch the corpora, write MANIFEST.json and go offline

**Goal:** Download the two corpora, record them and the tool versions in `testdata/MANIFEST.json`, and switch cargo offline; this is the lockdown task.

**Files touched:** `crates/oracle/src/manifest.rs`, `crates/oracle/src/bin/manifest.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/manifest.rs`, `testdata/MANIFEST.json`, `.cargo/config.toml`, `Makefile`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.2, §8 Phase 0 (P0-03), Operator notes (last bullet). Follow the clean-room protocol in CLAUDE.md.
- Download with two direct tool calls, never from inside a script, so each is logged with its URL: `curl -fL --retry 2 -o testdata/corpora/cantrbry.tar.gz https://corpus.canterbury.ac.nz/resources/cantrbry.tar.gz` and `curl -fL --retry 2 -o testdata/corpora/silesia.zip https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip`. These two URLs are the only ones the policy allows. If either fails, call `kukulu_task_block` naming the URL (SPEC §5.2: the owner supplies the file by hand); do not look for a mirror.
- Extract with `tar -xzf testdata/corpora/cantrbry.tar.gz -C testdata/corpora/cantrbry` and `unzip -q -o testdata/corpora/silesia.zip -d testdata/corpora/silesia` (create the directories first). Corpus files are opaque bytes. `testdata/corpora/` is gitignored and is never committed.
- `cargo run -p oracle --bin manifest -- corpora --fetched-at <UTC ISO-8601 time of the downloads>` writes `testdata/MANIFEST.json`: `{ "version": 1, "tools": { "oracle_binding": { "crate", "version" }, "lz4_cli", "rustc", "cargo", "cargo_fuzz" }, "corpora": [ { "name", "url", "sha256", "bytes", "fetched_at", "files": [ { "name", "sha256", "bytes" } ] } ], "conformance": null }`. `oracle_binding.version` is read from `Cargo.lock` for `ffi::BINDING_CRATE` (this is the oracle library's version: the surface in SPEC §6.5 has no version function); `lz4_cli` is the first line `lz4 -V` prints. File lists are sorted by name; output is pretty-printed with a trailing newline.
- `Makefile` gains `manifest-corpora`. DESIGN.md records the tool versions and that the installed CLI and the binding crate's library may differ in version.
- Last, after the manifest is written: run `cargo fetch` and `cargo +nightly fetch --manifest-path fuzz/Cargo.toml`, write `.cargo/config.toml` containing `[net]` and `offline = true`, and confirm `cargo build --workspace` and `cargo +nightly fuzz build` still pass. From this commit on nothing is downloaded, by any tool.

**Acceptance tests:** `crates/oracle/tests/manifest.rs`: `manifest_lists_both_corpora_with_hashes` (two entries, the two SPEC URLs, 64-hex hashes, non-zero sizes, non-empty file lists), `manifest_records_tool_versions`, `corpus_files_on_disk_match_manifest` (checks every listed file when `testdata/corpora/` exists; prints a notice and passes when it does not); `manifest.rs` unit tests `sha256_hex_of_known_bytes` and `manifest_round_trips_through_json`.

**Out of scope:** the `conformance` section (P1-11); committing corpus files; any other URL.

**Verification:** `kukulu_verify`. The log entry carries `Lockdown: network off from this commit` and both archives' SHA-256.

**Depends on:** P0-01

### P0-04: Phase 0 end: push and record the manual check

**Goal:** Push the branch and record Phase 0's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 0. Follow the phase-end protocol in CLAUDE.md. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): make check is green on the empty crates; the guard self-test passes; testdata/MANIFEST.json names both corpora, the binding crate's version and the lz4 CLI's version; .cargo/config.toml sets net.offline`.

**Depends on:** none

## Phase 1 — XXH32 and the block decoder

### P1-01: Oracle FFI: the block functions

**Goal:** Wrap the oracle's four block functions in safe Rust.

**Files touched:** `crates/oracle/src/ffi.rs`, `crates/oracle/src/block.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/ffi_block.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §3.7, §6.5 (Block). Follow the oracle protocol in CLAUDE.md: `ffi.rs` is the only file with `unsafe` blocks and the only one naming the binding crate, and it calls only `LZ4_compress_default`, `LZ4_compress_HC`, `LZ4_compressBound` and `LZ4_decompress_safe` here.
- `block.rs` (the oracle crate's, not cleanlz4's) exposes `compress_default(src: &[u8]) -> Vec<u8>`, `compress_hc(src: &[u8], level: i32) -> Vec<u8>`, `compress_bound(len: usize) -> usize`, and `decompress_safe(src: &[u8], capacity: usize) -> Result<Vec<u8>, Rejected>` where `Rejected` is a unit struct returned when the C function reports an error. Lengths that do not fit a C `int` panic with a clear message (this is test tooling, not the library).
- If the binding crate lacks one of the four functions, block naming it (SPEC §9 Q2).

**Acceptance tests:** `crates/oracle/tests/ffi_block.rs`: `roundtrip_default_and_hc`, `decompress_safe_rejects_when_capacity_is_too_small`, `compress_bound_is_at_least_the_input_length`, `empty_input_roundtrips`.

**Out of scope:** the frame functions (P1-02); `LZ4_decompress_safe_usingDict` (P1-10); anything in `crates/cleanlz4`.

**Verification:** `kukulu_verify`.

**Depends on:** P0-01

### P1-02: Oracle FFI: frame compression and decompression

**Goal:** Wrap the oracle's frame API so tests can compress with any preferences and decode any byte string.

**Files touched:** `crates/oracle/src/ffi.rs`, `crates/oracle/src/frame.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/ffi_frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §3.7, §6.5 (Frame compression, Frame decompression, Errors, Preferences). Follow the oracle protocol in CLAUDE.md. A missing function or preferences field blocks, naming it.
- `frame.rs` (the oracle crate's) exposes `Prefs { block_size_id: u8, linked: bool, content_checksum: bool, block_checksum: bool, content_size: Option<u64>, dict_id: u32, level: i32, auto_flush: bool }` (`block_size_id` 4 to 7, or 0 for the library default), and `compress_frame(chunks: &[&[u8]], prefs: &Prefs) -> Vec<u8>`: one `LZ4F_compressBegin`, one `LZ4F_compressUpdate` per chunk with a destination of `LZ4F_compressBound(chunk.len(), prefs)` bytes, one `LZ4F_compressEnd`.
- `decompress_frames(input: &[u8], cap: usize) -> Decoded` with `Decoded::{ Accepted(Vec<u8>), Rejected(String), OverCap }`: loop while input remains; for each frame use a fresh decompression context and call `LZ4F_decompress` until it returns 0; an error code gives `Rejected(<LZ4F_getErrorName text>)`; input exhausted while the last return value was not 0 gives `Rejected("truncated")`; a call that consumes nothing and produces nothing with input left gives `Rejected("stalled")`; more than `cap` output bytes gives `OverCap`. Empty input is `Accepted(vec![])`.
- `content_checksum_of(data: &[u8]) -> u32`: compress `data` with the content checksum on and return the frame's last four bytes as a little-endian value (Frame format, Content Checksum). This is the flight's only source of XXH32 vectors.
- The context-creation version argument: use the constant the binding crate exports for it if the compiler shows one; otherwise pass 100 and confirm `LZ4F_isError` is false, recording that as an observed black-box fact in DESIGN.md.

**Acceptance tests:** `crates/oracle/tests/ffi_frame.rs`: `frame_roundtrip_every_block_size`, `frame_roundtrip_linked_with_both_checksums`, `content_size_adds_eight_header_bytes`, `decompress_frames_handles_two_concatenated_frames`, `decompress_frames_rejects_every_truncation_of_a_small_frame`, `empty_input_is_accepted_as_empty`, `content_checksum_of_changes_with_the_input`.

**Out of scope:** dictionary compression (not on the permitted surface); anything in `crates/cleanlz4`.

**Verification:** `kukulu_verify`.

**Depends on:** P1-01

### P1-03: The typed error enum

**Goal:** Define the public error type every later task returns.

**Files touched:** `crates/cleanlz4/src/error.rs`, `crates/cleanlz4/src/lib.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.2. The invariants apply.
- `pub enum Error` with exactly these seventeen unit variants, deriving `Debug, Clone, Copy, PartialEq, Eq`: `BadMagic`, `UnsupportedVersion`, `ReservedBitSet`, `BadHeaderChecksum`, `InvalidBlockMaxSize`, `TruncatedInput`, `ZeroOffset`, `OffsetOutOfRange`, `EndOfBlockViolation`, `BlockChecksumMismatch`, `ContentChecksumMismatch`, `ContentSizeMismatch`, `DictionaryRequired`, `DictionaryMismatch`, `OutputLimitExceeded`, `TrailingData`, `BlockSizeExceeded`. No variant carries a string.
- `pub const ALL: [Error; 17]`, `impl core::fmt::Display` (one distinct sentence per variant), `impl std::error::Error`, and `pub type Result<T> = core::result::Result<T, Error>;`. `lib.rs` re-exports `Error` and `Result`.
- DESIGN.md records why `BlockSizeExceeded` exists (Frame format, Data Blocks: "Block_Size shall never be larger than Block_Maximum_Size").

**Acceptance tests:** unit tests in `error.rs`: `all_lists_every_variant_once`, `every_variant_has_a_distinct_display_message`, `error_implements_std_error`.

**Out of scope:** returning these errors from any decoder.

**Verification:** `kukulu_verify`.

**Depends on:** P0-01

### P1-04: XXH32, one-shot and streaming

**Goal:** Implement XXH32 from the xxHash spec and check it against the oracle's frame checksums.

**Files touched:** `crates/cleanlz4/src/xxh32.rs`, `crates/cleanlz4/src/lib.rs`, `crates/oracle/tests/xxh32.rs`, `docs/DESIGN.md`

**Design constraints:**

- xxHash spec, "XXH32 Algorithm Description", Steps 1 to 7, and "Operation notations". XXH64 and XXH3 are not built (SPEC §2). The invariants apply; `xxh32.rs` imports `core`/`std` only.
- `pub fn xxh32(input: &[u8], seed: u32) -> u32` and `pub struct Xxh32` with `new(seed: u32)`, `update(&mut self, input: &[u8])` and `digest(&self) -> u32`. The streaming form keeps a 16-byte stripe buffer and a 64-bit total length, of which Step 4 adds the low 32 bits.
- Every addition and multiplication is wrapping (`wrapping_add`, `wrapping_mul`, `rotate_left`); tests run with overflow checks on. No indexing: read lanes with `chunks_exact` and `u32::from_le_bytes` on arrays obtained without `[]`.
- No digest literal appears in any test (SPEC §8 Phase 1: vectors come from the oracle path, never from an implementation). Only seed 0 can be oracle-checked; say so in DESIGN.md.
- DESIGN.md gets one entry per Step, citing it.

**Acceptance tests:** unit tests in `xxh32.rs`: `streaming_equals_one_shot_for_every_split` (lengths 0 to 64, every split point, and byte-at-a-time), `seed_changes_the_digest`, `lengths_around_stripe_boundaries_differ`. `crates/oracle/tests/xxh32.rs`: `matches_oracle_content_checksum_for_lengths_0_to_300`, `matches_oracle_for_large_random_inputs` (sizes from 1 KiB to 1 MiB + 3, bytes from `Prng` seeded with `fuzz.seed`), `streaming_in_odd_chunks_matches_oracle`, `header_checksum_byte_matches_oracle_frames` (Frame format, Header Checksum, on a 7-byte oracle header), `block_checksum_matches_oracle_frames` (Frame format, Block checksum).

**Out of scope:** XXH64, XXH3; frame parsing beyond the two fixed-offset reads the last two tests need.

**Verification:** `kukulu_verify`.

**Depends on:** P1-02

### P1-05: Block decoder: sequences, overlap copies and strict end-of-block rules

**Goal:** Decode an LZ4 block into a growing output buffer with every bound checked and all three end-of-block rules enforced.

**Files touched:** `crates/cleanlz4/src/block.rs`, `crates/cleanlz4/src/lib.rs`, `docs/DESIGN.md`

**Design constraints:**

- Block format, "Compressed block format", "End of block conditions", and "Implementation notes" (Large lengths, Safe decoding). The invariants apply; `block.rs` may import only `crate::error`.
- `pub(crate) fn decode_block(src: &[u8], dst: &mut Vec<u8>, max_len: usize) -> Result<()>` appends to `dst`. A match offset resolves against the bytes already in `dst`, including bytes that were there before the call (the frame layer uses that for linked blocks).
- Errors: offset 0 → `ZeroOffset`; offset greater than `dst.len()` at that moment → `OffsetOutOfRange`; `src` ending inside length bytes, literals or the offset → `TruncatedInput`; empty `src` → `TruncatedInput`; `dst.len()` about to exceed `max_len` → `OutputLimitExceeded`, checked before any byte is copied and without reserving more than is about to be written; a length sum that overflows `usize` (`checked_add`) → `OutputLimitExceeded`.
- Match bytes are copied one at a time in increasing order, so a match longer than its offset repeats bytes it just wrote (Block format: "bytes just copied are copied again").
- End of block, all enforced here, each behind one field of a private `const EOB: EndOfBlockPolicy` (three `bool`s, all `true`): rule 1, `src` ends right after a match → `EndOfBlockViolation`; rule 2, a block with at least one match whose final literal run is shorter than 5 → `EndOfBlockViolation`; rule 3, a block with at least one match where fewer than 12 bytes of this block's output lie at or after the position its last match's copy begins → `EndOfBlockViolation`. A block of literals only is valid at any length, including zero literals. The low four bits of the final token are ignored (the document sets no requirement); record that as an Interpretation.
- `pub fn decompress_block(input: &[u8], max_output: usize) -> Result<Vec<u8>>` in `lib.rs` calls it with an empty `dst`.
- DESIGN.md gets one entry per rule, citing its section.

**Acceptance tests:** unit tests in `block.rs`, with blocks built by a test-only helper from the document's text: `literal_only_block`, `single_zero_token_is_the_empty_block`, `literal_length_examples_48_280_and_15` (the document's three examples), `match_copies_from_history`, `match_reads_bytes_present_before_the_call`, `overlapping_match_with_offset_1_repeats_the_last_byte`, `match_length_extension_bytes`, `zero_offset_is_rejected`, `offset_before_output_start_is_rejected`, `truncation_in_length_literals_and_offset_is_rejected`, `output_limit_is_enforced_before_copying`, `rule_1_block_ending_after_a_match`, `rule_2_fewer_than_five_final_literals`, `rule_3_last_match_within_twelve_bytes_of_the_end`.

**Out of scope:** dictionaries (P1-08); relaxing any end-of-block rule (P1-07); frames.

**Verification:** `kukulu_verify`.

**Depends on:** P1-03

### P1-06: Spike: the oracle's end-of-block behaviour

**Goal:** Observe, as a black box, which end-of-block violations the oracle's block decoder rejects, and write the result down.

**Files touched:** `crates/oracle/src/craft.rs`, `crates/oracle/src/bin/probe_eob.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/craft.rs`, `docs/spikes/P1-06.md`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §9 Q1, §3.7. Follow the spike protocol and the oracle protocol in CLAUDE.md. Time box: 60 minutes of work after the builder's tests pass.
- Question: for each probe below, does `oracle::block::decompress_safe` accept it, with a capacity equal to the expected decoded length and with that length plus 65,536?
- `craft.rs`: `Seq { literals: Vec<u8>, offset: u16, match_len: usize }` and `build_block(seqs: &[Seq], final_literals: &[u8], final_low_nibble: u8) -> Vec<u8>`, encoding exactly as Block format, "Compressed block format" describes. It can also emit a block with no final sequence.
- Probes: E0 empty input; E1 the single byte `0x00`; R1 a block ending right after a match; R2 a match followed by a final run of 0, 1, 2, 3 and 4 literals (5 is the control); R3 a last match whose copy begins 5, 6, 7, 8, 9, 10 and 11 bytes before the end of the output (12 is the control); N1 a literal-only block whose final token has a non-zero low nibble; L1 blocks with one match decoding to 11 and 12 bytes in total.
- `cargo run -p oracle --bin probe_eob` prints the table. `docs/spikes/P1-06.md` holds the question, the method, the table (probe, bytes in hex, verdict at exact capacity, verdict with slack) and a `## Policy` section: for each of rule 1, rule 2, rule 3, the empty input and the final nibble, "enforce" when the oracle rejected every probe of that rule with slack capacity, else "do not enforce", and the exact shapes where the two capacities disagree, if any. DESIGN.md links to it.

**Acceptance tests:** `crates/oracle/tests/craft.rs`: `crafted_valid_block_decodes_with_the_oracle`, `crafted_length_extensions_roundtrip`, `crafted_overlap_match_decodes_with_the_oracle`.

**Out of scope:** changing `crates/cleanlz4`; any probe that needs a tool the policy's `commandPatterns` deny.

**Verification:** `kukulu_verify`; `docs/spikes/P1-06.md` exists with the table and the Policy section.

**Depends on:** P1-01

### P1-07: Align the end-of-block policy with the oracle

**Goal:** Set each end-of-block rule to what the spike observed, and prove agreement on crafted, truncated and corrupted blocks.

**Files touched:** `crates/cleanlz4/src/block.rs`, `crates/oracle/tests/eob.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §9 Q1, §6.4; Block format, "End of block conditions". Read the `## Policy` section of `docs/spikes/P1-06.md` first. The invariants apply.
- Decision rule (fixed by the plan): a rule is enforced exactly when the Policy section says "enforce". Set the three `EOB` fields accordingly; make empty input return `Ok` only if the Policy says the oracle accepts it; keep ignoring the final token's low nibble unless the Policy says the oracle rejects it, in which case a non-zero nibble is `EndOfBlockViolation`.
- If the oracle's behaviour needs a further condition, express it in the document's own quantities (literal counts, match positions, block length) and name it in DESIGN.md. If three attempts cannot reach agreement on every probe, block and list the disagreeing probes.
- Update P1-05's three rule tests so a rule the oracle does not enforce now expects `Ok` with the right bytes. DESIGN.md records each rule's decision, citing the document section and the spike.

**Acceptance tests:** `crates/oracle/tests/eob.rs`: `end_of_block_probes_agree_with_the_oracle` (every P1-06 probe, slack capacity: cleanlz4's accept/reject equals the oracle's, and outputs are equal when both accept), `every_prefix_of_small_oracle_blocks_agrees` (three inline inputs under 200 bytes, compressed by the oracle; every prefix), `single_byte_corruptions_of_small_oracle_blocks_agree` (every position, three replacement values). In the last two, both sides get the original length plus 65,536 as their capacity. The unit tests of P1-05 pass in their updated form.

**Out of scope:** dictionaries; frames.

**Verification:** `kukulu_verify`.

**Depends on:** P1-05, P1-06

### P1-08: Dictionary-prefixed block decoding

**Goal:** Let a block's matches reach into a caller-supplied dictionary that sits just before the output.

**Files touched:** `crates/cleanlz4/src/block.rs`, `crates/cleanlz4/src/lib.rs`, `docs/DESIGN.md`

**Design constraints:**

- Block format, "Compressed block format" (the offset) and "Safe decoding"; Frame format, "Dictionary ID" ("a kind of known prefix"); SPEC §6.1. The invariants apply.
- The signature becomes `decode_block(src: &[u8], dict: &[u8], dst: &mut Vec<u8>, max_len: usize) -> Result<()>`: an offset that reaches before `dst[0]` continues into the tail of `dict`, whose last byte is the byte just before `dst[0]`. An offset beyond `dst.len() + dict.len()` → `OffsetOutOfRange`. A match may start in the dictionary and run on into `dst`, including into bytes it is writing.
- No explicit trimming is needed: an offset is at most 65,535, so only the dictionary's last 65,535 bytes can ever be reached.
- `pub fn decompress_block_with_dict(input: &[u8], dict: &[u8], max_output: usize) -> Result<Vec<u8>>`; `decompress_block` passes an empty dictionary.

**Acceptance tests:** unit tests in `block.rs`: `match_reaches_into_the_dictionary_tail`, `match_starts_in_the_dictionary_and_continues_into_output`, `offset_past_the_dictionary_start_is_rejected`, `empty_dictionary_behaves_like_no_dictionary`, `only_the_reachable_tail_of_a_long_dictionary_matters`.

**Out of scope:** oracle-made dictionary blocks (P1-10); frame dictionaries (P2-08).

**Verification:** `kukulu_verify`.

**Depends on:** P1-05

### P1-09: Synthetic inputs and block round-trip tests against the oracle

**Goal:** Generate SPEC §6.3's synthetic inputs deterministically and prove cleanlz4 decodes the oracle's blocks of them byte for byte.

**Files touched:** `crates/oracle/src/synth.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/block.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.3 (synthetic inputs), §5.1 (`fuzz.seed`), §1 property 2.
- `synth::inputs(seed: u64) -> Vec<(String, Vec<u8>)>`, every byte a function of `seed` through `Prng`: `empty`; `one-byte`; `tiny-2` to `tiny-12` (literal-only sizes); `incompressible-64k` (65,553 random bytes); `incompressible-1m`; `zeros-1m`; `repetitive-text` (one 37-byte phrase repeated to 300,000 bytes); `window-boundary` (a 512-byte random marker repeated at distances 65,534, 65,535, 65,536 and 65,537 with random filler between); `min-match` (4-byte repeats between random bytes); `long-match` (a 300-byte unit repeated to 2 MiB). Also `synth::word_salad(seed, len)` for compressible text of any length.
- Compression levels come from `conformance.hc_levels`; the random case count from `conformance.random_block_cases`. Neither number is written in the test.

**Acceptance tests:** `crates/oracle/tests/block.rs`: `synthetic_inputs_roundtrip_default_and_hc` (for each input and each of default plus every HC level, `cleanlz4::decompress_block(block, len)` equals the input, and so does the oracle), `random_blocks_roundtrip` (sizes 0 to 70,000, mixed compressibility), `exact_capacity_is_enough_and_one_less_is_rejected`; `synth.rs` unit test `inputs_are_deterministic_and_named_uniquely`.

**Out of scope:** corpus files (P1-11); frames.

**Verification:** `kukulu_verify`.

**Depends on:** P1-01, P1-05

### P1-10: Oracle dictionary blocks and the dictionary differential test

**Goal:** Produce blocks that depend on a dictionary using only the permitted oracle surface, and check cleanlz4 decodes them as the oracle does.

**Files touched:** `crates/oracle/src/ffi.rs`, `crates/oracle/src/block.rs`, `crates/oracle/src/dict.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/dict.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.5 (`LZ4_decompress_safe_usingDict`), §9 Q3; Frame format, "Block Independence flag", "Data Blocks". Follow the oracle protocol; a missing `LZ4_decompress_safe_usingDict` blocks, naming it.
- `block::decompress_safe_using_dict(src, dict, capacity) -> Result<Vec<u8>, Rejected>`.
- The permitted surface has no dictionary compressor, so `dict::dict_block(dict: &[u8], data: &[u8]) -> Option<DictBlock>` uses linked blocks: `compress_frame(&[dict, data], &Prefs { block_size_id: 7, linked: true, auto_flush: true, checksums off, no content size, level 0 })` makes the first block from `dict` and the second from `data`, and the second may refer back into the first. Walk the frame by the Frame format: a 7-byte header, then each block as a 4-byte little-endian size whose top bit marks an uncompressed block. Return the second block's bytes (or `None` if it is uncompressed) with `dict_dependent`, true when the oracle's plain `decompress_safe` rejects the block or gives other bytes. `dict` and `data` must each fit one 4 MiB block.
- Dictionary sizes come from `dict.sizes_bytes`. Record the method in DESIGN.md, citing the two Frame format sections.

**Acceptance tests:** `crates/oracle/tests/dict.rs`: `dict_block_decodes_with_the_oracle_using_the_dictionary`, `cleanlz4_matches_the_oracle_for_every_configured_dictionary_size`, `at_least_one_block_per_size_is_dictionary_dependent`, `wrong_dictionary_never_panics_and_agrees_with_the_oracle` (same accept/reject, same bytes when both accept).

**Out of scope:** frames with a Dict-ID (P2-08); corpus slices (P2-11).

**Verification:** `kukulu_verify`.

**Depends on:** P1-02, P1-08, P1-09

### P1-11: Conformance generator: the block suite and the manifest's entries

**Goal:** Generate the block part of the conformance corpus deterministically and record every entry in `testdata/MANIFEST.json`.

**Files touched:** `crates/oracle/src/corpus.rs`, `crates/oracle/src/manifest.rs`, `crates/oracle/src/bin/gen_corpus.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/corpus.rs`, `Makefile`, `testdata/MANIFEST.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.3, §5.4, §3.6. Corpus files are opaque bytes.
- Inputs: every `synth::inputs(fuzz.seed)` entry and every corpus file the manifest's `corpora` section lists. A corpus file missing from disk is an error that names it.
- Block suite: each input compressed whole with `compress_default` and with `compress_hc` at each `conformance.hc_levels` level; an input larger than `conformance.large_input_bytes` gets the default and the first HC level only.
- Files go to `testdata/conformance/<suite>/…` (gitignored). The manifest's `conformance` section becomes `{ "seed", "corpus_sha256", "entries": [ { "id", "suite", "category", "kind", "tool", "input", "input_sha256", "input_bytes", "params", "file", "file_sha256", "file_bytes" } ] }`, with `id` = `<suite>/<input>/<params slug>`, `kind` `"block"`, `tool` `"lz4"`, `category` `"block"`, entries sorted by `id`, and no timestamp. `corpus_sha256` is the SHA-256 of every `id`, a newline, `file_sha256`, a newline, in that order.
- `cargo run --release -p oracle --bin gen_corpus -- --suites <list|all>` regenerates files and rewrites the section, keeping other suites' entries; `make corpus` runs it for all suites that exist.

**Acceptance tests:** `crates/oracle/tests/corpus.rs`: `generation_is_deterministic` (the synthetic part generated twice into temporary directories gives equal hashes), `manifest_entries_are_sorted_and_unique`, `block_suite_covers_every_input_and_level`, `corpus_sha256_matches_its_definition`.

**Out of scope:** frame suites (P2-10, P2-11); decoding (P1-12).

**Verification:** `kukulu_verify`; `make corpus` exits zero and `git status` shows only `testdata/MANIFEST.json` changed under `testdata/`.

**Depends on:** P0-03, P1-09

### P1-12: Conformance runner and the results for category block

**Goal:** Decode every block entry with cleanlz4, compare with the recorded input hash, and commit the counts.

**Files touched:** `crates/oracle/src/results.rs`, `crates/oracle/src/runner.rs`, `crates/oracle/src/bin/conformance.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/results.rs`, `Makefile`, `testdata/results/conformance-P1-12.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.4, §3.6, §8 Phase 1. Follow the results protocol and the long-run protocol in CLAUDE.md.
- `results.rs`: one `serde` struct per result kind in SPEC §5.4 (`deny_unknown_fields`, keys exactly as written there), and a writer that pretty-prints with a trailing newline. The ten category names are constants here.
- `runner.rs`: for each selected entry, generate its file if missing, decode with cleanlz4 (`decompress_block` with `max_output = input_bytes` for `kind` `"block"`; later tasks add frames), and pass it when the output's SHA-256 equals `input_sha256`. A panic is caught with `catch_unwind` and counts as failed.
- `cargo run --release -p oracle --bin conformance -- --task <id> --categories <list|all> [--check]` writes `testdata/results/conformance-<id>.json` with `by_category` holding only the categories run (`--check` writes nothing). It exits non-zero when any entry fails or the corpus cannot be produced. `Makefile` gains `conformance` (`TASK`, `CATEGORIES`) and `conformance-check` (`--check` over all categories).
- The counts in the committed file are the tool's output, never typed.

**Acceptance tests:** `crates/oracle/tests/results.rs`: `every_result_file_matches_its_schema` (each file under `testdata/results/` parsed by its name's prefix), `conformance_counts_are_consistent` (`failed == total - passed`, category sums equal the totals), `runner_counts_a_wrong_output_as_failed`, `runner_counts_a_panic_as_failed`.

**Out of scope:** frame entries; changing the decoder (a failing entry blocks this task, naming the entry ids and the error).

**Verification:** `kukulu_verify` with the Files touched; the path `testdata/results/conformance-P1-12.json` makes it run `make conformance TASK=P1-12 CATEGORIES=block` with its own timeout. `passed` must equal `total`.

**Depends on:** P1-11

### P1-13: DESIGN.md: the Phase 1 spec-rule report

**Goal:** Fill DESIGN.md's rule table with every Block format and XXH32 rule implemented so far and the section each implements.

**Files touched:** `docs/DESIGN.md`, `crates/oracle/tests/design.rs`

**Design constraints:**

- SPEC §3.4, §8 Phase 1 (manual check). Follow the design-note protocol in CLAUDE.md.
- Under `## Spec rules implemented`, one table row per rule: the rule in one line, the document and section heading, the function that implements it, and the test that covers it. Cover xxHash spec Steps 1 to 7; Block format token, literal length, literals, offset, match length, overlap copy, the three end-of-block rules with their P1-07 decisions, large lengths and safe decoding; and the dictionary prefix. A rule belonging to a blocked or skipped task is listed as "not implemented" with the task id.
- Read the code to write the table; do not copy from this plan.

**Acceptance tests:** `crates/oracle/tests/design.rs`: `design_report_cites_each_phase_1_section` (DESIGN.md contains "XXH32 Algorithm Description", "Step 1" through "Step 7", "Compressed block format", "End of block conditions" and "Safe decoding").

**Out of scope:** code changes.

**Verification:** `kukulu_verify`.

**Depends on:** none

### P1-14: Phase 1 end: push and record the manual check

**Goal:** Push the branch and record Phase 1's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 1. Follow the phase-end protocol in CLAUDE.md. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): read DESIGN.md's rule table against docs/spec/lz4_Block_format.md and the XXH32 section of docs/spec/xxhash_spec.md; read docs/spikes/P1-06.md and confirm each end-of-block decision follows its table; testdata/results/conformance-P1-12.json shows passed == total for category block`.

**Depends on:** none

## Phase 2 — Frames

### P2-01: Frame descriptor parsing and introspection

**Goal:** Parse and validate the frame header into a public descriptor type.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/lib.rs`, `crates/oracle/tests/descriptor.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "General Structure of LZ4 Frame format" (Magic Number) and all of "Frame Descriptor"; SPEC §6.1 (Introspection), §6.2. The invariants apply; `frame.rs` may import `block`, `xxh32` and `error`, never `stream`.
- Public types, re-exported from `lib.rs`: `BlockMaxSize { Max64KB, Max256KB, Max1MB, Max4MB }` with `bytes()`, and `FrameDescriptor { block_max_size, block_independence: bool, block_checksum: bool, content_checksum: bool, content_size: Option<u64>, dict_id: Option<u32> }`.
- `pub fn parse_frame_descriptor(input: &[u8]) -> Result<(FrameDescriptor, usize)>`: `input` starts at the magic number; the `usize` is the header length (7 to 19). Checks, in this order: fewer than 4 bytes → `TruncatedInput`; magic not `0x184D2204` (little endian) → `BadMagic`; no FLG byte → `TruncatedInput`; version bits not `01` → `UnsupportedVersion`; header bytes missing for the length FLG implies → `TruncatedInput`; FLG bit 1 set → `ReservedBitSet`; BD bit 7 or any of bits 3 to 0 set → `ReservedBitSet`; Block MaxSize value 0 to 3 → `InvalidBlockMaxSize`; header checksum byte not `(xxh32(descriptor, 0) >> 8) & 0xFF` → `BadHeaderChecksum`, where the descriptor is every byte from FLG up to, not including, the checksum byte.
- The header checksum is always verified (see Decisions). DESIGN.md gets one entry per field, citing its paragraph.

**Acceptance tests:** unit tests in `frame.rs`: `parses_the_minimal_descriptor`, `parses_content_size_and_dictionary_id`, `rejects_a_bad_magic`, `rejects_versions_other_than_01`, `rejects_the_reserved_flg_bit`, `rejects_reserved_bd_bits`, `rejects_block_max_size_values_0_to_3`, `rejects_a_bad_header_checksum`, `every_truncation_of_a_full_header_is_truncated_input`. `crates/oracle/tests/descriptor.rs`: `descriptor_matches_oracle_prefs_for_every_combination` (4 block sizes × linked × block checksum × content checksum × content size × Dict-ID zero or non-zero).

**Out of scope:** data blocks; skippable and legacy magic numbers.

**Verification:** `kukulu_verify`.

**Depends on:** P1-03, P1-04

### P2-02: Options, and one regular frame with independent blocks

**Goal:** Add the public options and result types and decode a single regular frame whose blocks are independent.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "General Structure" (EndMark), "Data Blocks"; SPEC §6.1. The invariants apply.
- `lib.rs`: `DecodeOptions { max_output_bytes: u64, dictionary: Option<Dictionary>, trailing_data: TrailingData, verify_checksums: bool }` whose `Default` is `DEFAULT_MAX_OUTPUT_BYTES`, `None`, `TrailingData::Error`, `true`; `Dictionary { bytes: Vec<u8>, id: Option<u32> }`; `TrailingData { Error, ReturnRemainder }`; `FrameKind { Regular, Skippable, Legacy }`; `FrameOutput { data: Vec<u8>, consumed: usize, checksums_verified: bool, descriptors: Vec<FrameDescriptor>, kinds: Vec<FrameKind> }`; `decompress_frame(input: &[u8]) -> Result<Vec<u8>>` and `decompress_frame_with(input: &[u8], options: &DecodeOptions) -> Result<FrameOutput>`.
- `frame.rs` holds the primitives that P3-01's streaming decoder will reuse, so shape them around whole units: `parse_block_header(u32) -> BlockHeader { size, uncompressed }` (top bit = uncompressed, Frame format "Block Size"), and a `FrameState` created from a descriptor with `decode_data_block(&mut self, header, data: &[u8], out: &mut Vec<u8>) -> Result<()>` and `finish(...)`. The one-shot driver in `frame.rs` walks a complete input slice and calls them.
- Rules: `0x00000000` is the EndMark; `0x80000000` is a valid empty uncompressed block; a block size field above the frame's Block Maximum Size → `BlockSizeExceeded`; a compressed block that decodes to more than the Block Maximum Size → `BlockSizeExceeded` (decode with `max_len` = that size and map the limit error); every missing byte → `TruncatedInput`.
- In this task: each block is decoded on its own (linked frames come in P2-03); a present block checksum or content checksum is read and skipped (P2-04); content size is not checked (P2-05); `max_output_bytes` and `dictionary` are stored but not applied (P3-04, P2-08); bytes after the frame → `TrailingData` (P2-06 replaces this with the frame loop).
- Never allocate from a header field: buffers grow with bytes actually decoded.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs` (frames built by a test-only helper that computes the header checksum with `cleanlz4::xxh32`): `uncompressed_blocks_frame_decodes`, `empty_frame_decodes_to_empty`, `zero_size_uncompressed_block_is_valid`, `compressed_block_frame_decodes`, `block_size_field_over_the_maximum_is_rejected`, `decoded_block_over_the_maximum_is_rejected`, `every_truncation_of_a_small_frame_is_truncated_input`, `default_options_have_the_documented_values`. `crates/oracle/tests/frame.rs`: `independent_frames_roundtrip_every_block_size` (synthetic inputs, including multi-block ones).

**Out of scope:** linked blocks, checksum verification, content size, concatenation, skippable and legacy frames, the output limit.

**Verification:** `kukulu_verify`.

**Depends on:** P1-05, P2-01

### P2-03: Linked blocks

**Goal:** Decode frames whose blocks depend on the previous 64 KB of output.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Block Independence flag" ("up to LZ4 window size, which is 64 KB"). The invariants apply.
- When the flag is 0, `FrameState` keeps the last 65,536 bytes of the frame's decoded output (compressed and uncompressed blocks alike) and each compressed block is decoded with that history ahead of it, so its matches can reach back. When the flag is 1 the history is empty at every block, and an offset reaching before the block → `OffsetOutOfRange`.
- Memory stays bounded: history never exceeds 65,536 bytes plus the block being decoded.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `linked_block_copies_from_the_previous_block`, `linked_block_copies_from_a_previous_uncompressed_block`, `independent_frame_rejects_an_offset_into_the_previous_block`, `history_reaches_back_65535_bytes_across_blocks`. `crates/oracle/tests/frame.rs`: `linked_frames_roundtrip_every_block_size` (inputs spanning several blocks, `window-boundary` included).

**Out of scope:** dictionaries; checksums.

**Verification:** `kukulu_verify`.

**Depends on:** P2-02

### P2-04: Block and content checksums

**Goal:** Verify block checksums and the content checksum, with an option to skip them that is reported in the result.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Block checksum flag", "Content checksum flag", "Block checksum", "Content Checksum"; SPEC §6.1 (Options). The invariants apply.
- Block checksum: XXH32, seed 0, of the raw block data as stored (compressed or not), four bytes little endian after the data, checked before the block is decoded; an empty block still carries one. Mismatch → `BlockChecksumMismatch`.
- Content checksum: XXH32, seed 0, of all decoded bytes of the frame, fed through the streaming `Xxh32` as blocks are produced; four bytes little endian after the EndMark. Mismatch → `ContentChecksumMismatch`.
- `verify_checksums = false` skips both comparisons (the bytes are still consumed), and `FrameOutput.checksums_verified` equals `options.verify_checksums`. The header checksum is verified regardless.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `block_checksum_mismatch_is_rejected`, `content_checksum_mismatch_is_rejected`, `empty_block_still_carries_a_block_checksum`, `verification_off_accepts_bad_checksums_and_reports_it`, `verification_off_still_rejects_a_bad_header_checksum`. `crates/oracle/tests/frame.rs`: `checksummed_frames_roundtrip`, `a_flipped_payload_byte_is_caught_by_the_block_checksum`.

**Out of scope:** content size.

**Verification:** `kukulu_verify`.

**Depends on:** P2-02

### P2-05: Content size

**Goal:** Use the header's content size to validate the decoded length.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Content Size flag", "Content Size" ("It can be skipped by a decoder, or used to validate content correctness"); SPEC §6.2, §6.4. The document permits either; SPEC lists the error, so cleanlz4 validates. Record the choice in DESIGN.md. The invariants apply.
- `ContentSizeMismatch` as soon as the frame's decoded byte count exceeds the content size, and at the EndMark when it is smaller. The content size never drives an allocation.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `matching_content_size_is_accepted`, `content_size_smaller_than_the_data_is_rejected`, `content_size_larger_than_the_data_is_rejected`, `content_size_zero_with_an_empty_frame_is_accepted`. `crates/oracle/tests/frame.rs`: `content_size_frames_roundtrip`.

**Out of scope:** the output limit's use of content size (P3-04).

**Verification:** `kukulu_verify`.

**Depends on:** P2-02

### P2-06: Concatenated frames, skippable frames and trailing data

**Goal:** Decode a flow of frames in order, skip skippable frames, and apply the trailing-data policy.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Frame Concatenation", "Skippable Frames"; SPEC §6.1. The invariants apply.
- The one-shot driver loops over frames. At a frame boundary with `n` bytes left: `n = 0` → finished (empty input gives `Ok` with empty data); `n ≥ 4` and the magic is `0x184D2204` → a regular frame; `0x184D2A50` to `0x184D2A5F` → a skippable frame: read the 4-byte little-endian size, skip that many bytes, `TruncatedInput` if they are not all there; `1 ≤ n ≤ 3` and the bytes are a prefix of the little-endian bytes of a known magic number (regular, any skippable, or legacy `0x184C2102`) → `TruncatedInput`; anything else → `BadMagic` when no frame has been decoded or skipped yet, otherwise by policy: `TrailingData::Error` → `Error::TrailingData`, `ReturnRemainder` → stop, with `FrameOutput.consumed` at the boundary.
- `consumed` equals the input length when everything was decoded. `descriptors` gets one entry per regular frame and `kinds` one per frame of any kind, in order. The legacy magic is recognised as a boundary here and decoded in P2-07; until then it returns `BadMagic`.
- DESIGN.md records the boundary rule as a decision (the document leaves concatenation "outside of this specification").

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `empty_input_is_ok_and_empty`, `two_concatenated_frames_decode_in_order`, `all_sixteen_skippable_magics_are_skipped`, `skippable_frame_between_two_frames`, `truncated_skippable_frame_is_truncated_input`, `garbage_at_the_start_is_bad_magic`, `garbage_after_a_frame_is_trailing_data`, `return_remainder_reports_where_it_stopped`, `a_partial_magic_at_the_end_is_truncated_input`, `a_short_non_magic_tail_follows_the_trailing_policy`. `crates/oracle/tests/frame.rs`: `concatenated_oracle_frames_decode`.

**Out of scope:** legacy frames; dictionaries.

**Verification:** `kukulu_verify`.

**Depends on:** P2-02

### P2-07: The legacy frame

**Goal:** Decode the legacy frame format.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/src/cli.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/legacy.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Legacy frame" (all of it, including the Extension). SPEC §6.5 (last paragraph): legacy frames come from the `lz4` program run as a black box (`-l`) or are built by hand. The invariants apply.
- Magic `0x184C2102`. Then repeated: at a block boundary with no bytes left → the frame ends; 1 to 3 bytes → `TruncatedInput`; a 4-byte little-endian value equal to a known magic number (regular, skippable, legacy) → the frame ends and that frame begins; zero → the frame ends and the four bytes are consumed (the documented extension); otherwise it is a compressed block size: above `LEGACY_MAX_COMPRESSED` (8 MiB + 8 MiB / 255 + 16, the block format's worst-case growth) → `BlockSizeExceeded`; missing data → `TruncatedInput`. Each block is decoded with `max_len` 8 MiB (more → `BlockSizeExceeded`), with no checksum.
- Blocks are decoded as independent. One observation changes that, and only this one: if `cli_legacy_frame_decodes` fails with `OffsetOutOfRange` in a block after the first, carry a 65,536-byte history across legacy blocks instead, and record in DESIGN.md that the document is silent and the CLI's output, observed as a black box, decided it.
- "All blocks must be completely filled, except the last one" is not enforced. A dictionary is never applied to a legacy frame. `kinds` records `Legacy`.
- `cli.rs`: `compress_legacy(data: &[u8]) -> Vec<u8>` runs `lz4 -l` through `std::process::Command` with temporary files, and `cli_version()`. Run `lz4 -h` to learn its options if needed; do not use any denied tool on it.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `legacy_frame_with_one_block`, `legacy_frame_ends_at_end_of_input`, `legacy_frame_ends_at_the_next_magic_number`, `legacy_frame_ends_at_a_zero_block_size`, `legacy_block_size_out_of_range_is_rejected`, `legacy_truncated_block_is_truncated_input`. `crates/oracle/tests/legacy.rs`: `hand_built_legacy_frame_from_oracle_blocks_decodes`, `cli_legacy_frame_decodes` (20 MiB of `synth::word_salad`, so there are at least three blocks), `legacy_then_regular_frame_decodes`.

**Out of scope:** streaming (P3-02).

**Verification:** `kukulu_verify`.

**Depends on:** P2-06

### P2-08: Preset dictionary and Dict-ID

**Goal:** Apply a caller-supplied dictionary to frames, with the Dict-ID rules.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/frame.rs`, `crates/oracle/src/dict.rs`, `crates/oracle/tests/dict_frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- Frame format, "Dictionary ID flag", "Dictionary ID"; SPEC §6.1 (Options), §6.2. The invariants apply.
- Rules: frame has a Dict-ID and `options.dictionary` is `None` → `DictionaryRequired`; the frame's Dict-ID and `dictionary.id` are both present and differ → `DictionaryMismatch`; otherwise a supplied dictionary is used, also for a frame without a Dict-ID.
- Use: with independent blocks every block is decoded with the dictionary as its prefix; with linked blocks the dictionary is the initial history only (its last 65,536 bytes), "used once, at the beginning of the frame". Each regular frame of a concatenation starts again from the dictionary.
- Oracle side: `dict::dict_frame(dict, chunks, independent: bool, dict_id: Option<u32>) -> Vec<u8>` assembles a frame by hand from P1-10's `dict_block` output (linked: one compression of `[dict, chunk1, chunk2, …]`, taking blocks 2 onward; independent: one `dict_block` per chunk), writing the header per the Frame format with the header checksum from `cleanlz4::xxh32`, and checking that the oracle's `decompress_frames` at least accepts the header (feed the header alone and expect "truncated", not an error name). The oracle cannot decode these frames (no dictionary on the permitted surface), so the expected output is the known chunks; say so in DESIGN.md.

**Acceptance tests:** `crates/cleanlz4/tests/frame.rs`: `dict_id_without_a_dictionary_is_dictionary_required`, `different_ids_are_dictionary_mismatch`, `a_dictionary_without_an_id_is_used_for_any_frame`, `a_dictionary_is_used_for_a_frame_without_dict_id`, `linked_frame_uses_the_dictionary_once_at_the_start`, `independent_frame_uses_the_dictionary_for_every_block`. `crates/oracle/tests/dict_frame.rs`: `assembled_linked_dictionary_frames_decode`, `assembled_independent_dictionary_frames_decode`, `the_wrong_dictionary_gives_an_error_or_other_bytes_and_never_panics`.

**Out of scope:** streaming; the conformance entries (P2-11).

**Verification:** `kukulu_verify`.

**Depends on:** P1-10, P2-03

### P2-09: Spike: the oracle's frame behaviour, and the differential compare module

**Goal:** Observe how the oracle's frame decoder treats each spec-permitted choice, and build the one function that compares cleanlz4 with the oracle.

**Files touched:** `crates/oracle/src/craft_frame.rs`, `crates/oracle/src/diff.rs`, `crates/oracle/src/bin/probe_frame.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/diff.rs`, `docs/spikes/P2-09.md`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.4, §3.7, §1 property 2. Follow the spike protocol and the oracle protocol. Time box: 90 minutes after the builder works.
- `craft_frame.rs`: `FrameSpec` (flags, block max size, optional content size and Dict-ID, a list of blocks each compressed or uncompressed with optional checksum override, optional content checksum override, overrides for the magic, FLG, BD and header checksum bytes) and `build(&FrameSpec) -> Vec<u8>` per the Frame format; plus `skippable(magic_low_nibble, user_data)` and `legacy(blocks, zero_terminated)`.
- Question: what does `oracle::frame::decompress_frames` return for each probe? Probes: empty input; a skippable frame alone; skippable then regular; regular then 1, 3 and 8 garbage bytes; a legacy frame; a frame with a Dict-ID and no dictionary; content size too small and too large; an uncompressed block size above the maximum; a compressed block decoding above the maximum; FLG reserved bit; BD reserved bits; each wrong version; each invalid block max size; a bad header checksum; a bad block checksum; a bad content checksum; a zero-size uncompressed block.
- cleanlz4's verdicts are fixed by the plan and do not move: empty → accept; skippable alone → accept; trailing garbage → reject; legacy → accept; Dict-ID without a dictionary → reject; wrong content size → reject; block over the maximum → reject; reserved bits, version, block max size, any bad checksum → reject; zero-size uncompressed block → accept.
- `diff.rs`: `Verdict { Agree { accepted: bool }, Skipped(&'static str), Diverge(String) }`, `compare_frame(input: &[u8], cfg: &Config) -> Verdict` and `compare_block(input: &[u8], max_output: usize) -> Verdict`, both built on a pure `judge` function so tests can inject outcomes. Rules: cleanlz4 runs with `max_output_bytes = diff.max_output_bytes` and the oracle with that cap; either side over the cap → `Skipped("output-cap")`; cleanlz4 returning `DictionaryRequired` is re-run with `Dictionary { bytes: vec![], id: None }` and that result is used; both accept → the outputs must be equal, else `Diverge`; both reject → `Agree`; one accepts and the other rejects → `Diverge`, unless a named normalization applies. A normalization exists only for a probe where the spike saw the oracle differ from the fixed verdicts above; each is one entry in a `const NORMALIZATIONS` table (id, what it matches, the Frame format section that permits or requires cleanlz4's side) and one DESIGN.md entry under `## Differential rules`. Expected candidates: `legacy-frame` (cleanlz4 accepted and `kinds` contains `Legacy`), `content-size`, `skippable`.
- `docs/spikes/P2-09.md`: question, method, the probe table (probe, oracle verdict, cleanlz4's fixed verdict, normalization id or "none"), and the list of normalizations with their justification.

**Acceptance tests:** `crates/oracle/tests/diff.rs`: `valid_oracle_frames_compare_as_agree`, `every_probe_is_agree_or_a_named_normalization`, `unequal_outputs_are_reported_as_diverge`, `dictionary_required_is_retried_with_an_empty_dictionary`, `an_unlisted_disagreement_is_diverge`, `compare_block_agrees_on_the_p1_06_probes`.

**Out of scope:** changing cleanlz4's verdicts; fuzzing.

**Verification:** `kukulu_verify`; `docs/spikes/P2-09.md` exists with the table.

**Depends on:** P1-07, P2-06

### P2-10: Conformance generator: oracle-compressed frame suites

**Goal:** Generate the frame suites that the oracle's frame API can produce, and record every combination.

**Files touched:** `crates/oracle/src/corpus.rs`, `crates/oracle/src/runner.rs`, `crates/oracle/tests/corpus.rs`, `testdata/MANIFEST.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.3, §5.4.
- A frame combination is (block size id 4 to 7) × (linked or independent) × (block checksum on or off) × (content checksum on or off) × (content size present or absent): 64 combinations. The compression level for combination `i` is `conformance.frame_levels[i % len]`, so every level meets every flag value.
- Suites: `synthetic` = every synthetic input × all 64; `frames-cantrbry` = every Canterbury file × all 64; `frames-silesia` = every Silesia file × `conformance.silesia_combos_per_file` combinations drawn with `Prng` seeded by `fuzz.seed` and the file's name. Inputs over 1 MiB are fed to `compress_frame` in 1 MiB chunks with `auto_flush` on.
- Category: `synthetic` for the synthetic suite; for the two corpus suites, in this precedence: `content-size` when content size is present, else `checksums` when either checksum is on, else `frame-linked` or `frame-independent`. Record the rule in DESIGN.md. Entries have `kind` `"frame"`, `tool` `"lz4f"`, and `params` listing all five flags and the level.
- `runner.rs` learns `kind` `"frame"`: decode with `decompress_frame_with` and `max_output_bytes = input_bytes`.

**Acceptance tests:** `crates/oracle/tests/corpus.rs`: `frame_suites_cover_all_64_flag_combinations`, `every_frame_level_and_block_size_appears`, `categories_follow_the_precedence_rule`, `the_oracle_decodes_every_generated_synthetic_frame`.

**Out of scope:** dictionary, skippable, legacy and concatenated suites (P2-11); the results file (P2-12).

**Verification:** `kukulu_verify`; `make corpus` exits zero.

**Depends on:** P1-02, P1-11

### P2-11: Conformance generator: dictionary, skippable, legacy and concatenated suites

**Goal:** Generate the four suites the oracle's frame API cannot produce directly.

**Files touched:** `crates/oracle/src/corpus.rs`, `crates/oracle/src/runner.rs`, `crates/oracle/tests/corpus.rs`, `testdata/MANIFEST.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.3, §6.5 (last paragraph), §9 Q3.
- `dictionary`: for each `dict.sizes_bytes` size and each Canterbury file at least twice that size, the dictionary is the file's first `size` bytes and the data its next 256 KiB (or what remains). Entries: the raw dictionary block (`kind` `"block-dict"`, P1-10's `dict_block`), a linked dictionary frame with a Dict-ID, an independent dictionary frame with a Dict-ID, and a linked one without (P2-08's `dict_frame`). Each entry records the dictionary's slice and SHA-256 and `dict_dependent`. `tool` `"hand"`.
- `skippable`: built by hand with `craft_frame`: each of the 16 magic values alone with small user data; zero-length user data; a skippable frame before, between and after regular oracle frames. Expected output is the regular frames' content in order.
- `legacy`: `cli::compress_legacy` of every Canterbury file and of 20 MiB of `word_salad`; a hand-built two-block legacy frame; a hand-built zero-terminated one. `tool` `"cli"` or `"hand"`; each CLI entry records `cli_version()`.
- `concatenated`: two, three and four oracle frames with different preferences; a legacy frame followed by a regular frame; a regular frame, a skippable frame and a regular frame.
- `runner.rs` learns `kind` `"block-dict"` (`decompress_block_with_dict`) and passes an entry's dictionary through `DecodeOptions` for frames.

**Acceptance tests:** `crates/oracle/tests/corpus.rs`: `the_four_special_suites_each_have_entries`, `dictionary_entries_exist_for_every_configured_size`, `legacy_cli_entries_record_the_cli_version`, `the_oracle_accepts_every_hand_built_regular_and_concatenated_entry_it_can_decode`.

**Out of scope:** the results file.

**Verification:** `kukulu_verify`; `make corpus` exits zero.

**Depends on:** P1-10, P2-10

### P2-12: The full conformance run and its results

**Goal:** Run every category and commit the counts; the bar is 100%.

**Files touched:** `testdata/results/conformance-P2-12.json`, `testdata/MANIFEST.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.4, §3.6, §8 Phase 2. Follow the results protocol and the long-run protocol.
- The file is the tool's output for all ten categories. `passed` must equal `total` and no category may be empty. If any entry fails, do not edit the decoder here: block, quoting the failing entry ids and the error each returned.
- DESIGN.md's `## Measurements` gets the per-category totals, copied from the file.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s existing `every_result_file_matches_its_schema` and `conformance_counts_are_consistent` pass over the new file.

**Out of scope:** decoder changes; generator changes.

**Verification:** `kukulu_verify` with the Files touched; the path `testdata/results/conformance-P2-12.json` makes it run `make conformance TASK=P2-12 CATEGORIES=all` with its own two-hour timeout. From this task on, any task touching `crates/cleanlz4/src/` also runs `make conformance-check`.

**Depends on:** P2-03, P2-04, P2-05, P2-06, P2-07, P2-08, P2-10, P2-11

### P2-13: The malformed corpus and its results

**Goal:** Build the malformed corpus by hand from the spec's rules, each file with its expected error variant, and commit the result counts.

**Files touched:** `crates/oracle/src/malformed.rs`, `crates/oracle/src/bin/malformed.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/malformed.rs`, `Makefile`, `testdata/malformed/`, `testdata/results/malformed-P2-13.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.4, §5.4, §6.2. Follow the results protocol.
- `malformed.rs` builds every file deterministically with `craft` and `craft_frame`; each file breaks exactly one rule and is otherwise valid (a file testing a reserved bit has a correct header checksum for its bytes). `testdata/malformed/index.json` lists `{ "file", "kind": "frame" | "block", "expected", "rule", "max_output" }`, where `expected` is the `Debug` name of the `Error` variant and `rule` cites the document and section.
- Cases: a bad magic; the FLG reserved bit; BD bit 7; BD low bits; each wrong version; each invalid block max size; a bad header checksum; one file per truncation offset 1 to len − 1 of a small valid frame (`TruncatedInput`), and the same for a frame with both checksums and content size; a zero offset; an offset before the output start (first block), into the previous block of an independent frame, and beyond the history of a linked frame; each end-of-block rule that `docs/spikes/P1-06.md` says is enforced; a wrong block checksum; a wrong content checksum; a content size too small and too large; a block size above the maximum; a Dict-ID with no dictionary; trailing garbage after a frame; raw-block versions of the zero offset, out-of-range offset and truncation cases.
- `cargo run -p oracle --bin malformed -- --write` regenerates the files and index; `-- --task <id>` runs every file through cleanlz4 (frames with default options, blocks with `max_output`) under `catch_unwind` and writes `testdata/results/malformed-<id>.json`. `Makefile` gains `malformed` (`TASK`). The files and the index are committed.

**Acceptance tests:** `crates/oracle/tests/malformed.rs`: `every_malformed_file_returns_its_expected_variant`, `no_malformed_file_panics`, `index_and_directory_agree`, `malformed_files_are_reproducible` (regenerated into a temporary directory, byte-identical), `every_expected_name_is_an_error_variant`, `each_frame_file_is_agree_or_a_named_normalization_under_compare`.

**Out of scope:** decoder changes (a wrong variant blocks, naming the files); fuzzing.

**Verification:** `kukulu_verify`; `make malformed TASK=P2-13` exits zero with `matched_expected_error == total` and `panics == 0`.

**Depends on:** P1-07, P2-04, P2-05, P2-08, P2-09

### P2-14: Phase 2 end: push and record the manual check

**Goal:** Push the branch and record Phase 2's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 2. Follow the phase-end protocol. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): testdata/results/conformance-P2-12.json shows passed == total with all ten categories present; testdata/results/malformed-P2-13.json shows matched_expected_error == total and panics == 0; read docs/spikes/P2-09.md and DESIGN.md's Differential rules and confirm each normalization cites a Frame format section`.

**Depends on:** none

## Phase 3 — Streaming and limits

### P3-01: Streaming decoder: regular frames

**Goal:** Add the streaming decoder object for regular frames, accepting input and output buffers of any size.

**Files touched:** `crates/cleanlz4/src/stream.rs`, `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/tests/stream.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.1 (Streaming, Introspection), §2 (identical output however chunked); Frame format, Introduction ("using only an a priori bounded amount of intermediate storage"). The invariants apply; `stream.rs` imports `frame`, `block`, `xxh32` and `error`.
- API, re-exported from `lib.rs`: `Decoder::new(options: DecodeOptions)`, `decode(&mut self, input: &[u8], output: &mut [u8], end_of_input: bool) -> Result<Progress>`, `descriptor(&self) -> Option<&FrameDescriptor>` (the current or last regular frame's), `checksums_verified(&self) -> bool`; `Progress { consumed: usize, produced: usize, status: Status }`; `Status { NeedsInput, OutputFull, FrameDone, Done }`.
- Status of one call, first that applies: decoded bytes are waiting and `output` is full → `OutputFull`; a frame ended during this call and its bytes are all delivered → `FrameDone` (once per frame; the call returns there); `end_of_input` is true, all input is consumed and the decoder is at a frame boundary → `Done`; otherwise → `NeedsInput`. After `Done` every call returns `Done` with nothing consumed. After an error every call returns the same error.
- `end_of_input` with a frame unfinished and no input left → `TruncatedInput`.
- The state machine buffers only the unit in progress (a header of at most 19 bytes, a 4-byte block header, one block's data and checksum, the EndMark's content checksum) and calls the `frame.rs` primitives from P2-02 to P2-05 on whole units; it holds at most one decoded block waiting for delivery. Nothing is allocated from a header field. Adjust the primitives' signatures in `frame.rs` only as far as reuse needs; the one-shot path must keep passing its tests.
- In this task a frame boundary accepts only a regular frame's magic (prefix rule from Decisions for 1 to 3 bytes; anything else `BadMagic` for the first frame and `TrailingData` after); P3-02 completes it.

**Acceptance tests:** `crates/cleanlz4/tests/stream.rs` (hand-built frames, each compared with `decompress_frame_with`): `one_byte_input_chunks_match_one_shot`, `one_byte_output_buffers_match_one_shot`, `status_goes_needs_input_then_frame_done_then_done`, `output_full_is_reported_and_decoding_resumes`, `an_empty_output_buffer_with_pending_bytes_is_output_full`, `end_of_input_inside_a_frame_is_truncated_input`, `errors_are_sticky`, `descriptor_is_available_once_the_header_is_parsed`, `linked_checksummed_frame_streams_bytewise`.

**Out of scope:** skippable and legacy frames, dictionaries, the trailing-data policy, the output limit.

**Verification:** `kukulu_verify`.

**Depends on:** P2-03, P2-04, P2-05

### P3-02: Streaming decoder: skippable, legacy and concatenated frames, dictionaries and trailing data

**Goal:** Bring the streaming decoder to the one-shot path's full behaviour.

**Files touched:** `crates/cleanlz4/src/stream.rs`, `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/stream.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §6.1; Frame format, "Frame Concatenation", "Skippable Frames", "Legacy frame", "Dictionary ID". The rules are P2-06's, P2-07's and P2-08's, restated: frame boundary rule (0 bytes and `end_of_input` → `Done`; a known magic → that frame; 1 to 3 bytes that prefix a known magic → wait for more, or `TruncatedInput` at `end_of_input`; anything else → `BadMagic` for the first frame, else `TrailingData` or, under `ReturnRemainder`, `Done` with the trailing bytes left unconsumed); skippable frames are skipped by counting down their size, never buffered; a legacy frame ends at end of input, at a known magic number or at a zero block size, so at a legacy block boundary with no buffered bytes and `end_of_input` false the status is `NeedsInput`; dictionary rules and use exactly as in P2-08.
- `FrameDone` is reported for skippable and legacy frames too. The invariants apply.

**Acceptance tests:** `crates/cleanlz4/tests/stream.rs`: `skippable_user_data_is_skipped_without_buffering` (10 MiB of declared user data fed in 1 MiB pieces), `legacy_frame_ends_at_end_of_input`, `legacy_frame_then_regular_frame`, `concatenated_frames_report_frame_done_each`, `dictionary_frames_stream`, `dict_id_without_a_dictionary_is_dictionary_required_when_streaming`, `return_remainder_stops_before_the_trailing_bytes`, `trailing_garbage_is_trailing_data`, `every_one_shot_test_frame_matches_when_streamed_bytewise` (each valid and invalid frame built in `tests/frame.rs`'s helper: same bytes, or the same `Error` variant).

**Out of scope:** the output limit; the oracle-driven property test (P3-03).

**Verification:** `kukulu_verify`.

**Depends on:** P2-06, P2-07, P2-08, P3-01

### P3-03: Chunking-invariance property test

**Goal:** Prove, over `stream.chunking_cases` random cases, that streaming with random input and output chunk sizes gives exactly the one-shot result.

**Files touched:** `crates/oracle/tests/chunking.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 3 (first bullet), §2. The case count is `Config::load().stream.chunking_cases`; the number is never written in the test (constraint `no-hardcoded-chunking-cases`). The seed is `fuzz.seed`.
- Each case, drawn from `Prng`: an input of 0 to 16 KiB (every 50th case up to 300 KiB) of mixed compressibility; random `Prefs` (block size, linked, both checksums, content size, a level from `conformance.frame_levels`); every 10th case a concatenation of two frames, a skippable frame in front, or a hand-built legacy frame; an input-chunk regime and an output-buffer regime each drawn from `{1, 2, 3, 7, 64, 4096, random up to 70,000}`, with the 1-byte regimes used only for inputs up to 4 KiB.
- Assert: the streamed bytes equal `decompress_frame`'s and the original input; the total consumed equals the input length; the final status is `Done`.
- Print the case index and seed on failure so a case can be replayed.

**Acceptance tests:** `crates/oracle/tests/chunking.rs`: `chunking_invariance_random_chunks` (all cases), `chunking_invariance_on_errors` (a tenth as many cases: a truncated or one-byte-corrupted frame gives the same `Error` variant streamed as one-shot, or the same bytes when both accept), `regimes_include_one_byte_input_and_one_byte_output` (a guard that the 1-byte regimes were drawn at least once each).

**Out of scope:** decoder changes (a failing case blocks, quoting the case index).

**Verification:** `kukulu_verify`.

**Depends on:** P1-02, P3-02

### P3-04: The output limit and bounded memory

**Goal:** Enforce the output limit in both paths so a decompression bomb fails fast with bounded memory.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/stream.rs`, `crates/cleanlz4/src/lib.rs`, `crates/cleanlz4/tests/limits.rs`, `crates/oracle/tests/limits.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §2 (output limit goal), §5.1 (`limits.max_output_bytes`), §6.2, §8 Phase 3 (second bullet). The default is `DEFAULT_MAX_OUTPUT_BYTES` and nothing else spells the number (constraint `no-hardcoded-output-limit`). The invariants apply.
- The limit counts decoded bytes over the whole call or decoder (all concatenated frames together). Exceeding it → `OutputLimitExceeded`, raised before the bytes are written: each block is decoded with `max_len` = min(Block Maximum Size, remaining allowance); when the allowance was the smaller bound and it is hit, the error is `OutputLimitExceeded`, otherwise `BlockSizeExceeded`.
- Fail fast: a frame header whose content size exceeds the remaining allowance → `OutputLimitExceeded` at the header.
- The one-shot output `Vec` grows as blocks arrive and is never reserved from a header field. In the streaming decoder, bytes decoded but not yet delivered count.
- DESIGN.md records the rule and the memory bound: limit + one block + 65,536 + the dictionary.

**Acceptance tests:** `crates/cleanlz4/tests/limits.rs`: `one_shot_over_the_limit_is_output_limit_exceeded`, `streaming_over_the_limit_is_output_limit_exceeded`, `a_limit_equal_to_the_output_is_ok`, `content_size_over_the_limit_fails_at_the_header`, `the_limit_spans_concatenated_frames`, `block_decode_limit_applies_to_decompress_block`. `crates/oracle/tests/limits.rs`: `a_small_frame_expanding_past_the_limit_fails` (the oracle's frame of 64 MiB of zeros, limit 1 MiB), `bomb_peak_allocation_is_bounded` (a counting `GlobalAlloc` in this test binary only: peak bytes allocated during the failing decode stay under the limit + 4 MiB + 65,536 + the input length + 1 MiB).

**Out of scope:** changing the default (P3-05).

**Verification:** `kukulu_verify`.

**Depends on:** P2-06, P3-01

### P3-05: Tune limits.max_output_bytes

**Goal:** Measure the output limit's cost and protection at three values and record whether the default stands.

**Files touched:** `crates/oracle/examples/limit_probe.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.1 (`limits.max_output_bytes`, default 1 GiB ⚠️ ASSUMPTION). Follow the tuning protocol in CLAUDE.md.
- `cargo run --release -p oracle --example limit_probe -- <limit bytes>…` decodes one bomb (the oracle's linked frame of 2 GiB of zeros, compressed in 4 MiB chunks) at each limit given on the command line and prints, per limit: time to the error and peak allocated bytes (a counting allocator in the example). The limits measured are 1 MiB, 64 MiB and the default, the last passed as `cleanlz4::DEFAULT_MAX_OUTPUT_BYTES` by the example itself when given the word `default`.
- Before = the default's row; after = the same key after the decision. Criterion: the default stays unless peak allocation at the default exceeds the default plus 16 MiB, which would be a bounded-memory defect: then block this task with the numbers instead of changing the key.
- Record the table under DESIGN.md `## Measurements`, in the commit body's `Measurement:` line and in the log.

**Acceptance tests:** none new; `crates/oracle/tests/config.rs`'s `limits_default_matches_library_constant` still passes.

**Out of scope:** changing the decoder.

**Verification:** `kukulu_verify`; the example's output is quoted in DESIGN.md.

**Depends on:** P3-04

### P3-06: Tune stream.chunking_cases

**Goal:** Measure what the chunking property test covers and costs at two case counts and record whether 10,000 stands.

**Files touched:** `crates/oracle/tests/chunking.rs`, `testdata/config.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 3 ("at least 10,000 cases ⚠️ ASSUMPTION"); key `stream.chunking_cases`. Follow the tuning protocol.
- Add an ignored test `chunking_coverage_report` that takes the case count from the environment variable `CHUNKING_CASES_OVERRIDE` (falling back to the config key), runs the same generator, and prints the wall time and how many of the distinct shapes were hit: (input regime × output regime × linked or independent × frame kind mix), 7 × 7 × 2 × 4 = 392 shapes, less the 1-byte regimes' size restriction.
- Measure at one tenth of the key and at the key. Criterion: keep the key unless fewer than all reachable shapes are hit at the key's value; then raise it to the smallest multiple of the current value that hits them all, provided `chunking_invariance_random_chunks` still finishes within five minutes. The key never goes below SPEC's 10,000.
- Record both rows under DESIGN.md `## Measurements`, in the commit body's `Measurement:` line and in the log.

**Acceptance tests:** `crates/oracle/tests/chunking.rs`: `chunking_coverage_report` exists, is `#[ignore]`d, and passes when run with `cargo test -p oracle --test chunking -- --ignored`.

**Out of scope:** changing the generator's distribution.

**Verification:** `kukulu_verify`.

**Depends on:** P3-03

### P3-07: Phase 3 end: push and record the manual check

**Goal:** Push the branch and record Phase 3's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 3 (it names no manual check; the list below is the plan's). Follow the phase-end protocol. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): read the streaming API in crates/cleanlz4/src/stream.rs for ergonomics; confirm DESIGN.md's Measurements has the limit table and the chunking coverage table; run cargo test -p oracle --test limits and watch the bomb test's peak figure`.

**Depends on:** none

## Phase 4 — Fuzzing and measurement

### P4-01: Fuzz targets (a): no panic on arbitrary bytes

**Goal:** Add the three no-panic fuzz targets, one per entry point, with their logic testable on stable Rust.

**Files touched:** `crates/oracle/src/fuzz_logic.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/fuzz_logic.rs`, `fuzz/Cargo.toml`, `fuzz/Cargo.lock`, `fuzz/fuzz_targets/no_panic_block.rs`, `fuzz/fuzz_targets/no_panic_frame.rs`, `fuzz/fuzz_targets/no_panic_stream.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 4 (a), §1 property 3. Each `fuzz_targets/*.rs` file is only `fuzz_target!(|data: &[u8]| oracle::fuzz_logic::<name>(data));`; the logic lives in `fuzz_logic.rs`. Config is loaded once through a `OnceLock`. `fuzz/Cargo.toml` must not name the binding crate; adding a `[[bin]]` needs no new dependency, so nothing is downloaded.
- `no_panic_block(data)`: the first two bytes choose `max_output` (their little-endian value × 16) and where to split the rest into a dictionary and a block; call `decompress_block` and `decompress_block_with_dict`; results are ignored.
- `no_panic_frame(data)`: the first byte's bits choose the trailing policy, checksum verification and whether a dictionary (the next 64 bytes, with or without an id) is supplied; `max_output_bytes` is `fuzz.max_output_bytes`; call `decompress_frame_with`.
- `no_panic_stream(data)`: the first two bytes choose the input and output chunk regimes (as in P3-03); run the streaming decoder and assert its outcome equals the one-shot outcome (same bytes, or the same `Error` variant).
- None of the three calls the oracle.

**Acceptance tests:** `crates/oracle/tests/fuzz_logic.rs`: `no_panic_logic_survives_the_malformed_corpus` (every file under `testdata/malformed/`, when that directory exists, through all three), `no_panic_logic_survives_random_buffers` (`conformance.random_block_cases` buffers of 0 to 4,096 random bytes), `no_panic_logic_survives_valid_frames_with_every_prefix_byte` (a small oracle frame behind each of 256 first bytes).

**Out of scope:** the differential targets; running for the full duration.

**Verification:** `kukulu_verify` with the Files touched (the `fuzz/` prefix runs the fuzz build).

**Depends on:** P3-02, P3-04

### P4-02: Fuzz targets (b) and (c): differential

**Goal:** Add the round-trip differential target and the accept/reject differential target.

**Files touched:** `crates/oracle/src/fuzz_logic.rs`, `crates/oracle/tests/fuzz_logic.rs`, `fuzz/Cargo.toml`, `fuzz/Cargo.lock`, `fuzz/fuzz_targets/diff_roundtrip.rs`, `fuzz/fuzz_targets/diff_accept.rs`, `testdata/results/divergences/.gitkeep`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 4 (b) and (c), §6.4, §5.1 (`fuzz.seed`). Follow the oracle protocol.
- `diff_roundtrip(data)`: the first four bytes choose `Prefs` (block size, linked, both checksums, content size, a level from `conformance.frame_levels`), the target length (up to 300 KiB) and how the input is cut into `compress_frame` chunks; the rest is expanded to the target length by repeating it with perturbations from `Prng` seeded with `fuzz.seed` and a hash of the bytes. Compress with the oracle, decode with cleanlz4 (one-shot) and with the oracle; both must equal the input, else panic. Every eighth input also does the block form: `compress_default` or `compress_hc`, then `decompress_block`.
- `diff_accept(data)`: `diff::compare_frame(data, cfg)` and `diff::compare_block(data, cap)` from P2-09. On `Diverge`: when the environment variable `CLEANLZ4_DIVERGENCE_DIR` is set, write the input to `<dir>/<first 16 hex of its SHA-256>.bin` unless `fuzz.max_divergence_files` files are already there, and continue; when it is not set, panic with the verdict's text.
- `Skipped` verdicts are not divergences.

**Acceptance tests:** `crates/oracle/tests/fuzz_logic.rs`: `diff_roundtrip_logic_passes_on_fixed_inputs` (256 small buffers from `Prng`), `diff_accept_logic_agrees_on_the_malformed_corpus` (when that directory exists), `diff_accept_logic_agrees_on_random_buffers`, `a_divergence_is_written_to_the_directory_when_one_is_configured` (through the helper that records a divergence, with a temporary directory), `the_divergence_file_cap_is_honoured`.

**Out of scope:** changing `diff.rs`'s normalizations; full-duration runs.

**Verification:** `kukulu_verify` with the Files touched.

**Depends on:** P2-09, P4-01

### P4-03: The fuzz runner, its results writer and regression replay

**Goal:** Run a fuzz target for a configured duration and turn the outcome into a SPEC §5.4 result file, with every crash kept as a replayable regression input.

**Files touched:** `crates/oracle/src/fuzzrun.rs`, `crates/oracle/src/bin/fuzz_run.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/fuzzrun.rs`, `crates/oracle/tests/regressions.rs`, `Makefile`, `testdata/regressions/index.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §7 (fuzz), §5.4, §5.1, §3.5, §3.6. No duration or seed literal anywhere (constraint `no-hardcoded-fuzz-tunables`).
- `cargo run --release -p oracle --bin fuzz_run -- --target <t> --task <id> [--smoke]`: copies seed inputs (the malformed corpus, when present) into `fuzz/corpus/<t>/`; runs `cargo +nightly fuzz run -a <t> fuzz/corpus/<t> -- -max_total_time=<fuzz.seconds_per_target, or fuzz.smoke_seconds with --smoke> -seed=<fuzz.seed> -print_final_stats=1` with `CLEANLZ4_DIVERGENCE_DIR` set to `testdata/results/divergences/` (absolute); relays its output to `fuzz/logs/<t>-<id>.log` with each line prefixed by the elapsed seconds; then writes `testdata/results/fuzz-<t>-<id>.json` with `task`, `target`, `seconds` (whole seconds the fuzzer ran), `runs` (the final stats' executed-units figure), `crashes` (new files under `fuzz/artifacts/<t>/`), `divergences` (new files in the divergence directory during this run), `corpus_entries` (files in `fuzz/corpus/<t>/`) and `seed`. With `--smoke` the result goes to a temporary directory instead.
- A crash ends the fuzzer early: the runner copies each new artifact to `testdata/regressions/<t>-<first 16 hex of its SHA-256>.bin`, writes the result with `crashes` > 0, and exits zero (a result is data). `testdata/regressions/index.json` starts as `[]`; an entry `{ "file", "target", "resolution" }` is added only when an input has been triaged (P4-09), so an untriaged input never turns a later task's tests red. It exits non-zero only when the fuzzer could not be built or started, or stopped early without a crash.
- `fuzzrun.rs` holds the pure parts (argument list from the config, final-stats parsing, result assembly) so they are unit-tested. `Makefile` gains `fuzz-run` (`TARGET`, `TASK`) and `fuzz-smoke` (every target with `--smoke`).

**Acceptance tests:** `crates/oracle/tests/fuzzrun.rs`: `arguments_come_from_the_config`, `final_stats_are_parsed`, `a_result_serializes_to_the_spec_schema`, `smoke_uses_the_smoke_duration_key`. `crates/oracle/tests/regressions.rs`: `every_indexed_regression_input_passes_its_target_logic` (each entry of `testdata/regressions/index.json` through its target's `fuzz_logic` function, with no divergence directory configured so a divergence panics; passes with an empty index), `every_index_entry_names_an_existing_file`.

**Out of scope:** full-duration runs (P4-04 to P4-08).

**Verification:** `kukulu_verify`; `make fuzz-smoke` exits zero in the foreground (five targets × `fuzz.smoke_seconds`, plus the build).

**Depends on:** P4-02

### P4-04: Run no_panic_block for the configured duration

**Goal:** Fuzz `no_panic_block` for `fuzz.seconds_per_target` and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_block-P4-04.json`

**Design constraints:** SPEC §8 Phase 4, §5.4, §3.6. Follow the fuzz-run protocol and the long-run protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s `every_result_file_matches_its_schema` passes over the new file; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` > 0.

**Out of scope:** fixing a crash (P4-09); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_block-P4-04.json`, which runs `make fuzz-run TARGET=no_panic_block TASK=P4-04` under its own 75-minute timeout. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** P4-03

### P4-05: Run no_panic_frame for the configured duration

**Goal:** Fuzz `no_panic_frame` for `fuzz.seconds_per_target` and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_frame-P4-05.json`

**Design constraints:** SPEC §8 Phase 4, §5.4, §3.6. Follow the fuzz-run protocol and the long-run protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s `every_result_file_matches_its_schema` passes over the new file; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` > 0.

**Out of scope:** fixing a crash (P4-09); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_frame-P4-05.json`, which runs `make fuzz-run TARGET=no_panic_frame TASK=P4-05` under its own 75-minute timeout. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** P4-03

### P4-06: Run no_panic_stream for the configured duration

**Goal:** Fuzz `no_panic_stream` for `fuzz.seconds_per_target` and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_stream-P4-06.json`

**Design constraints:** SPEC §8 Phase 4, §5.4, §3.6. Follow the fuzz-run protocol and the long-run protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s `every_result_file_matches_its_schema` passes over the new file; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` > 0.

**Out of scope:** fixing a crash (P4-09); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_stream-P4-06.json`, which runs `make fuzz-run TARGET=no_panic_stream TASK=P4-06` under its own 75-minute timeout. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** P4-03

### P4-07: Run diff_roundtrip for the configured duration

**Goal:** Fuzz `diff_roundtrip` for `fuzz.seconds_per_target` and commit its result file.

**Files touched:** `testdata/results/fuzz-diff_roundtrip-P4-07.json`

**Design constraints:** SPEC §8 Phase 4 (b), §5.4, §3.6. Follow the fuzz-run protocol and the long-run protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s `every_result_file_matches_its_schema` passes over the new file; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` > 0.

**Out of scope:** fixing a crash (P4-09); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-diff_roundtrip-P4-07.json`, which runs `make fuzz-run TARGET=diff_roundtrip TASK=P4-07` under its own 75-minute timeout. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** P4-03

### P4-08: Run diff_accept for the configured duration

**Goal:** Fuzz `diff_accept` for `fuzz.seconds_per_target` and commit its result file and any divergence inputs.

**Files touched:** `testdata/results/fuzz-diff_accept-P4-08.json`

**Design constraints:** SPEC §8 Phase 4 (c), §5.4, §3.6. Follow the fuzz-run protocol and the long-run protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/` and `testdata/results/divergences/`.

**Acceptance tests:** `crates/oracle/tests/results.rs`'s `every_result_file_matches_its_schema` passes over the new file; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` > 0.

**Out of scope:** triaging a divergence (P4-09); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-diff_accept-P4-08.json`, which runs `make fuzz-run TARGET=diff_accept TASK=P4-08` under its own 75-minute timeout. The log entry quotes `runs`, `crashes`, `divergences` and `corpus_entries`.

**Depends on:** P4-03

### P4-09: Triage crashes and divergences

**Goal:** Resolve every crash and divergence the five runs found, or record that there were none.

**Files touched:** `docs/DESIGN.md`, `crates/cleanlz4/src/`, `crates/oracle/src/diff.rs`, `testdata/regressions/`, `testdata/malformed/`

**Design constraints:**

- SPEC §8 Phase 4 (manual check), §6.4, §1 ("zero crashes and zero divergences"). The invariants apply to any decoder change; follow the design-note protocol and the fuzz-run protocol.
- Read the five result files `testdata/results/fuzz-*-P4-0[4-8].json` (a missing one belongs to a blocked run: name it in DESIGN.md and go on). If every `crashes` and `divergences` is 0, write "none" under DESIGN.md `## Divergence triage` with the five `runs` figures, change no code, and finish.
- Otherwise, for each file in `testdata/regressions/` and `testdata/results/divergences/`, add a row under `## Divergence triage`: the file, the target, both sides' outcomes, the spec document and section that decides it, and the resolution, which is one of: (1) a cleanlz4 defect, fixed in `crates/cleanlz4/src/` with a unit test and a new malformed-corpus file where it fits; (2) a choice the spec permits, encoded as a new entry in `diff.rs`'s `NORMALIZATIONS` with its section. Copy each divergence input into `testdata/regressions/` as `diff_accept-<same name>`, and add every triaged input to `testdata/regressions/index.json` with its target and resolution. Never resolve by deleting an input: when this task is done every file in `testdata/regressions/` is in the index and the regression test passes over all of them.
- Then re-run each affected target for the full duration: call `kukulu_verify` with the Files touched plus `testdata/results/fuzz-<target>-P4-09.json` for each affected target only (each such path runs `make fuzz-run TARGET=<target> TASK=P4-09`), and commit those result files. If a re-run still reports a crash or a divergence, block with the file names.

**Acceptance tests:** `crates/oracle/tests/regressions.rs`: `every_indexed_regression_input_passes_its_target_logic` and `every_index_entry_names_an_existing_file` pass with every input indexed; each fix has a named unit test, listed in the commit body.

**Out of scope:** weakening a fuzz target; changing a fixed verdict of P2-09 without a spec section that requires it.

**Verification:** `kukulu_verify` as described above; with no crash and no divergence, `kukulu_verify` with `docs/DESIGN.md` alone.

**Depends on:** P4-03

### P4-10: Benchmarks against the oracle

**Goal:** Measure decode throughput of cleanlz4 and the oracle on every corpus file and commit the figures.

**Files touched:** `crates/oracle/examples/bench_decode.rs`, `crates/oracle/src/bench.rs`, `crates/oracle/src/lib.rs`, `crates/oracle/tests/bench.rs`, `Makefile`, `testdata/results/bench-P4-10.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 4 (benchmarks), §5.4, §5.1 (`bench.repeats`), §1 ("Performance is measured and reported. It is not a gate."), §3.4 (an optimization cites cleanlz4's own profile, never another implementation). Follow the results protocol. The repeat count is read from the config (constraint `no-hardcoded-bench-repeats`).
- For each corpus file in the manifest: compress once with the oracle (`Prefs`: 4 MiB blocks, independent, content checksum on, level 0); decode `bench.repeats` times with `cleanlz4::decompress_frame` and as many with `oracle::frame::decompress_frames`; keep each side's best time (`std::time::Instant`); report MB/s as decoded bytes / 1,000,000 / seconds, rounded to one decimal.
- `cargo run --release -p oracle --example bench_decode -- --task <id> [--repeats <n>] [--only <corpus name>]` writes `testdata/results/bench-<id>.json`; `--repeats` exists for P4-11's measurement only. `Makefile` gains `bench` (`TASK`).
- There is no target. Do not optimize the decoder in this task.

**Acceptance tests:** `crates/oracle/tests/bench.rs`: `measure_reports_positive_rates_for_a_small_input`, `rate_is_bytes_over_seconds_in_megabytes`; `results.rs`'s schema test passes over the new file.

**Out of scope:** decoder changes; profiling the oracle (denied by the policy).

**Verification:** `kukulu_verify`; `make bench TASK=P4-10` in the foreground (well under ten minutes). The log entry quotes the slowest and fastest `cleanlz4_mb_s`.

**Depends on:** P0-03, P2-04

### P4-11: Tune bench.repeats

**Goal:** Measure how stable the benchmark is at three repeat counts and record whether 10 stands.

**Files touched:** `testdata/config.json`, `testdata/results/bench-P4-11.json`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.1 (`bench.repeats`, default 10 ⚠️ ASSUMPTION). Follow the tuning protocol and the results protocol.
- Run `bench_decode` on the Canterbury files only (`--only cantrbry`), three times each at `--repeats 3`, at the key's value and at three times the key's value, writing to a scratch task id that is not committed. For each count record the spread: the largest (max − min) / max of `cleanlz4_mb_s` over the three runs, across files.
- Criterion: keep the key unless its spread exceeds 5% and the tripled count's spread is under half of it; then set the key to the tripled value. Whatever is decided, finish with `make bench TASK=P4-11` at the key's final value and commit that file as the "after".
- Record the three rows under DESIGN.md `## Measurements`, in the commit body's `Measurement:` line and in the log.

**Acceptance tests:** `results.rs`'s schema test passes over `bench-P4-11.json`; `config.rs`'s tests still pass.

**Out of scope:** decoder changes.

**Verification:** `kukulu_verify`.

**Depends on:** P4-10

### P4-12: Tune fuzz.seconds_per_target

**Goal:** Measure, from the full runs' logs, whether an hour per target was enough, and record the answer.

**Files touched:** `docs/DESIGN.md`

**Design constraints:**

- SPEC §5.1 (`fuzz.seconds_per_target`, default 3600 ⚠️ ASSUMPTION). Follow the tuning protocol.
- For each target, read `fuzz/logs/<target>-P4-0x.log` (lines carry elapsed seconds; these logs are untracked files this run wrote). Tabulate the fuzzer's coverage figure at 60 s, at 600 s and at the end, and the elapsed second of the last line reporting new coverage. Before = the 60 s column; after = the end column. If a log is missing, run `make fuzz-smoke`, use its figures for the before column, and write "log unavailable" for the rest.
- Criterion: the key stands when every target's last new coverage came before 90% of the run. If any came later, do not change the key (the hour-long verify commands' timeouts in `docs/kukulu.json` are sized for 3,600 s and that file is protected): record a recommendation to raise it, naming the targets, in DESIGN.md and in the log.
- Record the table under DESIGN.md `## Measurements`, in the commit body's `Measurement:` line and in the log.

**Acceptance tests:** none new.

**Out of scope:** re-running a full fuzz run; editing `testdata/config.json`.

**Verification:** `kukulu_verify`.

**Depends on:** P4-03

### P4-13: Phase 4 end: push and record the manual check

**Goal:** Push the branch and record Phase 4's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 4. Follow the phase-end protocol. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): every testdata/results/fuzz-*.json shows crashes == 0 and divergences == 0 at its newest task id, or DESIGN.md's Divergence triage resolves each one; every run's seconds is at least fuzz.seconds_per_target; read bench-P4-10.json (actuals only, no target); act on any recommendation P4-12 recorded`.

**Depends on:** none

## Phase 5 — Evidence and hand-off

### P5-01: The evidence index and the Done checklist

**Goal:** List every piece of evidence the summarize stage must attest, and state SPEC §1's Done items against it.

**Files touched:** `docs/EVIDENCE.md`, `README.md`, `crates/oracle/tests/results.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §1 (Done means), §5.4, §8 Phase 5, §3.6. `kukulu_similarity`, `kukulu_attest` and `SUMMARY.md` are the summarize stage's work, not this task's: do not call those tools and do not write `docs/SIMILARITY.md`, `docs/attestation/` or `docs/SUMMARY.md`.
- `docs/EVIDENCE.md`: (1) `## Evidence files`, one repo-relative path per line in a fenced block, sorted: every tracked file under `testdata/results/` (divergence inputs included), `testdata/MANIFEST.json`, `testdata/malformed/index.json`, `docs/DESIGN.md` and every `docs/spikes/*.md`; this is the `evidence` list for the attestation. (2) `## Results`, a table quoting each result file's headline counts, read from the files. (3) `## Done checklist`, SPEC §1's five items, each "met", "not met" or "owed to the summarize stage", with the file that shows it; blocked and skipped tasks are named here.
- `README.md`: replace the status line with what was built, the commands of SPEC §7, and a pointer to `docs/EVIDENCE.md`. Do not describe the pipeline.

**Acceptance tests:** `crates/oracle/tests/results.rs`: `evidence_index_lists_every_result_file` (every tracked file under `testdata/results/` except `.gitkeep` appears in `docs/EVIDENCE.md`'s fenced list, and every listed path exists).

**Out of scope:** the similarity score, the attestation bundle, the summary.

**Verification:** `kukulu_verify`.

**Depends on:** P1-12

### P5-02: Phase 5 end: push and record the manual check

**Goal:** Push the branch and record Phase 5's manual check and the summarize stage's inputs for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 5. Follow the phase-end protocol. No code change. `docs/HANDOFF.md`, written when the run finishes, must name `docs/EVIDENCE.md`'s fenced list as the evidence for the attestation.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify`. The log entry carries `Manual check: NOT VERIFIED (human): after the summarize stage, docs/SIMILARITY.md exists with both controls, and node docs/attestation/verify.mjs docs/attestation --key sha256:<owner's fingerprint> passes; the owner runs that verifier from a clean checkout on a second machine; SUMMARY.md quotes the result files, the denial counts and verdicts, the interventions and the similarity scores`.

**Depends on:** none

## Spec issues

- **Phase 5 is not implementer work.** `kukulu_similarity` refuses while an implement run is armed and until the review approves, and `kukulu_attest` needs the committed summary; both, and `SUMMARY.md`, belong to the summarize stage. Resolution: Phase 5 here builds the evidence index (P5-01) and records the checks (P5-02). Proposed SPEC edit: say so in §8 Phase 5.
- **§6.5 lists names, not signatures.** §3.1 permits "the oracle's C API signatures, exactly as listed in §6.5", but §6.5 gives function names only, and the C headers and the binding crate's source are forbidden. Resolution: parameter lists are learned from compiler diagnostics on the binding crate's declarations; a call that cannot be made to compile blocks its task. Proposed SPEC edit: list the signatures.
- **§6.5 has no dictionary compressor, no dictionary frame decoder and no XXH32 function.** Resolution: dictionary-dependent blocks come from linked-block frames (P1-10), dictionary frames are assembled by hand and checked against known content (P2-08), and XXH32 vectors are the oracle's frame checksums (P1-04). The oracle therefore never decodes a dictionary frame. Proposed SPEC edit: add `LZ4F_compressBegin_usingDict`-style entries, or state this limit.
- **§6.2 has no variant for a block above the Block Maximum Size**, which the Frame format says "shall never" happen. Resolution: a seventeenth variant, `BlockSizeExceeded`, also used for an out-of-range legacy block size.
- **§5.1's table omits two tunables** SPEC itself names: Phase 3's "at least 10,000 cases ⚠️ ASSUMPTION" and §9 Q3's dictionary sizes. Resolution: keys `stream.chunking_cases` and `dict.sizes_bytes`.
- **§3.3's attribute and "tests are exempt" conflict** for unit tests inside the crate. Resolution: `#![cfg_attr(not(test), deny(…))]`.
- **The installed `lz4` CLI is v1.9.4**, while the pinned Frame format (1.6.5) says `lz4` accepts the zero-size legacy end marker only since v1.10.1, and the binding crate's library version may differ again. Resolution: both versions are recorded in the manifest; the zero-terminated legacy frame is tested with hand-built files only; cleanlz4 accepts it.
- **The Frame format does not say whether legacy blocks are linked.** Resolution: independent, with one named black-box observation in P2-07 that would switch it.
- **§6.1's "a preset dictionary for frames that carry a dictionary ID"** would leave a dictionary frame without a Dict-ID undecodable, though the Frame format allows the dictionary to be known off-band. Resolution: a supplied dictionary is used for any regular frame.
- **§7's fuzz command is a minimum.** The runner adds `-a` (overflow and debug assertions), `-seed=<fuzz.seed>`, `-print_final_stats=1` and a corpus directory.
- **Hour-long verify commands.** Each fuzz run is one `kukulu_verify` call of up to 75 minutes, and the full conformance run up to two hours; the operator's `MCP_TOOL_TIMEOUT` must exceed 7,200,000 ms or those calls fail on the client side.
- **§5.2 does not say whether the corpora are committed.** Resolution: they are not (size); the manifest's hashes stand in for them, so the conformance results can be reproduced only on a machine that fetches the same archives.
- **`lockdown.after` is P0-03, but the oracle and fuzz crates need downloads.** Resolution: all dependencies are resolved in P0-01 and cargo is switched offline at the end of P0-03; a dependency discovered to be missing later blocks its task.
- **§1 names a "Kūkulu 0.7.x signed bundle"**; the installed plugin and §7's guard path are 0.8.0. No plan impact.

## Review fixes (round 1)

### R1-01: Spike binary reads the differential cap from the config

**Goal:** Stop writing diff.max_output_bytes as a literal in crates/oracle/src/bin/probe_frame.rs and add a test that catches the next large config value spelled in oracle code (REVIEW F1).
**Files touched:** crates/oracle/src/bin/probe_frame.rs, crates/oracle/tests/config.rs
**Design constraints:** CLAUDE.md constraint 10 and SPEC §3.5: every value in testdata/config.json is read from the config, never written elsewhere. Replace `const CAP: usize = 1 << 26;` with `Config::load()` and `cfg.diff.max_output_bytes`. Change nothing else in the spike binary's output. The invariants apply.
**Acceptance tests:** crates/oracle/tests/config.rs: `large_config_values_are_not_spelled_in_oracle_sources`. Take every numeric leaf of testdata/config.json that is at least 1,000,000. Assert that no file under crates/oracle/src/ or crates/oracle/examples/ (except crates/oracle/src/config.rs) contains its decimal form (with or without `_` separators), or, for a power of two 2^n, `1 << n` or `1<<n`. The test must fail on the current probe_frame.rs line 6.
**Out of scope:** docs/kukulu.json (protected); the spike's recorded table in docs/spikes/P2-09.md; other binaries' behaviour.
**Verification:** kukulu_verify with the files touched; `cargo run -p oracle --bin probe_frame` still prints the probe table.
**Depends on:** none

### R1-02: Hold the output limit to its memory bound, and test it past several blocks

**Goal:** Make the one-shot and streaming paths' peak allocation stay within the limit plus one block plus 65,536 bytes (plus the input and any dictionary), at any limit, not only at powers of two (REVIEW F2).
**Files touched:** crates/cleanlz4/src/frame.rs, crates/cleanlz4/src/block.rs, crates/cleanlz4/tests/limits.rs, crates/oracle/tests/limits.rs, crates/oracle/examples/limit_probe.rs, docs/DESIGN.md
**Design constraints:** SPEC §1 property 3, §2 (output limit goal), §8 Phase 3. Today frame.rs:278 grows the one-shot output by amortized doubling, with no cap at the allowance: at a 72 MiB limit the peak was 150,994,934 bytes. frame.rs:233-235 extends the history by a whole block before draining it, and Vec keeps the capacity. The per-block scratch Vec can overshoot its bound.
Required: the output's capacity never exceeds bytes already decoded plus the remaining allowance. Grow it geometrically, capped at that, with try_reserve_exact or similar; never reserve the whole allowance up front. The history never holds more than 65,536 bytes of capacity beyond one block, so trim before extending. Any per-block buffer stays within the block bound. Nothing is ever reserved from a header field (content size included).
The one-shot and streaming outputs must not change: every existing test, and `make conformance-check` when it is active, stays green. Follow the design-note and tuning protocols. Replace DESIGN.md's bound sentence with the measured one. State the accounting: a reallocation counts only its size change. Re-measure `limit_probe` at 1 MiB, 64 MiB, 75,497,472 (72 MiB) and `default` under that accounting, and replace the P3-05 table rows. The invariants apply.
**Acceptance tests:** Give the counting allocator in crates/oracle/tests/limits.rs (and in limit_probe.rs) a `realloc` that forwards to `System.realloc` and adds or subtracts only the size difference.
New crates/oracle/tests/limits.rs `bomb_peak_allocation_is_bounded_past_several_blocks`: the oracle's frame of 64 MiB of zeros with 4 MiB blocks, both linked and independent, decoded one-shot and streamed (64 KiB output buffer) with limit = 3 * 4 MiB + 1. The decode fails with OutputLimitExceeded after three whole blocks. Peak stays under limit + 4 MiB + 65,536 + the frame's length + 1 MiB. This fails on today's code (output capacity 16 MiB, plus the scratch block and history).
New crates/cleanlz4/tests/limits.rs `one_shot_output_capacity_never_exceeds_the_limit`: a hand-built frame of four uncompressed 4 MiB blocks decoded with max_output_bytes equal to its 16 MiB output, and the same data truncated to 12 MiB + 1 bytes with the limit equal to that length. Each returned `data.capacity()` is at most the limit.
The existing `bomb_peak_allocation_is_bounded` keeps passing.
**Out of scope:** Changing `limits.max_output_bytes` or DEFAULT_MAX_OUTPUT_BYTES; streaming API changes; dictionary semantics (P2-08).
**Verification:** kukulu_verify with the files touched (runs `make conformance-check` once P2-12 has run); `cargo run --release -p oracle --example limit_probe -- 1048576 67108864 75497472 default` and quote its table in DESIGN.md and the log.
**Depends on:** none

### R1-03: Test the differential module's normalizations and cap rule

**Goal:** Make crates/oracle/tests/diff.rs hold the zero-offset normalization, the legacy-frame normalization's Legacy-kind guard and compare_block's oracle-over-cap rule. Each one survived a mutant (REVIEW F3).
**Files touched:** crates/oracle/tests/diff.rs, docs/DESIGN.md
**Design constraints:** SPEC §6.4 ("the differential test encodes it"), §1 property 2.
Three mutants of crates/oracle/src/diff.rs survived `cargo test --workspace`:
- deleting the `(Err(Error::ZeroOffset), Decoded::Accepted(_)) => Some("zero-offset")` arm (line 62);
- deleting `Ok(v) if v.len() > max_output => Decoded::OverCap` in compare_block (line 170);
- widening `ours.kinds.contains(&FrameKind::Legacy)` to `!ours.kinds.is_empty()` (line 59).
tests/eob.rs keeps its own inline zero-offset exception and never reaches diff.rs. Change diff.rs only if a test exposes a real defect. Correct DESIGN.md's Differential rules "Held by" lists to name the new tests.
**Acceptance tests:** crates/oracle/tests/diff.rs:
- `zero_offset_block_is_agree_through_its_named_normalization`: a crafted block with a zero offset that the oracle accepts (build it with craft::build_block; if no crafted form is accepted, use a corruption of a small oracle block, as tests/eob.rs's sweep found). compare_block gives Agree { accepted: false }, normalization_for names "zero-offset", and the same block inside a hand-built frame through compare_frame_using gives (Agree, Some("zero-offset")).
- `legacy_normalization_requires_a_legacy_frame`: judge of cleanlz4 Ok with kinds [Regular] and with [Skippable, Regular] against an oracle reject is Diverge.
- `compare_block_skips_when_only_the_oracle_goes_over_the_cap`: a block the oracle accepts with more than max_output bytes while cleanlz4 rejects it with a non-limit error (for example a zero offset followed by a long match) gives Skipped("output-cap").
Each test must fail under the mutant it targets.
**Out of scope:** New normalizations; changing cleanlz4's verdicts; tests/eob.rs's own loop.
**Verification:** kukulu_verify with the files touched; kukulu_mutate of each of the three mutants above is killed.
**Depends on:** none

### R1-04: Assert the Dict-ID probe's fixed verdict once dictionaries exist

**Goal:** Replace the Dict-ID probe exemption in every_probe_is_agree_or_a_named_normalization with a direct check that cleanlz4 itself returns DictionaryRequired.
**Files touched:** crates/oracle/tests/diff.rs
**Design constraints:** SPEC §6.2 (dictionary required); PLAN P2-09's fixed verdict "Dict-ID without a dictionary → reject". tests/diff.rs:51-54 skips the verdict check for the "Dict-ID and no dictionary" probe because P2-08 had not run. Through compare_frame the retry with an empty dictionary makes the verdict "accepted" by design, so the check for this probe must call cleanlz4 directly, not read the comparison's verdict.
**Acceptance tests:** crates/oracle/tests/diff.rs `every_probe_is_agree_or_a_named_normalization`: for the "Dict-ID and no dictionary" probe, `cleanlz4::decompress_frame_with(&p.bytes, &DecodeOptions::default())` is `Err(Error::DictionaryRequired)`, and the comparison is still Agree. Every other probe keeps its fixed-verdict assertion. This fails before P2-08 and passes after it.
**Out of scope:** diff.rs's retry rule; docs/spikes/P2-09.md (a record).
**Verification:** kukulu_verify with the files touched.
**Depends on:** P2-08

### R1-05: Streaming ReturnRemainder leaves a trailing magic prefix to the caller

**Goal:** Under TrailingData::ReturnRemainder, make the streaming decoder leave the caller exactly the bytes the one-shot path leaves unconsumed, even when the trailing run begins with one to three bytes of a known magic number (REVIEW F4).
**Files touched:** crates/cleanlz4/src/stream.rs, crates/cleanlz4/tests/stream.rs, docs/DESIGN.md
**Design constraints:** SPEC §6.1 (trailing-data policy: return the remainder), §2 (identical however chunked); Frame format, "Frame Concatenation". Today stream.rs:323-344 pushes each magic-prefix byte into `buf` and advances `pos`. After a frame, a tail of `04 22 4D 00 AA` therefore ends with three tail bytes reported as consumed, while decompress_frame_with stops before them.
Choose the mechanism: do not report prefix bytes as consumed until the magic is confirmed, or expose the held-back bytes. Record the choice in DESIGN.md with its section. Keep P3-02's other boundary rules and every existing stream test unchanged in result. Under TrailingData::Error the result stays TrailingData. The invariants apply.
**Acceptance tests:** crates/cleanlz4/tests/stream.rs `return_remainder_leaves_a_trailing_magic_prefix_unconsumed`: a valid frame followed by each of these tails: `04 22 4D 00 AA` (regular-magic prefix), `50 2A 4D 00` (skippable prefix), `02 21 4C 00` (legacy prefix) and `04` alone with end_of_input. Feed each whole, bytewise, and in random splits; the harness keeps unconsumed bytes and re-presents them with the next piece. Under ReturnRemainder, the bytes left to the caller equal `input[one_shot.consumed..]` of decompress_frame_with with the same options, and the decoded bytes are equal. The lone `04` with end_of_input is TruncatedInput on both paths, as P2-06 decided.
**Out of scope:** The one-shot path's rules; changing which tails are TruncatedInput.
**Verification:** kukulu_verify with the files touched (runs `make conformance-check`).
**Depends on:** P3-02

### R1-06: Frame suites: every level meets every block size

**Goal:** Reorder P2-10's 64 flag combinations so the compression level `frame_levels[i % len]` meets every block size as well as both values of every other flag, as PLAN P2-10 intended (REVIEW F5).
**Files touched:** crates/oracle/src/corpus.rs, crates/oracle/tests/corpus.rs, testdata/MANIFEST.json, docs/DESIGN.md, docs/EVIDENCE.md
**Design constraints:** SPEC §6.3; PLAN P2-10 ("so every level meets every flag value"). Today corpus.rs:147-160 varies block_size_id fastest, so with four levels each level is tied to one block size. Pick an order in which block size is not the fastest-varying flag. Generation stays deterministic from fuzz.seed. Regenerate with `make corpus` and commit the new MANIFEST.json. Replace DESIGN.md's P2-10 interpretation with the new rule.
If testdata/results/conformance-P2-12.json exists when this task runs: write a fresh full result, `make conformance TASK=<this task's id> CATEGORIES=all`, in the same commit (results protocol), and add it to docs/EVIDENCE.md. If that run would take more than ten minutes in the foreground, block instead (no extraVerify entry exists for it).
**Acceptance tests:** crates/oracle/tests/corpus.rs `every_frame_level_meets_every_block_size`: in the synthetic and frames-cantrbry suites, every (level, block_size_id) pair from conformance.frame_levels × 4..=7 appears, and every level appears with both values of linked, block checksum, content checksum and content size. This fails on today's manifest. The existing corpus tests stay green.
**Out of scope:** frames-silesia's sampled combinations; decoder changes; P2-11's suites.
**Verification:** kukulu_verify with the files touched; `make corpus` and `make conformance-check` exit zero; `cargo test -p oracle --test results` passes.
**Depends on:** none

### R1-07: Refresh the evidence index after the reopened tasks

**Goal:** Bring docs/EVIDENCE.md's evidence list, Results table and Done checklist up to date with the result files the reopened tasks wrote.
**Files touched:** docs/EVIDENCE.md, README.md
**Design constraints:** SPEC §1 (Done means), §5.4, §3.6; PLAN P5-01's layout. Read every tracked file under testdata/results/ and quote its headline counts. Re-state each of SPEC §1's five Done items as met, not met or owed to the summarize stage, with its file. Name any task still blocked or skipped. Add testdata/malformed/index.json to the fenced list if it now exists. Do not call kukulu_similarity or kukulu_attest.
**Acceptance tests:** crates/oracle/tests/results.rs `evidence_index_lists_every_result_file` passes, and every path in the fenced list exists.
**Out of scope:** The similarity score, the attestation bundle, SUMMARY.md.
**Verification:** kukulu_verify with the files touched.
**Depends on:** P2-12, P2-13, P4-09

## Review fixes (round 2)

### R2-01: Test that a legacy frame's output capacity stays within the limit

**Goal:** Hold SPEC §1 property 3 and §2 (bounded memory under the output limit) for legacy frames with a test, so the capacity cap R1-02 put in decode_legacy_frame cannot be removed silently (REVIEW round 2 F1).
**Files touched:** crates/cleanlz4/tests/limits.rs, docs/DESIGN.md
**Design constraints:** SPEC §1 property 3, §2 (output limit goal); Frame format, "Legacy frame". crates/cleanlz4/src/frame.rs:467 `append_capped(out, &block, *remaining)?;` in decode_legacy_frame survived the mutant `out.extend_from_slice(&block);` because every R1-02 test uses regular frames. Build the legacy frame by hand in the test file (magic 0x184C2102, then 4-byte little-endian compressed sizes each followed by a compressed block; cleanlz4 has no dev-dependencies). A block that decodes to exactly 8 MiB can be one literal followed by a long offset-1 match and a final literal-only sequence that satisfies the end-of-block rules DESIGN.md records. Change frame.rs only if the test exposes a real defect. Add the new test to the 'Held by' list of DESIGN.md's Output limit entry. The invariants apply.
**Acceptance tests:** crates/cleanlz4/tests/limits.rs `legacy_output_capacity_never_exceeds_the_limit`: a hand-built legacy frame of three blocks that each decode to 8 MiB plus a fourth that decodes to a few bytes, decoded with decompress_frame_with and max_output_bytes equal to the total output length (24 MiB plus those bytes). The output equals the expected bytes and `data.capacity()` is at most the limit. The test must fail under the mutant above (amortized doubling reaches 32 MiB).
**Out of scope:** Streaming legacy frames (P3-02); the decoder's scratch-buffer reuse; changing limits.max_output_bytes.
**Verification:** kukulu_verify with the files touched; kukulu_mutate of crates/cleanlz4/src/frame.rs replacing `        append_capped(out, &block, *remaining)?;` with `        out.extend_from_slice(&block);` is killed by the new test.
**Depends on:** none

### R2-02: Test the frame-combination generator itself, and the manifest against it

**Goal:** Make crates/oracle/tests/corpus.rs check frame_combos() and frame_level() directly, and check that the committed manifest's frame entries match what they generate, so a generator change or a stale MANIFEST.json fails cargo test (REVIEW round 2 F2).
**Files touched:** crates/oracle/tests/corpus.rs
**Design constraints:** SPEC §6.3 ("Every combination is recorded in testdata/MANIFEST.json"); PLAN P2-10 and R1-06. Two mutants of crates/oracle/src/corpus.rs frame_combos survived because every coverage test reads the committed manifest, which cargo test never regenerates: `block_size_id: 4 + ((hi + lo) % 4) as u8` to `4 + (hi % 4) as u8`, and `linked: hi & 8 != 0` to `hi & 4 != 0`. Read levels from Config (conformance.frame_levels); write no level or count as a literal. Do not regenerate the manifest unless a test shows it is stale. The invariants apply.
**Acceptance tests:** crates/oracle/tests/corpus.rs: (1) `frame_combos_are_distinct_and_every_level_meets_every_block_size`: frame_combos() has 64 pairwise-distinct entries, and for each level index k, the combinations i with frame_level(&cfg, i) equal to that level cover every block_size_id 4..=7 and both values of linked, block_checksum, content_checksum and content_size. This test calls no manifest. (2) `manifest_frame_entries_match_the_generator`: for each input of the synthetic and frames-cantrbry suites, the set of (level, block_size_id, linked, block_checksum, content_checksum, content_size) in its manifest entries equals the set {(frame_level(&cfg, i), frame_combos()[i] fields) for i in 0..64}. Both tests must fail under each mutant above.
**Out of scope:** frames-silesia's sampled combinations beyond what draw_combinations already tests; changing the order R1-06 chose; corpus.rs itself.
**Verification:** kukulu_verify with the files touched; kukulu_mutate (targeted cargo, filter frame_combos_are_distinct_and_every_level_meets_every_block_size, then manifest_frame_entries_match_the_generator) of each of the two corpus.rs mutants above is killed.
**Depends on:** none

## Review fixes (round 3)

### R3-01: Streaming decoder: skippable, legacy and concatenated frames and the trailing-data policy (no dictionaries)

**Goal:** Make the streaming Decoder treat frame boundaries as the one-shot path does, so the streaming and one-shot APIs give identical output on skippable, legacy and concatenated input (REVIEW round 3 F1). This is P3-02 without its dictionary part.
**Files touched:** crates/cleanlz4/src/stream.rs, crates/cleanlz4/src/frame.rs, crates/cleanlz4/tests/stream.rs, docs/DESIGN.md
**Design constraints:** SPEC section 2 (identical output however chunked), 6.1 (decompress_frame decodes concatenated frames and skips skippable frames; Streaming statuses); Frame format, "Frame Concatenation", "Skippable Frames", "Legacy frame". The rules are the one-shot path's (P2-06, P2-07 and DESIGN.md's Decisions), restated in PLAN P3-02. At a frame boundary: 0 bytes with end_of_input is Done. A known magic starts that frame. 1 to 3 bytes that begin a known magic wait for more, or are TruncatedInput at end_of_input. Anything else is BadMagic for the first frame; after that it is TrailingData, or under ReturnRemainder it is Done with the bytes left unconsumed. Skippable frames are skipped by counting down their declared size and are never buffered. A legacy frame ends at end of input, at a known magic or at a zero block size; at a legacy block boundary with no buffered bytes and end_of_input false, the status is NeedsInput. FrameDone is reported for skippable and legacy frames too. Reuse frame.rs primitives on whole units, as P3-01 does; adjust their signatures only as far as reuse needs. The output limit already in stream.rs must keep holding: a legacy block is decoded with max_len = min(8 MiB, remaining allowance), as in the one-shot path. Keep CLAUDE.md's invariants: no assert!, no index slicing, no unchecked arithmetic on input-derived values in library code. Update DESIGN.md's 'Streaming frame boundary (this task only)' entry to the full rule, citing the Frame format sections.
**Acceptance tests:** crates/cleanlz4/tests/stream.rs: skippable_user_data_is_skipped_without_buffering (10 MiB of declared user data fed in 1 MiB pieces; the decoder's memory does not grow with it, which is checked by the bytes consumed per call and by no error), legacy_frame_ends_at_end_of_input, legacy_frame_then_regular_frame, concatenated_frames_report_frame_done_each, skippable_frame_first_streams_like_one_shot, return_remainder_stops_before_the_trailing_bytes, trailing_garbage_is_trailing_data, every_one_shot_test_frame_matches_when_streamed_bytewise (each valid and invalid frame that tests/frame.rs builds, except those with a Dict-ID, gives the same bytes or the same Error variant when streamed one byte at a time). Each of these fails on HEAD because step_boundary accepts only a regular magic.
**Out of scope:** Dictionaries and Dict-ID handling (P2-08 and P3-02's two dictionary tests stay with P1-10's chain); the oracle-driven chunking property test (P3-03); scratch-buffer reuse; changing the one-shot path's behaviour.
**Verification:** kukulu_verify with the files touched (make conformance-check runs for crates/cleanlz4/src/ and must stay green); kukulu_mutate of stream.rs that removes the skippable-magic arm of the frame boundary is killed by skippable_frame_first_streams_like_one_shot.
**Depends on:** none

### R3-02: The malformed corpus and its results, without the Dict-ID case

**Goal:** Build SPEC section 6.4's malformed corpus by hand, each file with its expected Error variant, and commit testdata/results/malformed-R3-02.json (REVIEW round 3 F2). This is P2-13 without its one dictionary case.
**Files touched:** crates/oracle/src/malformed.rs, crates/oracle/src/bin/malformed.rs, crates/oracle/src/lib.rs, crates/oracle/tests/malformed.rs, Makefile, testdata/malformed/, testdata/results/malformed-R3-02.json, docs/DESIGN.md, docs/EVIDENCE.md
**Design constraints:** SPEC sections 6.4, 5.4 (malformed keys exactly: task, total, matched_expected_error, panics), 6.2; CLAUDE.md results protocol (the binary writes the file, staged in the same commit; quote the headline counts in the log). Everything in PLAN P2-13's design constraints applies: files built deterministically with craft and craft_frame; each breaks exactly one rule and is otherwise valid; testdata/malformed/index.json lists { file, kind, expected, rule, max_output }; the cases are P2-13's, minus 'a Dict-ID with no dictionary'; `cargo run -p oracle --bin malformed -- --write` regenerates the files and `-- --task <id>` runs them under catch_unwind and writes the result; the Makefile gains `malformed` (TASK). Read no number that is a config key as a literal. Add the result file to docs/EVIDENCE.md's fenced list (evidence_index_lists_every_result_file requires it).
**Acceptance tests:** crates/oracle/tests/malformed.rs: every_malformed_file_returns_its_expected_variant, no_malformed_file_panics, index_and_directory_agree, malformed_files_are_reproducible (regenerated into a temporary directory, byte-identical), every_expected_name_is_an_error_variant, each_frame_file_is_agree_or_a_named_normalization_under_compare. These would have caught F2: none of them exists on HEAD.
**Out of scope:** Decoder changes (a wrong variant blocks, naming the files); the Dict-ID-without-dictionary case (stays with P2-13 and P1-10's chain); fuzzing.
**Verification:** kukulu_verify; `make malformed TASK=R3-02` exits zero with matched_expected_error == total and panics == 0; `cargo test -p oracle --test results` passes with the new file.
**Depends on:** none
