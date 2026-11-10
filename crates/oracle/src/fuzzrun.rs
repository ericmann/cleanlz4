//! Running a fuzz target for a configured time and turning the outcome into a
//! result file (SPEC sections 5.4 and 7). The pure parts are here; the process
//! handling is in `bin/fuzz_run.rs`.

use crate::config::{repo_root, Config};
use crate::manifest::sha256_hex;
use crate::results::FuzzResult;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Every fuzz target, in the order `make fuzz-smoke` runs them.
pub const TARGETS: [&str; 5] = [
    "no_panic_block",
    "no_panic_frame",
    "no_panic_stream",
    "diff_roundtrip",
    "diff_accept",
];

/// How long the fuzzer may run: `fuzz.seconds_per_target`, or `fuzz.smoke_seconds` for a smoke run.
pub fn duration_seconds(cfg: &Config, smoke: bool) -> u64 {
    if smoke {
        cfg.fuzz.smoke_seconds
    } else {
        cfg.fuzz.seconds_per_target
    }
}

/// The arguments to `cargo` for one run.
pub fn fuzz_args(cfg: &Config, target: &str, smoke: bool) -> Vec<String> {
    vec![
        "+nightly".into(),
        "fuzz".into(),
        "run".into(),
        "-a".into(),
        target.into(),
        format!("fuzz/corpus/{target}"),
        "--".into(),
        format!("-max_total_time={}", duration_seconds(cfg, smoke)),
        format!("-seed={}", cfg.fuzz.seed),
        "-print_final_stats=1".into(),
    ]
}

/// The executed-units figure from the fuzzer's final statistics
/// (`stat::number_of_executed_units: N`).
pub fn parse_final_stats(output: &str) -> Option<u64> {
    output.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("stat::number_of_executed_units:")?;
        rest.trim().parse().ok()
    })
}

/// Assemble a result from what a run observed.
#[allow(clippy::too_many_arguments)]
pub fn assemble_result(
    cfg: &Config,
    task: &str,
    target: &str,
    seconds: u64,
    runs: u64,
    crashes: u64,
    divergences: u64,
    corpus_entries: u64,
) -> FuzzResult {
    FuzzResult {
        task: task.into(),
        target: target.into(),
        seconds,
        runs,
        crashes,
        divergences,
        corpus_entries,
        seed: cfg.fuzz.seed,
    }
}

/// The names of the files directly inside `dir` (none when it does not exist).
pub fn file_names(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.filter_map(Result::ok) {
            if e.path().is_file() {
                out.insert(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    out
}

/// `testdata/regressions/`.
pub fn regressions_dir() -> PathBuf {
    repo_root().join("testdata").join("regressions")
}

/// The regression file name for an input: `<target>-<first 16 hex of its SHA-256>.bin`.
pub fn regression_name(target: &str, bytes: &[u8]) -> String {
    let h = sha256_hex(bytes);
    format!("{target}-{}.bin", h.get(..16).unwrap_or(&h))
}
