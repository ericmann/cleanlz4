# Review: build/2026-11-10
Round: 1

## Verdict

CHANGES REQUESTED

The 36 completed tasks are mostly sound. `kukulu_verify` passed every check: all 14 constraints, with no fixture failures and no hits; fmt, clippy, tests, the deps scripts, spec sources and the guard self-test (568 cases). I also ran `make conformance-check` over every generated suite. It decoded 2,236 of 2,236 entries: block, synthetic, frames-cantrbry and frames-silesia. That run is evidence for this review only, not a result file. The verdict is not an approval for three reasons. One task is blocked, and 17 tasks were skipped because of it. The output-limit memory bound is wrong. One module's differential rules are untested.

## Findings (most severe first)

### F1. Category 1: a config value is written as a literal (R1-01)

- **Where:** `crates/oracle/src/bin/probe_frame.rs:6`, `const CAP: usize = 1 << 26;`.
- **What is wrong:** 67,108,864 is the value of `diff.max_output_bytes`. CLAUDE.md constraint 10 says the other keys of `testdata/config.json` are never written as literals either; it is reviewer-checked. SPEC §3.5 says the same. No `docs/kukulu.json` rule covers this key, so no fixture is missing (F-14 does not apply).
- **What breaks:** if the key is changed, the P2-09 spike binary keeps the old cap, and the spike's table no longer matches what `diff.rs` does.
- **Minimal fix:** read `Config::load()?.diff.max_output_bytes`. Add a test that would catch the next such literal.

### F2. Category 4 and 3: the output limit does not hold memory to the recorded bound, and its test cannot notice (R1-02)

- **Where:**
  - `crates/cleanlz4/src/frame.rs:278`, `out.extend_from_slice(&block)`: the one-shot output grows by amortized doubling, with no cap at the allowance.
  - `frame.rs:233-235`: the history is extended by a whole block, then drained, and `Vec` keeps the capacity.
  - `frame.rs:268`: a scratch `Vec` per block, whose `reserve` calls in `block.rs` can overshoot.
- **The recorded bound:** `docs/DESIGN.md:55` (P3-04) says memory is "the limit plus one block plus 65,536 bytes of history".
- **What I measured:** I ran the task's own probe with `cargo run --release -p oracle --example limit_probe -- 75497472 104857600`.

  | limit (bytes) | peak allocated (bytes) | peak above the limit (bytes) |
  |---|---|---|
  | 75,497,472 (72 MiB) | 150,994,934 | 75,497,462 |
  | 104,857,600 (100 MiB) | 150,994,934 | 46,137,334 |

  At a 72 MiB limit the peak is twice the limit. P3-05 measured only 1 MiB, 64 MiB and 1 GiB. Those are powers of two, where 4 MiB blocks fill the output buffer exactly, so P3-05 missed this. Its own 16,777,206-byte excess is already four times the recorded block-plus-window term.
- **The test cannot notice:** `crates/oracle/tests/limits.rs:83` `bomb_peak_allocation_is_bounded` uses a 1 MiB limit. There the first 4 MiB block fails before any large buffer exists, as DESIGN.md:129 admits. So the test does not exercise output growth, history or the scratch block. Deleting the cap on output growth could never fail it.
- **What breaks:** SPEC §1 property 3 and §2 ("bounded memory"), and §8 Phase 3 ("fails fast with bounded memory"). A caller who sets a limit of 1 GiB + 4 MiB can see about 2 GiB allocated.
- **Minimal fix:** in `frame.rs`, cap the output's capacity growth at the remaining allowance. Trim the history before extending it. Keep any per-block buffer within the block bound. Then test at a limit that lets several blocks decode before it fails (see R1-02), and correct DESIGN.md's bound and Measurements.

### F3. Category 3: three of `diff.rs`'s differential rules are untested (R1-03)

`kukulu_mutate` on `crates/oracle/src/diff.rs` (full `cargo test --workspace`):

| Mutant | Result |
|---|---|
| deleted the `zero-offset` arm (line 62) | **survived** |
| deleted `compare_block`'s `OverCap` arm (line 170) | **survived** |
| widened the `legacy-frame` arm (line 59) from `kinds.contains(&FrameKind::Legacy)` to `!kinds.is_empty()` | **survived** |
| changed the both-reject arm (line 89) to `Skipped` | killed |

- **Zero offset:** DESIGN.md says `zero-offset` is "Held by `every_probe_is_agree_or_a_named_normalization` and `tests/eob.rs`". No probe has a zero offset. `tests/eob.rs:68` keeps its own inline copy of the rule and never calls `compare_block`. So the normalization that P4-02's `diff_accept` target will depend on is untested.
- **Legacy frame:** the "only with the Legacy kind" check in `an_unlisted_disagreement_is_diverge` uses empty `kinds`. A Regular-only flow that cleanlz4 accepts and the oracle rejects would be hidden.
- **Requirement:** SPEC §6.4 says "the differential test encodes it". That is why this task's source is `review`, not `mutation`.
- **Minimal fix:** tests only, plus the DESIGN "Held by" correction.

