//! Block decoding into a growing output buffer (Block format, "Compressed block
//! format" and "End of block conditions").

use crate::error::{Error, Result};

/// The minimum match length (Block format, "Compressed block format": `minmatch`).
const MIN_MATCH: usize = 4;
/// The value of a four-bit length field that means "more bytes follow".
const NIBBLE_MAX: usize = 15;

/// Which of the three end-of-block conditions are enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndOfBlockPolicy {
    /// Rule 1: the last sequence contains only literals.
    pub(crate) last_sequence_is_literals_only: bool,
    /// Rule 2: the last 5 bytes of the block are literals.
    pub(crate) last_five_are_literals: bool,
    /// Rule 3: the last match starts at least 12 bytes before the end of the block.
    pub(crate) last_match_twelve_from_end: bool,
}

/// Strict: every rule is enforced.
const EOB: EndOfBlockPolicy = EndOfBlockPolicy {
    last_sequence_is_literals_only: true,
    last_five_are_literals: true,
    last_match_twelve_from_end: true,
};

/// Rule 2: the minimum size of the final literal run after a match.
const FINAL_LITERALS: usize = 5;
/// Rule 3: the minimum distance from the last match's start to the block's end.
const LAST_MATCH_DISTANCE: usize = 12;

/// Take `n` bytes from `src` at `*pos`, advancing `*pos`.
fn take<'a>(src: &'a [u8], pos: &mut usize, n: usize) -> Result<&'a [u8]> {
    let end = pos.checked_add(n).ok_or(Error::TruncatedInput)?;
    let bytes = src.get(*pos..end).ok_or(Error::TruncatedInput)?;
    *pos = end;
    Ok(bytes)
}

/// Read one byte from `src` at `*pos`, advancing `*pos`.
fn byte(src: &[u8], pos: &mut usize) -> Result<u8> {
    let b = *src.get(*pos).ok_or(Error::TruncatedInput)?;
    *pos = pos.saturating_add(1);
    Ok(b)
}

/// Read a length whose four-bit field is `nibble`: when it is 15, add bytes
/// until one is below 255 (Block format, "Compressed block format").
fn read_length(src: &[u8], pos: &mut usize, nibble: usize) -> Result<usize> {
    let mut total = nibble;
    if nibble == NIBBLE_MAX {
        loop {
            let b = byte(src, pos)?;
            total = total
                .checked_add(usize::from(b))
                .ok_or(Error::OutputLimitExceeded)?;
            if b != u8::MAX {
                break;
            }
        }
    }
    Ok(total)
}

/// Fail before writing when `extra` more bytes would pass `max_len`.
fn check_limit(dst_len: usize, extra: usize, max_len: usize) -> Result<()> {
    match dst_len.checked_add(extra) {
        Some(n) if n <= max_len => Ok(()),
        _ => Err(Error::OutputLimitExceeded),
    }
}

/// Copy `len` bytes from `offset` bytes back; bytes just written are read again
/// when `len` exceeds `offset` (Block format: "bytes just copied are copied again").
fn copy_match(dst: &mut Vec<u8>, offset: usize, len: usize) -> Result<()> {
    let start = dst
        .len()
        .checked_sub(offset)
        .ok_or(Error::OffsetOutOfRange)?;
    let mut remaining = len;
    while remaining > 0 {
        // The pattern repeats with period `offset`, so each pass copies from `start`.
        let n = remaining.min(offset);
        let end = start.checked_add(n).ok_or(Error::OffsetOutOfRange)?;
        if end > dst.len() {
            return Err(Error::OffsetOutOfRange);
        }
        dst.extend_from_within(start..end);
        remaining = remaining.saturating_sub(n);
    }
    Ok(())
}

/// Decode one block, appending to `dst`; `dst.len()` never exceeds `max_len`.
/// Offsets resolve against everything already in `dst`.
pub(crate) fn decode_block(src: &[u8], dst: &mut Vec<u8>, max_len: usize) -> Result<()> {
    decode_block_with(src, dst, max_len, EOB)
}

