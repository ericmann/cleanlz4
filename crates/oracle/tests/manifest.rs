use oracle::config::repo_root;
use oracle::manifest::{sha256_hex, Manifest};

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[test]
fn manifest_lists_both_corpora_with_hashes() {
    let m = Manifest::load().expect("manifest");
    assert_eq!(m.corpora.len(), 2);
    let urls: Vec<&str> = m.corpora.iter().map(|c| c.url.as_str()).collect();
    assert!(urls.contains(&"https://corpus.canterbury.ac.nz/resources/cantrbry.tar.gz"));
    assert!(urls.contains(&"https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip"));
    for c in &m.corpora {
        assert!(is_hex64(&c.sha256));
        assert!(c.bytes > 0);
        assert!(!c.files.is_empty());
        for f in &c.files {
            assert!(is_hex64(&f.sha256));
            assert!(f.bytes > 0);
        }
    }
}

#[test]
fn manifest_records_tool_versions() {
    let m = Manifest::load().expect("manifest");
    assert_eq!(m.version, 1);
    assert!(!m.tools.oracle_binding.crate_name.is_empty());
    assert!(!m.tools.oracle_binding.version.is_empty());
    assert!(!m.tools.lz4_cli.is_empty());
    assert!(m.tools.rustc.starts_with("rustc"));
    assert!(m.tools.cargo.starts_with("cargo"));
    assert!(!m.tools.cargo_fuzz.is_empty());
}

#[test]
fn corpus_files_on_disk_match_manifest() {
    let base = repo_root().join("testdata").join("corpora");
    if !base.exists() {
        println!("testdata/corpora/ absent: nothing to check");
        return;
    }
    let m = Manifest::load().expect("manifest");
    for c in &m.corpora {
        let dir = base.join(if c.name == "canterbury" {
            "cantrbry"
        } else {
            "silesia"
        });
        for f in &c.files {
            let data = std::fs::read(dir.join(&f.name)).expect("corpus file");
            assert_eq!(data.len() as u64, f.bytes, "{}", f.name);
            assert_eq!(sha256_hex(&data), f.sha256, "{}", f.name);
        }
    }
}
