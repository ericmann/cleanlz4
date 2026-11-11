//! Dictionary entries of the conformance corpus: the `lz4` program (the
//! oracle for dictionaries, SPEC section 6.5) against cleanlz4.

use cleanlz4::{DecodeOptions, Dictionary, Error};
use oracle::config::Config;
use oracle::corpus::conformance_dir;
use oracle::manifest::{sha256_hex, Entry, Manifest};
use oracle::runner::entry_dictionary;

fn dictionary_entries() -> Vec<Entry> {
    Manifest::load()
        .expect("manifest")
        .conformance
        .expect("conformance section; run `make corpus`")
        .entries
        .into_iter()
        .filter(|e| e.category == "dictionary")
        .collect()
}

fn read(entry: &Entry) -> Vec<u8> {
    let path = conformance_dir().join(&entry.file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {}: {e}", entry.id, path.display()))
}

#[test]
fn every_dictionary_entry_decodes_identically_with_the_lz4_program_and_cleanlz4() {
    let entries = dictionary_entries();
    assert!(!entries.is_empty(), "no dictionary entries");
    for entry in &entries {
        let id = &entry.id;
        let file = read(entry);
        let (bytes, dict_id) = entry_dictionary(entry).unwrap_or_else(|e| panic!("{id}: {e}"));
        let max = entry.input_bytes;
        let (theirs, ours) = match entry.kind.as_str() {
            "frame" => {
                let theirs = oracle::cli::decompress_frame_with_dict(&file, &bytes)
                    .unwrap_or_else(|_| panic!("{id}: the lz4 program rejected the frame"));
                let options = DecodeOptions {
                    dictionary: Some(Dictionary {
                        bytes: bytes.clone(),
                        id: dict_id,
                    }),
                    max_output_bytes: max,
                    ..DecodeOptions::default()
                };
                let ours = cleanlz4::decompress_frame_with(&file, &options)
                    .unwrap_or_else(|e| panic!("{id}: cleanlz4 rejected the frame: {e}"))
                    .data;
                (theirs, ours)
            }
            "block-dict" => {
                let theirs = oracle::cli::decompress_block_with_dict(&file, &bytes)
                    .unwrap_or_else(|_| panic!("{id}: the lz4 program rejected the block"));
                let ours = cleanlz4::decompress_block_with_dict(&file, &bytes, max as usize)
                    .unwrap_or_else(|e| panic!("{id}: cleanlz4 rejected the block: {e}"));
                (theirs, ours)
            }
            other => panic!("{id}: unexpected kind {other}"),
        };
        for (who, out) in [("lz4 program", &theirs), ("cleanlz4", &ours)] {
            assert_eq!(out.len() as u64, entry.input_bytes, "{id}: {who} length");
            assert_eq!(sha256_hex(out), entry.input_sha256, "{id}: {who} sha256");
        }
        assert_eq!(theirs, ours, "{id}: outputs differ");
    }
}

#[test]
fn dict_id_frames_without_a_dictionary_are_dictionary_required() {
    let mut checked = 0;
    for entry in dictionary_entries()
        .iter()
        .filter(|e| e.kind == "frame" && e.extra.contains_key("dict_id"))
    {
        let file = read(entry);
        let got = cleanlz4::decompress_frame_with(&file, &DecodeOptions::default());
        assert_eq!(
            got.err(),
            Some(Error::DictionaryRequired),
            "{}: expected DictionaryRequired",
            entry.id
        );
        checked += 1;
    }
    assert!(checked > 0, "no frame entry carries a dict_id");
}

#[test]
fn the_dictionary_category_covers_blocks_frames_and_every_configured_size() {
    let entries = dictionary_entries();
    let has = |kind: &str, with_id: Option<bool>| {
        entries
            .iter()
            .any(|e| e.kind == kind && with_id.is_none_or(|w| e.extra.contains_key("dict_id") == w))
    };
    assert!(has("block-dict", None), "no block-dict entry");
    assert!(has("frame", Some(true)), "no frame entry with a dict_id");
    assert!(
        has("frame", Some(false)),
        "no frame entry without a dict_id"
    );
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        assert!(
            entries
                .iter()
                .any(|e| e.extra.get("dict_bytes").and_then(|v| v.as_u64()) == Some(*size)),
            "no entry with dict_bytes {size}"
        );
    }
}
