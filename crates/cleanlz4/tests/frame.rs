//! Frame tests with frames built by hand from the Frame format document.

use cleanlz4::xxh32::xxh32;
use cleanlz4::{
    decompress_frame, decompress_frame_with, DecodeOptions, Error, TrailingData,
    DEFAULT_MAX_OUTPUT_BYTES,
};

const MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

/// FLG: version 01, independent blocks, plus `extra` flag bits.
fn flg(extra: u8) -> u8 {
    0x40 | 0x20 | extra
}

/// BD for block maximum size code `code` (4 to 7).
fn bd(code: u8) -> u8 {
    code << 4
}

/// A header with a valid checksum.
fn header(flg: u8, bd: u8) -> Vec<u8> {
    let d = [flg, bd];
    let mut h = MAGIC.to_vec();
    h.extend_from_slice(&d);
    h.push(((xxh32(&d, 0) >> 8) & 0xFF) as u8);
    h
}

fn block_word(size: usize, uncompressed: bool) -> [u8; 4] {
    let w = size as u32 | if uncompressed { 0x8000_0000 } else { 0 };
    w.to_le_bytes()
}

/// A frame of 64 KB-max blocks, each `(uncompressed, data)`.
fn frame(blocks: &[(bool, Vec<u8>)]) -> Vec<u8> {
    frame_with(flg(0), blocks)
}

