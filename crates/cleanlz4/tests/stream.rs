//! Streaming tests: every result is compared with the one-shot decoder.

use cleanlz4::xxh32::xxh32;
use cleanlz4::{
    decompress_frame_with, parse_frame_descriptor, DecodeOptions, Decoder, Error, Status,
};

const MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

fn header(flg: u8, code: u8) -> Vec<u8> {
    let d = [flg, code << 4];
    let mut h = MAGIC.to_vec();
    h.extend_from_slice(&d);
    h.push(((xxh32(&d, 0) >> 8) & 0xFF) as u8);
    h
}

fn literal_block(lits: &[u8]) -> Vec<u8> {
    assert!(lits.len() < 15);
    let mut b = vec![(lits.len() as u8) << 4];
    b.extend_from_slice(lits);
    b
}

/// A frame of 64 KB-max blocks with the given FLG byte; each block is
/// `(uncompressed, data)`; checksums follow FLG's bits.
fn frame(flg: u8, blocks: &[(bool, Vec<u8>)], content: &[u8]) -> Vec<u8> {
    let mut f = header(flg, 4);
    for (uncompressed, data) in blocks {
        let w = data.len() as u32 | if *uncompressed { 0x8000_0000 } else { 0 };
        f.extend_from_slice(&w.to_le_bytes());
        f.extend_from_slice(data);
        if flg & 0x10 != 0 {
            f.extend_from_slice(&xxh32(data, 0).to_le_bytes());
        }
    }
    f.extend_from_slice(&[0, 0, 0, 0]);
    if flg & 0x04 != 0 {
        f.extend_from_slice(&xxh32(content, 0).to_le_bytes());
    }
    f
}

/// A mix of valid frames: independent, linked, checksummed, many blocks.
fn sample_frames() -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    v.push(frame(0x60, &[], b""));
    v.push(frame(
        0x60,
        &[(true, b"hello world".to_vec())],
        b"hello world",
    ));
    v.push(frame(
        0x60,
        &[
            (false, literal_block(b"abc")),
            (true, b"def".to_vec()),
            (true, vec![]),
            (false, literal_block(b"ghi")),
        ],
        b"abcdefghi",
    ));
    // Linked blocks with a match reaching into the previous block.
    let mut m = vec![0x04, 8, 0];
    m.extend(literal_block(b"12345"));
    v.push(frame(
        0x40 | 0x10 | 0x04,
        &[(false, literal_block(b"abcdefgh")), (false, m)],
        b"abcdefghabcdefgh12345",
    ));
    let big: Vec<u8> = (0..60_000u32).map(|i| (i % 253) as u8).collect();
    v.push(frame(
        0x60 | 0x10 | 0x04,
        &[(true, big.clone()), (true, big.clone())],
        &[big.clone(), big].concat(),
    ));
    v
}

/// Stream `input` with `in_chunk`-byte input chunks and an `out_size`-byte
/// output buffer; returns the output or the error.
fn run(input: &[u8], in_chunk: usize, out_size: usize) -> Result<Vec<u8>, Error> {
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = Vec::new();
    let mut buf = vec![0u8; out_size];
    let mut pos = 0usize;
    for _ in 0..10_000_000 {
        let end = (pos + in_chunk).min(input.len());
        let chunk = &input[pos..end];
        let p = d.decode(chunk, &mut buf, end == input.len())?;
        pos += p.consumed;
        out.extend_from_slice(&buf[..p.produced]);
        if p.status == Status::Done {
            assert_eq!(pos, input.len());
            return Ok(out);
        }
    }
    panic!("no progress");
}

fn one_shot(input: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_frame_with(input, &DecodeOptions::default()).map(|o| o.data)
}

#[test]
fn one_byte_input_chunks_match_one_shot() {
    for f in sample_frames() {
        assert_eq!(run(&f, 1, 1 << 16), one_shot(&f));
    }
}

#[test]
fn one_byte_output_buffers_match_one_shot() {
    for f in sample_frames() {
        assert_eq!(run(&f, usize::MAX / 2, 1), one_shot(&f));
        assert_eq!(run(&f, 7, 3), one_shot(&f));
    }
}

#[test]
fn status_goes_needs_input_then_frame_done_then_done() {
    let f = frame(0x60, &[(true, b"hello".to_vec())], b"hello");
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 64];
    // Half the frame: the decoder wants more.
    let half = f.len() / 2;
    let p = d.decode(&f[..half], &mut out, false).expect("ok");
    assert_eq!(p.status, Status::NeedsInput);
    assert_eq!(p.consumed, half);
    // The rest: the frame ends and its bytes are delivered.
    let p = d.decode(&f[half..], &mut out, false).expect("ok");
    assert_eq!(p.status, Status::FrameDone);
    assert_eq!(p.consumed, f.len() - half);
    assert_eq!(&out[..p.produced], b"hello");
    // No more input yet: needs input; at the end: done.
    let p = d.decode(&[], &mut out, false).expect("ok");
    assert_eq!(
        (p.status, p.consumed, p.produced),
        (Status::NeedsInput, 0, 0)
    );
    let p = d.decode(&[], &mut out, true).expect("ok");
    assert_eq!(p.status, Status::Done);
    // After Done every call returns Done and takes nothing.
    let p = d.decode(&[1, 2, 3], &mut out, true).expect("ok");
    assert_eq!((p.status, p.consumed, p.produced), (Status::Done, 0, 0));
}

