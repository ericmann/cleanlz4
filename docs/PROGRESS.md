# cleanlz4 build progress

Branch: build/2026-11-10
Started: 2026-11-10T00:53:38.909Z

## Tasks

- [x] P0-01 Cargo workspace, make check, the deps script and the config file
- [x] P0-02 Verify docs/spec against SOURCES.json
- [x] P0-03 Fetch the corpora, write MANIFEST.json and go offline
- [x] P0-04 Phase 0 end: push and record the manual check
- [x] P1-01 Oracle FFI: the block functions
- [x] P1-02 Oracle FFI: frame compression and decompression
- [x] P1-03 The typed error enum
- [x] P1-04 XXH32, one-shot and streaming
- [x] P1-05 Block decoder: sequences, overlap copies and strict end-of-block rules
- [x] P1-06 Spike: the oracle's end-of-block behaviour
- [x] P1-07 Align the end-of-block policy with the oracle
- [ ] P1-08 Dictionary-prefixed block decoding
- [ ] P1-09 Synthetic inputs and block round-trip tests against the oracle
- [ ] P1-10 Oracle dictionary blocks and the dictionary differential test
- [ ] P1-11 Conformance generator: the block suite and the manifest's entries
- [ ] P1-12 Conformance runner and the results for category block
- [ ] P1-13 DESIGN.md: the Phase 1 spec-rule report
- [ ] P1-14 Phase 1 end: push and record the manual check
- [ ] P2-01 Frame descriptor parsing and introspection
- [ ] P2-02 Options, and one regular frame with independent blocks
- [ ] P2-03 Linked blocks
- [ ] P2-04 Block and content checksums
- [ ] P2-05 Content size
- [ ] P2-06 Concatenated frames, skippable frames and trailing data
- [ ] P2-07 The legacy frame
- [ ] P2-08 Preset dictionary and Dict-ID
- [ ] P2-09 Spike: the oracle's frame behaviour, and the differential compare module
- [ ] P2-10 Conformance generator: oracle-compressed frame suites
- [ ] P2-11 Conformance generator: dictionary, skippable, legacy and concatenated suites
- [ ] P2-12 The full conformance run and its results
- [ ] P2-13 The malformed corpus and its results
- [ ] P2-14 Phase 2 end: push and record the manual check
- [ ] P3-01 Streaming decoder: regular frames
- [ ] P3-02 Streaming decoder: skippable, legacy and concatenated frames, dictionaries and trailing data
- [ ] P3-03 Chunking-invariance property test
- [ ] P3-04 The output limit and bounded memory
- [ ] P3-05 Tune limits.max_output_bytes
- [ ] P3-06 Tune stream.chunking_cases
- [ ] P3-07 Phase 3 end: push and record the manual check
- [ ] P4-01 Fuzz targets (a): no panic on arbitrary bytes
- [ ] P4-02 Fuzz targets (b) and (c): differential
- [ ] P4-03 The fuzz runner, its results writer and regression replay
- [ ] P4-04 Run no_panic_block for the configured duration
- [ ] P4-05 Run no_panic_frame for the configured duration
- [ ] P4-06 Run no_panic_stream for the configured duration
- [ ] P4-07 Run diff_roundtrip for the configured duration
- [ ] P4-08 Run diff_accept for the configured duration
- [ ] P4-09 Triage crashes and divergences
- [ ] P4-10 Benchmarks against the oracle
- [ ] P4-11 Tune bench.repeats
- [ ] P4-12 Tune fuzz.seconds_per_target
- [ ] P4-13 Phase 4 end: push and record the manual check
- [ ] P5-01 The evidence index and the Done checklist
- [ ] P5-02 Phase 5 end: push and record the manual check

## Log

(one entry per task, appended by implement)

### P0-01 — 143d24b

Workspace, fuzz crate, make check, deps script, config.json created; cargo build and fuzz build ok (lz4-sys 1.11.1 resolved). Tests: oracle/tests/config.rs x3, prng unit test, test-check-deps.sh. Interpretation: none. Config struct sections: limits, fuzz, bench, stream, dict, conformance, diff.

### P0-02 — e26b635

