//! Frame parsing, block sequencing, checksums and the one-shot driver.

use crate::block::{decode_block, reserve_capped};
use crate::error::{Error, Result};
use crate::xxh32::{xxh32, Xxh32};
use crate::{DecodeOptions, Dictionary, FrameKind, FrameOutput, TrailingData};

/// The frame magic number, read little-endian (Frame format, "Magic Number").
pub(crate) const FRAME_MAGIC: u32 = 0x184D_2204;

/// The frame's maximum uncompressed block size (Frame format, "Block Maximum Size").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockMaxSize {
    Max64KB,
    Max256KB,
    Max1MB,
    Max4MB,
}

impl BlockMaxSize {
    /// The size in bytes.
    pub fn bytes(self) -> usize {
        match self {
            BlockMaxSize::Max64KB => 64 << 10,
            BlockMaxSize::Max256KB => 256 << 10,
            BlockMaxSize::Max1MB => 1 << 20,
            BlockMaxSize::Max4MB => 4 << 20,
        }
    }

    /// The BD byte's three-bit value (4 to 7) for this size.
    fn from_code(code: u8) -> Result<BlockMaxSize> {
        match code {
            4 => Ok(BlockMaxSize::Max64KB),
            5 => Ok(BlockMaxSize::Max256KB),
            6 => Ok(BlockMaxSize::Max1MB),
            7 => Ok(BlockMaxSize::Max4MB),
            _ => Err(Error::InvalidBlockMaxSize),
        }
    }
}

/// A parsed frame descriptor (Frame format, "Frame Descriptor").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDescriptor {
    pub block_max_size: BlockMaxSize,
    pub block_independence: bool,
    pub block_checksum: bool,
    pub content_checksum: bool,
    pub content_size: Option<u64>,
    pub dict_id: Option<u32>,
}

/// FLG bit masks (Frame format, "FLG byte").
const FLG_DICT_ID: u8 = 1 << 0;
const FLG_RESERVED: u8 = 1 << 1;
const FLG_CONTENT_CHECKSUM: u8 = 1 << 2;
const FLG_CONTENT_SIZE: u8 = 1 << 3;
const FLG_BLOCK_CHECKSUM: u8 = 1 << 4;
const FLG_BLOCK_INDEPENDENCE: u8 = 1 << 5;
/// The version field's value (Frame format, "Version Number").
const VERSION: u8 = 1;
/// BD bits that must be zero (Frame format, "BD byte", "Reserved bits").
const BD_RESERVED: u8 = 0b1000_1111;

pub(crate) fn read_u32_le(input: &[u8], at: usize) -> Option<u32> {
    let bytes = input.get(at..at.checked_add(4)?)?;
    let mut a = [0u8; 4];
    for (d, s) in a.iter_mut().zip(bytes.iter()) {
        *d = *s;
    }
    Some(u32::from_le_bytes(a))
}

fn read_u64_le(input: &[u8], at: usize) -> Option<u64> {
    let bytes = input.get(at..at.checked_add(8)?)?;
    let mut a = [0u8; 8];
    for (d, s) in a.iter_mut().zip(bytes.iter()) {
        *d = *s;
    }
    Some(u64::from_le_bytes(a))
}

