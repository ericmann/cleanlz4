# Design notes

## Decisions

- Lint attributes use the `cfg_attr(not(test), deny(...))` form in `crates/cleanlz4/src/lib.rs`, so the no-panic lints bind library code while tests may use `unwrap`, `expect` and indexing (SPEC section 3.3). Implements: project rule on no reachable panic; held by the `no-panic-lint-suppression` constraint and by `make check` (clippy).
- Tool versions are recorded in `testdata/MANIFEST.json` `tools`: the oracle library version is read from `Cargo.lock` (the SPEC section 6.5 surface has no version function), and the installed `lz4` program's first `-V` line is recorded separately; the two may differ in version (CLI 1.9.4, library 1.10.0 at the time of recording). Implements: SPEC section 5.2 (corpus provenance); held by `crates/oracle/tests/manifest.rs` `manifest_records_tool_versions`.
- The manifest is written by the `manifest` binary, never by hand; cleanlz4's own measurement (a SHA-256 over each downloaded archive and each extracted file) is the provenance. Held by `corpus_files_on_disk_match_manifest`.
- The oracle's block functions are wrapped in `crates/oracle/src/ffi.rs` as safe slice-taking functions, the only place with `unsafe`; `crates/oracle/src/block.rs` adds length checks (lengths beyond a C `int` panic, as this is test tooling) and a unit `Rejected` error. Implements: SPEC section 6.5 (Block); cleanlz4's own tooling decision. Held by `crates/oracle/tests/ffi_block.rs`.
- The oracle's frame functions are wrapped by RAII contexts in `crates/oracle/src/ffi.rs`; `crates/oracle/src/frame.rs` offers `compress_frame`, `decompress_frames` and `content_checksum_of`. The context version argument is the constant the binding exports. XXH32 vectors come only from the last four bytes of a frame written with the content checksum on. Implements: Frame format, Content Checksum; SPEC section 6.5 (Frame compression, Frame decompression). Held by `crates/oracle/tests/ffi_frame.rs`.
- `Error::BlockSizeExceeded` exists as a seventeenth variant because the frame format forbids a data block larger than the frame's maximum block size and SPEC section 6.2's list has no variant for it. Implements: Frame format, Data Blocks ("Block_Size shall never be larger than Block_Maximum_Size"). Held by `error.rs` unit tests; later by the frame tests.
- XXH32 Step 1 (xxHash spec, "Step 1. Initialize internal accumulators"): four accumulators from the seed, and the single-accumulator `seed + PRIME32_5` start when the total length is under 16. Held by `xxh32.rs` unit tests and `crates/oracle/tests/xxh32.rs` `matches_oracle_content_checksum_for_lengths_0_to_300`.
- XXH32 Step 2 (xxHash spec, "Step 2. Process stripes"): lanes read little-endian from 16-byte stripes with wrapping arithmetic; the streaming form buffers a partial stripe. Held by `streaming_equals_one_shot_for_every_split`.
- XXH32 Step 3 (xxHash spec, "Step 3. Accumulator convergence"): rotations 1, 7, 12, 18, applied only when at least one stripe was consumed (total length of 16 or more). Held by `matches_oracle_for_large_random_inputs`.
- XXH32 Step 4 (xxHash spec, "Step 4. Add input length"): the total length is kept as 64 bits and its low 32 bits are added. Held by `lengths_around_stripe_boundaries_differ`.
- XXH32 Step 5 (xxHash spec, "Step 5. Consume remaining input"): four-byte words, then single bytes. Held by the oracle length sweep 0 to 300.
- XXH32 Steps 6 and 7 (xxHash spec, "Step 6. Final mix (avalanche)" and "Step 7. Output"): the digest is the numeric value; the canonical big-endian form is not needed because the frame format stores checksums little-endian (Frame format, Header Checksum, Block checksum). Held by `header_checksum_byte_matches_oracle_frames` and `block_checksum_matches_oracle_frames`.
- XXH32 test vectors come only from the oracle: the last four bytes of a frame written with the content checksum on are XXH32 of the content with seed 0 (Frame format, Content Checksum). Seeds other than 0 cannot be checked against the oracle; they are covered only by `seed_changes_the_digest` and by streaming-equals-one-shot. Held by `crates/oracle/tests/xxh32.rs`.

## Spec rules implemented

## Differential rules

## Divergence triage

## Measurements
