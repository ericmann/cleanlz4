//! The generated conformance corpus (SPEC sections 6.3 and 5.4). Generation is
//! deterministic in `fuzz.seed`; corpus files are read as opaque bytes.

use crate::block::{compress_default, compress_hc};
use crate::config::{repo_root, Config};
use crate::craft_frame::{build, legacy, skippable, BlockSpec, FrameSpec};
use crate::dict::{dict_block, dict_frame};
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

/// The dictionary suite (category `dictionary`).
pub const SUITE_DICTIONARY: &str = "dictionary";
/// The skippable-frame suite (category `skippable`).
pub const SUITE_SKIPPABLE: &str = "skippable";
/// The legacy-frame suite (category `legacy`).
pub const SUITE_LEGACY: &str = "legacy";
/// The concatenated-frame suite (category `concatenated`).
pub const SUITE_CONCATENATED: &str = "concatenated";

/// Every suite this generator knows, in the order `--suites all` runs them.
pub const SUITES: [&str; 8] = [
    SUITE_BLOCK,
    SUITE_SYNTHETIC,
    SUITE_FRAMES_CANTRBRY,
    SUITE_FRAMES_SILESIA,
    SUITE_DICTIONARY,
    SUITE_SKIPPABLE,
    SUITE_LEGACY,
    SUITE_CONCATENATED,
];

/// The data after a dictionary is at most this many bytes (SPEC section 6.3, P2-11).
const DICT_DATA_BYTES: usize = 256 << 10;
/// Dictionary frames are assembled from chunks of this size, so they hold several blocks.
const DICT_CHUNK_BYTES: usize = 64 << 10;
/// The legacy suite's synthetic input.
const LEGACY_SYNTHETIC_BYTES: usize = 20 << 20;

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

/// All 64 combinations, in a fixed order. Combination `i` has `hi = i / 4` and
/// `lo = i % 4`: `hi` (0 to 15) gives the four flags, linked, block checksum,
/// content checksum and content size, from the highest bit to the lowest, and
/// the block size id is `4 + (hi + lo) % 4`. The level is chosen by `i % len`
/// (see `frame_level`), so with four levels it follows `lo`: each level sees
/// all 16 flag settings, and the block size skew means each level also sees
/// every block size.
pub fn frame_combos() -> Vec<FrameCombo> {
    (0..64usize)
        .map(|i| {
            let (hi, lo) = (i / 4, i % 4);
            FrameCombo {
                block_size_id: 4 + ((hi + lo) % 4) as u8,
                linked: hi & 8 != 0,
                block_checksum: hi & 4 != 0,
                content_checksum: hi & 2 != 0,
                content_size: hi & 1 != 0,
            }
        })
        .collect()
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

/// A hand-assembled entry: `expected` is the decoded output, `file` the stored bytes.
#[allow(clippy::too_many_arguments)]
fn hand_entry(
    root: &Path,
    suite: &str,
    tail: &str,
    kind: &str,
    tool: &str,
    input: &str,
    expected: &[u8],
    bytes: &[u8],
    params: serde_json::Value,
    extra: serde_json::Map<String, serde_json::Value>,
) -> Result<Entry, String> {
    let rel = format!("{suite}/{tail}.lz4");
    write_file(root, &rel, bytes)?;
    Ok(Entry {
        id: format!("{suite}/{tail}"),
        suite: suite.into(),
        category: suite.into(),
        kind: kind.into(),
        tool: tool.into(),
        input: input.into(),
        input_sha256: sha256_hex(expected),
        input_bytes: expected.len() as u64,
        params,
        file: rel,
        file_sha256: sha256_hex(bytes),
        file_bytes: bytes.len() as u64,
        extra,
    })
}

fn extra_of(pairs: Vec<(&str, serde_json::Value)>) -> serde_json::Map<String, serde_json::Value> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// The dictionary suite: for each configured size and each Canterbury file of
/// at least twice that size, the dictionary is the file's first `size` bytes
/// and the data its next 256 KiB (or what remains).
pub fn dictionary_suite(
    root: &Path,
    cfg: &Config,
    manifest: &Manifest,
) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for (name, path) in corpus_file_inputs(manifest)? {
        if !name.starts_with("canterbury/") {
            continue;
        }
        let file = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for &size in &cfg.dict.sizes_bytes {
            let size = size as usize;
            if file.len() < size * 2 {
                continue;
            }
            let dict = &file[..size];
            let data = &file[size..file.len().min(size + DICT_DATA_BYTES)];
            let dict_rel = format!("{SUITE_DICTIONARY}/{name}/dict-{size}.bin");
            write_file(root, &dict_rel, dict)?;
            let dict_sha = sha256_hex(dict);
            let chunks: Vec<&[u8]> = data.chunks(DICT_CHUNK_BYTES).collect();
            let id = name_hash(&format!("{name}/{size}")) as u32;
            let base = |id_used: Option<u32>, dependent: bool| {
                let mut pairs = vec![
                    ("dict_file", serde_json::json!(dict_rel)),
                    ("dict_sha256", serde_json::json!(dict_sha)),
                    ("dict_bytes", serde_json::json!(size)),
                    ("dict_source", serde_json::json!(name)),
                    ("dict_offset", serde_json::json!(0)),
                    ("dict_dependent", serde_json::json!(dependent)),
                ];
                if let Some(i) = id_used {
                    pairs.push(("dict_id", serde_json::json!(i)));
                }
                extra_of(pairs)
            };
            let stem = format!("{name}/d{size}");
            let block = dict_block(dict, data);
            let dependent = block.as_ref().is_some_and(|b| b.dict_dependent);
            if let Some(b) = block {
                entries.push(hand_entry(
                    root,
                    SUITE_DICTIONARY,
                    &format!("{stem}/block"),
                    "block-dict",
                    "hand",
                    &name,
                    data,
                    &b.block,
                    serde_json::json!({ "mode": "block" }),
                    base(None, dependent),
                )?);
            }
            for (slug, independent, with_id) in [
                ("linked-id", false, true),
                ("indep-id", true, true),
                ("linked", false, false),
            ] {
                let used = with_id.then_some(id);
                let frame = dict_frame(dict, &chunks, independent, used);
                entries.push(hand_entry(
                    root,
                    SUITE_DICTIONARY,
                    &format!("{stem}/{slug}"),
                    "frame",
                    "hand",
                    &name,
                    data,
                    &frame,
                    serde_json::json!({
                        "linked": !independent,
                        "dict_id": used,
                    }),
                    // Whether the blocks need the dictionary is that of the raw block.
                    base(used, dependent),
                )?);
            }
        }
    }
    Ok(entries)
}

