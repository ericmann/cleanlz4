use oracle::fuzz_logic::{
    diff_accept_in, diff_roundtrip, no_panic_block, no_panic_frame, no_panic_stream,
};
use oracle::fuzzrun::regressions_dir;
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    file: String,
    target: String,
    #[allow(dead_code)]
    resolution: String,
}

fn index() -> Vec<Entry> {
    let p = regressions_dir().join("index.json");
    let text = std::fs::read_to_string(&p).expect("index.json");
    serde_json::from_str(&text).expect("index entries")
}

#[test]
fn every_indexed_regression_input_passes_its_target_logic() {
    for e in index() {
        let bytes = std::fs::read(regressions_dir().join(&e.file)).expect("input");
        match e.target.as_str() {
            "no_panic_block" => no_panic_block(&bytes),
            "no_panic_frame" => no_panic_frame(&bytes),
            "no_panic_stream" => no_panic_stream(&bytes),
            "diff_roundtrip" => diff_roundtrip(&bytes),
            // No divergence directory: a divergence panics.
            "diff_accept" => diff_accept_in(&bytes, None),
            other => panic!("{}: unknown target {other}", e.file),
        }
    }
}

#[test]
fn every_index_entry_names_an_existing_file() {
    for e in index() {
        assert!(regressions_dir().join(&e.file).is_file(), "{}", e.file);
    }
}
