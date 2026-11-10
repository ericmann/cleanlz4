use oracle::config::Config;
use oracle::corpus::{
    block_suite_synthetic, corpus_frame_category, draw_combinations, frame_combos, frame_level,
    synthetic_frame_suite, synthetic_inputs,
};
use oracle::manifest::{sha256_hex, Manifest};
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cleanlz4-corpus-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

#[test]
fn generation_is_deterministic() {
    let cfg = Config::load().expect("config");
    let (a, b) = (temp_dir("a"), temp_dir("b"));
    let ea = block_suite_synthetic(&a, &cfg).expect("first");
    let eb = block_suite_synthetic(&b, &cfg).expect("second");
    assert_eq!(ea, eb);
    for e in &ea {
        let bytes = std::fs::read(a.join(&e.file)).expect("file");
        assert_eq!(sha256_hex(&bytes), e.file_sha256);
        assert_eq!(bytes.len() as u64, e.file_bytes);
    }
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

fn conformance() -> oracle::manifest::Conformance {
    Manifest::load()
        .expect("manifest")
        .conformance
        .expect("conformance section; run `make corpus`")
}

#[test]
fn manifest_entries_are_sorted_and_unique() {
    let c = conformance();
    let ids: Vec<&String> = c.entries.iter().map(|e| &e.id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len());
}

#[test]
fn block_suite_covers_every_input_and_level() {
    let cfg = Config::load().expect("config");
    let c = conformance();
    assert_eq!(c.seed, cfg.fuzz.seed);
    let manifest = Manifest::load().expect("manifest");
    let mut inputs: Vec<(String, u64)> = synthetic_inputs(cfg.fuzz.seed)
        .into_iter()
        .map(|(n, d)| (n, d.len() as u64))
        .collect();
    for corpus in &manifest.corpora {
        for f in &corpus.files {
            inputs.push((format!("{}/{}", corpus.name, f.name), f.bytes));
        }
    }
    for (name, bytes) in inputs {
        let want_levels = if bytes > cfg.conformance.large_input_bytes {
            1
        } else {
            cfg.conformance.hc_levels.len()
        };
        let got: Vec<_> = c
            .entries
            .iter()
            .filter(|e| e.suite == "block" && e.input == name)
            .collect();
        assert_eq!(got.len(), 1 + want_levels, "{name}");
        assert!(got
            .iter()
            .all(|e| e.input_bytes == bytes && e.category == "block"));
        assert!(got.iter().any(|e| e.id.ends_with("/default")));
    }
}

#[test]
fn corpus_sha256_matches_its_definition() {
    let c = conformance();
    let mut text = String::new();
    for e in &c.entries {
        text.push_str(&format!("{}\n{}\n", e.id, e.file_sha256));
    }
    assert_eq!(sha256_hex(text.as_bytes()), c.corpus_sha256);
}

type ParamKey = (u64, bool, bool, bool, bool);

fn param_key(e: &oracle::manifest::Entry) -> ParamKey {
    let p = &e.params;
    (
        p["block_size_id"].as_u64().expect("id"),
        p["linked"].as_bool().expect("linked"),
        p["block_checksum"].as_bool().expect("bc"),
        p["content_checksum"].as_bool().expect("cc"),
        p["content_size"].as_bool().expect("cs"),
    )
}

#[test]
fn frame_suites_cover_all_64_flag_combinations() {
    let cfg = Config::load().expect("config");
    let c = conformance();
    for suite in ["synthetic", "frames-cantrbry"] {
        let mut inputs: std::collections::BTreeMap<&str, std::collections::BTreeSet<ParamKey>> =
            Default::default();
        for e in c.entries.iter().filter(|e| e.suite == suite) {
            assert!(inputs.entry(&e.input).or_default().insert(param_key(e)));
        }
        assert!(!inputs.is_empty(), "{suite}");
        for (input, keys) in inputs {
            assert_eq!(keys.len(), 64, "{suite} {input}");
        }
    }
    let mut per_file: std::collections::BTreeMap<&str, usize> = Default::default();
    for e in c.entries.iter().filter(|e| e.suite == "frames-silesia") {
        *per_file.entry(&e.input).or_default() += 1;
    }
    assert!(!per_file.is_empty());
    for (input, n) in per_file {
        assert_eq!(n as u64, cfg.conformance.silesia_combos_per_file, "{input}");
    }
    assert_eq!(frame_combos().len(), 64);
    assert_eq!(draw_combinations(1, "x", 8), draw_combinations(1, "x", 8));
    assert_ne!(draw_combinations(1, "x", 8), draw_combinations(1, "y", 8));
}

#[test]
fn every_frame_level_and_block_size_appears() {
    let cfg = Config::load().expect("config");
    let c = conformance();
    for suite in ["synthetic", "frames-cantrbry", "frames-silesia"] {
        let frames: Vec<_> = c.entries.iter().filter(|e| e.suite == suite).collect();
        for level in &cfg.conformance.frame_levels {
            assert!(
                frames
                    .iter()
                    .any(|e| e.params["level"].as_i64() == Some(i64::from(*level))),
                "{suite} level {level}"
            );
        }
        for id in 4u64..=7 {
            assert!(
                frames
                    .iter()
                    .any(|e| e.params["block_size_id"].as_u64() == Some(id)),
                "{suite} block size {id}"
            );
        }
    }
    // The level of combination i is frame_levels[i % len].
    for (i, _) in frame_combos().iter().enumerate() {
        let want = cfg.conformance.frame_levels[i % cfg.conformance.frame_levels.len()];
        assert_eq!(frame_level(&cfg, i), want);
    }
}

#[test]
fn categories_follow_the_precedence_rule() {
    let c = conformance();
    for e in c.entries.iter().filter(|e| e.kind == "frame") {
        if e.suite == "synthetic" {
            assert_eq!(e.category, "synthetic");
            continue;
        }
        let (_, linked, bc, cc, cs) = param_key(e);
        let want = if cs {
            "content-size"
        } else if bc || cc {
            "checksums"
        } else if linked {
            "frame-linked"
        } else {
            "frame-independent"
        };
        assert_eq!(e.category, want, "{}", e.id);
        assert_eq!(e.tool, "lz4f");
    }
    let plain = frame_combos()[0];
    assert_eq!(corpus_frame_category(&plain), "frame-independent");
}

#[test]
fn the_oracle_decodes_every_generated_synthetic_frame() {
    let cfg = Config::load().expect("config");
    let dir = temp_dir("oracle-decodes");
    let entries = synthetic_frame_suite(&dir, &cfg).expect("generate");
    assert_eq!(entries.len(), synthetic_inputs(cfg.fuzz.seed).len() * 64);
    for e in &entries {
        let bytes = std::fs::read(dir.join(&e.file)).expect("file");
        match oracle::frame::decompress_frames(&bytes, e.input_bytes as usize) {
            oracle::frame::Decoded::Accepted(v) => {
                assert_eq!(sha256_hex(&v), e.input_sha256, "{}", e.id)
            }
            other => panic!("{}: {other:?}", e.id),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_frame_level_meets_every_block_size() {
    let cfg = Config::load().expect("config");
    let c = conformance();
    for suite in ["synthetic", "frames-cantrbry"] {
        let frames: Vec<_> = c.entries.iter().filter(|e| e.suite == suite).collect();
        for level in &cfg.conformance.frame_levels {
            let at_level: Vec<_> = frames
                .iter()
                .filter(|e| e.params["level"].as_i64() == Some(i64::from(*level)))
                .collect();
            for id in 4u64..=7 {
                assert!(
                    at_level
                        .iter()
                        .any(|e| e.params["block_size_id"].as_u64() == Some(id)),
                    "{suite} level {level} block size {id}"
                );
            }
            for flag in [
                "linked",
                "block_checksum",
                "content_checksum",
                "content_size",
            ] {
                for value in [false, true] {
                    assert!(
                        at_level
                            .iter()
                            .any(|e| e.params[flag].as_bool() == Some(value)),
                        "{suite} level {level} {flag}={value}"
                    );
                }
            }
        }
    }
}

type FrameKey = (i64, u64, bool, bool, bool, bool);

#[test]
fn frame_combos_are_distinct_and_every_level_meets_every_block_size() {
    let cfg = Config::load().expect("config");
    let combos = frame_combos();
    assert_eq!(combos.len(), 64);
    for (i, a) in combos.iter().enumerate() {
        for b in combos.iter().skip(i + 1) {
            assert_ne!(a, b);
        }
    }
    let mut levels = cfg.conformance.frame_levels.clone();
    levels.sort_unstable();
    levels.dedup();
    assert!(!levels.is_empty());
    for level in levels {
        let mine: Vec<_> = combos
            .iter()
            .enumerate()
            .filter(|(i, _)| frame_level(&cfg, *i) == level)
            .map(|(_, c)| c)
            .collect();
        assert!(!mine.is_empty(), "level {level}");
        for id in 4u8..=7 {
            assert!(
                mine.iter().any(|c| c.block_size_id == id),
                "level {level} block size {id}"
            );
        }
        for v in [false, true] {
            assert!(mine.iter().any(|c| c.linked == v), "level {level} linked");
            assert!(
                mine.iter().any(|c| c.block_checksum == v),
                "level {level} block checksum"
            );
            assert!(
                mine.iter().any(|c| c.content_checksum == v),
                "level {level} content checksum"
            );
            assert!(
                mine.iter().any(|c| c.content_size == v),
                "level {level} content size"
            );
        }
    }
}

#[test]
fn manifest_frame_entries_match_the_generator() {
    let cfg = Config::load().expect("config");
    let c = conformance();
    let want: std::collections::BTreeSet<FrameKey> = frame_combos()
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                i64::from(frame_level(&cfg, i)),
                u64::from(f.block_size_id),
                f.linked,
                f.block_checksum,
                f.content_checksum,
                f.content_size,
            )
        })
        .collect();
    assert_eq!(want.len(), 64);
    for suite in ["synthetic", "frames-cantrbry"] {
        let mut inputs: std::collections::BTreeMap<&str, std::collections::BTreeSet<FrameKey>> =
            Default::default();
        for e in c.entries.iter().filter(|e| e.suite == suite) {
            let (id, linked, bc, cc, cs) = param_key(e);
            let level = e.params["level"].as_i64().expect("level");
            inputs
                .entry(&e.input)
                .or_default()
                .insert((level, id, linked, bc, cc, cs));
        }
        assert!(!inputs.is_empty(), "{suite}");
        for (input, got) in inputs {
            assert_eq!(got, want, "{suite} {input}");
        }
    }
}
