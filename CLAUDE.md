# cleanlz4

A clean-room, memory-safe, zero-dependency Rust decoder for the LZ4 block and frame formats and XXH32, written from the three documents in `docs/spec/` alone. `docs/SPEC.md` is the source of truth.

## Principles

- [ ] Clean room: the only inputs are `docs/SPEC.md`, `docs/spec/`, Rust std documentation, general Rust and computer-science knowledge, the oracle's C function names in SPEC §6.5, the `lz4` program run as a black box, and corpus files as opaque bytes.
- [ ] No `unsafe` and no dependencies in `crates/cleanlz4`.
- [ ] No reachable panic in library code: every length, offset and index is checked and returns a typed `Error`.
- [ ] Every non-trivial decision is in `docs/DESIGN.md` with the `docs/spec/` document and section it implements.
- [ ] Every tunable is a key in `testdata/config.json`; its number is written nowhere else.
- [ ] Every conformance, malformed, fuzz and bench count is committed under `testdata/results/` in the commit of the work that produced it.
- [ ] The oracle is a black box, reached only through `crates/oracle/src/ffi.rs`.

## Commands

- fmt: `cargo fmt --all -- --check`
- lint: `cargo clippy --workspace --all-targets -- -D warnings`
- test: `cargo test --workspace`; one test: `cargo test --workspace <filter>`
- guard: `node /home/ericmann/.claude/plugins/cache/displacetech/kukulu/0.8.0/scripts/cleanroom-guard.mjs --self-test docs/cleanroom.json`
- deps: `bash scripts/check-deps.sh` (its own test: `bash scripts/test-check-deps.sh`)
- spec sources: `node scripts/verify-spec-sources.mjs` (its own test: `node scripts/test-verify-spec-sources.mjs`)
- all of the above: `make check`
- data: `make corpus`, `make conformance TASK=<id> CATEGORIES=<list|all>`, `make conformance-check`, `make malformed TASK=<id>`
- fuzz: `make fuzz-smoke`, `make fuzz-run TARGET=<target> TASK=<id>`; fuzz build: `cargo +nightly fuzz build`
- bench: `make bench TASK=<id>`

## Module map

| Path | Responsibility | May import |
|---|---|---|
| `crates/cleanlz4/src/xxh32.rs` | XXH32, one-shot and streaming | `core`, `std` |
| `crates/cleanlz4/src/block.rs` | Block decoding into a caller-provided buffer | `core`, `std`, `crate::error` |
| `crates/cleanlz4/src/frame.rs` | Frame parsing, block sequencing, checksums, skippable and legacy frames, the one-shot driver | `block`, `xxh32`, `error` |
| `crates/cleanlz4/src/stream.rs` | Streaming decoder state machine | `frame`, `block`, `xxh32`, `error` |
| `crates/cleanlz4/src/error.rs` | The typed `Error` enum and `Result` | none |
| `crates/cleanlz4/src/lib.rs` | Public API, options, `DEFAULT_MAX_OUTPUT_BYTES` | all of the above |
| `crates/oracle/` | FFI to the C library (`src/ffi.rs` only), config, corpus generation, runners, the reference side of every differential test | `cleanlz4`, the binding crate |
| `fuzz/` | `cargo-fuzz` targets, five-line wrappers over `oracle::fuzz_logic` | `cleanlz4`, `oracle` |
| `testdata/` | `config.json`, `MANIFEST.json`, the malformed corpus, regressions, result files; corpora and the generated conformance corpus (both gitignored) | none |

## Constraints

Rules marked (rule: id) are checked by `kukulu_verify` from `docs/kukulu.json`.

