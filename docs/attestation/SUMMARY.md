# Summary: build/2026-11-10

**Merge:** `build/2026-11-10`, base `0211bbf` → head `1979257`, 172 commits. 4 reviews; the final one (Round: 4) is **APPROVED**. 65 of 65 tasks done, none blocked or skipped.

## What was built

- **Phase 0, scaffolding and inputs (P0-01 to P0-04).** Sets up the Cargo workspace (`crates/cleanlz4`, `crates/oracle`, `fuzz/`), `make check`, the dependency script and its self-test, the spec-source check and `testdata/config.json`. It fetches the Canterbury and Silesia corpora into `testdata/MANIFEST.json` with their hashes, then switches cargo offline (`.cargo/config.toml`, `net.offline = true`).
- **Phase 1, XXH32 and the block decoder (P1-01 to P1-14).** Adds the oracle FFI over the permitted C functions, the typed `Error` enum (17 unit variants), XXH32 (one-shot and streaming), and the block decoder (overlap copies, dictionary prefix, end-of-block rules chosen by the P1-06 spike). Also adds the block conformance suite and runner, with `conformance-P1-12.json` at 156 of 156, and the DESIGN.md rule table. P1-10 builds the dictionary oracle on the `lz4` program after the owner decision.
- **Phase 2, frames (P2-01 to P2-14).** Adds frame descriptor parsing, independent and linked blocks, block and content checksums, content size, concatenated and skippable frames, trailing-data policy, legacy frames and preset dictionaries with Dict-ID. Adds the differential compare module (`diff.rs`, spike P2-09), the oracle-compressed, dictionary, skippable, legacy and concatenated conformance suites, and the full run (`conformance-P2-12.json`, 2349 of 2349 across ten categories). Adds the hand-built malformed corpus (`malformed-P2-13.json`, 144 of 144, 0 panics).
- **Phase 3, streaming and limits (P3-01 to P3-07).** Adds the streaming `Decoder` state machine, which covers every frame kind, dictionaries, trailing data and `Decoder::remainder`. Adds a chunking-invariance property test over `stream.chunking_cases` cases, and the output limit with bounded memory (limit + one block + 65,536 bytes, measured). `stream.chunking_cases` was tuned.
- **Phase 4, fuzzing and measurement (P4-01 to P4-13).** Adds five `cargo-fuzz` targets and a runner. Each target ran for about an hour: 0 crashes in total. `diff_accept` found 100 divergences, all one case, triaged in DESIGN.md; its re-run found 0. Adds benchmarks against the oracle (`bench-P4-10.json`, `bench-P4-11.json`) and tuning records for `bench.repeats` and `fuzz.seconds_per_target`.
- **Phase 5, evidence (P5-01, P5-02).** Adds the evidence index `docs/EVIDENCE.md` and SPEC §1's Done checklist. The similarity score and the attestation are done in the summarize stage.
- **Review fixes (R1-01 to R1-07, R2-01, R2-02, R3-01, R3-02).** A config literal was replaced by a read from the config. Output, scratch and history capacity are now held to the memory bound. Tests were added for the untested `diff.rs` normalizations, for the legacy capacity and for the frame-combination generator. The combination order was fixed so every level meets every block size. The streaming frame-boundary rules were brought in line with the one-shot path. The malformed corpus was built early.

## Decisions that shaped it

