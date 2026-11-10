use oracle::block::compress_default;
use oracle::cli::{cli_version, compress_legacy};
use oracle::frame::{compress_frame, Prefs};
use oracle::synth::word_salad;

const LEGACY_MAGIC: [u8; 4] = [0x02, 0x21, 0x4C, 0x18];

#[test]
fn hand_built_legacy_frame_from_oracle_blocks_decodes() {
    let data = word_salad(3, 300_000);
    let mut f = LEGACY_MAGIC.to_vec();
    for chunk in data.chunks(100_000) {
        let block = compress_default(chunk);
        f.extend_from_slice(&(block.len() as u32).to_le_bytes());
        f.extend_from_slice(&block);
    }
    let out = cleanlz4::decompress_frame(&f).expect("decodes");
    assert!(out == data);
}

#[test]
fn cli_legacy_frame_decodes() {
    assert!(!cli_version().is_empty());
    let data = word_salad(4, 20 << 20);
    let f = compress_legacy(&data);
    assert_eq!(f[..4], LEGACY_MAGIC);
    let out =
        cleanlz4::decompress_frame_with(&f, &cleanlz4::DecodeOptions::default()).expect("decodes");
    assert!(out.data == data, "legacy output differs");
    assert_eq!(out.kinds, [cleanlz4::FrameKind::Legacy]);
}

#[test]
fn legacy_then_regular_frame_decodes() {
    let a = word_salad(5, 50_000);
    let b = word_salad(6, 60_000);
    let mut input = compress_legacy(&a);
    input.extend(compress_frame(&[&b], &Prefs::default()));
    let out = cleanlz4::decompress_frame_with(&input, &cleanlz4::DecodeOptions::default())
        .expect("decodes");
    let mut want = a.clone();
    want.extend(&b);
    assert!(out.data == want);
    assert_eq!(
        out.kinds,
        [cleanlz4::FrameKind::Legacy, cleanlz4::FrameKind::Regular]
    );
}