### F4. Category 5: streaming `ReturnRemainder` reports trailing bytes as consumed (R1-05, after P3-02)

- **Where:** `crates/cleanlz4/src/stream.rs:323-344`. At a frame boundary, each byte that keeps a known-magic prefix is pushed to `buf`, and `pos` is advanced.
- **What is wrong:** after a frame, trailing bytes `04 22 4D 00` are three magic-prefix bytes and then a mismatch. Under `ReturnRemainder` the stream ends with those three bytes reported as consumed. `decompress_frame_with` stops before them. HANDOFF's P3-01 note ("when they arrive whole") acknowledges this.
- **What breaks:** SPEC §6.1's "return the remainder" and §2's "identical output however input and output are chunked". A caller cannot recover the remainder. P3-02 owns the streaming trailing-data policy but its tests do not name this case, so R1-05 depends on it.
- **Minimal fix:** a test that pins the case. Whatever P3-02 builds must make it pass.

### F5. Category 6: P2-10's combination order defeats the plan's stated intent (R1-06)

- **Where:** `crates/oracle/src/corpus.rs:147-160` with `:171-174`.
- **What is wrong:** PLAN P2-10 sets the level to `frame_levels[i % len]` "so every level meets every flag value". The implementer chose block size as the fastest-varying of the 64 combinations, which is the one order that ties each level to a single block size. Their HANDOFF note says as much. So level 12 never meets 64 KB blocks, and level 0 never meets 4 MB blocks, in the synthetic and Canterbury frame suites. Any order with block size not fastest meets the plan's sentence.
- **What breaks:** SPEC §6.3's combinations are thinner than the plan intended (high-compression output framed in small blocks is never generated). `every_frame_level_and_block_size_appears` checks each value separately, never the pairs.
- **Minimal fix:** reorder, regenerate, and test the pairs.

### F6. Category 7: the P1-10 blocker is real for one function only, and it holds back 17 tasks (unblock P1-10)

- **The blocker:** the binding crate has no `LZ4_decompress_safe_usingDict` (E0425). The implementer read no binding source; the compiler diagnostic is the evidence, and it fits the oracle protocol. SPEC §9 Q2 leaves this gap to the owner (see Spec issues).
- **Why most of P1-10 is not blocked:**
  - Its main piece, `dict::dict_block`, needs only `compress_frame` and `decompress_safe`.
  - The oracle can still confirm the dictionary semantics inside the permitted surface. Decoding the whole `[dict, data]` linked frame with `decompress_frames` makes the oracle apply the first block as the second block's history, which is exactly a dictionary-prefixed decode.
- **What it holds back:** P2-08 needs only `dict_block`, yet through P1-10 the whole chain was skipped: P2-08, P2-11 to P2-13, P3-02, P3-03, P3-06, P4-01 to P4-09 and P4-12. That chain is most of SPEC §1's Done list: full conformance, the malformed corpus, chunking invariance and the fuzz campaign.
- **Unblock:** P1-10, with a narrowed scope given in its reason. Its dependents reopen with it.

## Blocked and skipped tasks

- **P1-10 (blocked): unblocked, scope narrowed.**
  - Drop `block::decompress_safe_using_dict`. Add no extern declaration.
  - Build `dict_block` as written.
  - The oracle's side becomes its decode of the whole linked frame.
  - The wrong-dictionary test checks cleanlz4 alone: an error or other bytes, never a panic.
  - DESIGN.md records the method and the gap.
- **P2-08, P2-11, P2-12, P2-13, P3-02, P3-03, P3-06, P4-01 to P4-09, P4-12 (skipped):** each waits only on the P1-10 chain, so each reopens as a dependent.

## Spec issues

