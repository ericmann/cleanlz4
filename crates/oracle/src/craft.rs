//! Hand-built LZ4 blocks, encoded exactly as Block format, "Compressed block
//! format" describes. Test tooling: it may build blocks that break the
//! end-of-block rules.

/// One sequence: literals, then a match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seq {
    pub literals: Vec<u8>,
    pub offset: u16,
    /// Total match length, at least 4.
    pub match_len: usize,
}

/// A four-bit field plus extension bytes for `value`.
fn nibble_and_extension(value: usize) -> (u8, Vec<u8>) {
    if value < 15 {
        return (value as u8, Vec::new());
    }
    let mut rest = value - 15;
    let mut ext = Vec::new();
    while rest >= 255 {
        ext.push(255);
        rest -= 255;
    }
    ext.push(rest as u8);
    (15, ext)
}

fn push_sequence(out: &mut Vec<u8>, s: &Seq) {
    let (lit_nibble, lit_ext) = nibble_and_extension(s.literals.len());
    let (match_nibble, match_ext) = nibble_and_extension(s.match_len.saturating_sub(4));
    out.push((lit_nibble << 4) | match_nibble);
    out.extend(lit_ext);
    out.extend_from_slice(&s.literals);
    out.extend_from_slice(&s.offset.to_le_bytes());
    out.extend(match_ext);
}

/// A block of `seqs` with no final literals-only sequence.
pub fn build_block_without_final(seqs: &[Seq]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in seqs {
        push_sequence(&mut out, s);
    }
    out
}

/// A block of `seqs` followed by a final sequence of `final_literals`, whose
/// token's low four bits are `final_low_nibble`.
pub fn build_block(seqs: &[Seq], final_literals: &[u8], final_low_nibble: u8) -> Vec<u8> {
    let mut out = build_block_without_final(seqs);
    let (nibble, ext) = nibble_and_extension(final_literals.len());
    out.push((nibble << 4) | (final_low_nibble & 0x0F));
    out.extend(ext);
    out.extend_from_slice(final_literals);
    out
}

/// The decoded output of a well-formed block built from `seqs` and
/// `final_literals`, by the format's rules (matches copy byte by byte).
pub fn expected_output(seqs: &[Seq], final_literals: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for s in seqs {
        out.extend_from_slice(&s.literals);
        let start = out.len() - usize::from(s.offset);
        for i in 0..s.match_len {
            let b = out[start + i];
            out.push(b);
        }
    }
    out.extend_from_slice(final_literals);
    out
}
