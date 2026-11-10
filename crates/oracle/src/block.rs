//! Safe block-format wrappers over the oracle, for test tooling.

use crate::ffi;

/// The oracle rejected the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejected;

fn to_int(n: usize) -> i32 {
    match i32::try_from(n) {
        Ok(v) => v,
        Err(_) => panic!("length {n} does not fit a C int"),
    }
}

/// Worst-case compressed size for `len` input bytes.
pub fn compress_bound(len: usize) -> usize {
    let b = ffi::compress_bound(to_int(len));
    usize::try_from(b).unwrap_or(0)
}

/// Compress with the default (fast) mode.
pub fn compress_default(src: &[u8]) -> Vec<u8> {
    let mut dst = vec![0u8; compress_bound(src.len()).max(1)];
    let n = ffi::compress_default(src, &mut dst);
    if n <= 0 && !src.is_empty() {
        panic!("oracle failed to compress {} bytes", src.len());
    }
    dst.truncate(usize::try_from(n).unwrap_or(0));
    dst
}

/// Compress with the high-compression mode at `level`.
pub fn compress_hc(src: &[u8], level: i32) -> Vec<u8> {
    let mut dst = vec![0u8; compress_bound(src.len()).max(1)];
    let n = ffi::compress_hc(src, &mut dst, level);
    if n <= 0 && !src.is_empty() {
        panic!(
            "oracle failed to compress {} bytes at level {level}",
            src.len()
        );
    }
    dst.truncate(usize::try_from(n).unwrap_or(0));
    dst
}

/// Decompress a block into at most `capacity` bytes.
pub fn decompress_safe(src: &[u8], capacity: usize) -> Result<Vec<u8>, Rejected> {
    let _ = to_int(src.len());
    let mut dst = vec![0u8; capacity];
    let n = ffi::decompress_safe(src, &mut dst);
    match usize::try_from(n) {
        Ok(len) => {
            dst.truncate(len);
            Ok(dst)
        }
        Err(_) => Err(Rejected),
    }
}
