//! The generated conformance corpus (SPEC sections 6.3 and 5.4). Generation is
//! deterministic in `fuzz.seed`; corpus files are read as opaque bytes.

use crate::block::{compress_default, compress_hc};
use crate::config::{repo_root, Config};
use crate::manifest::{sha256_hex, Conformance, Entry, Manifest};
use crate::synth;
use std::path::{Path, PathBuf};

/// The suite name of the block suite.
pub const SUITE_BLOCK: &str = "block";

/// Every suite this generator knows, in the order `--suites all` runs them.
pub const SUITES: [&str; 1] = [SUITE_BLOCK];

/// `testdata/conformance/`.
pub fn conformance_dir() -> PathBuf {
    repo_root().join("testdata").join("conformance")
}

/// The synthetic inputs, named `synthetic/<name>`.
pub fn synthetic_inputs(seed: u64) -> Vec<(String, Vec<u8>)> {
    synth::inputs(seed)
        .into_iter()
        .map(|(n, d)| (format!("synthetic/{n}"), d))
        .collect()
}

/// A corpus input: its name and where its bytes live.
pub fn corpus_file_inputs(manifest: &Manifest) -> Result<Vec<(String, PathBuf)>, String> {
    let base = repo_root().join("testdata").join("corpora");
    let mut out = Vec::new();
    for c in &manifest.corpora {
        let dir = base.join(if c.name == "canterbury" {
            "cantrbry"
        } else {
            c.name.as_str()
        });
        for f in &c.files {
            let path = dir.join(&f.name);
            if !path.is_file() {
                return Err(format!("corpus file missing: {}", path.display()));
            }
            out.push((format!("{}/{}", c.name, f.name), path));
        }
    }
    Ok(out)
}

/// Write `bytes` to `root/rel`, creating directories.
pub fn write_file(root: &Path, rel: &str, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Compress one input for the block suite and write the files under `root`.
pub fn block_entries(
    root: &Path,
    name: &str,
    data: &[u8],
    cfg: &Config,
) -> Result<Vec<Entry>, String> {
    let mut variants: Vec<(String, Vec<u8>)> = vec![("default".into(), compress_default(data))];
    let large = data.len() as u64 > cfg.conformance.large_input_bytes;
    let levels: Vec<i32> = if large {
        cfg.conformance.hc_levels.iter().copied().take(1).collect()
    } else {
        cfg.conformance.hc_levels.clone()
    };
    for level in levels {
        variants.push((format!("hc{level}"), compress_hc(data, level)));
    }
    let input_sha256 = sha256_hex(data);
    let mut entries = Vec::new();
    for (slug, block) in variants {
        let rel = format!("{SUITE_BLOCK}/{name}/{slug}.lz4block");
        write_file(root, &rel, &block)?;
        entries.push(Entry {
            id: format!("{SUITE_BLOCK}/{name}/{slug}"),
            suite: SUITE_BLOCK.into(),
            category: "block".into(),
            kind: "block".into(),
            tool: "lz4".into(),
            input: name.to_string(),
            input_sha256: input_sha256.clone(),
            input_bytes: data.len() as u64,
            params: serde_json::json!({ "mode": slug }),
            file: rel,
            file_sha256: sha256_hex(&block),
            file_bytes: block.len() as u64,
            extra: serde_json::Map::new(),
        });
    }
    Ok(entries)
}

/// The block suite over the synthetic inputs only.
pub fn block_suite_synthetic(root: &Path, cfg: &Config) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for (name, data) in synthetic_inputs(cfg.fuzz.seed) {
        entries.extend(block_entries(root, &name, &data, cfg)?);
    }
    Ok(entries)
}

/// The block suite over the synthetic inputs and every listed corpus file.
pub fn block_suite(root: &Path, cfg: &Config, manifest: &Manifest) -> Result<Vec<Entry>, String> {
    let files = corpus_file_inputs(manifest)?;
    let mut entries = block_suite_synthetic(root, cfg)?;
    for (name, path) in files {
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        entries.extend(block_entries(root, &name, &data, cfg)?);
    }
    Ok(entries)
}

/// Regenerate the named suites and rewrite the manifest's conformance section,
/// keeping other suites' entries.
pub fn generate(suites: &[String]) -> Result<(), String> {
    let cfg = Config::load()?;
    let mut manifest = Manifest::load()?;
    let root = conformance_dir();
    let mut conf = manifest.conformance.take().unwrap_or(Conformance {
        seed: cfg.fuzz.seed,
        corpus_sha256: String::new(),
        entries: Vec::new(),
    });
    conf.seed = cfg.fuzz.seed;
    for suite in suites {
        conf.entries.retain(|e| &e.suite != suite);
        match suite.as_str() {
            SUITE_BLOCK => conf.entries.extend(block_suite(&root, &cfg, &manifest)?),
            other => return Err(format!("unknown suite {other}")),
        }
    }
    conf.finalize();
    manifest.conformance = Some(conf);
    manifest.save()
}
