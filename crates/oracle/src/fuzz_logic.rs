//! The logic of the fuzz targets, testable on stable Rust. Each function takes
//! the fuzzer's bytes and panics only on a finding. None of them calls the oracle.

use crate::config::Config;
use cleanlz4::{
    decompress_block, decompress_block_with_dict, decompress_frame_with, DecodeOptions, Decoder,
    Dictionary, Error, Status, TrailingData,
};
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
