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
pub use frame::{parse_frame_descriptor, BlockMaxSize, FrameDescriptor};

/// Default cap on decoded output, in bytes. Config key `limits.max_output_bytes`.
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 1 << 30;

/// Decode one LZ4 block into a new buffer, failing if the output would exceed
/// `max_output` bytes.
pub fn decompress_block(input: &[u8], max_output: usize) -> Result<Vec<u8>> {
    decompress_block_with_dict(input, &[], max_output)
}

/// Decode one LZ4 block whose matches may reach into `dict`, the bytes that sit
/// just before the block's output.
pub fn decompress_block_with_dict(input: &[u8], dict: &[u8], max_output: usize) -> Result<Vec<u8>> {
    let mut dst = Vec::new();
    block::decode_block(input, dict, &mut dst, max_output)?;
    Ok(dst)
}

/// A preset dictionary for frames (SPEC section 6.1, Options).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dictionary {
    pub bytes: Vec<u8>,
    pub id: Option<u32>,
}

/// What to do with bytes that follow the last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingData {
    /// Report `Error::TrailingData`.
    Error,
    /// Stop and report how much input was consumed.
    ReturnRemainder,
}

/// The kind of a frame in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Regular,
    Skippable,
    Legacy,
}

/// Options for frame decoding (SPEC section 6.1, Options).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeOptions {
    /// Cap on decoded output; config key `limits.max_output_bytes` gives the default.
    pub max_output_bytes: u64,
    pub dictionary: Option<Dictionary>,
    pub trailing_data: TrailingData,
    pub verify_checksums: bool,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        DecodeOptions {
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            dictionary: None,
            trailing_data: TrailingData::Error,
            verify_checksums: true,
        }
    }
}

/// The result of decoding a flow of frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameOutput {
    pub data: Vec<u8>,
    /// How many input bytes were consumed.
    pub consumed: usize,
    /// Whether block and content checksums were verified.
    pub checksums_verified: bool,
    /// One descriptor per regular frame.
    pub descriptors: Vec<FrameDescriptor>,
    /// One kind per frame of any kind, in order.
    pub kinds: Vec<FrameKind>,
}

/// Decode the frames in `input` with default options.
pub fn decompress_frame(input: &[u8]) -> Result<Vec<u8>> {
    Ok(decompress_frame_with(input, &DecodeOptions::default())?.data)
}

/// Decode the frames in `input` with `options`.
pub fn decompress_frame_with(input: &[u8], options: &DecodeOptions) -> Result<FrameOutput> {
    frame::decode_frames(input, options)
}
