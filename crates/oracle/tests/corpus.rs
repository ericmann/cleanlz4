use oracle::config::Config;
use oracle::corpus::{block_suite_synthetic, synthetic_inputs};
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