From PLAN.md's Decisions:
- Plan: six phases run serially, with no parallel streams, because the corpora are untracked and the build is offline after P0-03. Flight slug `cleanlz4`, base `main`.
- SPEC §9 Q1: cleanlz4 enforces an end-of-block rule exactly when the oracle rejects that rule's probes (spike P1-06, then P1-07). Result: rules 1 and 2 are enforced, rule 3 is not; an empty block is rejected and `0x00` is accepted.
- SPEC §9 Q2: the binding crate is `lz4-sys`, with its Rust names learned from compiler diagnostics only. **Owner decision (P1-10, commit `9461374`):** the binding lacks `LZ4_decompress_safe_usingDict`, so the `lz4` program, run as a black box, is the dictionary oracle, and `ffi.rs` is unchanged.
- SPEC §9 Q3: `dict.sizes_bytes` defaults to `[4096, 65536, 131072]`.
- Tunables: all live in `testdata/config.json`. `DEFAULT_MAX_OUTPUT_BYTES` is held equal to `limits.max_output_bytes` by a test.
- Dependencies: every crate was downloaded in P0-01, and cargo went offline at the end of P0-03. The oracle uses only `lz4-sys`, `sha2`, `serde` and `serde_json`. The PRNG and benchmarks are hand-rolled.
- Corpora and the generated conformance corpus are gitignored. The manifest's hashes stand in for them, and generation is deterministic from `fuzz.seed`.
- `crates/cleanlz4` has no dev-dependencies. Tests that need the oracle, SHA-256 or JSON live in `crates/oracle/tests/`.
- XXH32 vectors come only from the oracle's frame content checksums (seed 0). Seeds other than 0 are not oracle-checked.
- The `Error` enum has 17 unit variants. `DictionaryRequired` and `DictionaryMismatch` are separate, and `BlockSizeExceeded` was added.
- Frame boundary rule: 0 bytes left means done. 1 to 3 bytes that begin a known magic are `TruncatedInput`. Any other bytes are `BadMagic` before the first frame, and fall under the trailing-data policy after it.
- Dictionaries: a dictionary is `{ bytes, id: Option<u32> }`. A Dict-ID with no dictionary is `DictionaryRequired`, and two different ids are `DictionaryMismatch`. A dictionary with no id is used for any regular frame. Legacy frames never use one.
- `verify_checksums = false` skips block and content checksums. The header checksum is always verified.
- Legacy frames: blocks are independent, each up to 8 MiB. A zero block size ends the frame. "All blocks full except the last" is not enforced.
- The one-shot and streaming drivers share primitives in `frame.rs`. `stream` imports `frame`, never the reverse.
- Differential normalizations are named in `diff.rs` and recorded in DESIGN.md. cleanlz4 stays strict wherever the spec says "must" or "shall".
- Fuzzing: five targets, each run as its own task. Conformance categories are assigned by the suite that made each entry, with a stated precedence.
- Operator settings: maxRounds was raised to 6 and maxRoundsHard to 8.

From HANDOFF.md's interpretation choices:
- P0-03: the corpus names are `canterbury` and `silesia`. The Canterbury files live in `testdata/corpora/cantrbry`.
- P1-05: the low four bits of the final token are ignored. The output-limit check comes before the literals-present check.
- P1-07: the oracle accepts a zero offset, but cleanlz4 rejects it (`ZeroOffset`). This is carried as the normalization `zero-offset`.
- P1-09: `ffi::compress_hc` passes real memory for empty input, because level 12 faulted on a dangling pointer.
- P1-10: `dict_block` compresses `[dict, data]` as linked blocks and keeps the second block.
- P1-11: entry `params` is a JSON object, paths are relative to `testdata/conformance/`, and there is a flattened `extra` map.
- P2-02: the block size field is checked against the maximum before the data is read.
- P2-05: the content size is validated, even when checksums are off.
- P2-06: 1 to 3 bytes that begin a magic are `TruncatedInput` under both trailing-data policies.
- P2-07: legacy blocks are independent. The 20 MiB output of `lz4 -l` decoded without carried history.
- P2-08: the dictionary's last 64 KiB is the prefix of every independent block, or the initial history of a linked frame. Streaming shares `FrameState::new`.
- P2-09: the normalizations are `legacy-frame` and `zero-offset`. A Dict-ID with no dictionary is retried with an empty dictionary. P4-09 later added `end-of-block-room`.
- P2-10 / R1-06: combination i uses hi = i / 4 and lo = i % 4. Flags come from the bits of hi, and the block size id is 4 + (hi + lo) % 4. This replaced P2-10's block-size-fastest order. R2-02 groups combinations by level value.
- P2-11: dictionaries are stored under `testdata/conformance/dictionary/`. Entries record `dict_file` and `dict_dependent`, and `dict_id` only for frames that carry one.
- P2-13: expected variants come from the rules, not from running the decoder. R3-02 re-ran the same 144 files.
- P3-01 / R1-05: held-back partial-magic bytes are exposed through `Decoder::remainder`.
- P3-04: a content size above the output limit fails at the header with `OutputLimitExceeded`.
- R1-02: an allocation failure maps to `OutputLimitExceeded`. A reallocation counts only its size change toward the bound.
- P3-06: `stream.chunking_cases` went from 10,000 to 80,000. At 10,000, 343 of 392 shapes were hit; at 80,000, 392 of 392.
- P4-01: `no_panic_block` splits the input into dictionary and block at `len * byte[2] / 256`. P4-02: `diff_roundtrip` packs the level index and chunk count into byte 1.
- P4-09: the 100 `diff_accept` divergences are resolved by the normalization `end-of-block-room`, with no decoder change. It covers any `EndOfBlockViolation` against an oracle accept.
- P4-11: the repeat counts compared were 3, 10 and 30. P4-12: `fuzz.seconds_per_target` was unchanged, with a recommendation recorded.

