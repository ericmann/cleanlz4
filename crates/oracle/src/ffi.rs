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

use crate::frame::Prefs;

fn to_binding_prefs(p: &Prefs) -> lz4_sys::LZ4FPreferences {
    use lz4_sys::*;
    LZ4FPreferences {
        frame_info: LZ4FFrameInfo {
            block_size_id: match p.block_size_id {
                4 => BlockSize::Max64KB,
                5 => BlockSize::Max256KB,
                6 => BlockSize::Max1MB,
                7 => BlockSize::Max4MB,
                _ => BlockSize::Default,
            },
            block_mode: if p.linked {
                BlockMode::Linked
            } else {
                BlockMode::Independent
            },
            content_checksum_flag: if p.content_checksum {
                ContentChecksum::ChecksumEnabled
            } else {
                ContentChecksum::NoChecksum
            },
            frame_type: FrameType::Frame,
            content_size: p.content_size.unwrap_or(0),
            dict_id: p.dict_id,
            block_checksum_flag: if p.block_checksum {
                BlockChecksum::BlockChecksumEnabled
            } else {
                BlockChecksum::NoBlockChecksum
            },
        },
        compression_level: p.level as _,
        auto_flush: p.auto_flush as _,
        favor_dec_speed: 0,
        reserved: [0; 3],
    }
}

/// Frame API version passed to the context constructors.
const FRAME_VERSION: u32 = lz4_sys::LZ4F_VERSION;

/// Whether a frame-API return code is an error code.
pub fn frame_is_error(code: usize) -> bool {
    // SAFETY: a pure function of its integer argument.
    unsafe { lz4_sys::LZ4F_isError(code) != 0 }
}

/// Text for a frame-API error code.
pub fn frame_error_name(code: usize) -> String {
    // SAFETY: the returned pointer is a static NUL-terminated string.
    unsafe {
        let p = lz4_sys::LZ4F_getErrorName(code);
        if p.is_null() {
            return String::from("unknown error");
        }
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// Upper bound of the compressed size of `len` bytes in one update call.
pub fn frame_compress_bound(len: usize, prefs: &Prefs) -> usize {
    let bp = to_binding_prefs(prefs);
    // SAFETY: the preferences pointer refers to a live local.
    unsafe { lz4_sys::LZ4F_compressBound(len, &bp) }
}

/// An owned compression context.
pub struct CompressCtx {
    ctx: lz4_sys::LZ4FCompressionContext,
}

impl CompressCtx {
    /// Create a context, or the error text.
    pub fn new() -> Result<CompressCtx, String> {
        let mut ctx = lz4_sys::LZ4FCompressionContext(std::ptr::null_mut());
        // SAFETY: `ctx` is a live out-parameter.
        let code = unsafe { lz4_sys::LZ4F_createCompressionContext(&mut ctx, FRAME_VERSION) };
        if frame_is_error(code) {
            return Err(frame_error_name(code));
        }
        Ok(CompressCtx { ctx })
    }

    /// Write the frame header into `dst`; returns bytes written or the error text.
    pub fn begin(&mut self, dst: &mut [u8], prefs: &Prefs) -> Result<usize, String> {
        let bp = to_binding_prefs(prefs);
        // SAFETY: pointers come from live slices and a live local, with lengths alongside.
        let n = unsafe { lz4_sys::LZ4F_compressBegin(self.ctx, dst.as_mut_ptr(), dst.len(), &bp) };
        if frame_is_error(n) {
            Err(frame_error_name(n))
        } else {
            Ok(n)
        }
    }

    /// Compress one chunk into `dst`; returns bytes written or the error text.
    pub fn update(&mut self, dst: &mut [u8], src: &[u8]) -> Result<usize, String> {
        // SAFETY: pointers come from live slices with lengths alongside; no options.
        let n = unsafe {
            lz4_sys::LZ4F_compressUpdate(
                self.ctx,
                dst.as_mut_ptr(),
                dst.len(),
                src.as_ptr(),
                src.len(),
                std::ptr::null(),
            )
        };
        if frame_is_error(n) {
            Err(frame_error_name(n))
        } else {
            Ok(n)
        }
    }

    /// Finish the frame into `dst`; returns bytes written or the error text.
    pub fn end(&mut self, dst: &mut [u8]) -> Result<usize, String> {
        // SAFETY: the pointer comes from a live slice with its length alongside.
        let n = unsafe {
            lz4_sys::LZ4F_compressEnd(self.ctx, dst.as_mut_ptr(), dst.len(), std::ptr::null())
        };
        if frame_is_error(n) {
            Err(frame_error_name(n))
        } else {
            Ok(n)
        }
    }
}

impl Drop for CompressCtx {
    fn drop(&mut self) {
        // SAFETY: the context was created by the matching constructor and is freed once.
        unsafe {
            lz4_sys::LZ4F_freeCompressionContext(self.ctx);
        }
    }
}

/// Result of one decompression call.
pub struct DecompressStep {
    pub consumed: usize,
    pub produced: usize,
    /// The raw return value: a hint (0 means the frame is complete) or an error code.
    pub code: usize,
}

/// An owned decompression context.
pub struct DecompressCtx {
    ctx: lz4_sys::LZ4FDecompressionContext,
}

impl DecompressCtx {
    /// Create a context, or the error text.
    pub fn new() -> Result<DecompressCtx, String> {
        let mut ctx = lz4_sys::LZ4FDecompressionContext(std::ptr::null_mut());
        // SAFETY: `ctx` is a live out-parameter.
        let code = unsafe { lz4_sys::LZ4F_createDecompressionContext(&mut ctx, FRAME_VERSION) };
        if frame_is_error(code) {
            return Err(frame_error_name(code));
        }
        Ok(DecompressCtx { ctx })
    }

    /// One decompression call.
    pub fn step(&mut self, dst: &mut [u8], src: &[u8]) -> DecompressStep {
        let mut dst_size = dst.len();
        let mut src_size = src.len();
        // SAFETY: pointers come from live slices; the size cells are live locals.
        let code = unsafe {
            lz4_sys::LZ4F_decompress(
                self.ctx,
                dst.as_mut_ptr(),
                &mut dst_size,
                src.as_ptr(),
                &mut src_size,
                std::ptr::null(),
            )
        };
        DecompressStep {
            consumed: src_size,
            produced: dst_size,
            code,
        }
    }
}

impl Drop for DecompressCtx {
    fn drop(&mut self) {
        // SAFETY: the context was created by the matching constructor and is freed once.
        unsafe {
            lz4_sys::LZ4F_freeDecompressionContext(self.ctx);
        }
    }
}