1. The token `unsafe` appears nowhere under `crates/cleanlz4/` (rule: `no-unsafe-token`); `lib.rs` starts with `#![forbid(unsafe_code)]` (checked by `scripts/check-deps.sh`).
2. `crates/cleanlz4/Cargo.toml` has an empty `[dependencies]` table (`scripts/check-deps.sh`) and no other dependency table (rule: `cleanlz4-no-extra-dependency-tables`).
3. `lib.rs` carries `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing))]`, and no `allow` or `expect` attribute under `crates/cleanlz4/src/` names those lints (rule: `no-panic-lint-suppression`).
4. No `unreachable!`, `todo!` or `unimplemented!` under `crates/cleanlz4/src/` (rule: `no-panicking-macros`). No `assert!`, unchecked arithmetic on input-derived values, or slicing by index outside `#[cfg(test)]` code: reviewer checks by reading.
5. `crates/cleanlz4/` never names `crates/oracle` or the binding crate (rule: `cleanlz4-independent-of-oracle`).
6. `xxh32.rs`, `block.rs` and `error.rs` import nothing crate-internal except `crate::error` (rule: `leaf-modules-import-only-error`); `frame.rs` never imports `stream` (rule: `frame-does-not-import-stream`).
7. No LZ4 or xxHash crate other than the binding crate is named in any manifest, lockfile or source file (rule: `no-prior-art-crates`).
8. The only C functions called are the fifteen named in SPEC §6.5 (rule: `oracle-calls-only-the-permitted-c-api`), and the binding crate is named only in `crates/oracle/src/ffi.rs` and `crates/oracle/Cargo.toml` (rule: `binding-crate-only-in-ffi-module`). Under `crates/oracle/src/` and `fuzz/fuzz_targets/`, `unsafe` appears only in `ffi.rs` (rule: `oracle-unsafe-only-in-ffi`). A counting allocator (`unsafe impl GlobalAlloc` over `std::alloc::System`, calling no C function) may live in `crates/oracle/tests/` and `crates/oracle/examples/`, and nothing else there is `unsafe`: reviewer checks by reading.
9. The 1 GiB default is written only on the `DEFAULT_MAX_OUTPUT_BYTES` line of `lib.rs` and in `testdata/config.json` (rule: `no-hardcoded-output-limit`).
10. `fuzz.seconds_per_target`, `fuzz.smoke_seconds`, `fuzz.seed`, `bench.repeats` and `stream.chunking_cases` are read from `testdata/config.json`, never written as literals (rules: `no-hardcoded-fuzz-tunables`, `no-hardcoded-bench-repeats`, `no-hardcoded-chunking-cases`), and so are `fuzz.max_divergence_files`, `dict.sizes_bytes` and the `conformance.*` keys (rule: `no-hardcoded-corpus-tunables`). The other keys of `testdata/config.json` likewise: reviewer checks by reading.
11. Never read under `~/.cargo/registry/`, `~/.cargo/git/`, any `target/**/build/` directory, any LZ4 or xxHash header or source tree, or the binding crate's source or generated documentation; never run `cargo doc`, `cargo expand` or `cargo vendor`. The guard enforces what it can see; the rule binds where it cannot. Reviewer checks by reading the log.
12. Never disassemble, profile or trace the oracle, `liblz4` or the `lz4` program (policy `commandPatterns`). Reviewer checks by reading the log.
13. No network after the flight's lockdown task (P0-03): `.cargo/config.toml` keeps `net.offline = true` and gains no registry or source table (rule: `cargo-stays-offline`), and no tool downloads anything. Reviewer checks the rest by reading.
14. No expected XXH32 digest is written as a literal in any test; vectors come from the oracle's frame checksums at test time. Reviewer checks by reading.
15. `docs/spec/`, `docs/cleanroom.json`, `docs/kukulu.json`, `.kukulu/` and the archived flights under `docs/flights/` are never edited. Reviewer checks by reading the diff.
16. Every decision entry in `docs/DESIGN.md` cites a `docs/spec/` document and section heading, or a spike file, or cleanlz4's own measurement; never another implementation. Reviewer checks by reading.
17. A file under `testdata/results/` is a tool's output, never typed by hand, and is committed with the work that produced it. Reviewer checks by reading.