## Assumptions still in play

| Key | Final default | Status |
|---|---|---|
| `limits.max_output_bytes` | 1073741824 (1 GiB) | Measured in P3-05; kept |
| `fuzz.seconds_per_target` | 3600 | Measured in P4-12; kept. Recommendation: raise it, since `diff_roundtrip` was still finding coverage at 93% of its run. Still a guess for that target |
| `bench.repeats` | 10 | Measured in P4-11 (7.28% spread at 10, 7.36% at 30); kept |
| `stream.chunking_cases` | 80000 | Tuned in P3-06 (up from 10,000) |
| `dict.sizes_bytes` | [4096, 65536, 131072] | Not tuned (a plan choice) |
| `fuzz.smoke_seconds`, `fuzz.seed`, `fuzz.max_*`, `conformance.*`, `diff.max_output_bytes` | see `testdata/config.json` | Not tuned |

## Spec issues

Edits for the owner to make to SPEC.md:
- **§6.5 vs §9 Q2.** §6.5 still lists `LZ4_decompress_safe_usingDict`. The owner decision (the `lz4` program as the dictionary oracle) exists only in PLAN.md. Record it in SPEC so that §6.5 and the rule `oracle-calls-only-the-permitted-c-api` match what was built. Also update PLAN P2-08's stale line "the oracle cannot decode these frames".
- **Zero offsets.** The oracle accepts zero offsets that the Block format calls invalid. SPEC should say `docs/spec/` wins over the oracle.
- **Counting allocators.** The oracle protocol needs a carve-out for counting allocators (`unsafe impl GlobalAlloc`) in tests and examples.
- **"Bounded memory" accounting.** This build counts only a reallocation's size change. SPEC or DESIGN should state the rule.
- **Phase 5.** Similarity, attestation and SUMMARY belong to the summarize stage. Say so in §8.
- **§6.5 signatures.** §6.5 lists names, not signatures. List the signatures.
- **§6.5 dictionary and XXH32 functions.** §6.5 has no dictionary compressor, no dictionary frame decoder and no XXH32 function. Add them, or state the limit.
- **§6.2 variants.** §6.2 has no variant for a block above the Block Maximum Size. `BlockSizeExceeded` was added.
- **§5.1 tunables.** §5.1 omits `stream.chunking_cases` and `dict.sizes_bytes`.
- **§3.3 lint attribute.** The §3.3 attribute conflicts with "tests are exempt". `cfg_attr(not(test), …)` was used.
- **`lz4` version.** The `lz4` program is v1.9.4, while the Frame format describes v1.10.1 behaviour for the zero-size legacy end marker.
- **Legacy block independence.** The Frame format does not say whether legacy blocks are linked. Independent was chosen.
- **§6.1 dictionaries.** §6.1 ties a dictionary to a Dict-ID, but the Frame format allows an off-band dictionary.
- **§7 fuzz command.** The §7 fuzz command is a minimum. The runner adds `-a`, `-seed`, `-print_final_stats=1` and a corpus directory.
- **Hour-long verify calls.** They need a large `MCP_TOOL_TIMEOUT`. In practice the 1,800 s idle timeout still aborted them; see Pipeline friction.
- **§5.2 corpora.** §5.2 does not say whether the corpora are committed. They are not.
- **Lockdown and downloads.** `lockdown.after` is P0-03, but later crates needed downloads. Everything was resolved in P0-01.
- **Kūkulu version.** §1 names Kūkulu 0.7.x, but 0.8.0 is installed.
- **Known-gap candidates:** none. The one guard inconsistency this round is a refusal, not a gap: a read-only `sed -n` on `docs/spec/` is denied while `grep` on the same file is allowed.

