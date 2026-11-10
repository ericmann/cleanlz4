//! The malformed corpus (SPEC sections 6.4 and 6.2): inputs built by hand from
//! the format rules, each breaking exactly one rule and otherwise valid, with the
//! `Error` variant cleanlz4 must return. Generation is deterministic.

use crate::config::{repo_root, Config};
use crate::craft::{build_block, Seq};
use crate::craft_frame::{build, BlockSpec, FrameSpec};
use crate::results::{write_result, MalformedResult};
use cleanlz4::{decompress_block, decompress_frame_with, DecodeOptions};
use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// One malformed input.
#[derive(Debug, Clone)]
pub struct Case {
    pub file: String,
    pub kind: &'static str,
    /// The `Debug` name of the expected `Error` variant.
    pub expected: &'static str,
    /// The document and section the broken rule comes from.
    pub rule: String,
    pub bytes: Vec<u8>,
}

/// One line of `testdata/malformed/index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub file: String,
    pub kind: String,
    pub expected: String,
    pub rule: String,
    pub max_output: u64,
}

/// `testdata/malformed/`.
pub fn malformed_dir() -> PathBuf {
    repo_root().join("testdata").join("malformed")
}

const FRAME_FORMAT: &str = "docs/spec/lz4_Frame_format.md";
const BLOCK_FORMAT: &str = "docs/spec/lz4_Block_format.md";

fn frame_case(name: &str, expected: &'static str, rule: &str, bytes: Vec<u8>) -> Case {
    Case {
        file: format!("{name}.lz4"),
        kind: "frame",
        expected,
        rule: format!("{FRAME_FORMAT}, {rule}"),
        bytes,
    }
}

fn block_case(name: &str, expected: &'static str, rule: &str, bytes: Vec<u8>) -> Case {
    Case {
        file: format!("{name}.block"),
        kind: "block",
        expected,
        rule: format!("{BLOCK_FORMAT}, {rule}"),
        bytes,
    }
}

/// A valid frame of one raw block `abc`.
fn simple() -> FrameSpec {
    FrameSpec {
        blocks: vec![BlockSpec::raw(b"abc")],
        content: b"abc".to_vec(),
        ..FrameSpec::default()
    }
}

/// A frame spec of one compressed block.
fn with_block(block: Vec<u8>) -> FrameSpec {
    FrameSpec {
        blocks: vec![BlockSpec::compressed(&block)],
        ..FrameSpec::default()
    }
}

fn seq(literals: &[u8], offset: u16, match_len: usize) -> Seq {
    Seq {
        literals: literals.to_vec(),
        offset,
        match_len,
    }
}

/// A valid small frame with a raw block and a compressed block.
fn small_valid(checksums_and_size: bool) -> Vec<u8> {
    let compressed = build_block(&[seq(b"abcdefgh", 8, 8)], b"12345", 0);
    let content: Vec<u8> = [b"hello".as_slice(), b"abcdefghabcdefgh12345"].concat();
    let spec = FrameSpec {
        blocks: vec![BlockSpec::raw(b"hello"), BlockSpec::compressed(&compressed)],
        content: content.clone(),
        block_checksum: checksums_and_size,
        content_checksum: checksums_and_size,
        content_size: checksums_and_size.then_some(content.len() as u64),
        ..FrameSpec::default()
    };
    build(&spec)
}

