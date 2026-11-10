# cleanlz4 build plan

Derived from docs/SPEC.md v0.3 on 2026-11-10. SPEC.md wins over this file.

## Decisions

- Phases → SPEC §8's four phases of the 0.2.0 flight (0 to 3), in order, none merged. Nothing built in 0.1.0 is rebuilt; a task changes existing code only where SPEC §8 names it.
- Parallel streams → none; the plan is fully serial. The corpora and the generated conformance corpus are untracked, so a stream worktree could not verify, and a benchmark or an hour-long fuzz run must not share the machine with another task.
- Flight slug → `cleanlz4-0-2-0`, not the `cleanlz4` that SPEC's title gives. `docs/flights/cleanlz4/` already holds the 0.1.0 archive, and `kukulu_flight_archive` refuses unless its destination is free. SPEC calls this "the 0.2.0 flight".
- `baseBranch` → `main`, the branch the plan is committed on.
- Task ids → restart at P0-01, because the policy fixes `lockdown.after` at `P0-03`. An id that names a 0.1.0 task is written "0.1.0's P2-12"; 0.1.0's plan is `docs/flights/cleanlz4/PLAN.md`. This flight's result files are `conformance-P0-03`, `bench-P1-05`, `bench-P1-07`, `conformance-P2-01`, `malformed-P2-02`, `fuzz-<target>-P2-03` to `P2-07` (and `-P2-08` for a re-run after triage) and `bench-P2-09`; none collides with a 0.1.0 file name.
- Lockdown → P0-03, the last of Phase 0's three checks. SPEC's one Phase 0 task is split in three so that the id the policy names exists. No task, before or after it, downloads anything.
- Phase 0 writes a result file → P0-03 runs `make conformance-check` as SPEC says and then `make conformance TASK=P0-03 CATEGORIES=all`, because SPEC §3.6 says a conformance run that is only printed does not exist, and the Phase 0 manual check compares numbers with 0.1.0's files.
- Phase 0 also checks the fuzz toolchain (P0-02) → `fuzz/target/` and `fuzz/corpus/` are absent from this checkout, and Phase 2 needs five offline fuzz builds. Finding a missing crate in Phase 0 costs minutes; finding it in Phase 2 costs the phase.
- §5.3 similarity pairs → `docs/kukulu.json` holds four of SPEC's six pairs: `rust-independent-vs-indep` (negative), `rust-derived-vs-indep` (positive), `subject-vs-indep` and `subject-vs-c` (tests). Kūkulu 0.8.0 refuses the six-pair block: "cleanroom.similarity.pairs needs exactly one pair with role "negative" (found 2)", and with that block in place every Kūkulu tool fails. One band can be kept, and the Rust-vs-Rust band is the one this flight exists to add (SPEC §5.3, "Why 0.3 adds controls"; §1 Done: "with the same-language controls"). `subject-vs-c` keeps its Rust-vs-C reading from 0.1.0's `docs/flights/cleanlz4/SIMILARITY.md`. See Spec issues, first entry.
- §9 Q1, Q2, Q3 → resolved in SPEC; no task.
- §9 Q4 (how `kukulu_similarity` pairs controls with tests) → not a spike: SPEC gives it to the summarizer, who reads `docs/SIMILARITY.md`. With one negative and one positive pair configured, every test pair is placed against the Rust-vs-Rust band. P3-03 puts the note for the summarizer in its log entry and in `docs/HANDOFF.md`.
- §5.1 tunables → SPEC 0.3 marks no key ⚠️ ASSUMPTION and says "This flight changes none of them", so there is no key-tuning task. The one before-and-after measurement is the scratch buffer (P1-05, P1-07). No task introduces a tunable.
- Dictionary differential test → `crates/oracle/tests/dict_corpus.rs`. Frame entries go to `oracle::cli::decompress_frame_with_dict`; the raw `block-dict` entries go to `oracle::cli::decompress_block_with_dict`, which wraps the block in a minimal hand-built frame (SPEC §6.5). cleanlz4 is called directly, not through the runner. "Expected bytes" means length `input_bytes` and SHA-256 `input_sha256`, and the two outputs equal to each other.
- "Frames whose Dict-ID has no dictionary" → every frame entry that carries a `dict_id`, decoded by cleanlz4 with no dictionary supplied, must return `DictionaryRequired`. The `lz4` program is not asked. (No corpus entry lacks its dictionary file, so the other reading would test nothing.)
- The two mutants → one task each (P1-02 hermetic, in `tests/dict.rs`; P1-03 over the dictionary entries of the corpus, in `tests/results.rs`). Each task applies its mutant by hand, sees the named test fail, and restores the file.
- `make conformance-check` binding → `docs/kukulu.json` runs it for `crates/cleanlz4/src/`, `crates/oracle/src/runner.rs` and `crates/oracle/src/dict.rs`. `make corpus` is not bound to `dict.rs`: a reviewer's mutation run would regenerate the corpus from a mutated generator and rewrite `testdata/MANIFEST.json`. A task that edits `dict.rs` runs `make corpus` by hand.
- Scratch buffer → `FrameState` owns one buffer for a regular frame; `decode_legacy_block` takes one from its caller; at most one scratch buffer is alive and none outlives its frame, so SPEC §6.1's bound is unchanged.
- Scratch-buffer criterion → the statistic is the aggregate throughput (total decoded bytes over the sum of every input's best time) of `bench-P1-07.json` against `bench-P1-05.json`. The threshold is 7.28%, the spread 0.1.0 measured at `bench.repeats` = 10 (`docs/DESIGN.md`, Measurements). Above it, one confirming run; above it twice, the change is reverted. A single file's figure is noise: the small Canterbury files finish in microseconds.
- Re-verify → one task per fuzz target; the committed divergence inputs are moved aside before both `diff_*` runs; the triage task (P2-08) always exists and writes "none" when there is nothing to triage.
- Phase 3 order → the CHANGELOG entry and versions first (P3-01), then the evidence refresh (P3-02), so the Done checklist can state the changelog item as met. Similarity, attestation and `SUMMARY.md` are the summarize stage's and have no implement task (SPEC §8 Phase 3).
- CHANGELOG date → the UTC date of P3-01's commit. A version-agnostic test (`crates/oracle/tests/release.rs`) holds SPEC §1's "crate versions match it".
- Review rounds → the defaults (3 non-converging, 6 hard). SPEC asks for nothing else.
- New constraint rules → `oracle-unsafe-only-in-ffi`, `no-hardcoded-corpus-tunables`, `cargo-stays-offline` (flight-agnostic), and `single-magic-classifier` (from P1-04) and `no-new-dependencies` (this flight). `oracle-calls-only-the-permitted-c-api` now names the fifteen functions and each prefix's own list.

## Conventions

- Commit template: `CLAUDE.md` `## Commit template`, title `<ID>: <title>`, written to a file and committed with `git commit -F <file>`.
- Every task's verification includes `kukulu_verify`. Shared protocols are in `CLAUDE.md` `## Task protocols`; a task names the ones it follows.
- Spec documents are cited as "Block format, <section>", "Frame format, <section>" and "xxHash spec, <section>"; SPEC sections as "SPEC §n".
- "The invariants" in a task means the `## Constraints` section of `CLAUDE.md` and the constraint lines of its `## Flight cleanlz4-0-2-0 notes`, which bind every file the task touches.
- "0.1.0's Pn-nn" is a task of the archived plan. Every other id is this plan's.

## Phase 0 — Baseline and lockdown

### P0-01: Confirm the corpora, the tool versions and the spec sources

**Goal:** Prove the inputs of 0.1.0 are the ones on disk: the two corpora, the oracle library and `lz4` program versions, and the three spec documents.

**Files touched:** none

**Design constraints:**

- SPEC §5.2 (last paragraph), §8 Phase 0 (first bullet), Operator notes ("This flight must run with the same versions. A mismatch blocks Phase 0."), §3.1. Follow the check-only protocol and the clean-room protocol in CLAUDE.md.
- This flight downloads nothing. No `curl`, `wget`, `cargo fetch`, `cargo update` or `make manifest-corpora`, in this task or any other.
- Check 1, the corpora are present: `ls testdata/corpora/cantrbry | wc -l` and `ls testdata/corpora/silesia | wc -l` both print a number above zero. This check is not optional: the test in check 2 passes with a notice when `testdata/corpora/` is absent.
- Check 2, the corpora match: `cargo test -p oracle --test manifest -- --nocapture` passes its three tests and does not print `testdata/corpora/ absent`.
- Check 3, the tool versions match `testdata/MANIFEST.json` `tools`: the first line of `lz4 -V` (`lz4 -V 2>&1 | head -n 1`) equals `tools.lz4_cli`, and in `Cargo.lock` the `version` line that follows the binding crate's `name` line equals `tools.oracle_binding.version` (the binding crate's name is `tools.oracle_binding.crate` in the manifest). 0.1.0 recorded `*** LZ4 command line interface 64-bits v1.9.4, by Yann Collet ***` and `1.11.1+lz4-1.10.0`. The `rustc`, `cargo` and `cargo_fuzz` lines may differ: quote them in the log and go on.
- Check 4, the spec documents are the pinned copies: `node scripts/verify-spec-sources.mjs` prints `spec sources: ok (3 files)`.
- A failed check blocks: call `kukulu_task_block` naming the check, the expected value and the observed one. Repair nothing (SPEC §5.2: the owner restores the corpora by hand and records an intervention).

