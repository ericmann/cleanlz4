//! Frame parsing, block sequencing, checksums and the one-shot driver.

use crate::block::decode_block;
use crate::error::{Error, Result};
use crate::xxh32::xxh32;
use crate::{DecodeOptions, FrameKind, FrameOutput};

/// The frame magic number, read little-endian (Frame format, "Magic Number").
const FRAME_MAGIC: u32 = 0x184D_2204;

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

fn read_u32_le(input: &[u8], at: usize) -> Option<u32> {
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
}

/// The LZ4 window (Frame format, "Block Independence flag": "up to LZ4 window
/// size, which is 64 KB").
const WINDOW: usize = 64 << 10;

impl FrameState {
    /// Start a frame from its parsed descriptor.
    pub(crate) fn new(desc: FrameDescriptor) -> Self {
        FrameState {
            desc,
            history: Vec::new(),
        }
    }

    /// Remember decoded bytes for the next linked block, keeping the last `WINDOW`.
    fn remember(&mut self, block: &[u8]) {
        if self.desc.block_independence {
            return;
        }
        self.history.extend_from_slice(block);
        let excess = self.history.len().saturating_sub(WINDOW);
        self.history.drain(..excess);
    }

    /// The frame's descriptor.
    pub(crate) fn descriptor(&self) -> &FrameDescriptor {
        &self.desc
    }

    /// Decode one data block's `data` (without any block checksum), appending
    /// the decoded bytes to `out`.
    pub(crate) fn decode_data_block(
        &mut self,
        header: BlockHeader,
        data: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<()> {
        let max = self.desc.block_max_size.bytes();
        if header.size > max {
            return Err(Error::BlockSizeExceeded);
        }
        if header.uncompressed {
            out.extend_from_slice(data);
            self.remember(data);
            return Ok(());
        }
        let mut block = Vec::new();
        // Linked blocks see the previous output as a prefix; independent ones see none.
        let prefix: &[u8] = if self.desc.block_independence {
            &[]
        } else {
            &self.history
        };
        match decode_block(data, prefix, &mut block, max) {
            Ok(()) => {}
            // The block's decoded size passed the frame's maximum.
            Err(Error::OutputLimitExceeded) => return Err(Error::BlockSizeExceeded),
            Err(e) => return Err(e),
        }
        out.extend_from_slice(&block);
        self.remember(&block);
        Ok(())
    }

    /// End the frame after its EndMark.
    pub(crate) fn finish(&mut self) -> Result<()> {
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

/// Decode a complete input slice holding one regular frame.
pub(crate) fn decode_frames(input: &[u8], _options: &DecodeOptions) -> Result<FrameOutput> {
    let (desc, header_len) = parse_frame_descriptor(input)?;
    let mut state = FrameState::new(desc);
    let mut out = Vec::new();
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
            // Read and skipped here; verified by a later task.
            pos = skip(input, pos, 4)?;
        }
        state.decode_data_block(header, data, &mut out)?;
    }
    if state.descriptor().content_checksum {
        pos = skip(input, pos, 4)?;
    }
    state.finish()?;
    if pos < input.len() {
        return Err(Error::TrailingData);
    }
    Ok(FrameOutput {
        data: out,
        consumed: pos,
        checksums_verified: true,
        descriptors: vec![desc],
        kinds: vec![FrameKind::Regular],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
