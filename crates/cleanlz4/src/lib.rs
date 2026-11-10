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

/// Default cap on decoded output, in bytes. Config key `limits.max_output_bytes`.
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 1 << 30;
