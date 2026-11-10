//! Hand-built frames, laid out as the Frame format describes, with overrides
//! to break any field. Test tooling.

use crate::block::compress_default;
use crate::frame::{compress_frame, Prefs};
use cleanlz4::xxh32::xxh32;

/// The frame magic number.
pub const FRAME_MAGIC: u32 = 0x184D_2204;
/// The legacy frame magic number.
pub const LEGACY_MAGIC: u32 = 0x184C_2102;

/// One data block.
#[derive(Debug, Clone)]
pub struct BlockSpec {
    /// The bytes as stored (compressed block or raw data).
    pub stored: Vec<u8>,
    pub uncompressed: bool,
    /// Replaces the size word when set (the highest bit is yours to set).
    pub size_word: Option<u32>,
    /// Replaces the block checksum when set.
    pub checksum: Option<u32>,
}

impl BlockSpec {
    /// A stored (uncompressed) block.
    pub fn raw(data: &[u8]) -> Self {
        BlockSpec {
            stored: data.to_vec(),
            uncompressed: true,
            size_word: None,
            checksum: None,
        }
    }

    /// A compressed block.
    pub fn compressed(block: &[u8]) -> Self {
        BlockSpec {
            stored: block.to_vec(),
            uncompressed: false,
            size_word: None,
            checksum: None,
        }
    }
}

/// A frame to build.
#[derive(Debug, Clone)]
pub struct FrameSpec {
    pub independent: bool,
    pub block_checksum: bool,
    pub content_checksum: bool,
    /// The BD byte's Block MaxSize code (4 to 7 are valid).
    pub block_max_code: u8,
    pub content_size: Option<u64>,
    pub dict_id: Option<u32>,
    /// The two version bits (1 is valid).
    pub version: u8,
    pub flg_reserved_bit: bool,
    /// OR-ed into the BD byte (reserved bits 7 and 3 to 0).
    pub bd_reserved_bits: u8,
    pub blocks: Vec<BlockSpec>,
    /// The decoded content, for the content checksum.
    pub content: Vec<u8>,
    pub content_checksum_override: Option<u32>,
    pub magic: u32,
    pub flg_override: Option<u8>,
    pub bd_override: Option<u8>,
    pub header_checksum_override: Option<u8>,
}

impl Default for FrameSpec {
    fn default() -> Self {
        FrameSpec {
            independent: true,
            block_checksum: false,
            content_checksum: false,
            block_max_code: 4,
            content_size: None,
            dict_id: None,
            version: 1,
            flg_reserved_bit: false,
            bd_reserved_bits: 0,
            blocks: Vec::new(),
            content: Vec::new(),
            content_checksum_override: None,
            magic: FRAME_MAGIC,
            flg_override: None,
            bd_override: None,
            header_checksum_override: None,
        }
    }
}

