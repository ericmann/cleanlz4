use oracle::config::Config;
use oracle::diff::{compare_frame_using, Verdict};
use oracle::malformed::{cases, malformed_dir, outcome, read_index, write_all};
use std::collections::BTreeSet;

#[test]
fn every_malformed_file_returns_its_expected_variant() {
    let dir = malformed_dir();
    for e in read_index(&dir).expect("index") {
        let got = outcome(&dir, &e).expect("outcome");
        assert_eq!(got, e.expected, "{}", e.file);
    }
}

#[test]
fn no_malformed_file_panics() {
    let dir = malformed_dir();
    for e in read_index(&dir).expect("index") {
        assert_ne!(outcome(&dir, &e).expect("outcome"), "panic", "{}", e.file);
    }
}

#[test]
fn index_and_directory_agree() {
    let dir = malformed_dir();
    let index = read_index(&dir).expect("index");
    let listed: BTreeSet<String> = index.iter().map(|e| e.file.clone()).collect();
    assert_eq!(listed.len(), index.len(), "duplicate file names");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n != "index.json")
        .collect();
    assert_eq!(listed, on_disk);
    for e in &index {
        assert!(e.kind == "frame" || e.kind == "block");
        assert!(!e.rule.is_empty());
    }
}

#[test]
fn malformed_files_are_reproducible() {
    let cfg = Config::load().expect("config");
    let tmp = std::env::temp_dir().join(format!("cleanlz4-malformed-{}", std::process::id()));
    write_all(&tmp, cfg.diff.max_output_bytes).expect("write");
    let committed = malformed_dir();
    let mut names: Vec<String> = std::fs::read_dir(&tmp)
        .expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(names.len() > 100);
    for n in &names {
        assert_eq!(
            std::fs::read(tmp.join(n)).expect("new"),
            std::fs::read(committed.join(n)).unwrap_or_default(),
            "{n}"
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn every_expected_name_is_an_error_variant() {
    let names: BTreeSet<String> = cleanlz4::Error::ALL
        .iter()
        .map(|e| format!("{e:?}"))
        .collect();
    for c in cases() {
        assert!(names.contains(c.expected), "{}: {}", c.file, c.expected);
    }
}

#[test]
fn each_frame_file_is_agree_or_a_named_normalization_under_compare() {
    let cfg = Config::load().expect("config");
    let dir = malformed_dir();
    let mut normalized = 0;
    for e in read_index(&dir)
        .expect("index")
        .iter()
        .filter(|e| e.kind == "frame")
    {
        let bytes = std::fs::read(dir.join(&e.file)).expect("file");
        let (verdict, norm) = compare_frame_using(&bytes, &cfg, &cleanlz4::decompress_frame_with);
        match verdict {
            Verdict::Agree { .. } => {
                if norm.is_some() {
                    normalized += 1;
                }
            }
            other => panic!("{}: {other:?}", e.file),
        }
    }
    // The zero-offset frame is the one named normalization among these files.
    assert!(normalized >= 1);
}
