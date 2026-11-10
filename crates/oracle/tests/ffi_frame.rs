use oracle::frame::{compress_frame, content_checksum_of, decompress_frames, Decoded, Prefs};
use oracle::prng::Prng;

fn sample(n: usize, seed: u64) -> Vec<u8> {
    let mut p = Prng::new(seed);
    let mut v = vec![0u8; n];
    p.fill(&mut v);
    for (i, b) in v.iter_mut().enumerate() {
        if i % 3 != 0 {
            *b = (i / 128) as u8;
        }
    }
    v
}

fn accepted(d: Decoded) -> Vec<u8> {
    match d {
        Decoded::Accepted(v) => v,
        other => panic!("expected accepted, got {other:?}"),
    }
}

#[test]
fn frame_roundtrip_every_block_size() {
    let data = sample(300_000, 1);
    for id in [0u8, 4, 5, 6, 7] {
        let prefs = Prefs {
            block_size_id: id,
            ..Prefs::default()
        };
        let f = compress_frame(&[&data], &prefs);
        assert_eq!(accepted(decompress_frames(&f, 1 << 24)), data, "id {id}");
    }
}

#[test]
fn frame_roundtrip_linked_with_both_checksums() {
    let data = sample(200_000, 2);
    let prefs = Prefs {
        block_size_id: 4,
        linked: true,
        content_checksum: true,
        block_checksum: true,
        ..Prefs::default()
    };
    let a = &data[..70_000];
    let b = &data[70_000..];
    let f = compress_frame(&[a, b], &prefs);
    assert_eq!(accepted(decompress_frames(&f, 1 << 24)), data);
}

#[test]
fn content_size_adds_eight_header_bytes() {
    let data = sample(1000, 3);
    let plain = compress_frame(&[&data], &Prefs::default());
    let sized = compress_frame(
        &[&data],
        &Prefs {
            content_size: Some(data.len() as u64),
            ..Prefs::default()
        },
    );
    assert_eq!(sized.len(), plain.len() + 8);
    assert_eq!(accepted(decompress_frames(&sized, 1 << 20)), data);
}

#[test]
fn decompress_frames_handles_two_concatenated_frames() {
    let a = sample(5000, 4);
    let b = sample(7000, 5);
    let mut both = compress_frame(&[&a], &Prefs::default());
    both.extend(compress_frame(&[&b], &Prefs::default()));
    let mut want = a.clone();
    want.extend(&b);
    assert_eq!(accepted(decompress_frames(&both, 1 << 20)), want);
}

#[test]
fn decompress_frames_rejects_every_truncation_of_a_small_frame() {
    let data = sample(300, 6);
    let prefs = Prefs {
        content_checksum: true,
        ..Prefs::default()
    };
    let f = compress_frame(&[&data], &prefs);
    for cut in 1..f.len() {
        match decompress_frames(&f[..cut], 1 << 20) {
            Decoded::Rejected(_) => {}
            other => panic!("cut {cut} of {}: {other:?}", f.len()),
        }
    }
}

#[test]
fn empty_input_is_accepted_as_empty() {
    assert_eq!(decompress_frames(&[], 10), Decoded::Accepted(vec![]));
}

#[test]
fn content_checksum_of_changes_with_the_input() {
    assert_ne!(content_checksum_of(b"abc"), content_checksum_of(b"abd"));
    assert_eq!(content_checksum_of(b"abc"), content_checksum_of(b"abc"));
}
