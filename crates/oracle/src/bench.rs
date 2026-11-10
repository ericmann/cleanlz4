//! Decode-throughput measurement (SPEC section 8, Phase 4: benchmarks). The
//! figures are reported, never a gate.

use crate::frame::{decompress_frames, Decoded};
use std::time::Instant;

/// Megabytes (10^6 bytes) per second, rounded to one decimal.
pub fn rate_mb_s(bytes: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return 0.0;
    }
    let rate = bytes as f64 / 1_000_000.0 / seconds;
    (rate * 10.0).round() / 10.0
}

/// The best time of `repeats` runs of `f`, in seconds (at least one run).
pub fn best_seconds(repeats: u64, mut f: impl FnMut()) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..repeats.max(1) {
        let start = Instant::now();
        f();
        best = best.min(start.elapsed().as_secs_f64());
    }
    best
}

/// Throughput of both decoders on one frame whose decoded size is `decoded_len`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub cleanlz4_mb_s: f64,
    pub c_mb_s: f64,
}

/// Decode `frame` `repeats` times with each decoder, keep each side's best
/// time, and report MB/s of decoded bytes. Panics if either side fails: a
/// benchmark of a wrong answer means nothing.
pub fn measure(frame: &[u8], decoded_len: usize, repeats: u64) -> Rates {
    let ours = best_seconds(repeats, || match cleanlz4::decompress_frame(frame) {
        Ok(v) => assert_eq!(v.len(), decoded_len),
        Err(e) => panic!("cleanlz4 failed: {e}"),
    });
    let theirs = best_seconds(repeats, || match decompress_frames(frame, decoded_len) {
        Decoded::Accepted(v) => assert_eq!(v.len(), decoded_len),
        other => panic!("oracle failed: {other:?}"),
    });
    Rates {
        cleanlz4_mb_s: rate_mb_s(decoded_len as u64, ours),
        c_mb_s: rate_mb_s(decoded_len as u64, theirs),
    }
}