/// Every malformed case, in a fixed order.
pub fn cases() -> Vec<Case> {
    let mut v: Vec<Case> = Vec::new();

    // Header.
    let mut f = simple();
    f.magic = 0x1234_5678;
    v.push(frame_case(
        "bad-magic",
        "BadMagic",
        "Magic Number",
        build(&f),
    ));
    let mut f = simple();
    f.flg_reserved_bit = true;
    v.push(frame_case(
        "flg-reserved-bit",
        "ReservedBitSet",
        "Reserved bits",
        build(&f),
    ));
    for bit in [0x80u8, 0x08, 0x04, 0x02, 0x01] {
        let mut f = simple();
        f.bd_reserved_bits = bit;
        v.push(frame_case(
            &format!("bd-reserved-{bit:02x}"),
            "ReservedBitSet",
            "Reserved bits",
            build(&f),
        ));
    }
    for ver in [0u8, 2, 3] {
        let mut f = simple();
        f.version = ver;
        v.push(frame_case(
            &format!("version-{ver}"),
            "UnsupportedVersion",
            "Version Number",
            build(&f),
        ));
    }
    for code in 0u8..=3 {
        let mut f = simple();
        f.block_max_code = code;
        v.push(frame_case(
            &format!("block-max-size-{code}"),
            "InvalidBlockMaxSize",
            "Block Maximum Size",
            build(&f),
        ));
    }
    let good_hc = build(&simple())[6];
    let mut f = simple();
    f.header_checksum_override = Some(good_hc ^ 0xFF);
    v.push(frame_case(
        "bad-header-checksum",
        "BadHeaderChecksum",
        "Header Checksum",
        build(&f),
    ));

    // Truncation of a small valid frame, and of one with both checksums and a content size.
    for (tag, checks) in [("plain", false), ("checked", true)] {
        let frame = small_valid(checks);
        for off in 1..frame.len() {
            v.push(frame_case(
                &format!("truncated-{tag}-{off:03}"),
                "TruncatedInput",
                "Data Blocks, EndMark, Content Checksum (the frame ends early)",
                frame[..off].to_vec(),
            ));
        }
    }

    // Offsets in frames.
    let zero = build_block(&[seq(b"abcdefgh", 0, 4)], b"12345", 0);
    v.push(frame_case(
        "frame-zero-offset",
        "ZeroOffset",
        "Data Blocks (block offset 0 is invalid, Block format, Compressed block format)",
        build(&with_block(zero)),
    ));
    let before = build_block(&[seq(b"abcdefgh", 9, 4)], b"12345", 0);
    v.push(frame_case(
        "frame-offset-before-output-start",
        "OffsetOutOfRange",
        "Data Blocks (offset reaches before the first byte)",
        build(&with_block(before)),
    ));
    let into_prev = build_block(&[seq(b"", 8, 4)], b"12345", 0);
    let spec = FrameSpec {
        blocks: vec![
            BlockSpec::raw(b"abcdefgh"),
            BlockSpec::compressed(&into_prev),
        ],
        ..FrameSpec::default()
    };
    v.push(frame_case(
        "frame-independent-offset-into-previous-block",
        "OffsetOutOfRange",
        "Block Independence flag (independent blocks do not reference earlier blocks)",
        build(&spec),
    ));
    let beyond = build_block(&[seq(b"", 20, 4)], b"12345", 0);
    let spec = FrameSpec {
        independent: false,
        blocks: vec![BlockSpec::raw(b"abcdefgh"), BlockSpec::compressed(&beyond)],
        ..FrameSpec::default()
    };
    v.push(frame_case(
        "frame-linked-offset-beyond-history",
        "OffsetOutOfRange",
        "Block Independence flag (linked blocks reach back only over earlier output)",
        build(&spec),
    ));

    // End-of-block rules the spike says are enforced (docs/spikes/P1-06.md).
    let r1 = [0x8c, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', 4, 0];
    v.push(block_case(
        "block-eob-rule-1",
        "EndOfBlockViolation",
        "End of block conditions, rule 1",
        r1.to_vec(),
    ));
    v.push(frame_case(
        "frame-eob-rule-1",
        "EndOfBlockViolation",
        "Data Blocks, with Block format, End of block conditions, rule 1",
        build(&with_block(r1.to_vec())),
    ));
    for k in 0..5usize {
        let lits: Vec<u8> = (0..k).map(|i| b'0' + i as u8).collect();
        let b = build_block(&[seq(b"abcdefgh", 8, 16)], &lits, 0);
        v.push(block_case(
            &format!("block-eob-rule-2-{k}"),
            "EndOfBlockViolation",
            "End of block conditions, rule 2",
            b.clone(),
        ));
        if k == 0 {
            v.push(frame_case(
                "frame-eob-rule-2-0",
                "EndOfBlockViolation",
                "Data Blocks, with Block format, End of block conditions, rule 2",
                build(&with_block(b)),
            ));
        }
    }
    v.push(block_case(
        "block-empty-input",
        "TruncatedInput",
        "Compressed block format (a block holds at least one token)",
        Vec::new(),
    ));

    // Checksums and sizes.
    let mut f = simple();
    f.block_checksum = true;
    f.blocks[0].checksum = Some(0xDEAD_BEEF);
    v.push(frame_case(
        "bad-block-checksum",
        "BlockChecksumMismatch",
        "Block checksum",
        build(&f),
    ));
    let mut f = simple();
    f.content_checksum = true;
    f.content_checksum_override = Some(0xDEAD_BEEF);
    v.push(frame_case(
        "bad-content-checksum",
        "ContentChecksumMismatch",
        "Content Checksum",
        build(&f),
    ));
    let mut f = simple();
    f.content_size = Some(2);
    v.push(frame_case(
        "content-size-too-small",
        "ContentSizeMismatch",
        "Content Size",
        build(&f),
    ));
    let mut f = simple();
    f.content_size = Some(4);
    v.push(frame_case(
        "content-size-too-large",
        "ContentSizeMismatch",
        "Content Size",
        build(&f),
    ));
    let max_code4 = 64usize << 10;
    let spec = FrameSpec {
        blocks: vec![BlockSpec::raw(&vec![7u8; max_code4 + 1])],
        ..FrameSpec::default()
    };
    v.push(frame_case(
        "raw-block-size-above-maximum",
        "BlockSizeExceeded",
        "Block Maximum Size, Block Size",
        build(&spec),
    ));
    let extra = max_code4 + 4_000 - 4 - 15;
    let mut big = vec![0x1F, b'a', 1, 0];
    big.extend(std::iter::repeat_n(255u8, extra / 255));
    big.push((extra % 255) as u8);
    big.extend_from_slice(&[0x50, b'1', b'2', b'3', b'4', b'5']);
    v.push(frame_case(
        "compressed-block-decodes-above-maximum",
        "BlockSizeExceeded",
        "Block Maximum Size",
        build(&with_block(big)),
    ));
    let mut f = simple();
    f.dict_id = Some(0x1234_5678);
    v.push(frame_case(
        "dict-id-without-dictionary",
        "DictionaryRequired",
        "Dictionary ID flag, Dictionary ID",
        build(&f),
    ));
    for (n, tag) in [(1usize, "1"), (8, "8")] {
        let mut b = build(&simple());
        b.extend(std::iter::repeat_n(0xABu8, n));
        v.push(frame_case(
            &format!("trailing-garbage-{tag}"),
            "TrailingData",
            "Frame Concatenation (bytes that are not a frame follow the frame)",
            b,
        ));
    }

    // Raw-block versions of the offset and truncation cases.
    v.push(block_case(
        "block-zero-offset",
        "ZeroOffset",
        "Compressed block format (offset 0 is invalid)",
        build_block(&[seq(b"abcdefgh", 0, 4)], b"12345", 0),
    ));
    v.push(block_case(
        "block-offset-out-of-range",
        "OffsetOutOfRange",
        "Compressed block format (offset reaches before the output start)",
        build_block(&[seq(b"abcdefgh", 9, 4)], b"12345", 0),
    ));
    v.push(block_case(
        "block-truncated-in-literals",
        "TruncatedInput",
        "Compressed block format (literals field)",
        vec![0x50, b'a', b'b'],
    ));
    v.push(block_case(
        "block-truncated-in-offset",
        "TruncatedInput",
        "Compressed block format (offset field)",
        vec![0x10, b'a', 0x01],
    ));
    v.push(block_case(
        "block-truncated-literal-length-extension",
        "TruncatedInput",
        "Compressed block format (literal length extension)",
        vec![0xF0],
    ));
    v.push(block_case(
        "block-truncated-match-length-extension",
        "TruncatedInput",
        "Compressed block format (match length extension)",
        vec![0x1F, b'a', 0x01, 0x00],
    ));
    v.push(block_case(
        "block-truncated-final-token",
        "TruncatedInput",
        "Compressed block format (every block ends with a literals sequence)",
        vec![0x10, b'a', 0x01, 0x00, 0x50, b'x'],
    ));
    v
}

/// Write every case and `index.json` into `dir` (created if needed; stale
/// files are removed).
pub fn write_all(dir: &Path, max_output: u64) -> Result<usize, String> {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let all = cases();
    let mut index = Vec::new();
    for c in &all {
        std::fs::write(dir.join(&c.file), &c.bytes).map_err(|e| e.to_string())?;
        index.push(IndexEntry {
            file: c.file.clone(),
            kind: c.kind.into(),
            expected: c.expected.into(),
            rule: c.rule.clone(),
            max_output,
        });
    }
    let mut text = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    text.push('\n');
    std::fs::write(dir.join("index.json"), text).map_err(|e| e.to_string())?;
    Ok(all.len())
}

/// Read `index.json` from `dir`.
pub fn read_index(dir: &Path) -> Result<Vec<IndexEntry>, String> {
    let p = dir.join("index.json");
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// What cleanlz4 did with one indexed file: an error name, `ok`, or `panic`.
pub fn outcome(dir: &Path, entry: &IndexEntry) -> Result<String, String> {
    let p = dir.join(&entry.file);
    let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let run = || match entry.kind.as_str() {
        "block" => decompress_block(&bytes, entry.max_output as usize).map(|_| ()),
        _ => decompress_frame_with(&bytes, &DecodeOptions::default()).map(|_| ()),
    };
    Ok(match catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(())) => String::from("ok"),
        Ok(Err(e)) => format!("{e:?}"),
        Err(_) => String::from("panic"),
    })
}

/// Run every indexed file and write `malformed-<task>.json`.
pub fn run(task: &str) -> Result<MalformedResult, String> {
    let dir = malformed_dir();
    let index = read_index(&dir)?;
    let mut result = MalformedResult {
        task: task.into(),
        total: 0,
        matched_expected_error: 0,
        panics: 0,
    };
    for e in &index {
        result.total += 1;
        match outcome(&dir, e)?.as_str() {
            "panic" => result.panics += 1,
            got if got == e.expected => result.matched_expected_error += 1,
            got => eprintln!("MISMATCH {}: expected {}, got {got}", e.file, e.expected),
        }
    }
    write_result(&format!("malformed-{task}.json"), &result)?;
    Ok(result)
}

/// The oracle-side cap every case is indexed with: `diff.max_output_bytes`.
pub fn index_cap(cfg: &Config) -> u64 {
    cfg.diff.max_output_bytes
}