/// The regular frame of `data` for flag combination `i` at its level.
fn regular_frame(cfg: &Config, data: &[u8], i: usize) -> Vec<u8> {
    let combos = frame_combos();
    let combo = combos[i % combos.len()];
    make_frame(data, &combo, frame_level(cfg, i))
}

/// Small deterministic contents for the hand-assembled suites.
fn small_contents(seed: u64) -> Vec<Vec<u8>> {
    [3_000usize, 70_000, 200_000, 11]
        .iter()
        .enumerate()
        .map(|(k, &len)| synth::word_salad(seed.wrapping_add(k as u64), len))
        .collect()
}

/// The skippable suite, by hand with `craft_frame`.
pub fn skippable_suite(root: &Path, cfg: &Config) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let none = serde_json::Map::new;
    for nibble in 0u8..16 {
        let frame = skippable(nibble, b"user data");
        entries.push(hand_entry(
            root,
            SUITE_SKIPPABLE,
            &format!("alone/magic-{nibble:x}"),
            "frame",
            "hand",
            "empty",
            &[],
            &frame,
            serde_json::json!({ "magic_low_nibble": nibble }),
            none(),
        )?);
    }
    let frame = skippable(0, &[]);
    entries.push(hand_entry(
        root,
        SUITE_SKIPPABLE,
        "alone/zero-length",
        "frame",
        "hand",
        "empty",
        &[],
        &frame,
        serde_json::json!({ "magic_low_nibble": 0 }),
        none(),
    )?);
    let contents = small_contents(cfg.fuzz.seed);
    let (a, b) = (&contents[0], &contents[1]);
    let (fa, fb) = (regular_frame(cfg, a, 0), regular_frame(cfg, b, 37));
    let skip = skippable(7, b"between");
    let expected_two = [a.as_slice(), b.as_slice()].concat();
    let cases: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
        ("before", [skip.clone(), fa.clone()].concat(), a.clone()),
        ("after", [fa.clone(), skip.clone()].concat(), a.clone()),
        (
            "between",
            [fa.clone(), skip.clone(), fb.clone()].concat(),
            expected_two.clone(),
        ),
        (
            "everywhere",
            [
                skippable(1, b"x"),
                fa.clone(),
                skippable(15, &[]),
                fb.clone(),
                skip.clone(),
            ]
            .concat(),
            expected_two,
        ),
    ];
    for (slug, bytes, expected) in cases {
        entries.push(hand_entry(
            root,
            SUITE_SKIPPABLE,
            &format!("with-regular/{slug}"),
            "frame",
            "hand",
            "synthetic/word-salad",
            &expected,
            &bytes,
            serde_json::json!({ "position": slug }),
            none(),
        )?);
    }
    Ok(entries)
}

