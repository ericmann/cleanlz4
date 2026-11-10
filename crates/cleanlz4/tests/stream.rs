//! Streaming tests: every result is compared with the one-shot decoder.

use cleanlz4::xxh32::xxh32;
use cleanlz4::{
    decompress_frame_with, parse_frame_descriptor, DecodeOptions, Decoder, Dictionary, Error,
    Status, TrailingData,
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
    for _ in 0..(20 * input.len() + 100) {
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
    panic!("no progress: {input:02x?}");
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

// Skippable, legacy and concatenated frames, dictionaries, trailing data.

fn skippable(low: u8, data: &[u8]) -> Vec<u8> {
    let mut v = (0x184D_2A50u32 | u32::from(low)).to_le_bytes().to_vec();
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(data);
    v
}

fn legacy(blocks: &[Vec<u8>], zero_end: bool) -> Vec<u8> {
    let mut v = 0x184C_2102u32.to_le_bytes().to_vec();
    for b in blocks {
        v.extend_from_slice(&(b.len() as u32).to_le_bytes());
        v.extend_from_slice(b);
    }
    if zero_end {
        v.extend_from_slice(&[0; 4]);
    }
    v
}

fn run_with(
    options: DecodeOptions,
    input: &[u8],
    in_chunk: usize,
) -> Result<(Vec<u8>, usize, usize), Error> {
    let mut d = Decoder::new(options);
    let mut out = Vec::new();
    let mut buf = vec![0u8; 4096];
    let mut pos = 0usize;
    let mut frames = 0usize;
    for _ in 0..10_000_000 {
        let end = (pos + in_chunk).min(input.len());
        let p = d.decode(&input[pos..end], &mut buf, end == input.len())?;
        pos += p.consumed;
        out.extend_from_slice(&buf[..p.produced]);
        match p.status {
            Status::FrameDone => frames += 1,
            Status::Done => return Ok((out, pos, frames)),
            _ => {}
        }
    }
    panic!("no progress");
}

#[test]
fn skippable_user_data_is_skipped_without_buffering() {
    let mib = 1usize << 20;
    let declared = 10 * mib;
    let mut head = (0x184D_2A5Fu32).to_le_bytes().to_vec();
    head.extend_from_slice(&(declared as u32).to_le_bytes());
    let tail = frame(0x60, &[(true, b"after".to_vec())], b"after");
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 64];
    let p = d.decode(&head, &mut out, false).expect("ok");
    assert_eq!((p.consumed, p.status), (8, Status::NeedsInput));
    let piece = vec![0xA5u8; mib];
    for k in 0..10 {
        let p = d.decode(&piece, &mut out, false).expect("ok");
        assert_eq!(p.consumed, mib);
        assert_eq!(p.produced, 0);
        let want = if k == 9 {
            Status::FrameDone
        } else {
            Status::NeedsInput
        };
        assert_eq!(p.status, want);
    }
    let p = d.decode(&tail, &mut out, true).expect("ok");
    assert_eq!(&out[..p.produced], b"after");
}

#[test]
fn legacy_frame_ends_at_end_of_input() {
    let lf = legacy(&[literal_block(b"abc"), literal_block(b"def")], false);
    let mut d = Decoder::new(DecodeOptions::default());
    let mut out = [0u8; 64];
    let p = d.decode(&lf, &mut out, false).expect("ok");
    // At a block boundary with nothing buffered and more input to come.
    assert_eq!(p.status, Status::NeedsInput);
    assert_eq!(&out[..p.produced], b"abcdef");
    let p = d.decode(&[], &mut out, true).expect("ok");
    assert_eq!(p.status, Status::FrameDone);
    let p = d.decode(&[], &mut out, true).expect("ok");
    assert_eq!(p.status, Status::Done);
    // Bytewise gives the same bytes as the one-shot decoder.
    assert_eq!(run(&lf, 1, 3), one_shot(&lf));
    // A zero-terminated one too.
    let lz = legacy(&[literal_block(b"abc")], true);
    assert_eq!(run(&lz, 1, 3), Ok(b"abc".to_vec()));
}

#[test]
fn legacy_frame_then_regular_frame() {
    let regular = frame(0x60, &[(true, b"tail".to_vec())], b"tail");
    let input = [legacy(&[literal_block(b"head")], false), regular].concat();
    let (out, pos, frames) = run_with(DecodeOptions::default(), &input, 1).expect("ok");
    assert_eq!(out, b"headtail");
    assert_eq!((pos, frames), (input.len(), 2));
    assert_eq!(Ok(out), one_shot(&input));
}

#[test]
fn concatenated_frames_report_frame_done_each() {
    let r = frame(0x60, &[(true, b"one".to_vec())], b"one");
    let input = [
        r.clone(),
        skippable(3, b"skip me"),
        legacy(&[literal_block(b"two")], true),
        skippable(0, &[]),
        r,
    ]
    .concat();
    for chunk in [1usize, 5, input.len()] {
        let (out, pos, frames) = run_with(DecodeOptions::default(), &input, chunk).expect("ok");
        assert_eq!(out, b"onetwoone");
        assert_eq!((pos, frames), (input.len(), 5), "chunk {chunk}");
    }
}

fn dict_options(dict: &[u8], id: Option<u32>) -> DecodeOptions {
    DecodeOptions {
        dictionary: Some(Dictionary {
            bytes: dict.to_vec(),
            id,
        }),
        ..DecodeOptions::default()
    }
}

/// A frame whose one compressed block copies eight dictionary bytes (offset 8).
fn dict_frame(flg: u8, id: Option<u32>) -> Vec<u8> {
    let mut m = vec![0x04, 8, 0];
    m.extend(literal_block(b"12345"));
    let mut d = vec![flg | u8::from(id.is_some()), 4 << 4];
    if let Some(id) = id {
        d.extend_from_slice(&id.to_le_bytes());
    }
    let mut f = MAGIC.to_vec();
    f.extend_from_slice(&d);
    f.push(((xxh32(&d, 0) >> 8) & 0xFF) as u8);
    f.extend_from_slice(&(m.len() as u32).to_le_bytes());
    f.extend_from_slice(&m);
    f.extend_from_slice(&[0; 4]);
    f
}

#[test]
fn dictionary_frames_stream() {
    for flg in [0x60u8, 0x40] {
        for id in [None, Some(9u32)] {
            let f = dict_frame(flg, id);
            let want = Ok((b"ABCDEFGH12345".to_vec(), f.len(), 1));
            assert_eq!(
                run_with(dict_options(b"ABCDEFGH", id), &f, 1),
                want,
                "{flg:#x} {id:?}"
            );
            assert_eq!(
                run_with(dict_options(b"ABCDEFGH", None), &f, 3),
                want,
                "{flg:#x} {id:?}"
            );
        }
    }
    let f = dict_frame(0x60, Some(9));
    assert_eq!(
        run_with(dict_options(b"ABCDEFGH", Some(10)), &f, 1),
        Err(Error::DictionaryMismatch)
    );
}

#[test]
fn dict_id_without_a_dictionary_is_dictionary_required_when_streaming() {
    let f = dict_frame(0x60, Some(9));
    assert_eq!(
        run_with(DecodeOptions::default(), &f, 1),
        Err(Error::DictionaryRequired)
    );
}

#[test]
fn return_remainder_stops_before_the_trailing_bytes() {
    let f = frame(0x60, &[(true, b"abc".to_vec())], b"abc");
    let input = [f.clone(), vec![0xAB; 6]].concat();
    for chunk in [1usize, 4, input.len()] {
        let options = DecodeOptions {
            trailing_data: TrailingData::ReturnRemainder,
            ..DecodeOptions::default()
        };
        let mut d = Decoder::new(options);
        let mut out = [0u8; 16];
        let mut pos = 0;
        let mut got = Vec::new();
        loop {
            let end = (pos + chunk).min(input.len());
            let p = d
                .decode(&input[pos..end], &mut out, end == input.len())
                .expect("ok");
            pos += p.consumed;
            got.extend_from_slice(&out[..p.produced]);
            if p.status == Status::Done {
                break;
            }
        }
        assert_eq!(got, b"abc");
        assert_eq!(pos, f.len(), "chunk {chunk}");
    }
}

#[test]
fn trailing_garbage_is_trailing_data() {
    let f = frame(0x60, &[(true, b"abc".to_vec())], b"abc");
    let input = [f, vec![0xAB; 6]].concat();
    assert_eq!(run(&input, 1, 8), Err(Error::TrailingData));
    assert_eq!(run(&input, input.len(), 8), Err(Error::TrailingData));
    assert_eq!(one_shot(&input), Err(Error::TrailingData));
}

#[test]
fn every_one_shot_test_frame_matches_when_streamed_bytewise() {
    let mut inputs = sample_frames();
    inputs.push(skippable(2, b"xyz"));
    inputs.push(legacy(
        &[literal_block(b"abc"), literal_block(b"def")],
        false,
    ));
    inputs.push(legacy(&[literal_block(b"abc")], true));
    inputs.push([inputs[1].clone(), skippable(1, b"q"), inputs[2].clone()].concat());
    inputs.push([legacy(&[literal_block(b"abc")], false), inputs[1].clone()].concat());
    inputs.push([inputs[1].clone(), vec![0xAB; 3]].concat());
    let mut cases = Vec::new();
    for f in &inputs {
        cases.push(f.clone());
        // Large frames are sampled: every offset of the small ones is covered.
        let step = (f.len() / 150).max(1);
        for cut in (0..f.len()).step_by(step) {
            cases.push(f[..cut].to_vec());
        }
        for at in (0..f.len()).step_by(step) {
            let mut b = f.clone();
            b[at] ^= 0xFF;
            cases.push(b);
        }
    }
    for c in &cases {
        let streamed = run(c, 1, 64);
        assert_eq!(streamed, one_shot(c), "{c:02x?}");
    }
}

/// Stream `input` under `ReturnRemainder` with the given piece sizes (cycled);
/// returns the decoded bytes and the bytes left to the caller: the decoder's
/// `remainder` followed by all input not consumed.
fn stream_remainder(input: &[u8], pieces: &[usize]) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let options = DecodeOptions {
        trailing_data: TrailingData::ReturnRemainder,
        ..DecodeOptions::default()
    };
    let mut d = Decoder::new(options);
    let mut out = Vec::new();
    let mut buf = [0u8; 64];
    let mut pos = 0usize;
    // Unconsumed bytes are re-presented with the next piece, as a caller would.
    for k in 0..(10 * input.len() + 100) {
        let want = pieces[k % pieces.len()].max(1);
        let end = (pos + want).min(input.len());
        let p = d.decode(&input[pos..end], &mut buf, end == input.len())?;
        pos += p.consumed;
        out.extend_from_slice(&buf[..p.produced]);
        if p.status == Status::Done {
            let mut left = d.remainder().to_vec();
            left.extend_from_slice(&input[pos..]);
            return Ok((out, left));
        }
    }
    panic!("no progress");
}

