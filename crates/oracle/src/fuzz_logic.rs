//! The logic of the fuzz targets, testable on stable Rust. Each function takes
//! the fuzzer's bytes and panics only on a finding. The `no_panic_*` functions
//! never call the oracle; `diff_roundtrip` and `diff_accept` compare cleanlz4
//! with it.

use crate::block::{compress_default, compress_hc, decompress_safe};
use crate::config::Config;
use crate::diff::{compare_block, compare_frame, Verdict, BLOCK_SLACK};
use crate::frame::{compress_frame, decompress_frames, Decoded, Prefs};
use crate::manifest::sha256_hex;
use crate::prng::Prng;
use cleanlz4::{
    decompress_block, decompress_block_with_dict, decompress_frame, decompress_frame_with,
    DecodeOptions, Decoder, Dictionary, Error, Status, TrailingData,
};
use std::path::Path;
use std::sync::OnceLock;

fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| match Config::load() {
        Ok(c) => c,
        Err(e) => panic!("cannot load testdata/config.json: {e}"),
    })
}

/// Block decoding must not panic. The first two bytes give `max_output` (their
/// little-endian value times 16); the third picks where the rest splits into a
/// dictionary and a block.
pub fn no_panic_block(data: &[u8]) {
    let (Some(&lo), Some(&hi), Some(&sel)) = (data.first(), data.get(1), data.get(2)) else {
        return;
    };
    let max_output = (usize::from(lo) | usize::from(hi) << 8) * 16;
    let rest = data.get(3..).unwrap_or(&[]);
    let split = rest.len() * usize::from(sel) / 256;
    let (dict, block) = rest.split_at(split);
    let _ = decompress_block(block, max_output);
    let _ = decompress_block_with_dict(block, dict, max_output);
}

/// The options a frame-level target derives from its first byte: bit 0 selects
/// `ReturnRemainder`, bit 1 turns checksum verification off, bit 2 supplies a
/// dictionary (the next 64 bytes) and bit 3 gives that dictionary an id (the
/// little-endian value of its first four bytes). Returns the options and the
/// bytes left for decoding.
fn frame_options(data: &[u8]) -> Option<(DecodeOptions, &[u8])> {
    let (&flags, rest) = data.split_first()?;
    let mut options = DecodeOptions {
        max_output_bytes: config().fuzz.max_output_bytes,
        trailing_data: if flags & 1 != 0 {
            TrailingData::ReturnRemainder
        } else {
            TrailingData::Error
        },
        verify_checksums: flags & 2 == 0,
        ..DecodeOptions::default()
    };
    let mut rest = rest;
    if flags & 4 != 0 {
        let take = rest.len().min(64);
        let (dict, after) = rest.split_at(take);
        let id = (flags & 8 != 0).then(|| {
            let mut b = [0u8; 4];
            for (d, s) in b.iter_mut().zip(dict.iter()) {
                *d = *s;
            }
            u32::from_le_bytes(b)
        });
        options.dictionary = Some(Dictionary {
            bytes: dict.to_vec(),
            id,
        });
        rest = after;
    }
    Some((options, rest))
}

/// One-shot frame decoding must not panic.
pub fn no_panic_frame(data: &[u8]) {
    let Some((options, rest)) = frame_options(data) else {
        return;
    };
    let _ = decompress_frame_with(rest, &options);
}

/// A chunk size from one byte: the regimes of the chunking test, the last one
/// scaled up to about 70,000.
fn regime(b: u8) -> usize {
    match b % 7 {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 7,
        4 => 64,
        5 => 4096,
        _ => 1 + usize::from(b) * 273,
    }
}

/// Stream `input` in `in_chunk`-byte pieces into `out_chunk`-byte buffers.
fn stream(
    options: &DecodeOptions,
    input: &[u8],
    in_chunk: usize,
    out_chunk: usize,
) -> Result<Vec<u8>, Error> {
    let mut d = Decoder::new(options.clone());
    let mut buf = vec![0u8; out_chunk];
    let mut out = Vec::new();
    let mut pos = 0usize;
    let bound = 100 * (input.len() + 100) + out_chunk;
    for _ in 0..bound {
        let end = pos.saturating_add(in_chunk).min(input.len());
        let chunk = input.get(pos..end).unwrap_or(&[]);
        let p = d.decode(chunk, &mut buf, end == input.len())?;
        pos += p.consumed;
        out.extend_from_slice(buf.get(..p.produced).unwrap_or(&[]));
        if p.status == Status::Done {
            return Ok(out);
        }
    }
    panic!("the streaming decoder made no progress");
}

/// Streaming must not panic and must agree with the one-shot decoder: the same
/// bytes, or the same `Error` variant. The first two bytes pick the chunk
/// regimes; the rest is decoded with default options under the fuzz output cap.
pub fn no_panic_stream(data: &[u8]) {
    let (Some(&a), Some(&b)) = (data.first(), data.get(1)) else {
        return;
    };
    let input = data.get(2..).unwrap_or(&[]);
    let options = DecodeOptions {
        max_output_bytes: config().fuzz.max_output_bytes,
        ..DecodeOptions::default()
    };
    let one_shot = decompress_frame_with(input, &options).map(|o| o.data);
    // Keep the number of calls bounded when the output is large.
    let out_len = one_shot.as_ref().map_or(0, Vec::len);
    let out_chunk = regime(b).max(out_len / (1 << 20) + 1);
    let streamed = stream(&options, input, regime(a), out_chunk);
    assert_eq!(streamed, one_shot, "streamed and one-shot results differ");
}

/// A stable 64-bit hash of the fuzzer's bytes (FNV-1a).
fn hash_bytes(data: &[u8]) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The longest input of the round-trip target: 300 KiB.
const ROUNDTRIP_MAX_BYTES: usize = 300 << 10;

