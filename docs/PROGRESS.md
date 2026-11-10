# cleanlz4 build progress

Branch: build/2026-11-10-2
Started: 2026-11-10T15:48:48.480Z

## Tasks

- [ ] P0-01 Confirm the corpora, the tool versions and the spec sources
- [ ] P0-02 Confirm make check and the fuzz toolchain
- [ ] P0-03 Regenerate the corpus, record the baseline conformance run and lock down
- [ ] P0-04 Phase 0 end: push and record the manual check
- [ ] P1-01 Dictionary differential test: every dictionary entry against the lz4 program
- [ ] P1-02 Kill the dict_dependent mutant in the oracle's dict module
- [ ] P1-03 Kill the dropped-dictionary mutant in the conformance runner
- [ ] P1-04 Merge is_magic into magic_kind
- [ ] P1-05 Benchmark before the scratch-buffer change
- [ ] P1-06 Reuse one scratch buffer across blocks
- [ ] P1-07 Measure the scratch-buffer change and decide
- [ ] P1-08 Phase 1 end: push and record the manual check
- [ ] P2-01 The full conformance run
- [ ] P2-02 The full malformed run
- [ ] P2-03 Run no_panic_block for the configured duration
- [ ] P2-04 Run no_panic_frame for the configured duration
- [ ] P2-05 Run no_panic_stream for the configured duration
- [ ] P2-06 Run diff_roundtrip for the configured duration
- [ ] P2-07 Run diff_accept for the configured duration
- [ ] P2-08 Triage crashes and divergences
- [ ] P2-09 The final benchmark
- [ ] P2-10 Phase 2 end: push and record the manual check
- [ ] P3-01 The CHANGELOG entry for 0.2.0 and the crate versions
- [ ] P3-02 Refresh EVIDENCE.md and README.md
- [ ] P3-03 Phase 3 end: push and record the manual check

## Log

(one entry per task, appended by implement)