#[test]
fn output_full_is_reported_and_decoding_resumes() {
    let data: Vec<u8> = (0..100u8).collect();
    let f = frame(0x60, &[(true, data.clone())], &data);
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 10];
    let mut got = Vec::new();
    let mut pos = 0;
    let mut saw_full = false;
    loop {
        let p = d.decode(&f[pos..], &mut out, true).expect("ok");
        pos += p.consumed;
        got.extend_from_slice(&out[..p.produced]);
        match p.status {
            Status::OutputFull => {
                saw_full = true;
                assert_eq!(p.produced, 10);
            }
            Status::Done => break,
            Status::FrameDone => {}
            Status::NeedsInput => panic!("unexpected NeedsInput"),
        }
    }
    assert!(saw_full);
    assert_eq!(got, data);
}

#[test]
fn an_empty_output_buffer_with_pending_bytes_is_output_full() {
    let f = frame(0x60, &[(true, b"hello".to_vec())], b"hello");
    let mut d = Decoder::new(DecodeOptions::default());
    let p = d.decode(&f, &mut [], false).expect("ok");
    assert_eq!(p.status, Status::OutputFull);
    assert_eq!(p.produced, 0);
    // Delivering with room finishes the frame.
    let mut out = [0u8; 16];
    let p2 = d.decode(&f[p.consumed..], &mut out, true).expect("ok");
    assert_eq!(&out[..p2.produced], b"hello");
    assert_eq!(p2.status, Status::FrameDone);
}

#[test]
fn end_of_input_inside_a_frame_is_truncated_input() {
    for f in sample_frames() {
        for cut in 1..f.len() {
            let mut d = Decoder::new(DecodeOptions::default());
            let mut out = vec![0u8; 1 << 17];
            let r = d.decode(&f[..cut], &mut out, true);
            assert_eq!(r.map(|p| p.status), Err(Error::TruncatedInput), "cut {cut}");
            assert_eq!(one_shot(&f[..cut]), Err(Error::TruncatedInput));
        }
    }
}

#[test]
fn errors_are_sticky() {
    let mut f = frame(0x60, &[(true, b"hello".to_vec())], b"");
    f[6] ^= 0xFF; // the header checksum
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 16];
    assert_eq!(
        d.decode(&f, &mut out, true).map(|p| p.status),
        Err(Error::BadHeaderChecksum)
    );
    for _ in 0..3 {
        let good = frame(0x60, &[], b"");
        assert_eq!(
            d.decode(&good, &mut out, true).map(|p| p.status),
            Err(Error::BadHeaderChecksum)
        );
    }
    // Other errors stick too.
    let mut d = Decoder::new(DecodeOptions::default());
    assert_eq!(
        d.decode(&[0xDE, 0xAD, 0xBE, 0xEF], &mut out, false)
            .map(|p| p.status),
        Err(Error::BadMagic)
    );
    assert_eq!(
        d.decode(&[], &mut out, false).map(|p| p.status),
        Err(Error::BadMagic)
    );
}

#[test]
fn descriptor_is_available_once_the_header_is_parsed() {
    let f = frame(0x60 | 0x10 | 0x04, &[(true, b"hi".to_vec())], b"hi");
    let (want, header_len) = parse_frame_descriptor(&f).expect("parses");
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 8];
    assert!(d.descriptor().is_none());
    for i in 0..header_len {
        assert!(d.descriptor().is_none(), "before byte {i}");
        d.decode(&f[i..=i], &mut out, false).expect("ok");
    }
    assert_eq!(d.descriptor(), Some(&want));
    assert!(d.checksums_verified());
    // It stays available after the frame ends.
    d.decode(&f[header_len..], &mut out, true).expect("ok");
    assert_eq!(d.descriptor(), Some(&want));
}

#[test]
fn linked_checksummed_frame_streams_bytewise() {
    let frames = sample_frames();
    let f = &frames[3];
    let (desc, _) = parse_frame_descriptor(f).expect("parses");
    assert!(!desc.block_independence && desc.block_checksum && desc.content_checksum);
    assert_eq!(run(f, 1, 1), one_shot(f));
    assert_eq!(run(f, 1, 1).expect("ok"), b"abcdefghabcdefgh12345");
    // A flipped payload byte is caught the same way, streamed or not.
    let mut bad = f.clone();
    bad[12] ^= 1;
    assert_eq!(run(&bad, 1, 1), one_shot(&bad));
    assert_eq!(one_shot(&bad), Err(Error::BlockChecksumMismatch));
}

#[test]
fn concatenated_regular_frames_report_frame_done_each() {
    let frames = sample_frames();
    let input: Vec<u8> = frames.iter().flatten().copied().collect();
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = vec![0u8; 1 << 18];
    let mut pos = 0;
    let mut done = 0;
    let mut got = Vec::new();
    loop {
        let p = d.decode(&input[pos..], &mut out, true).expect("ok");
        pos += p.consumed;
        got.extend_from_slice(&out[..p.produced]);
        match p.status {
            Status::FrameDone => done += 1,
            Status::Done => break,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(done, frames.len());
    assert_eq!(got, one_shot(&input).expect("ok"));
}