pub(crate) fn decode_block_with(
    src: &[u8],
    dst: &mut Vec<u8>,
    max_len: usize,
    policy: EndOfBlockPolicy,
) -> Result<()> {
    if src.is_empty() {
        return Err(Error::TruncatedInput);
    }
    let mut pos = 0usize;
    // Where the last match's copy began in `dst`, if the block has a match.
    let mut last_match_start: Option<usize> = None;
    loop {
        let token = byte(src, &mut pos)?;
        let literal_len = read_length(src, &mut pos, usize::from(token >> 4))?;
        check_limit(dst.len(), literal_len, max_len)?;
        let literals = take(src, &mut pos, literal_len)?;
        dst.reserve(literal_len);
        dst.extend_from_slice(literals);

        if pos == src.len() {
            // Rule 1 holds: the block ends right after literals. The low four
            // bits of this final token are ignored.
            if let Some(start) = last_match_start {
                if policy.last_five_are_literals && literal_len < FINAL_LITERALS {
                    return Err(Error::EndOfBlockViolation);
                }
                let tail = dst.len().saturating_sub(start);
                if policy.last_match_twelve_from_end && tail < LAST_MATCH_DISTANCE {
                    return Err(Error::EndOfBlockViolation);
                }
            }
            return Ok(());
        }

        let offset_bytes = take(src, &mut pos, 2)?;
        let offset = offset_bytes
            .iter()
            .rev()
            .fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
        if offset == 0 {
            return Err(Error::ZeroOffset);
        }
        if offset > dst.len() {
            return Err(Error::OffsetOutOfRange);
        }
        let extra = read_length(src, &mut pos, usize::from(token & 0x0F))?;
        let match_len = extra
            .checked_add(MIN_MATCH)
            .ok_or(Error::OutputLimitExceeded)?;
        check_limit(dst.len(), match_len, max_len)?;
        dst.reserve(match_len);
        last_match_start = Some(dst.len());
        copy_match(dst, offset, match_len)?;

        if pos == src.len() && policy.last_sequence_is_literals_only {
            return Err(Error::EndOfBlockViolation);
        }
        if pos == src.len() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sequence built from the document's text: token, extension bytes,
    /// literals, then (when given) offset and match-length extension.
    fn seq(literals: &[u8], m: Option<(u16, usize)>) -> Vec<u8> {
        fn ext(v: usize) -> (usize, Vec<u8>) {
            if v < 15 {
                return (v, vec![]);
            }
            let mut rest = v - 15;
            let mut bytes = vec![];
            while rest >= 255 {
                bytes.push(255);
                rest -= 255;
            }
            bytes.push(rest as u8);
            (15, bytes)
        }
        let (lh, lext) = ext(literals.len());
        let (mh, mext, off) = match m {
            Some((off, len)) => {
                let (h, e) = ext(len - 4);
                (h, e, Some(off))
            }
            None => (0, vec![], None),
        };
        let mut v = vec![((lh << 4) | mh) as u8];
        v.extend(lext);
        v.extend_from_slice(literals);
        if let Some(off) = off {
            v.extend_from_slice(&off.to_le_bytes());
            v.extend(mext);
        }
        v
    }

    fn decode(src: &[u8]) -> Result<Vec<u8>> {
        let mut d = Vec::new();
        decode_block(src, &mut d, 1 << 20)?;
        Ok(d)
    }

    /// A valid block: 4 literals, a 12-byte match at offset 4, 5 final literals.
    fn valid_block() -> Vec<u8> {
        let mut b = seq(b"abcd", Some((4, 12)));
        b.extend(seq(b"WXYZ!", None));
        b
    }

    #[test]
    fn literal_only_block() {
        assert_eq!(decode(&seq(b"hello", None)).expect("ok"), b"hello");
        assert_eq!(decode(&seq(b"x", None)).expect("ok"), b"x");
    }

    #[test]
    fn single_zero_token_is_the_empty_block() {
        assert_eq!(decode(&[0]).expect("ok"), Vec::<u8>::new());
        assert_eq!(decode(&[]), Err(Error::TruncatedInput));
    }

    #[test]
    fn literal_length_examples_48_280_and_15() {
        // The document's three examples, byte for byte.
        let lit48 = vec![7u8; 48];
        let mut b = vec![0xF0, 33];
        b.extend(&lit48);
        assert_eq!(decode(&b).expect("ok"), lit48);
        let lit280 = vec![9u8; 280];
        let mut b = vec![0xF0, 255, 10];
        b.extend(&lit280);
        assert_eq!(decode(&b).expect("ok"), lit280);
        let lit15 = vec![3u8; 15];
        let mut b = vec![0xF0, 0];
        b.extend(&lit15);
        assert_eq!(decode(&b).expect("ok"), lit15);
    }

    #[test]
    fn match_copies_from_history() {
        let out = decode(&valid_block()).expect("ok");
        assert_eq!(out, b"abcdabcdabcdabcdWXYZ!");
    }

    #[test]
    fn match_reads_bytes_present_before_the_call() {
        let mut dst = b"0123456789".to_vec();
        let mut b = seq(b"", Some((10, 12)));
        b.extend(seq(b"12345", None));
        decode_block(&b, &mut dst, 1 << 20).expect("ok");
        assert_eq!(dst, b"012345678901234567890112345");
    }

    #[test]
    fn overlapping_match_with_offset_1_repeats_the_last_byte() {
        let mut b = seq(b"z", Some((1, 20)));
        b.extend(seq(b"12345", None));
        let out = decode(&b).expect("ok");
        assert_eq!(&out[..21], vec![b'z'; 21].as_slice());
        assert_eq!(&out[21..], b"12345");
    }

    #[test]
    fn match_length_extension_bytes() {
        // Match lengths 18, 19, 274 and 600 straddle the extension boundaries.
        for len in [18usize, 19, 273, 274, 600] {
            let mut b = seq(b"abcd", Some((4, len)));
            b.extend(seq(b"12345", None));
            let out = decode(&b).expect("ok");
            assert_eq!(out.len(), 4 + len + 5, "len {len}");
        }
    }

    #[test]
    fn zero_offset_is_rejected() {
        let mut b = seq(b"abcd", Some((0, 12)));
        b.extend(seq(b"12345", None));
        assert_eq!(decode(&b), Err(Error::ZeroOffset));
    }

    #[test]
    fn offset_before_output_start_is_rejected() {
        let mut b = seq(b"abcd", Some((5, 12)));
        b.extend(seq(b"12345", None));
        assert_eq!(decode(&b), Err(Error::OffsetOutOfRange));
        let mut b = seq(b"abcd", Some((4, 12)));
        b.extend(seq(b"12345", None));
        assert!(decode(&b).is_ok());
    }

    #[test]
    fn truncation_in_length_literals_and_offset_is_rejected() {
        // Inside the literal-length extension.
        assert_eq!(decode(&[0xF0]), Err(Error::TruncatedInput));
        assert_eq!(decode(&[0xF0, 255]), Err(Error::TruncatedInput));
        // Inside the literals.
        assert_eq!(decode(&[0x50, 1, 2]), Err(Error::TruncatedInput));
        // Inside the offset.
        assert_eq!(decode(&[0x10, b'a', 1]), Err(Error::TruncatedInput));
        // Inside the match-length extension.
        assert_eq!(decode(&[0x1F, b'a', 1, 0]), Err(Error::TruncatedInput));
        assert_eq!(decode(&[0x1F, b'a', 1, 0, 255]), Err(Error::TruncatedInput));
    }

    #[test]
    fn output_limit_is_enforced_before_copying() {
        let mut dst = Vec::new();
        let b = valid_block();
        assert_eq!(
            decode_block(&b, &mut dst, 10),
            Err(Error::OutputLimitExceeded)
        );
        // The limit hit at the match: only the literals were written.
        assert_eq!(dst, b"abcd");
        let mut dst = Vec::new();
        decode_block(&b, &mut dst, 21).expect("exact fit");
        assert_eq!(dst.len(), 21);
        // A literal length far beyond the limit with no data behind it.
        let mut dst = Vec::new();
        assert_eq!(
            decode_block(&[0xF0, 255, 255, 255, 0], &mut dst, 100),
            Err(Error::OutputLimitExceeded)
        );
        assert!(dst.is_empty());
    }

    #[test]
    fn rule_1_block_ending_after_a_match() {
        let b = seq(b"abcd", Some((4, 12)));
        assert_eq!(decode(&b), Err(Error::EndOfBlockViolation));
    }

    #[test]
    fn rule_2_fewer_than_five_final_literals() {
        let mut b = seq(b"abcd", Some((4, 12)));
        b.extend(seq(b"WXYZ", None));
        assert_eq!(decode(&b), Err(Error::EndOfBlockViolation));
        // Only rule 2 is at work: with it off, the block decodes.
        let mut d = Vec::new();
        let mut p = EOB;
        p.last_five_are_literals = false;
        decode_block_with(&b, &mut d, 1 << 20, p).expect("ok without rule 2");
    }

    #[test]
    fn rule_3_last_match_within_twelve_bytes_of_the_end() {
        let mut b = seq(b"abcd", Some((4, 4)));
        b.extend(seq(b"12345", None));
        assert_eq!(decode(&b), Err(Error::EndOfBlockViolation));
        let mut d = Vec::new();
        let mut p = EOB;
        p.last_match_twelve_from_end = false;
        decode_block_with(&b, &mut d, 1 << 20, p).expect("ok without rule 3");
        // Exactly twelve bytes from the match's start to the end is accepted.
        let mut b = seq(b"abcd", Some((4, 7)));
        b.extend(seq(b"12345", None));
        assert!(decode(&b).is_ok());
    }
}