**Acceptance tests:** the existing `crates/oracle/tests/manifest.rs` tests `manifest_lists_both_corpora_with_hashes`, `manifest_records_tool_versions` and `corpus_files_on_disk_match_manifest` pass with the corpora present. No new test.

**Out of scope:** any file change; `make corpus` (P0-03); `make check` (P0-02).

**Verification:** the four checks in the foreground, then `kukulu_verify` with no files. The log entry quotes the two file counts, the two version strings and the spec-sources line.

**Depends on:** none

### P0-02: Confirm make check and the fuzz toolchain

**Goal:** Prove the 0.1.0 code is green on the base commit, and that the five fuzz targets still build and start offline.

**Files touched:** none

**Design constraints:**

- SPEC §8 Phase 0 (first bullet: "`make check` is green"), §7, §1 ("every command in §7 exits zero"). Follow the check-only protocol in CLAUDE.md.
- `make check` in the foreground with the shell timeout at ten minutes (it takes one to two minutes once built). It runs fmt, clippy, the tests, the guard self-test, both deps scripts and both spec-source scripts.
- `cargo test --workspace` reads files under `testdata/conformance/`. If tests fail only because those files are missing, run `make corpus` once (about two minutes), confirm `git status --short testdata/MANIFEST.json` prints nothing, run `make check` again, and say so in the log.
- The fuzz build first: call `kukulu_verify` with `fuzz/Cargo.toml` among the files, which runs `cargo +nightly fuzz build` under its own 30-minute timeout (`fuzz/target/` is absent, so everything compiles from the local cargo cache). Then `make fuzz-smoke` in the foreground: five targets for `fuzz.smoke_seconds` each, results in a temporary directory.
- If the fuzz build fails because a crate is not in the local cache (cargo is offline), block naming the crate. Never go online.
- Any other failure blocks with the failing output. Fix nothing in this task.

**Acceptance tests:** none new; the whole existing suite passes inside `make check`.

**Out of scope:** any file change; fixing a red check.

**Verification:** as above. The log entry quotes `guard self-test: ok (<n> cases)`, `spec sources: ok (3 files)` and the five `fuzz_run <target> smoke:` lines.

**Depends on:** none

### P0-03: Regenerate the corpus, record the baseline conformance run and lock down

**Goal:** Regenerate the conformance corpus, show it is 0.1.0's byte for byte, and commit 2349 of 2349 as this flight's baseline; this is the lockdown task.

**Files touched:** `testdata/results/conformance-P0-03.json`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 0 (second and third bullets), §5.2, §3.6, §5.4. Follow the results protocol in CLAUDE.md.
- Lockdown: `docs/cleanroom.json` names this task id in `lockdown.after`. Once this task is `[x]` no URL is allowed. The flight needs no network before or after it.
- Step 1: `make corpus` in the foreground (about two minutes). It rewrites the gitignored `testdata/conformance/` and `testdata/MANIFEST.json`. Then `git diff --exit-code -- testdata/MANIFEST.json` must exit 0: generation is deterministic from `fuzz.seed`. A difference means the corpora, the oracle library or the `lz4` program changed: quote `git diff --stat -- testdata/MANIFEST.json`, restore the file with `git checkout -- testdata/MANIFEST.json`, and block.
- Step 2: `make conformance-check` must print `conformance check: 2349 of 2349 passed` and exit 0.
- Step 3: `make conformance TASK=P0-03 CATEGORIES=all` writes the result file: `total` 2349, `passed` 2349, `failed` 0, ten categories. `diff <(grep -v '"task"' testdata/results/conformance-P2-12.json) <(grep -v '"task"' testdata/results/conformance-P0-03.json)` must print nothing: the same `corpus_sha256` and the same count in every category as 0.1.0's full run.
- Step 4: add `testdata/results/conformance-P0-03.json` to the fenced list in `docs/EVIDENCE.md`, as the results protocol says.
- Any other count, or any difference in step 3, blocks with the `FAIL <id>: <reason>` lines the tool printed. Change no code.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema`, `conformance_counts_are_consistent` and `evidence_index_lists_every_result_file` pass with the new file staged.

**Out of scope:** any change to code, the generator, the config or `testdata/MANIFEST.json`.

**Verification:** the four steps in the foreground; `git add` both files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched. The log entry carries `Lockdown: P0-03 done, no URL is allowed from here`, the counts and the `corpus_sha256`.

**Depends on:** P0-01

### P0-04: Phase 0 end: push and record the manual check

**Goal:** Push the branch and record Phase 0's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 0 (Manual check). Follow the phase-end protocol in CLAUDE.md. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify` with no files. The log entry carries `Manual check: NOT VERIFIED (human): testdata/results/conformance-P0-03.json equals 0.1.0's conformance-P2-12.json in corpus_sha256, total, passed and every category; the log entries of P0-01 and P0-02 show both corpora matching testdata/MANIFEST.json, the lz4 program at v1.9.4, the binding at 1.11.1+lz4-1.10.0 and make check green; kukulu_status reports cleanroom.lockdown true`.

**Depends on:** none

## Phase 1 — Close the 0.1.0 review notes

### P1-01: Dictionary differential test: every dictionary entry against the lz4 program

**Goal:** Hold SPEC §1's byte-identity claim for dictionary entries with a committed test: the `lz4` program and cleanlz4 decode every `dictionary` entry of the conformance corpus to the same bytes.