/// The same with an explicit FLG byte.
fn frame_with(flg: u8, blocks: &[(bool, Vec<u8>)]) -> Vec<u8> {
    let mut f = header(flg, bd(4));
    for (uncompressed, data) in blocks {
        f.extend_from_slice(&block_word(data.len(), *uncompressed));
        f.extend_from_slice(data);
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    f
}

/// A literals-only compressed block: a token with the literal count, then the literals.
fn literal_block(lits: &[u8]) -> Vec<u8> {
    assert!(lits.len() < 15);
    let mut b = vec![(lits.len() as u8) << 4];
    b.extend_from_slice(lits);
    b
}

#[test]
fn uncompressed_blocks_frame_decodes() {
    let f = frame(&[(true, b"hello ".to_vec()), (true, b"world".to_vec())]);
    assert_eq!(decompress_frame(&f).expect("ok"), b"hello world");
}

#[test]
fn empty_frame_decodes_to_empty() {
    let f = frame(&[]);
    assert_eq!(decompress_frame(&f).expect("ok"), Vec::<u8>::new());
}

#[test]
fn zero_size_uncompressed_block_is_valid() {
    // 0x80000000 is an empty uncompressed block, not the EndMark.
    let f = frame(&[(true, vec![]), (true, b"x".to_vec())]);
    assert_eq!(decompress_frame(&f).expect("ok"), b"x");
}

#[test]
fn compressed_block_frame_decodes() {
    let f = frame(&[
        (false, literal_block(b"abc")),
        (true, b"def".to_vec()),
        (false, literal_block(b"ghi")),
    ]);
    assert_eq!(decompress_frame(&f).expect("ok"), b"abcdefghi");
    // A compressed block with a match.
    let mut b = vec![0x14, b'a', 1, 0];
    b.extend(literal_block(b"12345"));
    let f = frame(&[(false, b)]);
    assert_eq!(decompress_frame(&f).expect("ok"), b"aaaaaaaaa12345");
}

#[test]
fn block_size_field_over_the_maximum_is_rejected() {
    // The size word alone decides: no data follows it.
    for uncompressed in [true, false] {
        let mut f = header(flg(0), bd(4));
        f.extend_from_slice(&block_word(65_537, uncompressed));
        assert_eq!(decompress_frame(&f), Err(Error::BlockSizeExceeded));
    }
    // The maximum itself is allowed.
    let mut f = header(flg(0), bd(4));
    f.extend_from_slice(&block_word(65_536, true));
    f.extend(vec![7u8; 65_536]);
    f.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(decompress_frame(&f).expect("ok").len(), 65_536);
}

#[test]
fn decoded_block_over_the_maximum_is_rejected() {
    // One literal, then a match of 70,000 bytes at offset 1: decodes past 64 KB.
    let match_extra = 70_000 - 4 - 15;
    let mut b = vec![0x1F, b'a', 1, 0];
    b.extend(std::iter::repeat_n(255u8, match_extra / 255));
    b.push((match_extra % 255) as u8);
    b.extend(literal_block(b"12345"));
    assert!(b.len() < 65_536);
    let f = frame(&[(false, b)]);
    assert_eq!(decompress_frame(&f), Err(Error::BlockSizeExceeded));
}

#[test]
fn every_truncation_of_a_small_frame_is_truncated_input() {
    let f = frame(&[(false, literal_block(b"abc")), (true, b"def".to_vec())]);
    for cut in 1..f.len() {
        assert_eq!(
            decompress_frame(&f[..cut]),
            Err(Error::TruncatedInput),
            "cut {cut} of {}",
            f.len()
        );
    }
    assert!(decompress_frame(&f).is_ok());
}

#[test]
fn bytes_after_the_frame_are_trailing_data() {
    let mut f = frame(&[(true, b"x".to_vec())]);
    f.push(0);
    assert_eq!(decompress_frame(&f), Err(Error::TrailingData));
}

#[test]
fn default_options_have_the_documented_values() {
    let o = DecodeOptions::default();
    assert_eq!(o.max_output_bytes, DEFAULT_MAX_OUTPUT_BYTES);
    assert_eq!(o.dictionary, None);
    assert_eq!(o.trailing_data, TrailingData::Error);
    assert!(o.verify_checksums);
}

/// FLG for linked blocks: version 01 and no independence bit.
const LINKED: u8 = 0x40;

/// A compressed block of no literals and one match, then five final literals.
fn match_block(offset: u16, match_len: usize) -> Vec<u8> {
    assert!((4..=18).contains(&match_len));
    let mut b = vec![(match_len - 4) as u8];
    b.extend_from_slice(&offset.to_le_bytes());
    b.extend(literal_block(b"12345"));
    b
}

#[test]
fn linked_block_copies_from_the_previous_block() {
    let blocks = [
        (false, literal_block(b"abcdefgh")),
        (false, match_block(8, 8)),
    ];
    let out = decompress_frame(&frame_with(LINKED, &blocks)).expect("ok");
    assert_eq!(out, b"abcdefghabcdefgh12345");
}

#[test]
fn linked_block_copies_from_a_previous_uncompressed_block() {
    let blocks = [(true, b"abcdefgh".to_vec()), (false, match_block(8, 8))];
    let out = decompress_frame(&frame_with(LINKED, &blocks)).expect("ok");
    assert_eq!(out, b"abcdefghabcdefgh12345");
}

#[test]
fn independent_frame_rejects_an_offset_into_the_previous_block() {
    let blocks = [(true, b"abcdefgh".to_vec()), (false, match_block(8, 8))];
    assert_eq!(
        decompress_frame(&frame(&blocks)),
        Err(Error::OffsetOutOfRange)
    );
}

#[test]
fn history_reaches_back_65535_bytes_across_blocks() {
    let big: Vec<u8> = (0..65_536u32).map(|i| (i % 251) as u8).collect();
    // One block back: 65,535 bytes before the end of a 65,536-byte block.
    let blocks = [(true, big.clone()), (false, match_block(65_535, 8))];
    let out = decompress_frame(&frame_with(LINKED, &blocks)).expect("ok");
    assert_eq!(out.len(), 65_536 + 8 + 5);
    assert_eq!(&out[65_536..65_544], &big[1..9]);
    // Across two blocks: a 10-byte block between, so the offset points at big[11..].
    let blocks = [
        (true, big.clone()),
        (true, b"0123456789".to_vec()),
        (false, match_block(65_535, 8)),
    ];
    let out = decompress_frame(&frame_with(LINKED, &blocks)).expect("ok");
    assert_eq!(&out[65_546..65_554], &big[11..19]);
    // Many blocks later the history is still only the last 64 KB.
    let mut blocks = vec![(true, big.clone()); 3];
    blocks.push((false, match_block(65_535, 8)));
    let out = decompress_frame(&frame_with(LINKED, &blocks)).expect("ok");
    assert_eq!(&out[3 * 65_536..3 * 65_536 + 8], &big[1..9]);
}

const BLOCK_CHECKSUM: u8 = 0x10;
const CONTENT_CHECKSUM: u8 = 0x04;

/// A frame with block checksums (computed per block) and a content checksum
/// for `content`; `tamper_*` adds to the stored values to make them wrong.
fn checked_frame(
    blocks: &[(bool, Vec<u8>)],
    content: &[u8],
    tamper_block: u32,
    tamper_content: u32,
) -> Vec<u8> {
    let mut f = header(flg(BLOCK_CHECKSUM | CONTENT_CHECKSUM), bd(4));
    for (uncompressed, data) in blocks {
        f.extend_from_slice(&block_word(data.len(), *uncompressed));
        f.extend_from_slice(data);
        f.extend_from_slice(&xxh32(data, 0).wrapping_add(tamper_block).to_le_bytes());
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    f.extend_from_slice(&xxh32(content, 0).wrapping_add(tamper_content).to_le_bytes());
    f
}

fn without_checks() -> DecodeOptions {
    DecodeOptions {
        verify_checksums: false,
        ..DecodeOptions::default()
    }
}

#[test]
fn block_checksum_mismatch_is_rejected() {
    let blocks = [(false, literal_block(b"abc")), (true, b"def".to_vec())];
    let good = checked_frame(&blocks, b"abcdef", 0, 0);
    assert_eq!(decompress_frame(&good).expect("ok"), b"abcdef");
    let bad = checked_frame(&blocks, b"abcdef", 1, 0);
    assert_eq!(decompress_frame(&bad), Err(Error::BlockChecksumMismatch));
}

#[test]
fn content_checksum_mismatch_is_rejected() {
    let blocks = [(true, b"abc".to_vec())];
    let bad = checked_frame(&blocks, b"abc", 0, 1);
    assert_eq!(decompress_frame(&bad), Err(Error::ContentChecksumMismatch));
    // The checksum covers the decoded bytes, not the stored ones.
    let wrong_content = checked_frame(&blocks, b"abd", 0, 0);
    assert_eq!(
        decompress_frame(&wrong_content),
        Err(Error::ContentChecksumMismatch)
    );
}

#[test]
fn empty_block_still_carries_a_block_checksum() {
    let blocks = [(true, vec![]), (true, b"x".to_vec())];
    let good = checked_frame(&blocks, b"x", 0, 0);
    assert_eq!(decompress_frame(&good).expect("ok"), b"x");
    let bad = checked_frame(&blocks, b"x", 1, 0);
    assert_eq!(decompress_frame(&bad), Err(Error::BlockChecksumMismatch));
    // Without the four bytes after the empty block the frame does not parse.
    let mut f = header(flg(BLOCK_CHECKSUM), bd(4));
    f.extend_from_slice(&block_word(0, true));
    f.extend_from_slice(&[0, 0, 0, 0]);
    assert!(decompress_frame(&f).is_err());
}

#[test]
fn verification_off_accepts_bad_checksums_and_reports_it() {
    let blocks = [(true, b"abc".to_vec())];
    let bad = checked_frame(&blocks, b"abc", 1, 1);
    let out = decompress_frame_with(&bad, &without_checks()).expect("ok");
    assert_eq!(out.data, b"abc");
    assert!(!out.checksums_verified);
    let good = checked_frame(&blocks, b"abc", 0, 0);
    let out = decompress_frame_with(&good, &DecodeOptions::default()).expect("ok");
    assert!(out.checksums_verified);
    // The checksum bytes are still consumed: a truncated one is an error.
    assert_eq!(
        decompress_frame_with(&bad[..bad.len() - 1], &without_checks()),
        Err(Error::TruncatedInput)
    );
}

#[test]
fn verification_off_still_rejects_a_bad_header_checksum() {
    let mut f = frame(&[(true, b"abc".to_vec())]);
    f[6] ^= 0xFF;
    assert_eq!(
        decompress_frame_with(&f, &without_checks()),
        Err(Error::BadHeaderChecksum)
    );
}

const CONTENT_SIZE: u8 = 0x08;

/// A frame declaring `declared` as its content size around `blocks`.
fn sized_frame(declared: u64, blocks: &[(bool, Vec<u8>)]) -> Vec<u8> {
    let mut d = vec![flg(CONTENT_SIZE), bd(4)];
    d.extend_from_slice(&declared.to_le_bytes());
    let mut f = MAGIC.to_vec();
    f.extend_from_slice(&d);
    f.push(((xxh32(&d, 0) >> 8) & 0xFF) as u8);
    for (uncompressed, data) in blocks {
        f.extend_from_slice(&block_word(data.len(), *uncompressed));
        f.extend_from_slice(data);
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    f
}

#[test]
fn matching_content_size_is_accepted() {
    let blocks = [(true, b"abc".to_vec()), (false, literal_block(b"defg"))];
    assert_eq!(
        decompress_frame(&sized_frame(7, &blocks)).expect("ok"),
        b"abcdefg"
    );
}

#[test]
fn content_size_smaller_than_the_data_is_rejected() {
    let blocks = [(true, b"abc".to_vec()), (true, b"defg".to_vec())];
    assert_eq!(
        decompress_frame(&sized_frame(6, &blocks)),
        Err(Error::ContentSizeMismatch)
    );
    // Caught as soon as the count passes it, before later blocks are needed.
    let mut f = sized_frame(2, &[(true, b"abc".to_vec())]);
    f.truncate(f.len() - 4);
    assert_eq!(decompress_frame(&f), Err(Error::ContentSizeMismatch));
}

#[test]
fn content_size_larger_than_the_data_is_rejected() {
    let blocks = [(true, b"abc".to_vec())];
    assert_eq!(
        decompress_frame(&sized_frame(4, &blocks)),
        Err(Error::ContentSizeMismatch)
    );
    // A huge declared size allocates nothing and is still just a mismatch.
    assert_eq!(
        decompress_frame(&sized_frame(u64::MAX, &blocks)),
        Err(Error::ContentSizeMismatch)
    );
}

#[test]
fn content_size_zero_with_an_empty_frame_is_accepted() {
    assert_eq!(
        decompress_frame(&sized_frame(0, &[])).expect("ok"),
        Vec::<u8>::new()
    );
    assert_eq!(
        decompress_frame(&sized_frame(0, &[(true, vec![])])).expect("ok"),
        Vec::<u8>::new()
    );
}
