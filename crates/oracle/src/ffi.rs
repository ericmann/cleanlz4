//! The only module that may reach the C library.

use std::os::raw::{c_char, c_int};

/// Name of the binding crate; no other file may name it.
pub const BINDING_CRATE: &str = "lz4-sys";

/// Worst-case compressed size for `len` input bytes, or 0 when out of range.
pub fn compress_bound(len: c_int) -> c_int {
    // SAFETY: a pure function of its integer argument.
    unsafe { lz4_sys::LZ4_compressBound(len) }
}

/// Fast compression into `dst`; returns the compressed size or 0 on failure.
pub fn compress_default(src: &[u8], dst: &mut [u8]) -> c_int {
    // SAFETY: both pointers come from live slices whose lengths are passed alongside.
    unsafe {
        lz4_sys::LZ4_compress_default(
            src.as_ptr() as *const c_char,
            dst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            dst.len() as c_int,
        )
    }
}

/// High-compression mode into `dst`; returns the compressed size or 0 on failure.
pub fn compress_hc(src: &[u8], dst: &mut [u8], level: c_int) -> c_int {
    // SAFETY: both pointers come from live slices whose lengths are passed alongside.
    unsafe {
        lz4_sys::LZ4_compress_HC(
            src.as_ptr() as *const c_char,
            dst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            dst.len() as c_int,
            level,
        )
    }
}

/// Safe block decompression into `dst`; negative on error.
pub fn decompress_safe(src: &[u8], dst: &mut [u8]) -> c_int {
    // SAFETY: both pointers come from live slices whose lengths are passed alongside.
    unsafe {
        lz4_sys::LZ4_decompress_safe(
            src.as_ptr() as *const c_char,
            dst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            dst.len() as c_int,
        )
    }
}