/// Round trip: the first four bytes choose the preferences (byte 0: block size
/// id in bits 0 and 1, linked, block checksum, content checksum, content size
/// in bits 2 to 5; byte 1: low nibble the frame level index, high nibble the
/// number of chunks minus one, modulo four), bytes 2 and 3 the target length
/// (up to 300 KiB). The rest is repeated up to that length with random
/// perturbations. The oracle compresses; cleanlz4 and the oracle must both
/// decode the input back. Every eighth input also takes the block form.
pub fn diff_roundtrip(data: &[u8]) {
    let (Some(&b0), Some(&b1), Some(&b2), Some(&b3)) =
        (data.first(), data.get(1), data.get(2), data.get(3))
    else {
        return;
    };
    let cfg = config();
    let seed = data.get(4..).unwrap_or(&[]);
    let levels = &cfg.conformance.frame_levels;
    let level = levels
        .get(usize::from(b1 & 0x0F) % levels.len().max(1))
        .copied()
        .unwrap_or(0);
    let prefs_for = |len: usize| Prefs {
        block_size_id: 4 + (b0 & 3),
        linked: b0 & 4 != 0,
        block_checksum: b0 & 8 != 0,
        content_checksum: b0 & 16 != 0,
        content_size: (b0 & 32 != 0).then_some(len as u64),
        level,
        ..Prefs::default()
    };
    let target = usize::from(u16::from_le_bytes([b2, b3])) * ROUNDTRIP_MAX_BYTES / 65_535;
    let mut rng = Prng::new(cfg.fuzz.seed ^ hash_bytes(data));
    let pattern: &[u8] = if seed.is_empty() { &[0] } else { seed };
    let mut input = Vec::with_capacity(target);
    'fill: loop {
        for &b in pattern {
            if input.len() >= target {
                break 'fill;
            }
            input.push(if rng.below(64) == 0 {
                rng.next_u64() as u8
            } else {
                b
            });
        }
    }
    let chunk_count = 1 + usize::from((b1 >> 4) % 4);
    let size = input.len().div_ceil(chunk_count).max(1);
    let chunks: Vec<&[u8]> = if input.is_empty() {
        vec![&input[..]]
    } else {
        input.chunks(size).collect()
    };
    let mut prefs = prefs_for(input.len());
    prefs.auto_flush = chunks.len() > 1;
    let frame = compress_frame(&chunks, &prefs);
    match decompress_frame(&frame) {
        Ok(out) => assert_eq!(out, input, "cleanlz4 round trip differs"),
        Err(e) => panic!("cleanlz4 rejected the oracle's frame: {e}"),
    }
    match decompress_frames(&frame, input.len() + BLOCK_SLACK) {
        Decoded::Accepted(out) => assert_eq!(out, input, "oracle round trip differs"),
        other => panic!("the oracle does not decode its own frame: {other:?}"),
    }
    if hash_bytes(data).is_multiple_of(8) {
        let hc = &cfg.conformance.hc_levels;
        let pick = (hash_bytes(data) >> 8) as usize % (hc.len() + 1);
        let block = match hc.get(pick) {
            Some(&l) => compress_hc(&input, l),
            None => compress_default(&input),
        };
        match decompress_block(&block, input.len()) {
            Ok(out) => assert_eq!(out, input, "cleanlz4 block round trip differs"),
            Err(e) => panic!("cleanlz4 rejected the oracle's block: {e}"),
        }
        match decompress_safe(&block, input.len() + BLOCK_SLACK) {
            Ok(out) => assert_eq!(out, input, "oracle block round trip differs"),
            Err(_) => panic!("the oracle does not decode its own block"),
        }
    }
}

/// Write `input` to `<dir>/<first 16 hex of its SHA-256>.bin` unless `limit`
/// files are already there. Returns whether a file was written.
pub fn record_divergence(dir: &Path, input: &[u8], limit: u64) -> Result<bool, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let existing = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "bin"))
        .count() as u64;
    if existing >= limit {
        return Ok(false);
    }
    let name = sha256_hex(input);
    let path = dir.join(format!("{}.bin", name.get(..16).unwrap_or(&name)));
    std::fs::write(&path, input).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// Handle one verdict: a divergence is recorded under `dir` when given, else it panics.
fn judge_verdict(what: &str, verdict: Verdict, input: &[u8], dir: Option<&Path>) {
    if let Verdict::Diverge(text) = verdict {
        match dir {
            Some(d) => {
                if let Err(e) = record_divergence(d, input, config().fuzz.max_divergence_files) {
                    panic!("cannot record a divergence: {e}");
                }
            }
            None => panic!("{what} divergence: {text}"),
        }
    }
}

/// Accept/reject differential on arbitrary bytes, as a frame flow and as a
/// block. With `dir` set (the target reads `CLEANLZ4_DIVERGENCE_DIR`) a
/// divergence is written there; without it the function panics.
pub fn diff_accept_in(data: &[u8], dir: Option<&Path>) {
    let cfg = config();
    judge_verdict("frame", compare_frame(data, cfg), data, dir);
    // A block decodes to at most about 255 times its size.
    let cap = (data.len() + 16)
        .saturating_mul(256)
        .min(cfg.fuzz.max_output_bytes as usize);
    judge_verdict("block", compare_block(data, cap), data, dir);
}

/// The fuzz target's entry: reads `CLEANLZ4_DIVERGENCE_DIR`.
pub fn diff_accept(data: &[u8]) {
    let dir = std::env::var_os("CLEANLZ4_DIVERGENCE_DIR").map(std::path::PathBuf::from);
    diff_accept_in(data, dir.as_deref());
}