**Files touched:** `crates/oracle/tests/dict_corpus.rs`, `crates/oracle/src/dict.rs`, `crates/oracle/src/cli.rs`, `crates/oracle/src/fuzz_logic.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 1 (first bullet), §6.5 (Dictionary oracle; What the oracle cannot do), §1 (Correctness), §6.1 (Options: the preset dictionary); Frame format, "Dictionary ID". Follow the oracle protocol and the design-note protocol. The invariants apply.
- The entries: `oracle::manifest::Manifest::load()`, then `conformance.entries` whose `category` is `dictionary`. An entry's file is `oracle::corpus::conformance_dir().join(&entry.file)`; its dictionary bytes and optional id come from `oracle::runner::entry_dictionary(&entry)`. The generated corpus must be on disk (P0-03 made it; `make corpus` remakes it); the test `expect`s the files, as `tests/corpus.rs` does.
- The oracle side, by `entry.kind`: `frame` goes to `oracle::cli::decompress_frame_with_dict(&file, &dict)`; `block-dict` goes to `oracle::cli::decompress_block_with_dict(&file, &dict)`, which wraps the block in a minimal hand-built frame (SPEC §6.5). Both return `Result<Vec<u8>, oracle::block::Rejected>`; an `Err` fails the test with the entry id. Any other kind fails the test, naming it.
- The cleanlz4 side, called directly and not through `oracle::runner`: `frame` goes to `cleanlz4::decompress_frame_with` with `DecodeOptions { dictionary: Some(Dictionary { bytes, id }), max_output_bytes: entry.input_bytes, ..DecodeOptions::default() }`, where `id` is the entry's `dict_id` (`None` when it has none), and the output is `.data`; `block-dict` goes to `cleanlz4::decompress_block_with_dict(&file, &dict, entry.input_bytes as usize)`.
- Expected bytes: for every entry, each side's output has length `input_bytes` and `oracle::manifest::sha256_hex` of it equals `input_sha256`, and the two outputs are equal. Every assertion message carries the entry id.
- Dict-ID with no dictionary (interpretation: every frame entry that carries a `dict_id`): `cleanlz4::decompress_frame_with(&file, &DecodeOptions::default())` must be `Err(Error::DictionaryRequired)`. These frames are not sent to the `lz4` program without a dictionary.
- The test must not be able to pass on nothing: assert at least one `block-dict` entry, one frame entry with a `dict_id`, one frame entry without, and one entry for every size in `dict.sizes_bytes` (`extra["dict_bytes"]`). Write no entry count as a literal; the counts follow from the config.
- If the two sides disagree on any entry, change neither the decoder nor the generator: block, quoting the entry ids and both outcomes.
- Stale wording, comments only, no behaviour change:
    - `crates/oracle/src/dict.rs`, `dict_frame`'s doc comment: replace "The oracle cannot decode the result (no dictionary decoder on the permitted surface); the header is checked to be one it accepts." with the fact that the C library's frame functions on the permitted surface take no dictionary, so the `lz4` program (`oracle::cli::decompress_frame_with_dict`) is the oracle for these frames, compared with cleanlz4 in `tests/dict_corpus.rs`; the header alone is still checked against the library's frame decoder.
    - `crates/oracle/src/cli.rs`, the module comment: "Used only to make legacy frames." becomes: it makes legacy frames (`lz4 -l`) and is the oracle for dictionary decoding (`lz4 -d -D <dict>`), SPEC section 6.5.
    - `crates/oracle/src/fuzz_logic.rs`, the module comment: "None of them calls the oracle." becomes: the `no_panic_*` functions never call the oracle; `diff_roundtrip` and `diff_accept` compare cleanlz4 with it.
    - Write no C function name followed by `(` in a comment unless it is one of SPEC §6.5's fifteen.
- `docs/DESIGN.md`: one new Decisions entry (dictionary frames and blocks against the `lz4` program; SPEC section 6.5, Dictionary oracle; Frame format, "Dictionary ID"; held by the new tests). Correct the three stale statements: in the "Preset dictionary and Dict-ID" entry and the "Special conformance suites" entry, "the oracle cannot decode them ... so the expected output is the known chunks" (and "the recorded output is the known data") become "the C library's frame API cannot decode them; the `lz4` program does, and `tests/dict_corpus.rs` compares it with cleanlz4"; in `## Spec rules implemented`, the row that ends "not implemented: task P1-10 is blocked ..." becomes a row citing SPEC section 6.5 (Dictionary oracle), `oracle::cli::decompress_block_with_dict`, and the tests `dict_block_decodes_with_the_oracle_using_the_dictionary` and `every_dictionary_entry_decodes_identically_with_the_lz4_program_and_cleanlz4`. `tests/design.rs` must still pass.

**Acceptance tests:** `crates/oracle/tests/dict_corpus.rs`: `every_dictionary_entry_decodes_identically_with_the_lz4_program_and_cleanlz4`, `dict_id_frames_without_a_dictionary_are_dictionary_required`, `the_dictionary_category_covers_blocks_frames_and_every_configured_size`.

**Out of scope:** any change to what `dict_frame`, `dict_block` or `cli.rs` do; the decoder; the generator; `testdata/MANIFEST.json`; a new C function; the mutant tests (P1-02, P1-03); `docs/EVIDENCE.md` (P3-02).

**Verification:** `kukulu_verify` with the Files touched (`dict.rs` makes it run `make conformance-check`); `cargo test -p oracle --test dict_corpus -- --nocapture`; `make corpus` and then `git diff --exit-code -- testdata/MANIFEST.json` exits 0 (a comment-only edit of `dict.rs` must not change generation). The log entry quotes how many entries of each kind were compared.

**Depends on:** P0-03

### P1-02: Kill the dict_dependent mutant in the oracle's dict module

**Goal:** A block whose data shares nothing with the dictionary is labelled not `dict_dependent`, and a test holds it.

**Files touched:** `crates/oracle/tests/dict.rs`

**Design constraints:**

- SPEC §8 Phase 1 (second bullet, first item). The mutant that survived 0.1.0's review: in `oracle::dict::dict_block`, `Ok(plain) => plain != data` changed to `Ok(_) => true`. Follow the mutant protocol in CLAUDE.md. `crates/oracle/src/dict.rs` is not changed by this task.
- The test, for every size in `Config::load()`'s `dict.sizes_bytes`: the dictionary is `oracle::synth::word_salad(cfg.fuzz.seed, size)` (ASCII text, every byte below `0x80`); the data is 16 KiB whose every byte is `0x80` or above and which compresses by itself (a short pattern such as the bytes `0x80..0xB0` repeated). The two alphabets are disjoint, so no match of four bytes can reach into the dictionary. Assert both premises (`all(|b| *b < 0x80)`, `all(|b| *b >= 0x80)`).
- Then: `oracle::dict::dict_block(&dict, &data)` is `Some(b)`; `b.dict_dependent` is `false`; `oracle::block::decompress_safe(&b.block, data.len())` returns the data; `cleanlz4::decompress_block(&b.block, data.len())` returns the data.
- If `dict_block` labels this block dependent on the unmutated code, do not edit `dict.rs`: block, quoting what `decompress_safe` returned at `data.len()` and at `data.len() + 65_536` bytes of capacity.
- The mutant: apply `Ok(_) => true` in `dict.rs`, run `cargo test -p oracle --test dict a_block_sharing_nothing`, see it fail, undo, and prove the undo with `git diff --exit-code -- crates/oracle/src/dict.rs`.

**Acceptance tests:** `crates/oracle/tests/dict.rs`: `a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent`.

**Out of scope:** changing `dict.rs`; the generator; `testdata/MANIFEST.json`.

**Verification:** `kukulu_verify` with the Files touched. The log entry quotes the assertion that failed under the mutant.

**Depends on:** none

### P1-03: Kill the dropped-dictionary mutant in the conformance runner

**Goal:** `cargo test --workspace` fails when the runner stops passing an entry's dictionary to cleanlz4.

**Files touched:** `crates/oracle/tests/results.rs`

**Design constraints:**

- SPEC §8 Phase 1 (second bullet, second item). The mutant that survived 0.1.0's review: in `oracle::runner::cleanlz4_decoder`, the `if entry.extra.contains_key("dict_file")` block for `frame` entries removed or never taken. Follow the mutant protocol. `crates/oracle/src/runner.rs` is not changed by this task.
- The test: load the manifest, take the entries whose `category` is `dictionary`, and run them through the real runner: `oracle::runner::run_entries(&entries, &oracle::corpus::conformance_dir(), &oracle::runner::cleanlz4_decoder)`. Assert `report.total` equals the number of entries and is above zero, `report.passed == report.total`, and `report.failures` is empty (print the failures in the assertion message).
- The test must not be able to pass on nothing: assert the selection holds at least one `frame` entry with a `dict_id` in `extra` and at least one `frame` entry without. Write no entry count as a literal.
- The generated corpus must be on disk (P0-03 made it); the test `expect`s the files.
- The mutant: replace the condition `entry.extra.contains_key("dict_file")` with `false` in `runner.rs`, run `cargo test -p oracle --test results runner_passes_the_dictionary`, see it fail (frames with a Dict-ID return `DictionaryRequired`), undo, and prove the undo with `git diff --exit-code -- crates/oracle/src/runner.rs`.
- The binding SPEC asks for is already in `docs/kukulu.json`, which is never edited: `make conformance-check` runs for any task that touches `crates/cleanlz4/src/`, `crates/oracle/src/runner.rs` or `crates/oracle/src/dict.rs`. Show it: call `kukulu_verify` with `crates/oracle/src/runner.rs` among the files and confirm `make conformance-check` is among the commands it ran.

**Acceptance tests:** `crates/oracle/tests/results.rs`: `runner_passes_the_dictionary_to_frame_entries`.

**Out of scope:** changing `runner.rs`; editing `docs/kukulu.json`; the generator.

**Verification:** `kukulu_verify` with `crates/oracle/tests/results.rs` and `crates/oracle/src/runner.rs`. The log entry quotes the failure under the mutant and that `make conformance-check` ran.

**Depends on:** P0-03

### P1-04: Merge is_magic into magic_kind

**Goal:** One function in `frame.rs` knows the three kinds of magic number; the duplicate is gone, with no change to the API or behaviour.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/tests/frame.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 1 (decoder cleanups, first item), §8 preamble ("The public API and every error variant stay as they are"), §3.2, §3.3; Frame format, "General Structure of LZ4 Frame format" (Magic Number), "Skippable Frames" (Magic Number), "Legacy frame" (Magic Number; EndMark: "if the frame is followed by a valid Frame Magic Number, it is considered completed"). Follow the clean-room protocol and the design-note protocol. The invariants apply: no `unsafe`, no panic, no indexing.
- Today `fn is_magic(word: u32) -> bool` (private, called once, in `decode_legacy_frame`) and `pub(crate) fn magic_kind(word: u32) -> Option<FrameKind>` (called from `stream.rs`) test the same three magic numbers. Delete `is_magic`; `decode_legacy_frame` calls `magic_kind(word).is_some()`. `magic_kind` keeps its name, signature and visibility.
- `is_magic_prefix` (the test of one to three bytes) is a different function and stays. `classify` may stay as it is.
- The constraint `single-magic-classifier` is active from this task: any `is_magic(` under `crates/cleanlz4/` fails `kukulu_verify`.
- No existing test changes what it expects.
- New unit test in `frame.rs`'s `mod tests`: `0x184D2204` is `Regular`; `0x184C2102` is `Legacy`; every value `0x184D2A50..=0x184D2A5F` is `Skippable`; `0`, `0x184D2203`, `0x184D2205`, `0x184D2A4F`, `0x184D2A60`, `0x184C2101` and `u32::MAX` are `None`.
- New test in `crates/cleanlz4/tests/frame.rs`, with that file's own `legacy` and `literal_block` helpers: two legacy frames back to back decode to both contents in order, `kinds` is `[Legacy, Legacy]` and `consumed` is the input length. (The existing tests cover a legacy frame followed by a regular and by a skippable frame, not by a legacy one.)
- Show the new tests bite, by the mutant protocol: remove the legacy arm of `magic_kind`, see both new tests fail, undo.
- `docs/DESIGN.md`: one Decisions entry (one function names a magic number's kind; the three Frame format sections above; held by the two tests).

**Acceptance tests:** `crates/cleanlz4/src/frame.rs`: `magic_kind_names_the_three_kinds_and_nothing_else`. `crates/cleanlz4/tests/frame.rs`: `legacy_frame_ends_before_a_second_legacy_frame`. Unchanged and passing: `legacy_frame_ends_at_the_next_magic_number`, `legacy_frame_ends_at_a_zero_block_size`, `all_sixteen_skippable_magics_are_skipped` (`tests/frame.rs`); `legacy_frame_then_regular_frame`, `every_one_shot_test_frame_matches_when_streamed_bytewise` (`tests/stream.rs`).

**Out of scope:** the scratch buffer (P1-06); any change to `stream.rs`, `lib.rs`, the public API or `Error`; any change in what a frame boundary accepts.

**Verification:** `kukulu_verify` with the Files touched (`frame.rs` makes it run `make conformance-check`: 2349 of 2349).

**Depends on:** none

### P1-05: Benchmark before the scratch-buffer change

**Goal:** Commit decode throughput at the commit just before the scratch-buffer change, as the "before" of P1-07.

**Files touched:** `testdata/results/bench-P1-05.json`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 1 (fourth bullet: "before and after"), §5.4, §3.6, §1 ("Performance is measured and reported. It is not a gate."). Follow the results protocol; this is the "before" of the tuning protocol.
- `make bench TASK=P1-05` in the foreground (release build, every corpus file in the manifest, `bench.repeats` repeats read from the config, best time; about a minute). Run nothing else on the machine at the same time.
- No code changes in this task: `git status --short crates/ fuzz/` prints nothing.
- The aggregate throughput, which P1-07 compares: `node -e 'const r=require("./testdata/results/bench-P1-05.json");let b=0,t=0;for(const i of r.inputs){b+=i.bytes;t+=i.bytes/i.cleanlz4_mb_s;}console.log((b/t).toFixed(1))'` (total decoded bytes over the sum of each input's best time, in MB/s).
- Add the file to the fenced list in `docs/EVIDENCE.md`, as the results protocol says.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged.

**Out of scope:** any code change; changing `bench.repeats`; profiling the oracle (denied by the policy).

**Verification:** `git add` both files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched. The log entry quotes the slowest and fastest `cleanlz4_mb_s` with their input names, and the aggregate.

**Depends on:** P0-01

### P1-06: Reuse one scratch buffer across blocks

**Goal:** A frame's compressed blocks decode into one reused buffer instead of a new `Vec` per block, with no change to the API, the behaviour or the memory bound.

**Files touched:** `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/stream.rs`, `crates/oracle/tests/scratch.rs`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 1 (decoder cleanups, second item), §6.1 (Bounded memory), §3.2, §3.3, §3.4, §3.7 (the counting-allocator carve-out); Frame format, "Block Maximum Size" ("This information is useful to help the decoder allocate memory"); Block format, "Implementation notes", Safe decoding. Follow the clean-room protocol and the design-note protocol. The invariants apply.
- Today `FrameState::decode_data_block` and `decode_legacy_block` each begin with `let mut block = Vec::new();`, so every compressed block allocates and frees a buffer.
- Regular frames: `FrameState` gains a private field `scratch: Vec<u8>`, empty in `new`. The compressed-block path of `decode_data_block` takes it (`core::mem::take`), clears it, decodes into it with the same `decode_block` call and the same bound as today, does the same accounting, append and history steps, and puts it back before returning `Ok`. On an error it may be dropped. The uncompressed-block path does not use it.
- Legacy frames: `decode_legacy_block` becomes `pub(crate) fn decode_legacy_block(data: &[u8], scratch: &mut Vec<u8>, out: &mut Vec<u8>, remaining: &mut u64) -> Result<()>`; it clears `scratch`, decodes into it and appends to `out` as today. `decode_legacy_frame` owns one `Vec` for the whole frame. In `stream.rs`, `Decoder` gains a private field `scratch: Vec<u8>` passed for legacy blocks, and replaced by `Vec::new()` at both places a legacy frame ends there: `end_frame`, and the `LegacySize` branch where a known magic number ends the frame.
- So at most one scratch buffer is alive, none outlives its frame, and its capacity never passes one block's bound. SPEC §6.1's bound (the limit, plus one block maximum size, plus 65,536 bytes of history) must hold exactly as before: the existing tests `bomb_peak_allocation_is_bounded` and `bomb_peak_allocation_is_bounded_past_several_blocks` (`crates/oracle/tests/limits.rs`) and `one_shot_output_capacity_never_exceeds_the_limit` and `legacy_output_capacity_never_exceeds_the_limit` (`crates/cleanlz4/tests/limits.rs`) pass with their bounds untouched.
- No public item changes: `lib.rs` is not touched, and `Decoder`'s and `FrameOutput`'s public fields and methods stay as they are.
- New test binary `crates/oracle/tests/scratch.rs`, with its own counting allocator in the pattern of `crates/oracle/tests/limits.rs` (`unsafe impl GlobalAlloc` forwarding to `std::alloc::System`, calling no C function): it counts calls to `alloc` only (fresh allocations; `realloc` and `dealloc` are forwarded and not counted). A `static SERIAL: Mutex<()>` makes the tests run one at a time. Every test also asserts the decoded bytes equal the input.
    - Regular frames come from `oracle::frame::compress_frame` over `word_salad` data split into 65,536-byte chunks, with `Prefs { block_size_id: 4, auto_flush: true, linked, ..Prefs::default() }`: one block per chunk. Compare a frame of 16 blocks with one of 64, for `linked` false and true.
    - Legacy frames are built by hand (Frame format, "Legacy frame"): the magic `0x184C2102` little-endian, then for each 4 KiB chunk a 4-byte little-endian size and `oracle::block::compress_default(chunk)`. Compare 16 blocks with 64.
    - The assertion in each test: the fresh allocations made while decoding the 64-block frame exceed those for the 16-block frame by at most 4. (With a `Vec` per block the difference is 48 or more.)
    - The streaming tests feed the whole input to `cleanlz4::Decoder` with a 65,536-byte output buffer allocated before counting starts.
- Write the tests first and run them on the unchanged decoder: they must fail. Quote the two counts in the log. Then make the change.
- `docs/DESIGN.md`: one Decisions entry (one scratch buffer per frame; the two spec sections above; held by the three new tests and the four memory tests), and in the "Output limit" entry "the per-block scratch buffer" becomes "the frame's scratch buffer".

**Acceptance tests:** `crates/oracle/tests/scratch.rs`: `one_shot_fresh_allocations_do_not_grow_with_the_block_count`, `streaming_fresh_allocations_do_not_grow_with_the_block_count`, `legacy_fresh_allocations_do_not_grow_with_the_block_count` (one-shot and streaming in the one test). The four memory tests named above pass unchanged.

**Out of scope:** decoding straight into the output; any other optimization; `block.rs`; `lib.rs`; the public API; `Error`; the benchmark (P1-07).

**Verification:** `kukulu_verify` with the Files touched plus `fuzz/Cargo.toml` (it runs `make conformance-check` and rebuilds the fuzz targets against the changed crate); `cargo test -p oracle --test limits` and `--test scratch`; `make fuzz-smoke` in the foreground exits zero.

**Depends on:** P1-05

### P1-07: Measure the scratch-buffer change and decide

**Goal:** Measure decode throughput after the scratch-buffer change against P1-05's figures, record the table, and keep or revert the change by SPEC's criterion.

**Files touched:** `testdata/results/bench-P1-07.json`, `docs/EVIDENCE.md`, `docs/DESIGN.md`; only on the revert path: `crates/cleanlz4/src/frame.rs`, `crates/cleanlz4/src/stream.rs`, `crates/oracle/tests/scratch.rs`

**Design constraints:**

- SPEC §8 Phase 1 (fourth bullet), §5.1 (`bench.repeats`, measured in 0.1.0 and kept), §1 ("Performance is measured and reported. It is not a gate."), §3.4. Follow the tuning protocol (before: `bench-P1-05.json`; after: `bench-P1-07.json`) and the results protocol.
- After: `make bench TASK=P1-07` in the foreground, with nothing else running.
- The statistic (interpretation: SPEC says "slower than before" without naming one): the aggregate throughput `A` of a result file, total decoded bytes over the sum of each input's best time, computed for both files with P1-05's command (`node -e 'const r=require("./testdata/results/bench-P1-07.json");let b=0,t=0;for(const i of r.inputs){b+=i.bytes;t+=i.bytes/i.cleanlz4_mb_s;}console.log((b/t).toFixed(1))'`). The slowdown is `1 - A(after) / A(before)`, as a percentage to two decimals; a negative value means faster.
- The threshold is 7.28%: the spread 0.1.0 measured at `bench.repeats` = 10 (`docs/DESIGN.md`, `## Measurements`, "bench.repeats (P4-11)", the row for 10 repeats). Quote it from there. It is a measured figure, not a config key, and appears in no code.
- Criterion: keep the change when the slowdown is at most 7.28%. When it is above, run `make bench TASK=P1-07` once more (it overwrites the file) and recompute; keep the change if the second run is at most 7.28%; revert only when both runs are above. Record both runs' aggregates.
- `docs/DESIGN.md`, `## Measurements`, new heading `### Scratch-buffer reuse (P1-07)`: one row per input (name, bytes, before MB/s, after MB/s, change in percent), a last row with the two aggregates and the slowdown, the number of inputs slower by more than 7.28% (information only), and one sentence stating the criterion and the decision.
- Keep path: no code changes.
- Revert path: find P1-06's commit with `git log --oneline -- crates/oracle/tests/scratch.rs`; restore the two decoder files to their state before it with `git checkout <that commit>~1 -- crates/cleanlz4/src/frame.rs crates/cleanlz4/src/stream.rs`; `git rm crates/oracle/tests/scratch.rs`; replace P1-06's Decisions entry in `docs/DESIGN.md` with one saying the reuse was measured and not kept, citing the table, and restore "the per-block scratch buffer" in the "Output limit" entry. `bench-P1-07.json` stays as measured.
- The commit body's `Measurement:` line and the log entry carry `A(before)`, `A(after)`, the slowdown and the decision.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged. On the revert path the whole suite passes without `scratch.rs`.

**Out of scope:** any further optimization; changing `bench.repeats` or any other key of `testdata/config.json`; profiling the oracle (denied by the policy).

**Verification:** `git add` the files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched that changed (on the revert path that includes the two decoder files, so `make conformance-check` runs).

**Depends on:** P1-05, P1-06

### P1-08: Phase 1 end: push and record the manual check

**Goal:** Push the branch and record Phase 1's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 1 (Manual check). Follow the phase-end protocol in CLAUDE.md. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify` with no files. The log entry carries `Manual check: NOT VERIFIED (human): read crates/oracle/tests/dict_corpus.rs (the dictionary differential test) and the two mutant-killing tests, a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent in crates/oracle/tests/dict.rs and runner_passes_the_dictionary_to_frame_entries in crates/oracle/tests/results.rs; read DESIGN.md Measurements, Scratch-buffer reuse (P1-07), and confirm the keep-or-revert decision follows its table`.

**Depends on:** none

## Phase 2 — Re-verify

### P2-01: The full conformance run

**Goal:** Run every conformance category on the 0.2.0 code and commit the counts; the bar is 2349 of 2349.

**Files touched:** `testdata/results/conformance-P2-01.json`, `docs/EVIDENCE.md`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 2 (first bullet), §5.4, §3.6, §6.3. Follow the results protocol.
- Step 1: `make corpus` in the foreground (about two minutes), then `git diff --exit-code -- testdata/MANIFEST.json` must exit 0: the corpus is still 0.1.0's. A difference blocks (quote `git diff --stat`, then `git checkout -- testdata/MANIFEST.json`).
- Step 2: `make conformance TASK=P2-01 CATEGORIES=all` in the foreground. The file must show `total` 2349, `passed` 2349, `failed` 0 and all ten categories (`block`, `frame-independent`, `frame-linked`, `checksums`, `content-size`, `dictionary`, `skippable`, `legacy`, `concatenated`, `synthetic`). `diff <(grep -v '"task"' testdata/results/conformance-P2-12.json) <(grep -v '"task"' testdata/results/conformance-P2-01.json)` must print nothing.
- If any entry fails, do not edit the decoder here: block, quoting the `FAIL <id>: <reason>` lines.
- `docs/DESIGN.md`, `## Measurements`, new heading `### Full conformance run, 0.2.0 (P2-01)`: the per-category totals, copied from the file.
- Add the file to the fenced list in `docs/EVIDENCE.md`.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema`, `conformance_counts_are_consistent` and `evidence_index_lists_every_result_file` pass with the new file staged.

**Out of scope:** decoder changes; generator changes; `testdata/MANIFEST.json`.

**Verification:** `git add` the files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched. The log entry quotes `passed`, `total` and `corpus_sha256`.

**Depends on:** P0-03

### P2-02: The full malformed run

**Goal:** Run the 144 malformed files through the 0.2.0 code and commit the counts; the bar is 144 of 144 with 0 panics.

**Files touched:** `testdata/results/malformed-P2-02.json`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 2 (first bullet), §5.4, §6.4, §3.6. Follow the results protocol.
- `make malformed TASK=P2-02` in the foreground must print `malformed P2-02: 144 of 144 matched, 0 panics` and exit 0.
- The malformed corpus is committed and is not regenerated: never pass `--write` to the tool, and `git status --short testdata/malformed/` prints nothing after the run.
- A wrong variant or a panic blocks, naming the files. Do not edit the decoder or `testdata/malformed/index.json` here.
- Add the file to the fenced list in `docs/EVIDENCE.md`.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged; `crates/oracle/tests/malformed.rs` passes unchanged.

**Out of scope:** decoder changes; any change under `testdata/malformed/`.

**Verification:** `git add` both files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched. The log entry quotes `total`, `matched_expected_error` and `panics`.

**Depends on:** none

### P2-03: Run no_panic_block for the configured duration

**Goal:** Fuzz `no_panic_block` for `fuzz.seconds_per_target` on the 0.2.0 code and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_block-P2-03.json`, `docs/EVIDENCE.md`

**Design constraints:** SPEC §8 Phase 2 (second bullet), §5.4, §3.6, §5.1 (`fuzz.seconds_per_target`, read from the config by the runner and never typed). Follow the fuzz-run protocol, the long-run protocol and the results protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`. Change no code.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` is above 0.

**Out of scope:** fixing a crash (P2-08); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_block-P2-03.json`, which runs `make fuzz-run TARGET=no_panic_block TASK=P2-03` under its own 75-minute timeout; if the call returns before the run ends, wait as the long-run protocol says and never start the run again. Then add the file to `docs/EVIDENCE.md`'s list, `git add`, `cargo test -p oracle --test results`, and `kukulu_verify` with `docs/EVIDENCE.md` alone. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** none

### P2-04: Run no_panic_frame for the configured duration

**Goal:** Fuzz `no_panic_frame` for `fuzz.seconds_per_target` on the 0.2.0 code and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_frame-P2-04.json`, `docs/EVIDENCE.md`

**Design constraints:** SPEC §8 Phase 2 (second bullet), §5.4, §3.6, §5.1 (`fuzz.seconds_per_target`, read from the config by the runner and never typed). Follow the fuzz-run protocol, the long-run protocol and the results protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`. Change no code.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` is above 0.

**Out of scope:** fixing a crash (P2-08); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_frame-P2-04.json`, which runs `make fuzz-run TARGET=no_panic_frame TASK=P2-04` under its own 75-minute timeout; if the call returns before the run ends, wait as the long-run protocol says and never start the run again. Then add the file to `docs/EVIDENCE.md`'s list, `git add`, `cargo test -p oracle --test results`, and `kukulu_verify` with `docs/EVIDENCE.md` alone. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** none

### P2-05: Run no_panic_stream for the configured duration

**Goal:** Fuzz `no_panic_stream` for `fuzz.seconds_per_target` on the 0.2.0 code and commit its result file.

**Files touched:** `testdata/results/fuzz-no_panic_stream-P2-05.json`, `docs/EVIDENCE.md`

**Design constraints:** SPEC §8 Phase 2 (second bullet), §5.4, §3.6, §5.1 (`fuzz.seconds_per_target`, read from the config by the runner and never typed). Follow the fuzz-run protocol, the long-run protocol and the results protocol in CLAUDE.md. Stage also any new file under `testdata/regressions/`. Change no code.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` is above 0.

**Out of scope:** fixing a crash (P2-08); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-no_panic_stream-P2-05.json`, which runs `make fuzz-run TARGET=no_panic_stream TASK=P2-05` under its own 75-minute timeout; if the call returns before the run ends, wait as the long-run protocol says and never start the run again. Then add the file to `docs/EVIDENCE.md`'s list, `git add`, `cargo test -p oracle --test results`, and `kukulu_verify` with `docs/EVIDENCE.md` alone. The log entry quotes `runs`, `crashes` and `corpus_entries`.

**Depends on:** none

### P2-06: Run diff_roundtrip for the configured duration

**Goal:** Fuzz `diff_roundtrip` for `fuzz.seconds_per_target` on the 0.2.0 code and commit its result file.

**Files touched:** `testdata/results/fuzz-diff_roundtrip-P2-06.json`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 2 (second bullet: "Before each `diff_*` run, empty the divergence directory: move the committed inputs aside and restore them afterwards"), §5.4, §3.6, §5.1 (`fuzz.seconds_per_target` and `fuzz.max_divergence_files`, both read from the config by the tools). Follow the fuzz-run protocol, the long-run protocol and the results protocol in CLAUDE.md. Change no code.
- Before the run: `mkdir -p target/divergences-aside && mv testdata/results/divergences/*.bin target/divergences-aside/` (`target/` is gitignored), then `ls testdata/results/divergences | wc -l` prints 0.
- After the result file is written: `mv -n target/divergences-aside/*.bin testdata/results/divergences/ && rmdir target/divergences-aside`. `git status --short testdata/results/divergences/` must then show no deleted file; a new (`??`) file there is one of this run's divergences. If `rmdir` fails, a file of the same name was found again: `cmp` the two copies and delete the one left aside when they are equal.
- A resumed task that finds files in `target/divergences-aside/`: if the result file exists, go straight to the restore step; if not, leave them aside and start the run.
- Stage also any new file under `testdata/regressions/` and `testdata/results/divergences/`, and add each new divergence file to `docs/EVIDENCE.md`'s list with the result file.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new files staged; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` is above 0.

**Out of scope:** triaging a crash or a divergence (P2-08); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-diff_roundtrip-P2-06.json`, which runs `make fuzz-run TARGET=diff_roundtrip TASK=P2-06` under its own 75-minute timeout; if the call returns before the run ends, wait as the long-run protocol says and never start the run again. Then restore the divergence inputs, add the new files to `docs/EVIDENCE.md`'s list, `git add`, `cargo test -p oracle --test results`, and `kukulu_verify` with `docs/EVIDENCE.md` alone. The log entry quotes `runs`, `crashes`, `divergences` and `corpus_entries`.

**Depends on:** none

### P2-07: Run diff_accept for the configured duration

**Goal:** Fuzz `diff_accept` for `fuzz.seconds_per_target` on the 0.2.0 code and commit its result file and any divergence inputs.

**Files touched:** `testdata/results/fuzz-diff_accept-P2-07.json`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 2 (second bullet: "Before each `diff_*` run, empty the divergence directory: move the committed inputs aside and restore them afterwards"), §5.4, §3.6, §6.4, §5.1 (`fuzz.seconds_per_target` and `fuzz.max_divergence_files`, both read from the config by the tools). Follow the fuzz-run protocol, the long-run protocol and the results protocol in CLAUDE.md. Change no code.
- Before the run: `mkdir -p target/divergences-aside && mv testdata/results/divergences/*.bin target/divergences-aside/` (`target/` is gitignored), then `ls testdata/results/divergences | wc -l` prints 0.
- After the result file is written: `mv -n target/divergences-aside/*.bin testdata/results/divergences/ && rmdir target/divergences-aside`. `git status --short testdata/results/divergences/` must then show no deleted file; a new (`??`) file there is one of this run's divergences. If `rmdir` fails, a file of the same name was found again: `cmp` the two copies and delete the one left aside when they are equal.
- A resumed task that finds files in `target/divergences-aside/`: if the result file exists, go straight to the restore step; if not, leave them aside and start the run.
- Stage also any new file under `testdata/regressions/` and `testdata/results/divergences/`, and add each new divergence file to `docs/EVIDENCE.md`'s list with the result file.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new files staged; `seconds` is at least `fuzz.seconds_per_target` unless `crashes` is above 0.

**Out of scope:** triaging a crash or a divergence (P2-08); any code change.

**Verification:** `kukulu_verify` with exactly `testdata/results/fuzz-diff_accept-P2-07.json`, which runs `make fuzz-run TARGET=diff_accept TASK=P2-07` under its own 75-minute timeout; if the call returns before the run ends, wait as the long-run protocol says and never start the run again. Then restore the divergence inputs, add the new files to `docs/EVIDENCE.md`'s list, `git add`, `cargo test -p oracle --test results`, and `kukulu_verify` with `docs/EVIDENCE.md` alone. The log entry quotes `runs`, `crashes`, `divergences` and `corpus_entries`.

**Depends on:** none

### P2-08: Triage crashes and divergences

**Goal:** Resolve every crash and divergence the five runs found, or record that there were none.

**Files touched:** `docs/DESIGN.md`; only when there is something to triage: `crates/cleanlz4/src/`, `crates/cleanlz4/tests/`, `crates/oracle/src/diff.rs`, `crates/oracle/tests/diff.rs`, `testdata/regressions/`, `testdata/results/`, `docs/EVIDENCE.md`

**Design constraints:**

- SPEC §8 Phase 2 (second bullet: "a triage task follows and records it in `DESIGN.md`'s Divergence triage"; Manual check), §6.4 ("`docs/spec/` wins over the oracle"; "A normalization never makes cleanlz4 accept what the spec forbids"), §1 (Done: "zero crashes, and every divergence is triaged"). Follow the design-note protocol, the fuzz-run protocol and the clean-room protocol. The invariants apply to any decoder change, and the public API and `Error` stay as they are.
- Read the five result files `testdata/results/fuzz-<target>-P2-03.json` to `-P2-07.json`. A missing one belongs to a blocked run: name it in `docs/DESIGN.md` and go on.
- Nothing found (every `crashes` and `divergences` is 0): under `## Divergence triage` add the heading `### 0.2.0 re-verification (P2-08)` with "none" and the five `runs` figures. Change no code and finish.
- Otherwise, under that heading, one table row for each new file (a file under `testdata/regressions/` that `testdata/regressions/index.json` does not list, and a file under `testdata/results/divergences/` that P2-06 or P2-07 added): the file, the target, cleanlz4's outcome, the oracle's outcome, the `docs/spec/` document and section that decides it, and the resolution, one of:
    - (1) a cleanlz4 defect: fix it in `crates/cleanlz4/src/` from the spec text, with a named unit test under `crates/cleanlz4/tests/`;
    - (2) a choice the spec permits, or an input the spec forbids and the oracle accepts: a named normalization in `crates/oracle/src/diff.rs` with its section, a test in `crates/oracle/tests/diff.rs`, and an entry under `docs/DESIGN.md` `## Differential rules`.
- Copy each divergence input into `testdata/regressions/` as `<target>-<same name>` and add every triaged input to `testdata/regressions/index.json` with its target and resolution. Never resolve by deleting an input: when this task is done every file in `testdata/regressions/` is in the index and `crates/oracle/tests/regressions.rs` passes over all of them.
- Then re-run each affected target for the full duration, one at a time: `kukulu_verify` with exactly `testdata/results/fuzz-<target>-P2-08.json` (it runs `make fuzz-run TARGET=<target> TASK=P2-08`), with the divergence inputs moved aside and restored as in P2-07 for a `diff_*` target. Commit those result files and list them in `docs/EVIDENCE.md`. If a re-run still reports a crash or a divergence, block with the file names.

**Acceptance tests:** `crates/oracle/tests/regressions.rs`: `every_indexed_regression_input_passes_its_target_logic` and `every_index_entry_names_an_existing_file` pass with every input indexed. Each fix or normalization has a named test, listed in the commit body. With nothing to triage: none new.

**Out of scope:** weakening a fuzz target; a normalization that hides a defect; changing `testdata/malformed/`; changing `fuzz.seconds_per_target`.

**Verification:** with nothing to triage, `kukulu_verify` with `docs/DESIGN.md` alone. Otherwise as described above, then `kukulu_verify` with the Files touched that changed, never with a `testdata/results/fuzz-*` path a second time. The log entry carries `Crashes: <n>` and `Divergences: <n>` over the five runs and how each was resolved.

**Depends on:** none

### P2-09: The final benchmark

**Goal:** Measure decode throughput of the 0.2.0 code and the oracle on every corpus file and commit the figures; there is no target.

**Files touched:** `testdata/results/bench-P2-09.json`, `docs/EVIDENCE.md`, `docs/DESIGN.md`

**Design constraints:**

- SPEC §8 Phase 2 (third bullet: "with actual figures and no target"), §5.4, §3.6, §1 ("Performance is measured and reported. It is not a gate."). Follow the results protocol.
- `make bench TASK=P2-09` in the foreground, with nothing else running (in particular no fuzz run: `pgrep -f fuzz_run` prints nothing).
- `docs/DESIGN.md`, `## Measurements`, new heading `### Decode throughput, 0.2.0 (P2-09)`: the slowest and fastest `cleanlz4_mb_s` and `c_mb_s` with their input names, and the aggregate cleanlz4 throughput (P1-05's command over this file), all read from the file. No target, no judgement against one.
- Add the file to the fenced list in `docs/EVIDENCE.md`.

**Acceptance tests:** the existing `crates/oracle/tests/results.rs` tests `every_result_file_matches_its_schema` and `evidence_index_lists_every_result_file` pass with the new file staged.

**Out of scope:** decoder changes; changing `bench.repeats`; profiling the oracle (denied by the policy).

**Verification:** `git add` the files; `cargo test -p oracle --test results`; `kukulu_verify` with the Files touched. The log entry quotes the slowest and fastest `cleanlz4_mb_s`.

**Depends on:** P0-01

### P2-10: Phase 2 end: push and record the manual check

**Goal:** Push the branch and record Phase 2's manual check for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 2 (Manual check). Follow the phase-end protocol in CLAUDE.md. No code change.

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs.

**Verification:** `kukulu_verify` with no files. The log entry carries `Manual check: NOT VERIFIED (human): testdata/results/conformance-P2-01.json shows 2349 of 2349 across ten categories and malformed-P2-02.json shows 144 of 144 with 0 panics; every testdata/results/fuzz-*-P2-0[3-7].json shows crashes == 0 and seconds of at least fuzz.seconds_per_target; DESIGN.md's Divergence triage, 0.2.0 re-verification (P2-08), says none or resolves every divergence; read bench-P2-09.json (actual figures, no target)`.

**Depends on:** none

## Phase 3 — Evidence and release prep

### P3-01: The CHANGELOG entry for 0.2.0 and the crate versions

**Goal:** Give the release its numbered changelog entry and set both crates to `0.2.0`, so the release tool works unaided.

**Files touched:** `CHANGELOG.md`, `crates/cleanlz4/Cargo.toml`, `crates/oracle/Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.lock`, `crates/oracle/tests/release.rs`

**Design constraints:**

- SPEC §8 Phase 3 (second bullet), §1 (Done: "`CHANGELOG.md` has a numbered entry for the release, and the crate versions match it"), §8 preamble (no breaking change, so the release is `0.2.0`). The invariants apply.
- `version = "0.2.0"` in `crates/cleanlz4/Cargo.toml` and `crates/oracle/Cargo.toml`. Nothing else in a manifest changes: `crates/cleanlz4`'s `[dependencies]` stays empty, no crate is added (constraint `no-new-dependencies`), and `fuzz/Cargo.toml` is not touched.
- Lockfiles: `cargo build --workspace` rewrites the `version` lines of `cleanlz4` and `oracle` in `Cargo.lock`, and `cargo +nightly fuzz build` does the same in `fuzz/Cargo.lock`. Cargo is offline. `git diff Cargo.lock fuzz/Cargo.lock` must show those version lines and nothing else; any other change blocks.
- `CHANGELOG.md`: a new entry headed `## [0.2.0] - <date>` above `## [0.1.0] - 2026-11-10`, where `<date>` is what `date -u +%F` prints on the day of this commit; and a link line `[0.2.0]: https://github.com/ericmann/cleanlz4/releases/tag/v0.2.0` above the `[0.1.0]:` line. Keep the file's format (Keep a Changelog). The 0.1.0 entry is not edited.
- The entry covers everything this flight changed. Read it off `git log --oneline main..HEAD` and the files, never from memory:
    - Added: the dictionary differential test against the `lz4` program; the two tests that kill 0.1.0's surviving mutants; the allocation tests for the scratch buffer (when it was kept); the release test.
    - Changed: `is_magic` merged into `magic_kind`; one scratch buffer per frame (or, when P1-07 reverted it, that it was measured and not kept); the corrected comments; `make conformance-check` also runs for changes to the runner and the dictionary module.
    - Verified: the baseline and the full conformance run, the malformed run, the five fuzz runs (runs, crashes, divergences) and the final benchmark, each with its result file's name and the figures read from that file.
    - Notes: no change to the public API, the `Error` variants or any key of `testdata/config.json`; the oracle library and `lz4` program versions are 0.1.0's; anything a blocked or skipped task left undone (from `docs/PROGRESS.md`).
- `crates/oracle/tests/release.rs`, with no version written as a literal: the `version` lines of the two crates' `[package]` sections are equal; `CHANGELOG.md`'s first `## [` heading is `## [<that version>] - YYYY-MM-DD` (four digits, two, two); and a line starts with `[<that version>]: `.

**Acceptance tests:** `crates/oracle/tests/release.rs`: `crate_versions_agree`, `changelog_newest_entry_is_the_crate_version`, `changelog_links_the_crate_version`.

**Out of scope:** tagging or publishing; `docs/EVIDENCE.md` and `README.md` (P3-02); `docs/SUMMARY.md`; any code change in `crates/cleanlz4/src/`.

**Verification:** `kukulu_verify` with the Files touched (`fuzz/Cargo.lock` makes it run the fuzz build); `grep -n '^version' crates/cleanlz4/Cargo.toml crates/oracle/Cargo.toml` shows `0.2.0` twice.

**Depends on:** none

### P3-02: Refresh EVIDENCE.md and README.md

**Goal:** Make the evidence index state what both flights produced and SPEC §1's Done items as they stand, and bring the README up to 0.2.0.

**Files touched:** `docs/EVIDENCE.md`, `README.md`, `crates/oracle/tests/results.rs`

**Design constraints:**

- SPEC §8 Phase 3 (first bullet), §1 (Done means: six items in SPEC 0.3), §5.4 ("`docs/DESIGN.md` and these files are the only places results live"), §3.6. `kukulu_similarity`, `kukulu_attest` and `SUMMARY.md` are the summarize stage's: do not call those tools and do not write `docs/SIMILARITY.md`, `docs/attestation/` or `docs/SUMMARY.md`.
- `## Evidence files`: the fenced list already names every result file (each task added its own). Check it against `git ls-files testdata/results`, keep it sorted, and keep `docs/DESIGN.md`, both spikes, `testdata/MANIFEST.json` and `testdata/malformed/index.json` in it. This list is the evidence for the attestation.
- `## Results`: the table gains a first column `Flight` (`0.1.0` or `0.2.0`). The existing rows are 0.1.0's (their task ids are 0.1.0's; say once that its plan is `docs/flights/cleanlz4/PLAN.md`). Add one row for every result file this flight added, with its headline counts read from the file: `conformance-P0-03` (baseline), `bench-P1-05` (before the scratch buffer), `bench-P1-07` (after), `conformance-P2-01`, `malformed-P2-02`, the five `fuzz-*-P2-03` to `P2-07`, any `fuzz-*-P2-08` re-run, and `bench-P2-09`.
- `## Done checklist`: rewrite it for SPEC 0.3 §1's six items, each "met", "not met" or "owed to the summarize stage", with the file or test that shows it:
    1. every §6 requirement has its tests (dictionary entries are now compared with the `lz4` program in `crates/oracle/tests/dict_corpus.rs`; remove the sentence that says frames with a dictionary have no oracle decode);
    2. every §7 command exits zero, naming the task of this flight that ran each one;
    3. the fuzz campaign ran for its configured duration with zero crashes, and every divergence is triaged in `docs/DESIGN.md` `## Divergence triage` (figures from the five result files);
    4. `kukulu_similarity` with the same-language controls and `kukulu_attest`: owed to the summarize stage;
    5. `CHANGELOG.md` has the `0.2.0` entry and the crate versions match it (P3-01; `crates/oracle/tests/release.rs`);
    6. a draft PR is open: owed to the end of the run.
- Name every blocked and skipped task of this flight under the checklist, from `docs/PROGRESS.md`, or say there are none.
- `README.md`: what 0.2.0 is (a hardening release on 0.1.0: the dictionary differential test, the mutant tests, the decoder cleanups, the re-verification), a pointer to `CHANGELOG.md` and `docs/EVIDENCE.md`, that 0.1.0's plan, review and summary are under `docs/flights/cleanlz4/`, and the commands of SPEC §7 (add `make conformance-check`, the guard and the deps script to the list). Do not describe the pipeline.
- New test in `crates/oracle/tests/results.rs`: every tracked `testdata/results/*.json` file's name appears in the text after the `## Results` heading of `docs/EVIDENCE.md`.

**Acceptance tests:** `crates/oracle/tests/results.rs`: `evidence_results_table_names_every_result_file` (new), `evidence_index_lists_every_result_file` (existing).

**Out of scope:** the similarity score, the attestation bundle, the summary; `CHANGELOG.md`; any code change outside the one test.

**Verification:** `kukulu_verify` with the Files touched.

**Depends on:** none

### P3-03: Phase 3 end: push and record the manual check

**Goal:** Push the branch and record Phase 3's manual check and the summarize stage's inputs for a human.

**Files touched:** none

**Design constraints:** SPEC §8 Phase 3 (third bullet; Manual check), §5.3 (Reading the result), §9 Q4. Follow the phase-end protocol in CLAUDE.md. No code change. `docs/HANDOFF.md`, written when the run finishes, must name `docs/EVIDENCE.md`'s fenced list as the evidence for the attestation, and must carry this note for the summarizer, word for word: `Similarity: docs/kukulu.json holds one negative pair (rust-independent-vs-indep) and one positive pair (rust-derived-vs-indep), both Rust-vs-Rust, because kukulu 0.8.0 refuses a second pair of either role; SPEC 5.3's Rust-vs-C controls (independent-vs-c, derived-vs-c) are not scored in this run. The Rust-vs-Rust band applies to subject-vs-indep. It does not apply to subject-vs-c, whose Rust-vs-C reading is 0.1.0's docs/flights/cleanlz4/SIMILARITY.md. SUMMARY.md says which band applies to each test pair and repeats the tool's verdict verbatim (SPEC 5.3, 9 Q4).`

**Acceptance tests:** none beyond `kukulu_verify` being green.

**Out of scope:** any change to code or docs; calling `kukulu_similarity` or `kukulu_attest`.

**Verification:** `kukulu_verify` with no files. The log entry carries the similarity note above and `Manual check: NOT VERIFIED (human): the owner reads docs/SIMILARITY.md (written by the summarize stage) against the same-language controls rust-independent-vs-indep and rust-derived-vs-indep, and reads subject-vs-c against 0.1.0's docs/flights/cleanlz4/SIMILARITY.md; the owner runs node docs/attestation/verify.mjs docs/attestation --key sha256:<owner's fingerprint> from a clean checkout on a second machine (the bundle is under docs/flights/cleanlz4-0-2-0/attestation once the flight is archived)`.

**Depends on:** none

## Spec issues

- **§5.3's six-pair similarity block cannot be written.** Kūkulu 0.8.0 validates `cleanroom.similarity` when it loads `docs/kukulu.json` and refuses SPEC's block: `docs/kukulu.json cleanroom.similarity.pairs needs exactly one pair with role "negative" (found 2)`; its own documentation says "exactly one `negative`, exactly one `positive` and at least one `test`". With the block as given, `kukulu_status` and every other tool fail, so the flight could not start. Resolution: the file holds the two Rust-vs-Rust controls and both test pairs; `independent-vs-c` and `derived-vs-c` are left out, against "Score every pair; no pair is dropped". Proposed SPEC edit: say which band a single run keeps, or wait for a Kūkulu that takes a band per language pairing. The owner can also score the Rust-vs-C band by hand after the flight, by swapping the two control pairs in `docs/kukulu.json` and running `kukulu_similarity` again. §9 Q4 is moot while the tool takes one band.
- **The flight slug from SPEC's title is taken.** The title gives `cleanlz4`, and `docs/flights/cleanlz4/` exists. Resolution: `cleanlz4-0-2-0`. Proposed SPEC edit: put the version in the title, or name the slug.
- **`lockdown.after` is `P0-03`, and §8 Phase 0 describes one task.** Resolution: Phase 0 is three check tasks, the third being the lockdown task. Proposed SPEC edit: name the lockdown task id.
- **§8 Phase 0 runs `make conformance-check`, which writes no file, and §3.6 says a result that is only printed does not exist.** Resolution: P0-03 runs both it and `make conformance TASK=P0-03 CATEGORIES=all`.
- **`cargo test -p oracle --test manifest` passes when `testdata/corpora/` is absent** (it prints a notice). §5.2 relies on it to show the corpora match. Resolution: P0-01 checks that the directories exist first. Proposed: a strict mode in a later flight.
- **Task ids repeat across flights, and §5.4 names result files by task id.** A review-fix task of this flight could mint a name 0.1.0 used (`malformed-R3-02.json` exists). Resolution: this plan's ids produce no such name, `docs/EVIDENCE.md` gains a Flight column, and CLAUDE.md's flight notes forbid reusing an existing result file name. Proposed SPEC edit: add the flight to the file name.
- **Binding `make conformance-check` to `dict.rs` proves little by itself.** The check decodes the corpus already on disk and never runs the generator. Resolution: bound as SPEC says; a task that edits `dict.rs` also runs `make corpus` and checks the manifest is unchanged. `make corpus` is not bound in the config, because a mutation run would regenerate the corpus from a mutant.
- **"Slower than before by more than the spread" names no statistic.** Resolution: the aggregate throughput over all inputs, one confirming run before a revert (Decisions).
- **"Decodes every `dictionary` entry ... with `oracle::cli::decompress_frame_with_dict`"**: 18 of the 72 entries are raw blocks. Resolution: they go through `decompress_block_with_dict`, which wraps them as §6.5 describes.
- **§1 and the Phase 3 manual check name `docs/attestation/`**, but the summarizer archives a finished flight, which moves the bundle to `docs/flights/<slug>/attestation/`. Resolution: P3-03's manual check names both paths.
- **SPEC does not say whether the PR stays a draft until the owner's Phase 3 checks are done.** §1 says "a draft PR is open"; Kūkulu's default `policies.finalize` marks a fully signed, non-breaking flight's PR ready and enables auto-merge. `policies` is left out, as the planner's rules require. The owner sets `"policies": { "finalize": "draft" }` before the flight to hold the PR.
- **Hour-long verify calls.** Each fuzz run is one `kukulu_verify` call of up to 75 minutes, past the MCP idle timeout of 1,800 s (SPEC, Operator notes). Resolution: the long-run protocol tells the implementer to wait for the result file and never to start the run twice.
- **§5.1 has a stray table header** (`| Key | Default | Meaning |` with no rows) above its opening paragraph. No plan impact.
- **§4 lets `xxh32.rs` import `error`**; CLAUDE.md's module map, written for 0.1.0, says `core` and `std` only. No plan impact: no task touches `xxh32.rs`, the rule `leaf-modules-import-only-error` already allows `crate::error`, and the map is left as 0.1.0 wrote it.
- **The prior-art trees of §5.3 were not checked.** The guard denies every path under them, to the planner as to everyone. `/home/ericmann/Projects/cleanflate/crates/cleanflate/src` exists. `kukulu_similarity` refuses a missing or empty pair directory, so the owner confirms the other three before the flight (SPEC, Operator notes).