#[test]
fn return_remainder_leaves_a_trailing_magic_prefix_unconsumed() {
    let f = frame(0x60, &[(true, b"abc".to_vec())], b"abc");
    let tails: [&[u8]; 3] = [
        &[0x04, 0x22, 0x4D, 0x00, 0xAA], // a regular-magic prefix, then a mismatch
        &[0x50, 0x2A, 0x4D, 0x00],       // a skippable-magic prefix
        &[0x02, 0x21, 0x4C, 0x00],       // a legacy-magic prefix
    ];
    let options = DecodeOptions {
        trailing_data: TrailingData::ReturnRemainder,
        ..DecodeOptions::default()
    };
    let mut seed = 12345u64;
    for tail in tails {
        let input = [f.clone(), tail.to_vec()].concat();
        let one = decompress_frame_with(&input, &options).expect("one-shot");
        assert_eq!(one.consumed, f.len());
        let want_left = &input[one.consumed..];
        let mut splits: Vec<Vec<usize>> = vec![vec![input.len()], vec![1], vec![2], vec![3]];
        for _ in 0..20 {
            let mut v = Vec::new();
            for _ in 0..8 {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                v.push(1 + (seed >> 33) as usize % 9);
            }
            splits.push(v);
        }
        for pieces in splits {
            let (out, left) = stream_remainder(&input, &pieces).expect("streamed");
            assert_eq!(out, one.data, "{tail:02x?} {pieces:?}");
            assert_eq!(left, want_left, "{tail:02x?} {pieces:?}");
        }
    }
    // A lone prefix byte at the end of the input is TruncatedInput on both paths.
    let input = [f.clone(), vec![0x04]].concat();
    assert_eq!(
        decompress_frame_with(&input, &options).map(|o| o.data),
        Err(Error::TruncatedInput)
    );
    for pieces in [vec![input.len()], vec![1]] {
        assert_eq!(
            stream_remainder(&input, &pieces).map(|r| r.0),
            Err(Error::TruncatedInput)
        );
    }
    // Under the default policy the tail is TrailingData.
    let input = [f, tails[0].to_vec()].concat();
    assert_eq!(run(&input, 1, 8), Err(Error::TrailingData));
}

#[test]
fn skippable_frame_first_streams_like_one_shot() {
    let regular = frame(0x60, &[(true, b"hello".to_vec())], b"hello");
    for low in [0u8, 7, 15] {
        let input = [skippable(low, b"user data"), regular.clone()].concat();
        let want = one_shot(&input);
        assert_eq!(want, Ok(b"hello".to_vec()));
        for (in_chunk, out_size) in [(1, 1), (3, 2), (input.len(), 64)] {
            assert_eq!(run(&input, in_chunk, out_size), want, "{low} {in_chunk}");
        }
        let (out, pos, frames) = run_with(DecodeOptions::default(), &input, 1).expect("ok");
        assert_eq!((out, pos, frames), (b"hello".to_vec(), input.len(), 2));
    }
    // A skippable frame alone decodes to nothing; its user data may be empty.
    for input in [skippable(1, b"xyz"), skippable(2, &[])] {
        assert_eq!(run(&input, 1, 8), Ok(Vec::new()));
        assert_eq!(one_shot(&input), Ok(Vec::new()));
    }
}