## Manual checks owed

- **Phase 0:**
  - `make check` is green and the guard self-test passes.
  - `testdata/MANIFEST.json` names both corpora, `lz4-sys` 1.11.1 (library 1.10.0) and `lz4` CLI 1.9.4.
  - `.cargo/config.toml` sets `net.offline`.
- **Phase 1:**
  - Read DESIGN.md's rule table against the Block format and the XXH32 section.
  - Confirm each end-of-block decision follows `docs/spikes/P1-06.md`.
  - Confirm `conformance-P1-12.json` shows `passed == total`.
- **Phase 2:**
  - Confirm `conformance-P2-12.json` shows 2349 of 2349.
  - Confirm `malformed-P2-13.json` shows 144 of 144 with 0 panics.
  - Confirm each normalization in `docs/spikes/P2-09.md` and DESIGN's Differential rules cites a section.
- **Phase 3:**
  - Read `stream.rs` for ergonomics.
  - Run `cargo test -p oracle --test limits` and watch the bomb figures.
  - Read the P3-06 chunking coverage table.
- **Phase 4:**
  - Confirm 0 crashes in all five runs, and that the 100 `diff_accept` divergences are triaged in DESIGN.md `## Divergence triage` (the re-run found 0).
  - **Decide whether `end-of-block-room` is acceptable.** It hides every `EndOfBlockViolation` against an oracle accept, not only the four-literal case.
  - Decide whether `testdata/results/divergences/*.bin` stays committed next to `testdata/regressions/`.
  - Act on P4-12's recommendation: lengthen `diff_roundtrip`'s run and the timeouts in `docs/kukulu.json`.
  - Read `bench-P4-10.json` and `bench-P4-11.json` (actual figures only; there is no target).
- **Phase 5:**
  - `docs/SIMILARITY.md` and the attestation bundle exist, and `verify.mjs` passes.
  - Run the verifier from a clean checkout on a second machine.

## Review history

- **Round 1: CHANGES REQUESTED.** 6 findings, 7 fix tasks (6 from review, 1 unblock of P1-10), R1-01 to R1-07. Not non-converging.
- **Round 2: CHANGES REQUESTED.** 2 findings, 2 fix tasks (1 from review, 1 from mutation), R2-01 and R2-02. P1-10 was still blocked, which is recurring blocked work, not a recurring finding id.
- **Round 3: CHANGES REQUESTED, flagged non-converging.** 2 findings, 2 fix tasks (both counted as from blocked work), R3-01 and R3-02, both carved out of P1-10's skipped chain. P1-10 was blocked for the third round, until the owner decision in `9461374`.
- **Round 4: APPROVED.** 0 findings, 0 fix tasks. **This approval has notes:** two advisory mutation survivors (below); the stale line "oracle cannot decode dictionary frames" in PLAN P2-08 and the `dict_frame` doc; stale comments in `cli.rs` and `fuzz_logic.rs`; duplication between `is_magic` and `magic_kind`; `malformed-R3-02.json` duplicating P2-13; and a new scratch `Vec` allocated per block. None of these was queued as work.
- No fix-task id recurred between consecutive rounds' findings. The recurring item was the P1-10 block in rounds 1 to 3.

## Pipeline friction

