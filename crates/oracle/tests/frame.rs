use oracle::config::Config;
use oracle::frame::{compress_frame, Prefs};
use oracle::synth::inputs;

#[test]
fn independent_frames_roundtrip_every_block_size() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed) {
        for id in 4u8..=7 {
            let prefs = Prefs {
                block_size_id: id,
                linked: false,
                ..Prefs::default()
            };
            let f = compress_frame(&[&data], &prefs);
            let out =
                cleanlz4::decompress_frame(&f).unwrap_or_else(|e| panic!("{name} id {id}: {e}"));
            assert!(out == data, "{name} id {id}: output differs");
        }
    }
}

#[test]
fn linked_frames_roundtrip_every_block_size() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed) {
        for id in 4u8..=7 {
            let prefs = Prefs {
                block_size_id: id,
                linked: true,
                ..Prefs::default()
            };
            let f = compress_frame(&[&data], &prefs);
            let out =
                cleanlz4::decompress_frame(&f).unwrap_or_else(|e| panic!("{name} id {id}: {e}"));
            assert!(out == data, "{name} id {id}: output differs");
        }
    }
}

#[test]
fn checksummed_frames_roundtrip() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed) {
        for id in [4u8, 7] {
            for linked in [false, true] {
                let prefs = Prefs {
                    block_size_id: id,
                    linked,
                    block_checksum: true,
                    content_checksum: true,
                    ..Prefs::default()
                };
                let f = compress_frame(&[&data], &prefs);
                let out = cleanlz4::decompress_frame(&f)
                    .unwrap_or_else(|e| panic!("{name} id {id} linked {linked}: {e}"));
                assert!(out == data, "{name} id {id} linked {linked}");
            }
        }
    }
}

#[test]
fn a_flipped_payload_byte_is_caught_by_the_block_checksum() {
    let data = oracle::synth::word_salad(9, 5000);
    let prefs = Prefs {
        block_checksum: true,
        ..Prefs::default()
    };
    let f = compress_frame(&[&data], &prefs);
    // A 7-byte header, a 4-byte size, then the payload.
    for offset in [11usize, 12, 40, 100] {
        let mut bad = f.clone();
        bad[offset] ^= 0x01;
        assert_eq!(
            cleanlz4::decompress_frame(&bad),
            Err(cleanlz4::Error::BlockChecksumMismatch),
            "offset {offset}"
        );
        assert!(matches!(
            oracle::frame::decompress_frames(&bad, 1 << 20),
            oracle::frame::Decoded::Rejected(_)
        ));
    }
}

#[test]
fn content_size_frames_roundtrip() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed) {
        for linked in [false, true] {
            let prefs = Prefs {
                block_size_id: 4,
                linked,
                content_size: Some(data.len() as u64),
                ..Prefs::default()
            };
            let f = compress_frame(&[&data], &prefs);
            let out = cleanlz4::decompress_frame(&f)
                .unwrap_or_else(|e| panic!("{name} linked {linked}: {e}"));
            assert!(out == data, "{name} linked {linked}");
        }
    }
}

#[test]
fn concatenated_oracle_frames_decode() {
    let cfg = Config::load().expect("config");
    let all = inputs(cfg.fuzz.seed);
    let mut input = Vec::new();
    let mut want = Vec::new();
    let mut frames = 0usize;
    for (i, (_, data)) in all.iter().enumerate().take(8) {
        let prefs = Prefs {
            block_size_id: 4 + (i % 4) as u8,
            linked: i % 2 == 0,
            content_checksum: i % 3 == 0,
            block_checksum: i % 5 == 0,
            ..Prefs::default()
        };
        input.extend(compress_frame(&[data], &prefs));
        want.extend_from_slice(data);
        frames += 1;
    }
    let out = cleanlz4::decompress_frame_with(&input, &cleanlz4::DecodeOptions::default())
        .expect("decodes");
    assert!(out.data == want);
    assert_eq!(out.descriptors.len(), frames);
    assert_eq!(out.consumed, input.len());
    // The oracle agrees on the same bytes.
    match oracle::frame::decompress_frames(&input, want.len() + 1) {
        oracle::frame::Decoded::Accepted(v) => assert!(v == want),
        other => panic!("oracle: {other:?}"),
    }
}