/// Parse the frame header at the start of `input`, which begins at the magic
/// number. Returns the descriptor and the header's length (7 to 19 bytes).
///
/// The header checksum is always verified: it guards the layout the decoder is
/// about to trust.
pub fn parse_frame_descriptor(input: &[u8]) -> Result<(FrameDescriptor, usize)> {
    let magic = read_u32_le(input, 0).ok_or(Error::TruncatedInput)?;
    if magic != FRAME_MAGIC {
        return Err(Error::BadMagic);
    }
    let flg = *input.get(4).ok_or(Error::TruncatedInput)?;
    if flg >> 6 != VERSION {
        return Err(Error::UnsupportedVersion);
    }
    let has_content_size = flg & FLG_CONTENT_SIZE != 0;
    let has_dict_id = flg & FLG_DICT_ID != 0;
    // FLG, BD, optional fields, then the checksum byte.
    let descriptor_len = 2usize
        .saturating_add(if has_content_size { 8 } else { 0 })
        .saturating_add(if has_dict_id { 4 } else { 0 });
    let header_len = 4usize.saturating_add(descriptor_len).saturating_add(1);
    let descriptor_end = header_len.saturating_sub(1);
    let descriptor = input.get(4..descriptor_end).ok_or(Error::TruncatedInput)?;
    let checksum = *input.get(descriptor_end).ok_or(Error::TruncatedInput)?;
    if flg & FLG_RESERVED != 0 {
        return Err(Error::ReservedBitSet);
    }
    let bd = *input.get(5).ok_or(Error::TruncatedInput)?;
    if bd & BD_RESERVED != 0 {
        return Err(Error::ReservedBitSet);
    }
    let block_max_size = BlockMaxSize::from_code((bd >> 4) & 0x07)?;
    if checksum != ((xxh32(descriptor, 0) >> 8) & 0xFF) as u8 {
        return Err(Error::BadHeaderChecksum);
    }
    let mut at = 6usize;
    let content_size = if has_content_size {
        let v = read_u64_le(input, at).ok_or(Error::TruncatedInput)?;
        at = at.saturating_add(8);
        Some(v)
    } else {
        None
    };
    let dict_id = if has_dict_id {
        Some(read_u32_le(input, at).ok_or(Error::TruncatedInput)?)
    } else {
        None
    };
    Ok((
        FrameDescriptor {
            block_max_size,
            block_independence: flg & FLG_BLOCK_INDEPENDENCE != 0,
            block_checksum: flg & FLG_BLOCK_CHECKSUM != 0,
            content_checksum: flg & FLG_CONTENT_CHECKSUM != 0,
            content_size,
            dict_id,
        },
        header_len,
    ))
}

/// The bound for decoding one block: the smaller of the block maximum and the
/// remaining output allowance, and whether the allowance is the smaller one.
pub(crate) fn block_bound(block_max: usize, allowance: u64) -> (usize, bool) {
    let allowed = usize::try_from(allowance).unwrap_or(usize::MAX);
    if allowed < block_max {
        (allowed, true)
    } else {
        (block_max, false)
    }
}

/// Name the limit a block decode hit: the output allowance when it was the
/// smaller bound, else the block's maximum size.
pub(crate) fn limit_error(e: Error, allowance_bounds: bool) -> Error {
    match e {
        Error::OutputLimitExceeded if !allowance_bounds => Error::BlockSizeExceeded,
        other => other,
    }
}

/// Append `bytes` to `out`, growing its capacity geometrically but never past
/// the bytes already in it plus the output allowance.
fn append_capped(out: &mut Vec<u8>, bytes: &[u8], allowance: u64) -> Result<()> {
    let room = usize::try_from(allowance).unwrap_or(usize::MAX);
    reserve_capped(out, bytes.len(), out.len().saturating_add(room))?;
    out.extend_from_slice(bytes);
    Ok(())
}

/// A data block's size word (Frame format, "Data Blocks", "Block Size").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockHeader {
    /// The data section's size in bytes, not counting any block checksum.
    pub(crate) size: usize,
    /// The highest bit: the block is stored uncompressed.
    pub(crate) uncompressed: bool,
}

/// The block size word that ends a frame's blocks (Frame format, "EndMark").
pub(crate) const END_MARK: u32 = 0;

/// Split a block size word: the highest bit marks an uncompressed block and
/// the other 31 bits give the data size (Frame format, "Block Size").
pub(crate) fn parse_block_header(word: u32) -> BlockHeader {
    BlockHeader {
        size: (word & 0x7FFF_FFFF) as usize,
        uncompressed: word & 0x8000_0000 != 0,
    }
}

/// The state of one regular frame being decoded, shared by the one-shot and
/// streaming drivers. It works on whole units: one block at a time.
#[derive(Debug)]
pub(crate) struct FrameState {
    desc: FrameDescriptor,
    /// The last `WINDOW` bytes of this frame's decoded output; kept only when
    /// blocks are linked.
    history: Vec<u8>,
    /// The last `WINDOW` bytes of the preset dictionary, the prefix of every
    /// block of an independent frame; empty for linked frames, whose history
    /// starts from it instead.
    preset: Vec<u8>,
    /// Whether block and content checksums are compared.
    verify: bool,
    /// XXH32 of the decoded bytes so far (fed only when verifying).
    content_hash: Xxh32,
    /// Decoded bytes so far, compared with the descriptor's content size.
    decoded: u64,
}

