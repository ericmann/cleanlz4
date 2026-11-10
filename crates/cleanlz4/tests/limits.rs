//! The output limit, in the one-shot and streaming paths.

use cleanlz4::xxh32::xxh32;
use cleanlz4::{decompress_block, decompress_frame_with, DecodeOptions, Decoder, Error, Status};

const MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

fn header(flg: u8, code: u8, size: Option<u64>) -> Vec<u8> {
    let mut d = vec![flg, code << 4];
    if let Some(s) = size {
        d.extend_from_slice(&s.to_le_bytes());
    }
    let mut h = MAGIC.to_vec();
    h.extend_from_slice(&d);
    h.push(((xxh32(&d, 0) >> 8) & 0xFF) as u8);
    h
}

/// A frame of uncompressed blocks, 64 KB maximum.
fn raw_frame(blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut f = header(0x60, 4, None);
    for b in blocks {
        f.extend_from_slice(&(b.len() as u32 | 0x8000_0000).to_le_bytes());
        f.extend_from_slice(b);
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    f
}

fn limit(n: u64) -> DecodeOptions {
    DecodeOptions {
        max_output_bytes: n,
        ..DecodeOptions::default()
    }
}

fn stream(input: &[u8], opts: DecodeOptions) -> Result<Vec<u8>, Error> {
    let mut d = Decoder::new(opts);
    let mut out = Vec::new();
    let mut buf = [0u8; 64];
    let mut pos = 0;
    loop {
        let p = d.decode(&input[pos..], &mut buf, true)?;
        pos += p.consumed;
        out.extend_from_slice(&buf[..p.produced]);
        if p.status == Status::Done {
            return Ok(out);
        }
    }
}

fn three_blocks() -> Vec<u8> {
    raw_frame(&[vec![1; 100], vec![2; 100], vec![3; 100]])
}

#[test]
fn one_shot_over_the_limit_is_output_limit_exceeded() {
    let f = three_blocks();
    for n in [0u64, 99, 150, 299] {
        assert_eq!(
            decompress_frame_with(&f, &limit(n)).map(|o| o.data.len()),
            Err(Error::OutputLimitExceeded),
            "limit {n}"
        );
    }
}

#[test]
fn streaming_over_the_limit_is_output_limit_exceeded() {
    let f = three_blocks();
    for n in [0u64, 99, 150, 299] {
        assert_eq!(
            stream(&f, limit(n)).map(|v| v.len()),
            Err(Error::OutputLimitExceeded),
            "limit {n}"
        );
    }
}

#[test]
fn a_limit_equal_to_the_output_is_ok() {
    let f = three_blocks();
    assert_eq!(
        decompress_frame_with(&f, &limit(300))
            .expect("ok")
            .data
            .len(),
        300
    );
    assert_eq!(stream(&f, limit(300)).expect("ok").len(), 300);
    assert_eq!(
        decompress_frame_with(&raw_frame(&[]), &limit(0))
            .expect("ok")
            .data
            .len(),
        0
    );
}

#[test]
fn content_size_over_the_limit_fails_at_the_header() {
    // Only the header: the declared size alone is enough to fail.
    let h = header(0x68, 4, Some(1000));
    assert_eq!(
        decompress_frame_with(&h, &limit(500)).map(|o| o.data.len()),
        Err(Error::OutputLimitExceeded)
    );
    let mut d = Decoder::new(limit(500));
    let mut out = [0u8; 16];
    assert_eq!(
        d.decode(&h, &mut out, false).map(|p| p.status),
        Err(Error::OutputLimitExceeded)
    );
    // At the limit it is not an error at the header.
    assert!(d.checksums_verified());
    let mut d = Decoder::new(limit(1000));
    assert_eq!(
        d.decode(&h, &mut out, false).expect("ok").status,
        Status::NeedsInput
    );
}

#[test]
fn the_limit_spans_concatenated_frames() {
    let one = raw_frame(&[vec![7; 100]]);
    let both: Vec<u8> = [one.clone(), one].concat();
    assert_eq!(
        decompress_frame_with(&both, &limit(199)).map(|o| o.data.len()),
        Err(Error::OutputLimitExceeded)
    );
    assert_eq!(
        decompress_frame_with(&both, &limit(200))
            .expect("ok")
            .data
            .len(),
        200
    );
    assert_eq!(
        stream(&both, limit(199)).map(|v| v.len()),
        Err(Error::OutputLimitExceeded)
    );
    assert_eq!(stream(&both, limit(200)).expect("ok").len(), 200);
}

#[test]
fn a_compressed_block_over_the_limit_is_named_by_the_smaller_bound() {
    // One literal then a 70,000-byte match at offset 1: 70,001 bytes decoded.
    let extra = 70_000 - 4 - 15;
    let mut b = vec![0x1F, b'a', 1, 0];
    b.extend(std::iter::repeat_n(255u8, extra / 255));
    b.push((extra % 255) as u8);
    b.extend([0x50, 1, 2, 3, 4, 5]);
    // 1 MiB blocks, so the block holds 70,006 bytes comfortably.
    let mut f = header(0x60, 6, None);
    f.extend_from_slice(&(b.len() as u32).to_le_bytes());
    f.extend_from_slice(&b);
    f.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(
        decompress_frame_with(&f, &limit(70_006))
            .expect("exact")
            .data
            .len(),
        70_006
    );
    // The allowance is the smaller bound: the limit error.
    assert_eq!(
        decompress_frame_with(&f, &limit(60_000)).map(|o| o.data.len()),
        Err(Error::OutputLimitExceeded)
    );
    assert_eq!(
        stream(&f, limit(60_000)).map(|v| v.len()),
        Err(Error::OutputLimitExceeded)
    );
    // With 64 KB blocks the block maximum is the smaller bound: the block error.
    let mut g = header(0x60, 4, None);
    g.extend_from_slice(&(b.len() as u32).to_le_bytes());
    g.extend_from_slice(&b);
    g.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(
        decompress_frame_with(&g, &limit(u64::MAX)).map(|o| o.data.len()),
        Err(Error::BlockSizeExceeded)
    );
}

#[test]
fn block_decode_limit_applies_to_decompress_block() {
    let extra = 1_000_000 - 4 - 15;
    let mut b = vec![0x1F, b'a', 1, 0];
    b.extend(std::iter::repeat_n(255u8, extra / 255));
    b.push((extra % 255) as u8);
    b.extend([0x50, 1, 2, 3, 4, 5]);
    assert_eq!(
        decompress_block(&b, 1_000_006).expect("exact").len(),
        1_000_006
    );
    assert_eq!(
        decompress_block(&b, 1_000_005),
        Err(Error::OutputLimitExceeded)
    );
    assert_eq!(decompress_block(&b, 10), Err(Error::OutputLimitExceeded));
}

#[test]
fn one_shot_output_capacity_never_exceeds_the_limit() {
    const BLOCK: usize = 4 << 20;
    let blocks: Vec<Vec<u8>> = (0..4u8).map(|i| vec![i; BLOCK]).collect();
    let mut f = header(0x60, 7, None);
    for b in &blocks {
        f.extend_from_slice(&(b.len() as u32 | 0x8000_0000).to_le_bytes());
        f.extend_from_slice(b);
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    let full = 4 * BLOCK;
    let out = decompress_frame_with(&f, &limit(full as u64)).expect("ok");
    assert_eq!(out.data.len(), full);
    assert!(
        out.data.capacity() <= full,
        "capacity {}",
        out.data.capacity()
    );

    // The same data cut to 12 MiB + 1 bytes.
    let cut = 3 * BLOCK + 1;
    let mut g = header(0x60, 7, None);
    for b in blocks.iter().take(3) {
        g.extend_from_slice(&(b.len() as u32 | 0x8000_0000).to_le_bytes());
        g.extend_from_slice(b);
    }
    g.extend_from_slice(&(1u32 | 0x8000_0000).to_le_bytes());
    g.push(3);
    g.extend_from_slice(&[0, 0, 0, 0]);
    let out = decompress_frame_with(&g, &limit(cut as u64)).expect("ok");
    assert_eq!(out.data.len(), cut);
    assert!(
        out.data.capacity() <= cut,
        "capacity {}",
        out.data.capacity()
    );
}