## Task protocols

- **Clean-room protocol.** Write decoder code from `docs/spec/` text only, with the section open. A denied tool call is never worked around: note it in the task's log entry and continue another way, or block. A fact the three documents do not give blocks the task; it is never looked up. Read `docs/spec/`, `docs/cleanroom.json`, `docs/kukulu.json` and `.kukulu/` with the Read tool or `grep`, never `sed`: the guard takes `sed` on a protected path for a write.
- **Oracle protocol.** Only `crates/oracle/src/ffi.rs` contains `unsafe` or the binding crate's name, and it calls only SPEC §6.5's functions. Learn the binding's Rust names and parameter types from compiler diagnostics alone. If a §6.5 function or preferences field is missing, call `kukulu_task_block` naming it. Do not write a C function's name followed by `(` in a comment unless it is one of the fifteen.
- **Design-note protocol.** A task that makes a decision adds an entry under `docs/DESIGN.md` `## Decisions`: the decision, the document and section heading it implements (for example "Frame format, Block checksum"), and the test that holds it.
- **Results protocol.** Result files follow SPEC §5.4's keys exactly, are written by the binary that ran the work, and are staged in the same commit. In that commit, add each new file under `testdata/results/` to the fenced list of `docs/EVIDENCE.md`, kept in byte order (`LC_ALL=C sort -c`): `git add` the files, then run `cargo test -p oracle --test results` before committing (`evidence_index_lists_every_result_file` sees only staged and tracked files). Quote the headline counts in the log entry.
- **Long-run protocol.** A command that takes more than ten minutes runs only through `kukulu_verify`: pass the task's result-file path among the files and the matching `extraVerify` command runs with its own timeout and writes the file. If that call returns an error or a timeout before the command has finished (the MCP idle timeout is about 30 minutes), the command is still running: never start it again. Wait, in shell calls of under ten minutes each (`until [ -f <result file> ]; do sleep 30; done`), until the result file exists and `pgrep -f fuzz_run` prints nothing. Afterwards run `cargo test -p oracle --test results` in the foreground, then commit. Never pass a `testdata/results/fuzz-*` or `conformance-*` path to `kukulu_verify` outside the task that owns it.
- **Fuzz-run protocol.** A run task is done when its result file is written and committed, even with `crashes` or `divergences` above zero: stage the new files under `testdata/regressions/` and `testdata/results/divergences/`, put `Crashes: <n>` and `Divergences: <n>` in the log, and change no code. The flight's triage task triages. A run that could not build or start blocks the task. Before a `diff_*` run, move the committed `testdata/results/divergences/*.bin` aside and move them back once the result file is written: `fuzz.max_divergence_files` would otherwise hide a new divergence.
- **Spike protocol.** Work to the stated time box, then write `docs/spikes/<id>.md` with the question, the method, the observations as a table and the outcome the task asks for. A spike observes the oracle through `crates/oracle` only.
- **Tuning protocol.** Measure with the key's current value (before) and with the alternatives the task names (after). Record the table under `docs/DESIGN.md` `## Measurements`, on the commit body's `Measurement:` line and in the log. Change the key only when the task's criterion says so, and only in `testdata/config.json` (or on the `DEFAULT_MAX_OUTPUT_BYTES` line, with the config, for the output limit).
- **Check-only protocol.** A task whose Files touched is `none` changes no file. Run its checks, quote what each printed in the log entry, and carry the task with an empty commit as in the phase-end protocol. A check that fails blocks the task with the failing output; nothing is repaired in a check-only task.
- **Mutant protocol.** A task that kills a named mutant shows it: apply the mutant with the Edit tool, run the named test and see it fail, undo the edit, prove the undo with `git diff --exit-code -- <file>`, and quote the failing assertion in the log entry. A mutant is never committed.
- **Phase-end protocol.** Push the branch if this run's policies say `push=on` and `origin` exists (else log `Push: skipped (policy)` or `Push: no remote configured`). Carry the task with an empty commit: `git commit --allow-empty -F <file>`. Put the task's `Manual check: NOT VERIFIED (human): …` line in the log and go on.