/// The LZ4 window (Frame format, "Block Independence flag": "up to LZ4 window
/// size, which is 64 KB").
const WINDOW: usize = 64 << 10;

impl FrameState {
    /// Start a frame from its parsed descriptor.
    ///
    /// Dictionary rules (Frame format, "Dictionary ID flag", "Dictionary ID"):
    /// a frame with a Dict-ID and no dictionary is `DictionaryRequired`; a frame
    /// Dict-ID and a dictionary id that are both present and differ are
    /// `DictionaryMismatch`; otherwise a supplied dictionary is used. With
    /// independent blocks it is every block's prefix; with linked blocks it is
    /// the initial history only, its last `WINDOW` bytes.
    pub(crate) fn new(
        desc: FrameDescriptor,
        verify: bool,
        dictionary: Option<&Dictionary>,
    ) -> Result<Self> {
        if desc.dict_id.is_some() && dictionary.is_none() {
            return Err(Error::DictionaryRequired);
        }
        let mut tail = Vec::new();
        if let Some(d) = dictionary {
            if let (Some(want), Some(have)) = (desc.dict_id, d.id) {
                if want != have {
                    return Err(Error::DictionaryMismatch);
                }
            }
            tail.extend_from_slice(
                d.bytes
                    .get(d.bytes.len().saturating_sub(WINDOW)..)
                    .unwrap_or(&[]),
            );
        }
        let (history, preset) = if desc.block_independence {
            (Vec::new(), tail)
        } else {
            (tail, Vec::new())
        };
        Ok(FrameState {
            desc,
            history,
            preset,
            verify,
            content_hash: Xxh32::new(0),
            decoded: 0,
        })
    }

    /// Check a block's stored checksum against its raw data, before the block
    /// is decoded (Frame format, "Block checksum"). `data` is the block as
    /// stored, compressed or not.
    pub(crate) fn verify_block_checksum(&self, data: &[u8], stored: u32) -> Result<()> {
        if self.verify && xxh32(data, 0) != stored {
            return Err(Error::BlockChecksumMismatch);
        }
        Ok(())
    }

    /// Remember decoded bytes for the next linked block, keeping the last `WINDOW`.
    fn remember(&mut self, block: &[u8]) {
        if self.desc.block_independence {
            return;
        }
        // Trim before extending, so the history's capacity stays near the window
        // however large the block is.
        let keep = WINDOW.saturating_sub(block.len());
        let excess = self.history.len().saturating_sub(keep);
        self.history.drain(..excess);
        let tail = block
            .get(block.len().saturating_sub(WINDOW)..)
            .unwrap_or(&[]);
        self.history.extend_from_slice(tail);
    }

    /// The frame's descriptor.
    pub(crate) fn descriptor(&self) -> &FrameDescriptor {
        &self.desc
    }

    /// Decode one data block's `data` (without any block checksum), appending
    /// the decoded bytes to `out`; returns how many. `allowance` is how many
    /// more bytes the output limit still permits: a block that would pass it
    /// is `OutputLimitExceeded`, raised before the bytes are written.
    pub(crate) fn decode_data_block(
        &mut self,
        header: BlockHeader,
        data: &[u8],
        out: &mut Vec<u8>,
        allowance: u64,
    ) -> Result<usize> {
        let max = self.desc.block_max_size.bytes();
        if header.size > max {
            return Err(Error::BlockSizeExceeded);
        }
        if header.uncompressed {
            if data.len() as u64 > allowance {
                return Err(Error::OutputLimitExceeded);
            }
            self.account(data.len())?;
            append_capped(out, data, allowance)?;
            self.produced(data);
            return Ok(data.len());
        }
        let (bound, allowance_bounds) = block_bound(max, allowance);
        let mut block = Vec::new();
        // Linked blocks see the previous output as a prefix; independent ones see
        // only the preset dictionary, if any.
        let prefix: &[u8] = if self.desc.block_independence {
            &self.preset
        } else {
            &self.history
        };
        decode_block(data, prefix, &mut block, bound)
            .map_err(|e| limit_error(e, allowance_bounds))?;
        self.account(block.len())?;
        append_capped(out, &block, allowance)?;
        self.produced(&block);
        Ok(block.len())
    }

