//! Dictionary-dependent blocks made with the permitted oracle surface (Frame
//! format, "Block Independence flag", "Data Blocks").

use crate::block::decompress_safe;
use crate::frame::{compress_frame, decompress_frames, Decoded, Prefs};

/// A compressed block and whether it needs the dictionary to decode.
#[derive(Debug, Clone)]
pub struct DictBlock {
    pub block: Vec<u8>,
    pub dict_dependent: bool,
}

/// The 4 MiB block size code's capacity.
const BLOCK_MAX: usize = 4 << 20;

/// Make a block for `data` that may refer back into `dict`, by compressing
/// `[dict, data]` as linked blocks and keeping the second block. `None` when the
/// second block is missing or stored uncompressed, or an input exceeds one block.
pub fn dict_block(dict: &[u8], data: &[u8]) -> Option<DictBlock> {
    if dict.len() > BLOCK_MAX || data.len() > BLOCK_MAX {
        return None;
    }
    let prefs = Prefs {
        block_size_id: 7,
        linked: true,
        auto_flush: true,
        ..Prefs::default()
    };
    let frame = compress_frame(&[dict, data], &prefs);
    let blocks = frame_blocks(&frame)?;
    // An empty chunk produces no block, so address blocks from the end.
    let (stored, body) = if data.is_empty() {
        return None;
    } else if dict.is_empty() {
        *blocks.first()?
    } else {
        *blocks.get(1)?
    };
    if stored {
        return None;
    }
    let dict_dependent = match decompress_safe(body, data.len()) {
        Ok(plain) => plain != data,
        Err(_) => true,
    };
    Some(DictBlock {
        block: body.to_vec(),
        dict_dependent,
    })
}

/// Split an oracle frame (7-byte header, no content size, no dictionary id)
/// into its data blocks: `(stored uncompressed, bytes)`.
fn frame_blocks(frame: &[u8]) -> Option<Vec<(bool, &[u8])>> {
    let mut pos = 7usize;
    let mut blocks = Vec::new();
    loop {
        let word = u32::from_le_bytes(frame.get(pos..pos + 4)?.try_into().ok()?);
        pos += 4;
        if word == 0 {
            return Some(blocks);
        }
        let stored = word & 0x8000_0000 != 0;
        let len = (word & 0x7FFF_FFFF) as usize;
        let body = frame.get(pos..pos.checked_add(len)?)?;
        pos += len;
        blocks.push((stored, body));
    }
}

/// Assemble a frame by hand whose blocks depend on `dict` (Frame format,
/// "Frame Descriptor", "Block Independence flag", "Dictionary ID", "Data
/// Blocks"): 4 MiB maximum, no checksums, no content size, a Dict-ID when
/// `dict_id` is given. The expected output is the concatenation of `chunks`.
///
/// Linked: one oracle compression of `[dict, chunk1, chunk2, ...]` keeps blocks
/// 2 onward. Independent: one `dict_block` per chunk. The oracle cannot decode
/// the result (no dictionary decoder on the permitted surface); the header is
/// checked to be one it accepts.
pub fn dict_frame(
    dict: &[u8],
    chunks: &[&[u8]],
    independent: bool,
    dict_id: Option<u32>,
) -> Vec<u8> {
    let flg: u8 = if independent { 0x60 } else { 0x40 } | u8::from(dict_id.is_some());
    let mut desc = vec![flg, 0x70];
    if let Some(id) = dict_id {
        desc.extend_from_slice(&id.to_le_bytes());
    }
    let hc = ((cleanlz4::xxh32::xxh32(&desc, 0) >> 8) & 0xff) as u8;
    let mut out = vec![0x04, 0x22, 0x4D, 0x18];
    out.extend_from_slice(&desc);
    out.push(hc);
    // The oracle must accept the header and wait for more.
    match decompress_frames(&out, BLOCK_MAX) {
        Decoded::Rejected(why) if why == "truncated" => {}
        other => panic!("oracle does not accept the assembled header: {other:?}"),
    }
    let mut push = |stored: bool, body: &[u8]| {
        let word = u32::try_from(body.len()).unwrap_or(0) | if stored { 0x8000_0000 } else { 0 };
        out.extend_from_slice(&word.to_le_bytes());
        out.extend_from_slice(body);
    };
    if independent {
        for chunk in chunks.iter().filter(|c| !c.is_empty()) {
            match dict_block(dict, chunk) {
                Some(b) => push(false, &b.block),
                None => push(true, chunk),
            }
        }
    } else {
        let mut all: Vec<&[u8]> = Vec::new();
        if !dict.is_empty() {
            all.push(dict);
        }
        all.extend(chunks.iter().copied().filter(|c| !c.is_empty()));
        let prefs = Prefs {
            block_size_id: 7,
            linked: true,
            auto_flush: true,
            ..Prefs::default()
        };
        let frame = compress_frame(&all, &prefs);
        let Some(blocks) = frame_blocks(&frame) else {
            panic!("cannot walk the oracle's frame");
        };
        let skip = usize::from(!dict.is_empty());
        for (stored, body) in blocks.into_iter().skip(skip) {
            push(stored, body);
        }
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}
