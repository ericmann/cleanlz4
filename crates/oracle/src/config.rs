//! Tunables, read from `testdata/config.json`. No default is written here.

use serde::Deserialize;
use std::path::PathBuf;

/// `limits.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_output_bytes: u64,
}

/// `fuzz.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuzzConfig {
    pub seconds_per_target: u64,
    pub seed: u64,
    pub smoke_seconds: u64,
    pub max_output_bytes: u64,
    pub max_divergence_files: u64,
}

/// `bench.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchConfig {
    pub repeats: u64,
}

/// `stream.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamConfig {
    pub chunking_cases: u64,
}

/// `dict.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictConfig {
    pub sizes_bytes: Vec<u64>,
}

/// `conformance.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConformanceConfig {
    pub hc_levels: Vec<i32>,
    pub frame_levels: Vec<i32>,
    pub silesia_combos_per_file: u64,
    pub large_input_bytes: u64,
    pub random_block_cases: u64,
}

/// `diff.*`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffConfig {
    pub max_output_bytes: u64,
}

/// The whole configuration file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub limits: Limits,
    pub fuzz: FuzzConfig,
    pub bench: BenchConfig,
    pub stream: StreamConfig,
    pub dict: DictConfig,
    pub conformance: ConformanceConfig,
    pub diff: DiffConfig,
}

/// The repository root.
pub fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

impl Config {
    /// Load `testdata/config.json`.
    pub fn load() -> Result<Config, String> {
        let path = repo_root().join("testdata").join("config.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}