    /// Count `n` more decoded bytes; more than the declared content size is
    /// `ContentSizeMismatch` (Frame format, "Content Size": "used to validate
    /// content correctness"). The size never drives an allocation.
    fn account(&mut self, n: usize) -> Result<()> {
        self.decoded = self.decoded.saturating_add(n as u64);
        match self.desc.content_size {
            Some(size) if self.decoded > size => Err(Error::ContentSizeMismatch),
            _ => Ok(()),
        }
    }

    /// Account for newly decoded bytes: history and the content checksum.
    fn produced(&mut self, block: &[u8]) {
        self.remember(block);
        if self.verify && self.desc.content_checksum {
            self.content_hash.update(block);
        }
    }

    /// End the frame after its EndMark, with the stored content checksum when
    /// the descriptor has one (Frame format, "Content Checksum").
    pub(crate) fn finish(&mut self, stored_content_checksum: Option<u32>) -> Result<()> {
        if self
            .desc
            .content_size
            .is_some_and(|size| self.decoded != size)
        {
            return Err(Error::ContentSizeMismatch);
        }
        if let Some(stored) = stored_content_checksum {
            if self.verify && self.content_hash.digest() != stored {
                return Err(Error::ContentChecksumMismatch);
            }
        }
        Ok(())
    }
}

fn skip(input: &[u8], pos: usize, n: usize) -> Result<usize> {
    let end = pos.checked_add(n).ok_or(Error::TruncatedInput)?;
    if end > input.len() {
        return Err(Error::TruncatedInput);
    }
    Ok(end)
}

/// The legacy frame's magic number (Frame format, "Legacy frame", "Magic Number").
const LEGACY_MAGIC: u32 = 0x184C_2102;
/// The first skippable magic number; the low nibble may be anything
/// (Frame format, "Skippable Frames", "Magic Number").
const SKIPPABLE_MAGIC_BASE: u32 = 0x184D_2A50;
const SKIPPABLE_MAGIC_MASK: u32 = 0xFFFF_FFF0;

/// The legacy frame's fixed block size (Frame format, "Legacy frame": "Fixed block size : 8 MB").
const LEGACY_BLOCK_BYTES: usize = 8 << 20;
/// The largest compressed legacy block: the block format's worst-case growth of
/// an 8 MiB block (one extra byte per 255, plus a small constant).
pub(crate) const LEGACY_MAX_COMPRESSED: usize = LEGACY_BLOCK_BYTES + LEGACY_BLOCK_BYTES / 255 + 16;

/// What a frame boundary holds.
enum Boundary {
    Regular,
    Skippable,
    Legacy,
    /// Fewer than four bytes that begin a known magic number.
    PartialMagic,
    /// Bytes that are not a magic number.
    Unknown,
}

/// Whether `bytes` (at most three) begin the little-endian form of a known magic number.
pub(crate) fn is_magic_prefix(bytes: &[u8]) -> bool {
    let n = bytes.len();
    let begins = |magic: u32| magic.to_le_bytes().get(..n) == Some(bytes);
    begins(FRAME_MAGIC)
        || begins(LEGACY_MAGIC)
        || (0..16u32).any(|low| begins(SKIPPABLE_MAGIC_BASE | low))
}

fn classify(rest: &[u8]) -> Boundary {
    match read_u32_le(rest, 0) {
        Some(m) if m == FRAME_MAGIC => Boundary::Regular,
        Some(m) if m & SKIPPABLE_MAGIC_MASK == SKIPPABLE_MAGIC_BASE => Boundary::Skippable,
        Some(m) if m == LEGACY_MAGIC => Boundary::Legacy,
        Some(_) => Boundary::Unknown,
        None if is_magic_prefix(rest) => Boundary::PartialMagic,
        None => Boundary::Unknown,
    }
}

