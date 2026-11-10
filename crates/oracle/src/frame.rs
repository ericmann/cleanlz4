//! Safe frame-format wrappers over the oracle, for test tooling.

use crate::ffi;

/// Compression preferences.
#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub block_size_id: u8,
    pub linked: bool,
    pub content_checksum: bool,
    pub block_checksum: bool,
    pub content_size: Option<u64>,
    pub dict_id: u32,
    pub level: i32,
    pub auto_flush: bool,
}

impl Default for Prefs {
    /// Library defaults: default block size, independent blocks, no checksums.
    fn default() -> Self {
        Prefs {
            block_size_id: 0,
            linked: false,
            content_checksum: false,
            block_checksum: false,
            content_size: None,
            dict_id: 0,
            level: 0,
            auto_flush: false,
        }
    }
}

/// Compress `chunks` into one frame: one begin, one update per chunk, one end.
pub fn compress_frame(chunks: &[&[u8]], prefs: &Prefs) -> Vec<u8> {
    let mut ctx = match ffi::CompressCtx::new() {
        Ok(c) => c,
        Err(e) => panic!("oracle context creation failed: {e}"),
    };
    let mut out = Vec::new();
    let mut head = vec![0u8; 64];
    match ctx.begin(&mut head, prefs) {
        Ok(n) => out.extend_from_slice(&head[..n]),
        Err(e) => panic!("oracle frame begin failed: {e}"),
    }
    for chunk in chunks {
        let mut dst = vec![0u8; ffi::frame_compress_bound(chunk.len(), prefs).max(1)];
        match ctx.update(&mut dst, chunk) {
            Ok(n) => out.extend_from_slice(&dst[..n]),
            Err(e) => panic!("oracle frame update failed: {e}"),
        }
    }
    let mut tail = vec![0u8; ffi::frame_compress_bound(0, prefs).max(64) + 16];
    match ctx.end(&mut tail) {
        Ok(n) => out.extend_from_slice(&tail[..n]),
        Err(e) => panic!("oracle frame end failed: {e}"),
    }
    out
}

/// The outcome of decoding a byte string with the oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    Accepted(Vec<u8>),
    Rejected(String),
    OverCap,
}

/// Decode every frame in `input` with the oracle, up to `cap` output bytes.
pub fn decompress_frames(input: &[u8], cap: usize) -> Decoded {
    let mut out: Vec<u8> = Vec::new();
    let mut pos = 0usize;
    let mut buf = vec![0u8; 1 << 16];
    while pos < input.len() {
        let mut ctx = match ffi::DecompressCtx::new() {
            Ok(c) => c,
            Err(e) => return Decoded::Rejected(e),
        };
        loop {
            let step = ctx.step(&mut buf, &input[pos..]);
            if ffi::frame_is_error(step.code) {
                return Decoded::Rejected(ffi::frame_error_name(step.code));
            }
            pos += step.consumed;
            out.extend_from_slice(&buf[..step.produced]);
            if out.len() > cap {
                return Decoded::OverCap;
            }
            if step.code == 0 {
                break;
            }
            if pos >= input.len() && step.produced < buf.len() {
                return Decoded::Rejected(String::from("truncated"));
            }
            if step.consumed == 0 && step.produced == 0 {
                return Decoded::Rejected(String::from("stalled"));
            }
        }
    }
    Decoded::Accepted(out)
}

/// XXH32 (seed 0) of `data`, read from the content checksum the oracle writes
/// at the end of a frame (Frame format, Content Checksum).
pub fn content_checksum_of(data: &[u8]) -> u32 {
    let prefs = Prefs {
        content_checksum: true,
        ..Prefs::default()
    };
    let frame = compress_frame(&[data], &prefs);
    let tail = &frame[frame.len() - 4..];
    u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]])
}
