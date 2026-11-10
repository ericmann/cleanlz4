#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]
//! cleanlz4: a clean-room LZ4 block and frame decoder and XXH32.

pub mod block;
pub mod error;
pub mod frame;
pub mod stream;
pub mod xxh32;

pub use error::{Error, Result};

/// Default cap on decoded output, in bytes. Config key `limits.max_output_bytes`.
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 1 << 30;

/// Decode one LZ4 block into a new buffer, failing if the output would exceed
/// `max_output` bytes.
pub fn decompress_block(input: &[u8], max_output: usize) -> Result<Vec<u8>> {
    let mut dst = Vec::new();
    block::decode_block(input, &mut dst, max_output)?;
    Ok(dst)
}