/// Build the frame's bytes.
pub fn build(spec: &FrameSpec) -> Vec<u8> {
    let mut flg = spec.version << 6;
    flg |= u8::from(spec.independent) << 5;
    flg |= u8::from(spec.block_checksum) << 4;
    flg |= u8::from(spec.content_size.is_some()) << 3;
    flg |= u8::from(spec.content_checksum) << 2;
    flg |= u8::from(spec.flg_reserved_bit) << 1;
    flg |= u8::from(spec.dict_id.is_some());
    let bd = (spec.block_max_code << 4) | spec.bd_reserved_bits;
    let mut descriptor = vec![
        spec.flg_override.unwrap_or(flg),
        spec.bd_override.unwrap_or(bd),
    ];
    if let Some(size) = spec.content_size {
        descriptor.extend_from_slice(&size.to_le_bytes());
    }
    if let Some(id) = spec.dict_id {
        descriptor.extend_from_slice(&id.to_le_bytes());
    }
    let hc = spec
        .header_checksum_override
        .unwrap_or(((xxh32(&descriptor, 0) >> 8) & 0xFF) as u8);
    let mut out = spec.magic.to_le_bytes().to_vec();
    out.extend_from_slice(&descriptor);
    out.push(hc);
    for b in &spec.blocks {
        let word = b
            .size_word
            .unwrap_or(b.stored.len() as u32 | if b.uncompressed { 0x8000_0000 } else { 0 });
        out.extend_from_slice(&word.to_le_bytes());
        out.extend_from_slice(&b.stored);
        if spec.block_checksum {
            let c = b.checksum.unwrap_or_else(|| xxh32(&b.stored, 0));
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    if spec.content_checksum {
        let c = spec
            .content_checksum_override
            .unwrap_or_else(|| xxh32(&spec.content, 0));
        out.extend_from_slice(&c.to_le_bytes());
    }
    out
}

/// A skippable frame with magic 0x184D2A5 followed by `low_nibble`.
pub fn skippable(low_nibble: u8, user_data: &[u8]) -> Vec<u8> {
    let magic = 0x184D_2A50u32 | u32::from(low_nibble & 0x0F);
    let mut out = magic.to_le_bytes().to_vec();
    out.extend_from_slice(&(user_data.len() as u32).to_le_bytes());
    out.extend_from_slice(user_data);
    out
}

/// A legacy frame around compressed `blocks`, optionally ended by a zero size.
pub fn legacy(blocks: &[Vec<u8>], zero_terminated: bool) -> Vec<u8> {
    let mut out = LEGACY_MAGIC.to_le_bytes().to_vec();
    for b in blocks {
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(b);
    }
    if zero_terminated {
        out.extend_from_slice(&[0, 0, 0, 0]);
    }
    out
}

/// A frame-level probe: input bytes and cleanlz4's verdict, fixed by the plan.
#[derive(Debug, Clone)]
pub struct Probe {
    pub id: String,
    pub bytes: Vec<u8>,
    pub fixed_accept: bool,
}

fn literal_block(lits: &[u8]) -> Vec<u8> {
    let mut b = vec![(lits.len() as u8) << 4];
    b.extend_from_slice(lits);
    b
}

fn simple() -> FrameSpec {
    FrameSpec {
        blocks: vec![BlockSpec::raw(b"abc")],
        content: b"abc".to_vec(),
        ..FrameSpec::default()
    }
}

pub fn probes() -> Vec<Probe> {
    let mut v = Vec::new();
    let mut add = |id: &str, bytes: Vec<u8>, ok: bool| {
        v.push(Probe {
            id: id.to_string(),
            bytes,
            fixed_accept: ok,
        })
    };
    let regular = compress_frame(&[b"hello hello hello hello"], &Prefs::default());
    add("control: valid frame", regular.clone(), true);
    add("empty input", vec![], true);
    add("skippable alone", skippable(0, b"user data"), true);
    let mut s_then_r = skippable(3, b"xx");
    s_then_r.extend(&regular);
    add("skippable then regular", s_then_r, true);
    for n in [1usize, 3, 8] {
        let mut b = regular.clone();
        b.extend(vec![0xAB; n]);
        add(&format!("regular then {n} garbage bytes"), b, false);
    }
    let lb = compress_default(&[b'z'; 100]);
    add(
        "legacy frame",
        legacy(std::slice::from_ref(&lb), false),
        true,
    );
    add(
        "legacy frame, zero-terminated",
        legacy(std::slice::from_ref(&lb), true),
        true,
    );
    let mut dict = simple();
    dict.dict_id = Some(0x1234_5678);
    add("Dict-ID and no dictionary", build(&dict), false);
    let mut small = simple();
    small.content_size = Some(2);
    add("content size too small", build(&small), false);
    let mut large = simple();
    large.content_size = Some(4);
    add("content size too large", build(&large), false);
    let mut ok_size = simple();
    ok_size.content_size = Some(3);
    add("content size exact (control)", build(&ok_size), true);
    let big_raw = FrameSpec {
        blocks: vec![BlockSpec::raw(&vec![7u8; 65_537])],
        ..FrameSpec::default()
    };
    add(
        "uncompressed block size above the maximum",
        build(&big_raw),
        false,
    );
    // A compressed block that decodes to 70,000 bytes (a 64 KB maximum).
    let extra = 70_000 - 4 - 15;
    let mut cb = vec![0x1F, b'a', 1, 0];
    cb.extend(std::iter::repeat_n(255u8, extra / 255));
    cb.push((extra % 255) as u8);
    cb.extend(literal_block(b"12345"));
    let over = FrameSpec {
        blocks: vec![BlockSpec::compressed(&cb)],
        ..FrameSpec::default()
    };
    add(
        "compressed block decoding above the maximum",
        build(&over),
        false,
    );
    let mut f = simple();
    f.flg_reserved_bit = true;
    add("FLG reserved bit", build(&f), false);
    for bit in [0x80u8, 0x08, 0x04, 0x02, 0x01] {
        let mut f = simple();
        f.bd_reserved_bits = bit;
        add(&format!("BD reserved bit {bit:#04x}"), build(&f), false);
    }
    for ver in [0u8, 2, 3] {
        let mut f = simple();
        f.version = ver;
        add(&format!("version {ver}"), build(&f), false);
    }
    for code in 0u8..=3 {
        let mut f = simple();
        f.block_max_code = code;
        add(&format!("block max size code {code}"), build(&f), false);
    }
    let mut f = simple();
    f.header_checksum_override = Some(0x00);
    let good_hc = build(&simple())[6];
    f.header_checksum_override = Some(good_hc ^ 0xFF);
    add("bad header checksum", build(&f), false);
    let mut f = simple();
    f.block_checksum = true;
    add("block checksum (control)", build(&f), true);
    f.blocks[0].checksum = Some(0xDEAD_BEEF);
    add("bad block checksum", build(&f), false);
    let mut f = simple();
    f.content_checksum = true;
    add("content checksum (control)", build(&f), true);
    f.content_checksum_override = Some(0xDEAD_BEEF);
    add("bad content checksum", build(&f), false);
    let zero = FrameSpec {
        blocks: vec![BlockSpec::raw(b""), BlockSpec::raw(b"x")],
        ..FrameSpec::default()
    };
    add("zero-size uncompressed block", build(&zero), true);
    v
}