/// Decode one regular frame at the start of `input`, appending to `out`;
/// returns its descriptor and the bytes it used.
fn decode_regular_frame(
    input: &[u8],
    options: &DecodeOptions,
    out: &mut Vec<u8>,
    remaining: &mut u64,
) -> Result<(FrameDescriptor, usize)> {
    let (desc, header_len) = parse_frame_descriptor(input)?;
    // Fail fast: a declared size past the allowance cannot be decoded.
    if desc.content_size.is_some_and(|size| size > *remaining) {
        return Err(Error::OutputLimitExceeded);
    }
    let mut state = FrameState::new(desc, options.verify_checksums, options.dictionary.as_ref())?;
    let mut pos = header_len;
    loop {
        let word = read_u32_le(input, pos).ok_or(Error::TruncatedInput)?;
        pos = pos.saturating_add(4);
        if word == END_MARK {
            break;
        }
        let header = parse_block_header(word);
        if header.size > state.descriptor().block_max_size.bytes() {
            return Err(Error::BlockSizeExceeded);
        }
        let end = skip(input, pos, header.size)?;
        let data = input.get(pos..end).ok_or(Error::TruncatedInput)?;
        pos = end;
        if state.descriptor().block_checksum {
            let stored = read_u32_le(input, pos).ok_or(Error::TruncatedInput)?;
            pos = skip(input, pos, 4)?;
            state.verify_block_checksum(data, stored)?;
        }
        let n = state.decode_data_block(header, data, out, *remaining)?;
        *remaining = remaining.saturating_sub(n as u64);
    }
    let mut stored_content = None;
    if state.descriptor().content_checksum {
        stored_content = Some(read_u32_le(input, pos).ok_or(Error::TruncatedInput)?);
        pos = skip(input, pos, 4)?;
    }
    state.finish(stored_content)?;
    Ok((desc, pos))
}

/// Decode one legacy frame at the start of `input` (Frame format, "Legacy
/// frame"), appending to `out`; returns the bytes it used. The frame ends at
/// the end of input, at a known magic number (left unconsumed) or at a zero
/// block size (consumed: the documented Linux kernel extension).
fn decode_legacy_frame(input: &[u8], out: &mut Vec<u8>, remaining: &mut u64) -> Result<usize> {
    let mut pos = 4usize;
    loop {
        if pos >= input.len() {
            return Ok(pos);
        }
        let word = read_u32_le(input, pos).ok_or(Error::TruncatedInput)?;
        if word == 0 {
            return Ok(pos.saturating_add(4));
        }
        if magic_kind(word).is_some() {
            return Ok(pos);
        }
        let size = word as usize;
        if size > LEGACY_MAX_COMPRESSED {
            return Err(Error::BlockSizeExceeded);
        }
        let start = pos.saturating_add(4);
        let end = skip(input, start, size)?;
        let data = input.get(start..end).ok_or(Error::TruncatedInput)?;
        decode_legacy_block(data, out, remaining)?;
        pos = end;
    }
}

/// Decode one legacy block, appending to `out` and charging `remaining`.
pub(crate) fn decode_legacy_block(
    data: &[u8],
    out: &mut Vec<u8>,
    remaining: &mut u64,
) -> Result<()> {
    let mut block = Vec::new();
    let (bound, allowance_bounds) = block_bound(LEGACY_BLOCK_BYTES, *remaining);
    decode_block(data, &[], &mut block, bound).map_err(|e| limit_error(e, allowance_bounds))?;
    append_capped(out, &block, *remaining)?;
    *remaining = remaining.saturating_sub(block.len() as u64);
    Ok(())
}

/// The kind of frame a magic number starts, if it is a known one.
pub(crate) fn magic_kind(word: u32) -> Option<FrameKind> {
    if word == FRAME_MAGIC {
        Some(FrameKind::Regular)
    } else if word == LEGACY_MAGIC {
        Some(FrameKind::Legacy)
    } else if word & SKIPPABLE_MAGIC_MASK == SKIPPABLE_MAGIC_BASE {
        Some(FrameKind::Skippable)
    } else {
        None
    }
}

/// Skip one skippable frame at the start of `input`; returns the bytes it used
/// (Frame format, "Skippable Frames").
fn skip_skippable_frame(input: &[u8]) -> Result<usize> {
    let size = read_u32_le(input, 4).ok_or(Error::TruncatedInput)?;
    skip(input, 8, size as usize)
}