1. review, tool-refusal: the reviewer's read-only `sed -n` on `.kukulu/cleanroom/log.jsonl` was denied as a protected path, though a `grep` on the same file had just been allowed. The skill tells the reviewer to read that log's denied entries, so the read cost a retry through the Read tool.
2. review, unblock-reason-ignored: round 1's unblock reason for P1-10 (do not declare `LZ4_decompress_safe_usingDict`; build `dict_block` without it) lived only in PROGRESS.md, while PLAN.md's P1-10 body still said a missing usingDict blocks. The implementer did the exact forbidden thing (declared it in `ffi.rs`) and re-blocked within a minute. The 17 reopened tasks were skipped again, which cost a whole round.
3. review, tool-refusal: the guard denied a read-only `sed -n 2910,2990p mcp/server.mjs` (cwd in the plugin cache) as "protected path: <plugin dir>/2910,2990p", resolving sed's script argument as a path. The reviewer did not retry another way.
4. review, tool-refusal: a read-only `sed -n` on `.kukulu/cleanroom/log.jsonl` piped to node was denied, though a `grep` was allowed. The skill gives no sanctioned way to read the log, so the reviewer relied on `kukulu_status`'s denied count.
5. review, tool-misconfig: `kukulu_mutate` with targeted `cargo` and a list filter expands to `cargo test --workspace 'a' 'b'`, which cargo rejects because it takes one TESTNAME. The result was inconclusive and one mutate call was wasted.
6. review, tool-refusal: a `sed -n` on the plugin's `mcp/server.mjs` was denied, though a `grep` had just been allowed. The reviewer wanted to learn how `kukulu_review_submit` treats CHANGES REQUESTED with zero tasks when the only open item is an owner block, and the skill does not say.
7. review, tool-refusal: `kukulu_review_submit` refused CHANGES REQUESTED with no tasks and no unblock, but the skill forbids APPROVED while a task is blocked. When the only open item is an owner-only block, the reviewer has no sanctioned verdict. The skill should say what to submit, for example halt for the owner.
8. implement, stall: `kukulu_verify` on a result path that runs `make fuzz-run` (3600 s) was aborted by the MCP idle timeout of 1800 s with no progress output, though the process kept running. The implementer had to poll for the result file by hand. Long-run `extraVerify` commands need progress notifications or a longer timeout.
9. review, tool-refusal: the guard refused a read-only `sed -n` on `docs/spec/lz4_Frame_format.md` as a "protected path" write, though `grep` on the same file had been allowed. The reviewer switched to the Read tool.

## Suggested CLAUDE.md additions

- The binding's names were learned from compiler output only. `LZ4_decompress_safe_usingDict` is absent from `lz4-sys` 1.11.1, and the `lz4` program is the dictionary oracle.
- The oracle's frame decoder rejects legacy frames ("unknown frame type") and accepts a Dict-ID frame without a dictionary.
- The oracle's block decoder needs spare output capacity. It rejects some valid blocks at exact capacity, and accepts a final run of four literals after a match of 19 or more bytes only with slack. Differential block tests give it 65,536 extra bytes.
- clippy under `-D warnings` wants `slice::as_chunks::<N>()` instead of `chunks_exact` with a constant, `std::iter::repeat_n`, `is_multiple_of` and `explicit_counter_loop` fixes. Run clippy before `kukulu_verify`.
- rustfmt reformats source, so scripted string patches fail silently. Check the file after patching.
- `copy_match` must not compute the in-buffer start when nothing is left after the dictionary part, because `dst.len() - offset` underflows.
- Counting allocators (`unsafe impl GlobalAlloc`) exist in `crates/oracle/tests/limits.rs` and `examples/limit_probe.rs`. The "only `ffi.rs` has `unsafe`" rule applies to C calls.
- `make corpus` takes about 90 to 110 s and leaves about 1 GB under `testdata/conformance/`. `MANIFEST.json` regenerates byte for byte.
- `record_divergence` stops writing at `fuzz.max_divergence_files`. Empty the divergence directory before a re-run, or new divergences can be hidden.
- Bytewise property tests over frames of 100 KB or more take minutes. Sample large frames.
- `kukulu_feedback_log` commits and moves HEAD, so call it after `kukulu_task_done`.
- Every commit carries `Kukulu-Stage` and `Kukulu-Task` trailers.

## Advisory mutation survivors

- `crates/oracle/src/dict.rs`: `Ok(plain) => plain != data` changed to `Ok(_) => true` survives. A block that decodes correctly without the dictionary would then be labelled `dict_dependent`. This is metadata in the manifest and does not affect any decoder verdict. A test could assert that a block of data with no shared text is reported as not dependent.
- `crates/oracle/src/runner.rs`: dropping the dictionary from frame entries (`contains_key("dict_file")`) survives `cargo test --workspace`. Only a conformance run would catch it, and it fails loudly rather than passing falsely: Dict-ID frames would return `DictionaryRequired`. `make conformance-check` is bound to `crates/cleanlz4/src/` only, so it does not run when only the runner changes.

## Breaking call

Breaking: no. This is the first implementation in the repository: the base held only docs and the spec. There is no existing public interface, config default or user behavior to break, and there is no CHANGELOG.md.
