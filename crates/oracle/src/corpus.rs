//! The generated conformance corpus (SPEC sections 6.3 and 5.4). Generation is
//! deterministic in `fuzz.seed`; corpus files are read as opaque bytes.

use crate::block::{compress_default, compress_hc};
use crate::config::{repo_root, Config};
use crate::frame::{compress_frame, Prefs};
use crate::manifest::{sha256_hex, Conformance, Entry, Manifest};
use crate::prng::Prng;
use crate::synth;
use std::path::{Path, PathBuf};

/// The suite name of the block suite.
pub const SUITE_BLOCK: &str = "block";

/// The synthetic frame suite.
pub const SUITE_SYNTHETIC: &str = "synthetic";
/// The Canterbury frame suite.
pub const SUITE_FRAMES_CANTRBRY: &str = "frames-cantrbry";
/// The Silesia frame suite.
pub const SUITE_FRAMES_SILESIA: &str = "frames-silesia";

/// Every suite this generator knows, in the order `--suites all` runs them.
pub const SUITES: [&str; 4] = [
    SUITE_BLOCK,
    SUITE_SYNTHETIC,
    SUITE_FRAMES_CANTRBRY,
    SUITE_FRAMES_SILESIA,
];

/// Inputs over this size are fed to the frame compressor in chunks of this size.
const FRAME_CHUNK: usize = 1 << 20;

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

/// One frame flag combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCombo {
    pub block_size_id: u8,
    pub linked: bool,
    pub block_checksum: bool,
    pub content_checksum: bool,
    pub content_size: bool,
}

/// All 64 combinations, in a fixed order: the block size id varies fastest,
/// then content size, content checksum, block checksum and linked.
pub fn frame_combos() -> Vec<FrameCombo> {
    let mut v = Vec::new();
    for linked in [false, true] {
        for block_checksum in [false, true] {
            for content_checksum in [false, true] {
                for content_size in [false, true] {
                    for block_size_id in 4u8..=7 {
                        v.push(FrameCombo {
                            block_size_id,
                            linked,
                            block_checksum,
                            content_checksum,
                            content_size,
                        });
                    }
                }
            }
        }
    }
    v
}

/// The compression level of combination `i`: `conformance.frame_levels[i % len]`.
pub fn frame_level(cfg: &Config, i: usize) -> i32 {
    let levels = &cfg.conformance.frame_levels;
    levels.get(i % levels.len().max(1)).copied().unwrap_or(0)
}

/// The category of a corpus frame entry: content size, then checksums, then
/// linked or independent.
pub fn corpus_frame_category(c: &FrameCombo) -> &'static str {
    if c.content_size {
        "content-size"
    } else if c.block_checksum || c.content_checksum {
        "checksums"
    } else if c.linked {
        "frame-linked"
    } else {
        "frame-independent"
    }
}

fn frame_slug(c: &FrameCombo, level: i32) -> String {
    format!(
        "b{}-{}-bc{}-cc{}-cs{}-l{}",
        c.block_size_id,
        if c.linked { "linked" } else { "indep" },
        u8::from(c.block_checksum),
        u8::from(c.content_checksum),
        u8::from(c.content_size),
        level
    )
}

/// Compress `data` into a frame with `combo` at `level`, in 1 MiB chunks with
/// auto-flush when the input is larger than that.
pub fn make_frame(data: &[u8], combo: &FrameCombo, level: i32) -> Vec<u8> {
    let big = data.len() > FRAME_CHUNK;
    let prefs = Prefs {
        block_size_id: combo.block_size_id,
        linked: combo.linked,
        block_checksum: combo.block_checksum,
        content_checksum: combo.content_checksum,
        content_size: combo.content_size.then_some(data.len() as u64),
        level,
        auto_flush: big,
        ..Prefs::default()
    };
    if big {
        let chunks: Vec<&[u8]> = data.chunks(FRAME_CHUNK).collect();
        compress_frame(&chunks, &prefs)
    } else {
        compress_frame(&[data], &prefs)
    }
}