verify-spec-sources.mjs and its test added; real spec files match (3 files ok). Makefile check extended. Interpretation: none.

### P0-03 — cb81e13

Lockdown: network off from this commit. Archive SHA-256: cantrbry.tar.gz f140e8a5b73d3f53198555a63bfb827889394a42f20825df33c810c3d5e3f8fb; silesia.zip 0626e25f45c0ffb5dc801f13b7c82a3b75743ba07e3a71835a41e3d9f63c77af. fetched_at 2026-11-10T00:56:35Z. Tools: lz4 CLI v1.9.4, binding lz4-sys 1.11.1+lz4-1.10.0. Corpus dirs under testdata/corpora/{cantrbry,silesia}; manifest corpus names "canterbury","silesia". oracle::manifest module has Manifest::load/save, sha256_hex. .cargo/config.toml offline.

### P0-04 — 4a0c1cd

Push: done (origin, build branch). Manual check: NOT VERIFIED (human): make check is green on the empty crates; the guard self-test passes; testdata/MANIFEST.json names both corpora, the binding crate's version and the lz4 CLI's version; .cargo/config.toml sets net.offline

### P1-01 — 559d84d

oracle::ffi has compress_bound/compress_default/compress_hc/decompress_safe (safe slice APIs, c_int); oracle::block has Rejected, compress_default, compress_hc(src, level), compress_bound, decompress_safe(src, capacity). Binding signatures guessed from convention, compiled first time. Tests: tests/ffi_block.rs x4.

### P1-02 — dbc95ae

oracle::frame: Prefs (Default impl), compress_frame(chunks,&Prefs), Decoded{Accepted,Rejected,OverCap}, decompress_frames(input,cap), content_checksum_of. ffi.rs: CompressCtx/DecompressCtx RAII, frame_is_error, frame_error_name, frame_compress_bound. Binding exports the version constant. Tests: ffi_frame.rs x7. Binding had all needed preferences fields incl. block checksum.

### P1-03 — 26e5f16

cleanlz4::Error (17 unit variants, Copy), Error::ALL, Display, std Error, Result alias, re-exported from lib. Tests: error.rs x3. DESIGN entry for BlockSizeExceeded.

### P1-04 — 103574e

cleanlz4::xxh32::{xxh32(input,seed), Xxh32{new,update,digest}}. Matches oracle content checksum for 0..=300 and large inputs, header-checksum byte and block checksum. Uses slice::as_chunks (clippy chunks_exact_to_as_chunks lint demands it for const sizes). Tests: 3 unit + 5 oracle. DESIGN entries per Step added.

### P1-05 — 13b602b

block.rs: pub(crate) decode_block(src,&mut Vec,max_len), decode_block_with(.., EndOfBlockPolicy) (policy fields last_sequence_is_literals_only, last_five_are_literals, last_match_twelve_from_end; const EOB all true); lib.rs decompress_block(input,max_output). Interpretation: final token low nibble ignored; length/limit check precedes literal-availability check (so a huge claimed literal length gives OutputLimitExceeded). Matches copy in passes of <=offset bytes. 14 unit tests.

### P1-06 — 8245176

Spike docs/spikes/P1-06.md. Policy observed: rule 1 enforce; rule 2 enforce; rule 3 DO NOT enforce (R3/9..11 and L1/11,12 rejected at exact capacity, accepted with +65536 slack); empty input enforce (reject); single 0x00 accepted; final low nibble not enforced (accepted). Oracle-side differentials must decode with slack capacity. craft.rs: Seq, build_block(seqs, final, nibble), build_block_without_final, expected_output. Note: oracle rejects short-final-run test blocks, so crafted tests need >=12 bytes from last match start to end.

### P1-07 — 9ffab37

EOB policy now: rule1 on, rule2 on, rule3 OFF (per spike); empty input rejected; low nibble ignored. Differentials agree on probes, all prefixes and single-byte corruptions (3 inputs). Interpretation / differential rule: oracle ACCEPTS a zero offset (found by corruption sweep) while cleanlz4 returns ZeroOffset per the document's "invalid"; allowed divergence recorded in DESIGN Differential rules; P2-09 diff.rs must carry it.
