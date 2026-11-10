//! The conformance runner: decode every entry and compare with the recorded
//! input hash.

use crate::manifest::{sha256_hex, Entry};
use crate::results::CategoryCounts;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// Decodes one entry's file bytes, or says why it could not.
pub type Decoder<'a> = &'a dyn Fn(&Entry, &[u8]) -> Result<Vec<u8>, String>;

/// The outcome of a run.
#[derive(Debug, Default)]
pub struct Report {
    pub total: u64,
    pub passed: u64,
    pub by_category: BTreeMap<String, CategoryCounts>,
    /// `(entry id, reason)` for each failure.
    pub failures: Vec<(String, String)>,
}

/// The dictionary an entry names: its bytes and its id, when the entry has one.
pub fn entry_dictionary(entry: &Entry) -> Result<(Vec<u8>, Option<u32>), String> {
    let rel = entry
        .extra
        .get("dict_file")
        .and_then(|v| v.as_str())
        .ok_or("entry has no dict_file")?;
    let path = crate::corpus::conformance_dir().join(rel);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let id = entry
        .extra
        .get("dict_id")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    Ok((bytes, id))
}

/// Decode with cleanlz4: blocks, dictionary blocks and frames.
pub fn cleanlz4_decoder(entry: &Entry, bytes: &[u8]) -> Result<Vec<u8>, String> {
    match entry.kind.as_str() {
        "block" => {
            cleanlz4::decompress_block(bytes, entry.input_bytes as usize).map_err(|e| e.to_string())
        }
        "block-dict" => {
            let (dict, _) = entry_dictionary(entry)?;
            cleanlz4::decompress_block_with_dict(bytes, &dict, entry.input_bytes as usize)
                .map_err(|e| e.to_string())
        }
        "frame" => {
            let mut options = cleanlz4::DecodeOptions {
                max_output_bytes: entry.input_bytes,
                ..cleanlz4::DecodeOptions::default()
            };
            if entry.extra.contains_key("dict_file") {
                let (bytes, id) = entry_dictionary(entry)?;
                options.dictionary = Some(cleanlz4::Dictionary { bytes, id });
            }
            cleanlz4::decompress_frame_with(bytes, &options)
                .map(|o| o.data)
                .map_err(|e| e.to_string())
        }
        other => Err(format!("unsupported entry kind {other}")),
    }
}

/// Run `entries` (files under `root`) through `decoder`. A panic in the decoder
/// counts as a failure.
pub fn run_entries(entries: &[&Entry], root: &Path, decoder: Decoder<'_>) -> Report {
    let mut report = Report::default();
    for entry in entries {
        let outcome = run_one(entry, root, decoder);
        let counts = report
            .by_category
            .entry(entry.category.clone())
            .or_insert(CategoryCounts {
                total: 0,
                passed: 0,
            });
        counts.total += 1;
        report.total += 1;
        match outcome {
            Ok(()) => {
                counts.passed += 1;
                report.passed += 1;
            }
            Err(reason) => report.failures.push((entry.id.clone(), reason)),
        }
    }
    report
}

fn run_one(entry: &Entry, root: &Path, decoder: Decoder<'_>) -> Result<(), String> {
    let path = root.join(&entry.file);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoded = match catch_unwind(AssertUnwindSafe(|| decoder(entry, &bytes))) {
        Ok(r) => r?,
        Err(_) => return Err(String::from("panic")),
    };
    if sha256_hex(&decoded) == entry.input_sha256 {
        Ok(())
    } else {
        Err(String::from(
            "output hash differs from the recorded input hash",
        ))
    }
}
