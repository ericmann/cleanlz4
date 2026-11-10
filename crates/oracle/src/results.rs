//! Result files under `testdata/results/` (SPEC section 5.4). Each is written by
//! the binary that ran the work; the keys are exactly SPEC's.

use crate::config::repo_root;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const CAT_BLOCK: &str = "block";
pub const CAT_FRAME_INDEPENDENT: &str = "frame-independent";
pub const CAT_FRAME_LINKED: &str = "frame-linked";
pub const CAT_CHECKSUMS: &str = "checksums";
pub const CAT_CONTENT_SIZE: &str = "content-size";
pub const CAT_DICTIONARY: &str = "dictionary";
pub const CAT_SKIPPABLE: &str = "skippable";
pub const CAT_LEGACY: &str = "legacy";
pub const CAT_CONCATENATED: &str = "concatenated";
pub const CAT_SYNTHETIC: &str = "synthetic";

/// The ten category names.
pub const CATEGORIES: [&str; 10] = [
    CAT_BLOCK,
    CAT_FRAME_INDEPENDENT,
    CAT_FRAME_LINKED,
    CAT_CHECKSUMS,
    CAT_CONTENT_SIZE,
    CAT_DICTIONARY,
    CAT_SKIPPABLE,
    CAT_LEGACY,
    CAT_CONCATENATED,
    CAT_SYNTHETIC,
];

/// Counts for one category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryCounts {
    pub total: u64,
    pub passed: u64,
}

/// `conformance-<task-id>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConformanceResult {
    pub task: String,
    pub corpus_sha256: String,
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub by_category: BTreeMap<String, CategoryCounts>,
}

/// `malformed-<task-id>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MalformedResult {
    pub task: String,
    pub total: u64,
    pub matched_expected_error: u64,
    pub panics: u64,
}

/// `fuzz-<target>-<task-id>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuzzResult {
    pub task: String,
    pub target: String,
    pub seconds: u64,
    pub runs: u64,
    pub crashes: u64,
    pub divergences: u64,
    pub corpus_entries: u64,
    pub seed: u64,
}

/// One input of a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchInput {
    pub name: String,
    pub bytes: u64,
    pub cleanlz4_mb_s: f64,
    pub c_mb_s: f64,
}

/// `bench-<task-id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchResult {
    pub task: String,
    pub inputs: Vec<BenchInput>,
}

/// `testdata/results/`.
pub fn results_dir() -> PathBuf {
    repo_root().join("testdata").join("results")
}

/// Write `value` pretty-printed with a trailing newline to `testdata/results/<file_name>`.
pub fn write_result<T: Serialize>(file_name: &str, value: &T) -> Result<PathBuf, String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    let dir = results_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(file_name);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}