/// Decode every frame in a complete input slice, in order.
///
/// Frame boundary rule (Frame format, "Frame Concatenation": decoding
/// concatenated frames "is left outside of this specification"): with no
/// bytes left the flow is finished; a regular or skippable magic starts that
/// frame; one to three bytes that begin a known magic are `TruncatedInput`;
/// any other bytes are `BadMagic` when nothing has been decoded or skipped yet
/// and follow the trailing-data policy afterwards.
pub(crate) fn decode_frames(input: &[u8], options: &DecodeOptions) -> Result<FrameOutput> {
    let mut out = Vec::new();
    let mut descriptors = Vec::new();
    let mut kinds = Vec::new();
    let mut remaining = options.max_output_bytes;
    let mut pos = 0usize;
    while pos < input.len() {
        let rest = input.get(pos..).ok_or(Error::TruncatedInput)?;
        match classify(rest) {
            Boundary::Regular => {
                let (desc, used) = decode_regular_frame(rest, options, &mut out, &mut remaining)?;
                descriptors.push(desc);
                kinds.push(FrameKind::Regular);
                pos = pos.saturating_add(used);
            }
            Boundary::Skippable => {
                let used = skip_skippable_frame(rest)?;
                kinds.push(FrameKind::Skippable);
                pos = pos.saturating_add(used);
            }
            Boundary::PartialMagic => return Err(Error::TruncatedInput),
            Boundary::Legacy => {
                let used = decode_legacy_frame(rest, &mut out, &mut remaining)?;
                kinds.push(FrameKind::Legacy);
                pos = pos.saturating_add(used);
            }
            Boundary::Unknown => {
                if kinds.is_empty() {
                    return Err(Error::BadMagic);
                }
                match options.trailing_data {
                    TrailingData::Error => return Err(Error::TrailingData),
                    TrailingData::ReturnRemainder => break,
                }
            }
        }
    }
    Ok(FrameOutput {
        data: out,
        consumed: pos,
        checksums_verified: options.verify_checksums,
        descriptors,
        kinds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_kind_names_the_three_kinds_and_nothing_else() {
        assert_eq!(magic_kind(0x184D_2204), Some(FrameKind::Regular));
        assert_eq!(magic_kind(0x184C_2102), Some(FrameKind::Legacy));
        for word in 0x184D_2A50..=0x184D_2A5F_u32 {
            assert_eq!(magic_kind(word), Some(FrameKind::Skippable), "{word:#x}");
        }
        for word in [
            0,
            0x184D_2203,
            0x184D_2205,
            0x184D_2A4F,
            0x184D_2A60,
            0x184C_2101,
            u32::MAX,
        ] {
            assert_eq!(magic_kind(word), None, "{word:#x}");
        }
    }

    /// Build a header by the document's layout, with a valid checksum.
    fn header(flg: u8, bd: u8, size: Option<u64>, dict: Option<u32>) -> Vec<u8> {
        let mut d = vec![flg, bd];
        if let Some(s) = size {
            d.extend_from_slice(&s.to_le_bytes());
        }
        if let Some(i) = dict {
            d.extend_from_slice(&i.to_le_bytes());
        }
        let hc = ((xxh32(&d, 0) >> 8) & 0xFF) as u8;
        let mut h = FRAME_MAGIC.to_le_bytes().to_vec();
        h.extend(d);
        h.push(hc);
        h
    }

    /// FLG with version 01 and the given flag bits.
    fn flg(bits: u8) -> u8 {
        0x40 | bits
    }

    /// BD with the given Block MaxSize code.
    fn bd(code: u8) -> u8 {
        code << 4
    }

    #[test]
    fn parses_the_minimal_descriptor() {
        let h = header(flg(FLG_BLOCK_INDEPENDENCE), bd(4), None, None);
        assert_eq!(h.len(), 7);
        let (d, n) = parse_frame_descriptor(&h).expect("ok");
        assert_eq!(n, 7);
        assert_eq!(
            d,
            FrameDescriptor {
                block_max_size: BlockMaxSize::Max64KB,
                block_independence: true,
                block_checksum: false,
                content_checksum: false,
                content_size: None,
                dict_id: None,
            }
        );
        // Trailing bytes after the header are left alone.
        let mut longer = h.clone();
        longer.extend_from_slice(&[1, 2, 3]);
        assert_eq!(parse_frame_descriptor(&longer).expect("ok").1, 7);
    }

    #[test]
    fn parses_content_size_and_dictionary_id() {
        let bits = FLG_CONTENT_SIZE | FLG_DICT_ID | FLG_BLOCK_CHECKSUM | FLG_CONTENT_CHECKSUM;
        let h = header(
            flg(bits),
            bd(7),
            Some(0x0102_0304_0506_0708),
            Some(0xA1B2_C3D4),
        );
        assert_eq!(h.len(), 19);
        let (d, n) = parse_frame_descriptor(&h).expect("ok");
        assert_eq!(n, 19);
        assert_eq!(d.content_size, Some(0x0102_0304_0506_0708));
        assert_eq!(d.dict_id, Some(0xA1B2_C3D4));
        assert_eq!(d.block_max_size, BlockMaxSize::Max4MB);
        assert!(d.block_checksum && d.content_checksum && !d.block_independence);
        let h = header(flg(FLG_CONTENT_SIZE), bd(5), Some(9), None);
        assert_eq!(parse_frame_descriptor(&h).expect("ok").1, 15);
        let h = header(flg(FLG_DICT_ID), bd(6), None, Some(1));
        assert_eq!(parse_frame_descriptor(&h).expect("ok").1, 11);
    }

    #[test]
    fn rejects_a_bad_magic() {
        let mut h = header(flg(0), bd(4), None, None);
        h[0] ^= 1;
        assert_eq!(parse_frame_descriptor(&h), Err(Error::BadMagic));
        assert_eq!(
            parse_frame_descriptor(&[0, 0, 0, 0, 0x40, 0x40, 0]),
            Err(Error::BadMagic)
        );
    }

    #[test]
    fn rejects_versions_other_than_01() {
        for version in [0u8, 2, 3] {
            let h = header((version << 6) | FLG_BLOCK_INDEPENDENCE, bd(4), None, None);
            assert_eq!(
                parse_frame_descriptor(&h),
                Err(Error::UnsupportedVersion),
                "version {version}"
            );
        }
    }

    #[test]
    fn rejects_the_reserved_flg_bit() {
        let h = header(flg(FLG_RESERVED), bd(4), None, None);
        assert_eq!(parse_frame_descriptor(&h), Err(Error::ReservedBitSet));
    }

    #[test]
    fn rejects_reserved_bd_bits() {
        for bit in [0x80u8, 0x08, 0x04, 0x02, 0x01] {
            let h = header(flg(0), bd(4) | bit, None, None);
            assert_eq!(
                parse_frame_descriptor(&h),
                Err(Error::ReservedBitSet),
                "bit {bit:#x}"
            );
        }
    }

    #[test]
    fn rejects_block_max_size_values_0_to_3() {
        for code in 0u8..=3 {
            let h = header(flg(0), bd(code), None, None);
            assert_eq!(
                parse_frame_descriptor(&h),
                Err(Error::InvalidBlockMaxSize),
                "code {code}"
            );
        }
    }

    #[test]
    fn rejects_a_bad_header_checksum() {
        let mut h = header(flg(0), bd(4), None, None);
        let last = h.len() - 1;
        h[last] ^= 0x55;
        assert_eq!(parse_frame_descriptor(&h), Err(Error::BadHeaderChecksum));
        // A flipped descriptor bit breaks it too.
        let mut h = header(flg(0), bd(4), None, None);
        h[5] ^= 0x10;
        assert_eq!(parse_frame_descriptor(&h), Err(Error::BadHeaderChecksum));
    }

    #[test]
    fn every_truncation_of_a_full_header_is_truncated_input() {
        let bits = FLG_CONTENT_SIZE | FLG_DICT_ID;
        let h = header(flg(bits), bd(7), Some(5), Some(6));
        for cut in 0..h.len() {
            assert_eq!(
                parse_frame_descriptor(&h[..cut]),
                Err(Error::TruncatedInput),
                "cut {cut}"
            );
        }
        assert!(parse_frame_descriptor(&h).is_ok());
    }
}