- **SPEC §6.5 and §9 Q2: the binding lacks `LZ4_decompress_safe_usingDict`.** The owner still owes a decision: swap the binding crate, or permit the `lz4` program (`-D`) as the dictionary oracle for frames. Until then, the oracle comparison of a *wrong* dictionary cannot be made. The correct-dictionary comparison is covered through the linked-frame decode described above. HANDOFF suggests declaring the C symbol in an extern block in `ffi.rs`. That is not permitted as SPEC stands: §6.5 says "through a Rust binding crate", and an extern block needs a C parameter list that §6.5 does not give (the plan's Spec issue "§6.5 lists names, not signatures"). Doing that needs a SPEC amendment.
- **SPEC §1 property 2 and §6.4 vs the Block format on zero offsets.** The oracle accepts a zero offset that the Block format calls "invalid (corrupted)". The build follows the document and carries the `zero-offset` normalization. SPEC should say that where the oracle departs from a "must" or "invalid" in `docs/spec/`, the document wins.
- **CLAUDE.md's oracle protocol says only `crates/oracle/src/ffi.rs` contains `unsafe`.** P3-04 and P3-05 required counting allocators (`unsafe impl GlobalAlloc`) in a test binary and in an example. The protocol should carve out test and example allocators.
- **"Bounded memory" has no stated measure.** P3-04's bound ("limit + one block + 65,536 + the dictionary") does not say whether a reallocation's copy, where the old and new buffers coexist, counts. R1-02 settles it for this build: a reallocation counts its size change only. SPEC or DESIGN should state that.

## Manual checks still owed (from HANDOFF.md)

- **Phase 0:**
  - `make check` is green, and the guard self-test passes.
  - `testdata/MANIFEST.json` names both corpora, the binding crate's version (1.11.1, library 1.10.0) and the `lz4` CLI's version (1.9.4).
  - `.cargo/config.toml` sets `net.offline`.
- **Phase 1:**
  - Read `docs/DESIGN.md`'s rule table against the Block format and the XXH32 section.
  - Read `docs/spikes/P1-06.md` and confirm each end-of-block decision follows its table.
  - `conformance-P1-12.json` shows `passed == total`.
- **Phase 2:** `conformance-P2-12.json` (all ten categories, 100%) and `malformed-P2-13.json` (`matched_expected_error == total`, `panics == 0`) are owed once the reopened tasks run. Also read `docs/spikes/P2-09.md` and DESIGN's Differential rules, and confirm each normalization cites a section.
- **Phase 3:**
  - Read `crates/cleanlz4/src/stream.rs` for ergonomics.
  - Run `cargo test -p oracle --test limits` and watch the bomb figure.
  - The chunking coverage table is owed (P3-03, P3-06).
- **Phase 4:**
  - Every `fuzz-*.json` shows zero crashes and zero divergences, or DESIGN triages each one.
  - Every run's `seconds` is at least `fuzz.seconds_per_target`.
  - Read `bench-P4-10.json` and `bench-P4-11.json` (actuals only).
- **Phase 5:**
  - After summarize, `docs/SIMILARITY.md` and the attestation bundle exist, and `verify.mjs` passes.
  - The owner runs the verifier from a clean checkout on a second machine.

## Notes

- **The one clean-room denial was not a workaround.** It is log seq 112 (P1-04): a `sed -n` on `docs/spec/lz4_Frame_format.md` refused as a protected path. The implementer read the same file with the Read tool at seq 113. `docs/spec/` is a permitted input (SPEC §3.1). The guard protects it from *writes*, and its read-only `sed` form tripped that rule. Nothing forbidden was read, and HANDOFF logged it.
- **No forbidden reads in the log.** I scanned it for reads under `~/.cargo/registry`, `target/**/build`, LZ4 headers or source trees, `cargo doc`/`expand`/`vendor`, disassembly or tracing tools, prior-art paths, and network commands after P0-03. There were none. The only network calls were P0-03's two allowed `curl` downloads.
- **My own denial.** A read-only `sed -n` of mine on `.kukulu/cleanroom/log.jsonl` was denied as a protected path. I read the log with the Read tool instead and logged the friction with `kukulu_feedback_log`.
- **Mutation sample, killed:**
  - `block.rs`: the rule-2 threshold; the dictionary-part length in `copy_match`. The second survived the targeted `dictionary` tests: no block unit test has a match lying wholly inside the dictionary. The frame test `history_reaches_back_65535_bytes_across_blocks` killed it.
  - `frame.rs`: the content-size `!=` changed to `>`; the header-time limit check removed.
  - `stream.rs`: the header-time limit check removed; sticky errors removed; `FrameDone` suppressed.
  - `xxh32.rs`: the Step 3 boundary changed from `>=` to `>`.
  - `diff.rs`: the both-reject arm.
- **Interpretation choices** (HANDOFF) other than P2-10's are the readings most consistent with SPEC and PLAN.
- **Imports from the crate root.** `frame.rs` and `stream.rs` import `DecodeOptions`, `FrameKind`, `FrameOutput` and `TrailingData` from `lib.rs`. The module map does not list the crate root, but PLAN P2-02 put those types in `lib.rs` and handed them to the frame driver, so this is plan-sanctioned.
- **Performance (not a gate):** `decode_data_block` allocates a fresh scratch `Vec` for every compressed block and copies it into the output. R1-02 may remove it as a side effect.
