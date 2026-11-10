//! Dictionary-dependent blocks made with the permitted oracle surface (Frame
//! format, "Block Independence flag", "Data Blocks").

use crate::block::decompress_safe;
use crate::frame::{compress_frame, Prefs};

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
    // 7-byte header (no content size, no dictionary id), then blocks.
    let mut pos = 7usize;
    let mut blocks: Vec<(bool, &[u8])> = Vec::new();
    loop {
        let word = u32::from_le_bytes(frame.get(pos..pos + 4)?.try_into().ok()?);
        pos += 4;
        if word == 0 {
            break;
        }
        let stored = word & 0x8000_0000 != 0;
        let len = (word & 0x7FFF_FFFF) as usize;
        let body = frame.get(pos..pos.checked_add(len)?)?;
        pos += len;
        blocks.push((stored, body));
    }
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
