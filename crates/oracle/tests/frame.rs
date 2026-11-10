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