## Flight cleanlz4-0-2-0 notes

The 0.2.0 hardening flight (SPEC 0.3 §8). It builds on 0.1.0 and rebuilds nothing.

- Task ids restart at P0-01. "0.1.0's P2-12" names a task of the archived plan, `docs/flights/cleanlz4/PLAN.md`; the ids inside 0.1.0's result file names (`P1-12`, `P2-12`, `P2-13`, `R3-02`, `P4-04` to `P4-11`) are 0.1.0's.
- Lockdown task: P0-03. Triage task: P2-08. Divergence inputs are moved aside to `target/divergences-aside/` (gitignored).
- Baseline, from 0.1.0's files: conformance 2349 of 2349 in ten categories; malformed 144 of 144 with 0 panics; `lz4` program v1.9.4; binding `1.11.1+lz4-1.10.0`. `make corpus` regenerates `testdata/MANIFEST.json` byte for byte; a diff there blocks the task.

Constraints of this flight:

1. `crates/cleanlz4/` has no `is_magic(`; `magic_kind` is the one function that names a magic number's kind (rule: `single-magic-classifier`, from P1-04).
2. `crates/oracle/Cargo.toml` and `fuzz/Cargo.toml` gain no dependency (rule: `no-new-dependencies`); a task that seems to need a new crate blocks. `Cargo.lock` and `fuzz/Cargo.lock` change only in P3-01, and only in the `version` lines of `cleanlz4` and `oracle`: reviewer checks by reading the diff.
3. No `pub` item of `crates/cleanlz4` is added, removed or changed in signature, and `error.rs` is not touched. Reviewer checks by reading `git diff main -- crates/cleanlz4/src/`.
4. `testdata/config.json`, `testdata/MANIFEST.json`, `testdata/malformed/` and 0.1.0's result files do not change. Reviewer checks by reading `git diff main --stat -- testdata/`.
5. No result file takes a name that already exists under `testdata/results/`; a review-fix task that produces one uses its own id and checks the name is free. Reviewer checks by reading.
6. Nothing is downloaded, and `make manifest-corpora` is never run. Never run `git clean -x` (it deletes the gitignored corpora, which cannot be fetched again), and leave `git stash list` alone. Reviewer checks by reading the log.
7. P1-07's 7.28% threshold is 0.1.0's measured spread, quoted from `docs/DESIGN.md`; it appears in no code and no config. Reviewer checks by reading.

Codebase notes carried from 0.1.0:

- clippy under `-D warnings` wants `slice::as_chunks::<N>()` instead of `chunks_exact` with a constant, `std::iter::repeat_n`, `is_multiple_of` and `explicit_counter_loop` fixes. Run clippy before `kukulu_verify`.
- rustfmt reformats source, so a scripted string patch can fail silently. Edit with the Edit tool and check the file.
- The oracle's block decoder needs spare output capacity: differential block tests give it 65,536 extra bytes.
- `cargo test --workspace` reads the generated corpus under `testdata/conformance/`; `make corpus` (about two minutes, about 1 GB) remakes it.
- `kukulu_feedback_log` commits and moves HEAD: call it after `kukulu_task_done`.

## Commit template

```text
<ID>: <title>

Goal: <one sentence>
Tests: <test files and names added or changed>
Interpretation: <choices made where the task or SPEC allowed two readings, or "none">
Measurement: <before and after, for a tuning task; else "n/a">
Manual check: <what a human must still check, or "none">
```

Write the message to a file and commit with `git commit -F <file>`.

SPEC.md wins over PLAN.md wins over code comments.