/// Generate the frame entries of one input for the combinations at `indices`.
pub fn frame_entries(
    root: &Path,
    suite: &str,
    name: &str,
    data: &[u8],
    indices: &[usize],
    cfg: &Config,
) -> Result<Vec<Entry>, String> {
    let combos = frame_combos();
    let input_sha256 = sha256_hex(data);
    let mut entries = Vec::new();
    for &i in indices {
        let combo = combos.get(i).ok_or("combination index out of range")?;
        let level = frame_level(cfg, i);
        let frame = make_frame(data, combo, level);
        let slug = frame_slug(combo, level);
        let rel = format!("{suite}/{name}/{slug}.lz4");
        write_file(root, &rel, &frame)?;
        let category = if suite == SUITE_SYNTHETIC {
            "synthetic"
        } else {
            corpus_frame_category(combo)
        };
        entries.push(Entry {
            id: format!("{suite}/{name}/{slug}"),
            suite: suite.into(),
            category: category.into(),
            kind: "frame".into(),
            tool: "lz4f".into(),
            input: name.to_string(),
            input_sha256: input_sha256.clone(),
            input_bytes: data.len() as u64,
            params: serde_json::json!({
                "block_size_id": combo.block_size_id,
                "linked": combo.linked,
                "block_checksum": combo.block_checksum,
                "content_checksum": combo.content_checksum,
                "content_size": combo.content_size,
                "level": level,
            }),
            file: rel,
            file_sha256: sha256_hex(&frame),
            file_bytes: frame.len() as u64,
            extra: serde_json::Map::new(),
        });
    }
    Ok(entries)
}

/// The synthetic frame suite: every synthetic input with all 64 combinations.
pub fn synthetic_frame_suite(root: &Path, cfg: &Config) -> Result<Vec<Entry>, String> {
    let all: Vec<usize> = (0..frame_combos().len()).collect();
    let mut entries = Vec::new();
    for (name, data) in synthetic_inputs(cfg.fuzz.seed) {
        entries.extend(frame_entries(
            root,
            SUITE_SYNTHETIC,
            &name,
            &data,
            &all,
            cfg,
        )?);
    }
    Ok(entries)
}

/// A stable 64-bit hash of a name (FNV-1a), to seed per-file choices.
fn name_hash(name: &str) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// `count` distinct combination indices drawn with `Prng` seeded by `seed` and
/// the file's name, sorted.
pub fn draw_combinations(seed: u64, name: &str, count: usize) -> Vec<usize> {
    let mut p = Prng::new(seed ^ name_hash(name));
    let mut pool: Vec<usize> = (0..frame_combos().len()).collect();
    let take = count.min(pool.len());
    for k in 0..take {
        let j = k + p.below((pool.len() - k) as u64) as usize;
        pool.swap(k, j);
    }
    let mut chosen: Vec<usize> = pool.into_iter().take(take).collect();
    chosen.sort_unstable();
    chosen
}

/// A corpus frame suite: `corpus` is the manifest's corpus name; Canterbury
/// files get all 64 combinations, others `conformance.silesia_combos_per_file`.
pub fn corpus_frame_suite(
    root: &Path,
    cfg: &Config,
    manifest: &Manifest,
    corpus: &str,
) -> Result<Vec<Entry>, String> {
    let suite = if corpus == "canterbury" {
        SUITE_FRAMES_CANTRBRY
    } else {
        SUITE_FRAMES_SILESIA
    };
    let files = corpus_file_inputs(manifest)?;
    let mut entries = Vec::new();
    for (name, path) in files {
        if !name.starts_with(&format!("{corpus}/")) {
            continue;
        }
        let indices: Vec<usize> = if corpus == "canterbury" {
            (0..frame_combos().len()).collect()
        } else {
            draw_combinations(
                cfg.fuzz.seed,
                &name,
                cfg.conformance.silesia_combos_per_file as usize,
            )
        };
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        entries.extend(frame_entries(root, suite, &name, &data, &indices, cfg)?);
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
            SUITE_SYNTHETIC => conf.entries.extend(synthetic_frame_suite(&root, &cfg)?),
            SUITE_FRAMES_CANTRBRY => {
                conf.entries
                    .extend(corpus_frame_suite(&root, &cfg, &manifest, "canterbury")?)
            }
            SUITE_FRAMES_SILESIA => conf
                .entries
                .extend(corpus_frame_suite(&root, &cfg, &manifest, "silesia")?),
            other => return Err(format!("unknown suite {other}")),
        }
    }
    conf.finalize();
    manifest.conformance = Some(conf);
    manifest.save()
}
