use oracle::manifest::Entry;
use oracle::results::{results_dir, BenchResult, ConformanceResult, FuzzResult, MalformedResult};
use oracle::runner::run_entries;

fn files() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(results_dir()).expect("results dir") {
        let p = e.expect("entry").path();
        if p.is_file() && p.extension().is_some_and(|x| x == "json") {
            v.push(p);
        }
    }
    v.sort();
    v
}

#[test]
fn every_result_file_matches_its_schema() {
    for p in files() {
        let name = p.file_name().expect("name").to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&p).expect("read");
        assert!(text.ends_with('\n'), "{name}: trailing newline");
        let ok = if name.starts_with("conformance-") {
            serde_json::from_str::<ConformanceResult>(&text).map(|_| ())
        } else if name.starts_with("malformed-") {
            serde_json::from_str::<MalformedResult>(&text).map(|_| ())
        } else if name.starts_with("fuzz-") {
            serde_json::from_str::<FuzzResult>(&text).map(|_| ())
        } else if name.starts_with("bench-") {
            serde_json::from_str::<BenchResult>(&text).map(|_| ())
        } else {
            panic!("{name}: unknown result file kind");
        };
        ok.unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn conformance_counts_are_consistent() {
    for p in files() {
        let name = p.file_name().expect("name").to_string_lossy().into_owned();
        if !name.starts_with("conformance-") {
            continue;
        }
        let r: ConformanceResult =
            serde_json::from_str(&std::fs::read_to_string(&p).expect("read")).expect("parse");
        assert_eq!(r.failed, r.total - r.passed, "{name}");
        let total: u64 = r.by_category.values().map(|c| c.total).sum();
        let passed: u64 = r.by_category.values().map(|c| c.passed).sum();
        assert_eq!(total, r.total, "{name}");
        assert_eq!(passed, r.passed, "{name}");
        assert!(r.by_category.values().all(|c| c.passed <= c.total));
    }
}

fn entry(file: &str, input: &[u8]) -> Entry {
    Entry {
        id: "block/test/x".into(),
        suite: "block".into(),
        category: "block".into(),
        kind: "block".into(),
        tool: "lz4".into(),
        input: "x".into(),
        input_sha256: oracle::manifest::sha256_hex(input),
        input_bytes: input.len() as u64,
        params: serde_json::json!({}),
        file: file.into(),
        file_sha256: String::new(),
        file_bytes: 0,
        extra: serde_json::Map::new(),
    }
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cleanlz4-runner-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).expect("dir");
    std::fs::write(d.join("f.bin"), b"payload").expect("write");
    d
}

#[test]
fn runner_counts_a_wrong_output_as_failed() {
    let d = scratch("wrong");
    let e = entry("f.bin", b"expected");
    let wrong = |_: &Entry, _: &[u8]| Ok(b"something else".to_vec());
    let r = run_entries(&[&e], &d, &wrong);
    assert_eq!((r.total, r.passed, r.failures.len()), (1, 0, 1));
    let right = |_: &Entry, _: &[u8]| Ok(b"expected".to_vec());
    let r = run_entries(&[&e], &d, &right);
    assert_eq!((r.total, r.passed, r.failures.len()), (1, 1, 0));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn runner_counts_a_panic_as_failed() {
    let d = scratch("panic");
    let e = entry("f.bin", b"expected");
    let boom = |_: &Entry, _: &[u8]| -> Result<Vec<u8>, String> { panic!("boom") };
    let r = run_entries(&[&e], &d, &boom);
    assert_eq!((r.total, r.passed), (1, 0));
    assert_eq!(r.failures[0].1, "panic");
    let _ = std::fs::remove_dir_all(&d);
}