/// The legacy suite: the CLI over every Canterbury file and 20 MiB of word
/// salad, plus two hand-built frames.
pub fn legacy_suite(root: &Path, cfg: &Config, manifest: &Manifest) -> Result<Vec<Entry>, String> {
    let version = crate::cli::cli_version();
    let mut entries = Vec::new();
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, path) in corpus_file_inputs(manifest)? {
        if name.starts_with("canterbury/") {
            let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            inputs.push((name, data));
        }
    }
    inputs.push((
        "synthetic/word-salad-20m".into(),
        synth::word_salad(cfg.fuzz.seed, LEGACY_SYNTHETIC_BYTES),
    ));
    for (name, data) in &inputs {
        let frame = crate::cli::compress_legacy(data);
        entries.push(hand_entry(
            root,
            SUITE_LEGACY,
            &format!("cli/{name}"),
            "frame",
            "cli",
            name,
            data,
            &frame,
            serde_json::json!({ "mode": "cli" }),
            extra_of(vec![("cli_version", serde_json::json!(version))]),
        )?);
    }
    let contents = small_contents(cfg.fuzz.seed);
    let blocks: Vec<Vec<u8>> = contents[..2].iter().map(|c| compress_default(c)).collect();
    let expected = [contents[0].as_slice(), contents[1].as_slice()].concat();
    for (slug, terminated) in [("two-blocks", false), ("zero-terminated", true)] {
        let frame = legacy(&blocks, terminated);
        entries.push(hand_entry(
            root,
            SUITE_LEGACY,
            &format!("hand/{slug}"),
            "frame",
            "hand",
            "synthetic/word-salad",
            &expected,
            &frame,
            serde_json::json!({ "mode": slug }),
            serde_json::Map::new(),
        )?);
    }
    Ok(entries)
}

/// The concatenated suite: several frames in one input.
pub fn concatenated_suite(root: &Path, cfg: &Config) -> Result<Vec<Entry>, String> {
    let contents = small_contents(cfg.fuzz.seed);
    // Different preferences for each frame: distinct combination indices.
    let picks = [5usize, 22, 41, 60];
    let mut entries = Vec::new();
    for count in 2..=4usize {
        let mut bytes = Vec::new();
        let mut expected = Vec::new();
        for k in 0..count {
            bytes.extend(regular_frame(cfg, &contents[k], picks[k]));
            expected.extend_from_slice(&contents[k]);
        }
        entries.push(hand_entry(
            root,
            SUITE_CONCATENATED,
            &format!("regular-x{count}"),
            "frame",
            "hand",
            "synthetic/word-salad",
            &expected,
            &bytes,
            serde_json::json!({ "frames": count, "combinations": &picks[..count] }),
            serde_json::Map::new(),
        )?);
    }
    // A legacy frame, then a regular frame.
    let legacy_frame = legacy(&[compress_default(&contents[0])], false);
    let bytes = [legacy_frame, regular_frame(cfg, &contents[1], picks[0])].concat();
    let expected = [contents[0].as_slice(), contents[1].as_slice()].concat();
    entries.push(hand_entry(
        root,
        SUITE_CONCATENATED,
        "legacy-then-regular",
        "frame",
        "hand",
        "synthetic/word-salad",
        &expected,
        &bytes,
        serde_json::json!({ "contains_legacy": true }),
        serde_json::Map::new(),
    )?);
    // A regular frame, a skippable frame and a regular frame.
    let bytes = [
        regular_frame(cfg, &contents[0], picks[1]),
        skippable(3, b"between"),
        regular_frame(cfg, &contents[2], picks[2]),
    ]
    .concat();
    let expected = [contents[0].as_slice(), contents[2].as_slice()].concat();
    entries.push(hand_entry(
        root,
        SUITE_CONCATENATED,
        "regular-skippable-regular",
        "frame",
        "hand",
        "synthetic/word-salad",
        &expected,
        &bytes,
        serde_json::json!({ "contains_legacy": false }),
        serde_json::Map::new(),
    )?);
    // A hand-built regular frame with raw blocks, to keep one frame the
    // compressor did not write.
    let raw = build(&FrameSpec {
        blocks: vec![BlockSpec::raw(&contents[3])],
        content: contents[3].clone(),
        content_checksum: true,
        ..FrameSpec::default()
    });
    let bytes = [raw, regular_frame(cfg, &contents[3], picks[3])].concat();
    let expected = [contents[3].as_slice(), contents[3].as_slice()].concat();
    entries.push(hand_entry(
        root,
        SUITE_CONCATENATED,
        "raw-then-regular",
        "frame",
        "hand",
        "synthetic/word-salad",
        &expected,
        &bytes,
        serde_json::json!({ "contains_legacy": false }),
        serde_json::Map::new(),
    )?);
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
            SUITE_DICTIONARY => conf
                .entries
                .extend(dictionary_suite(&root, &cfg, &manifest)?),
            SUITE_SKIPPABLE => conf.entries.extend(skippable_suite(&root, &cfg)?),
            SUITE_LEGACY => conf.entries.extend(legacy_suite(&root, &cfg, &manifest)?),
            SUITE_CONCATENATED => conf.entries.extend(concatenated_suite(&root, &cfg)?),
            other => return Err(format!("unknown suite {other}")),
        }
    }
    conf.finalize();
    manifest.conformance = Some(conf);
    manifest.save()
}
