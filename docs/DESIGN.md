# Design notes

## Decisions

- Lint attributes use the `cfg_attr(not(test), deny(...))` form in `crates/cleanlz4/src/lib.rs`, so the no-panic lints bind library code while tests may use `unwrap`, `expect` and indexing (SPEC section 3.3). Implements: project rule on no reachable panic; held by the `no-panic-lint-suppression` constraint and by `make check` (clippy).
- Tool versions are recorded in `testdata/MANIFEST.json` `tools`: the oracle library version is read from `Cargo.lock` (the SPEC section 6.5 surface has no version function), and the installed `lz4` program's first `-V` line is recorded separately; the two may differ in version (CLI 1.9.4, library 1.10.0 at the time of recording). Implements: SPEC section 5.2 (corpus provenance); held by `crates/oracle/tests/manifest.rs` `manifest_records_tool_versions`.
- The manifest is written by the `manifest` binary, never by hand; cleanlz4's own measurement (a SHA-256 over each downloaded archive and each extracted file) is the provenance. Held by `corpus_files_on_disk_match_manifest`.
- The oracle's block functions are wrapped in `crates/oracle/src/ffi.rs` as safe slice-taking functions, the only place with `unsafe`; `crates/oracle/src/block.rs` adds length checks (lengths beyond a C `int` panic, as this is test tooling) and a unit `Rejected` error. Implements: SPEC section 6.5 (Block); cleanlz4's own tooling decision. Held by `crates/oracle/tests/ffi_block.rs`.

## Spec rules implemented

## Differential rules

## Divergence triage

## Measurements
