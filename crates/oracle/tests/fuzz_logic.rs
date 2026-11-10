use oracle::config::Config;
use oracle::frame::{compress_frame, Prefs};
use oracle::fuzz_logic::{no_panic_block, no_panic_frame, no_panic_stream};
use oracle::malformed::malformed_dir;
use oracle::prng::Prng;

fn all_three(data: &[u8]) {
    no_panic_block(data);
    no_panic_frame(data);
    no_panic_stream(data);
}

#[test]
fn no_panic_logic_survives_the_malformed_corpus() {
    let dir = malformed_dir();
    if !dir.is_dir() {
        return;
    }
    let mut files = 0;
    for entry in std::fs::read_dir(&dir).expect("dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("file");
        all_three(&bytes);
        // Behind each mode byte too, so the option bits are exercised.
        for mode in [0u8, 1, 2, 4, 12, 15] {
            let mut with = vec![mode, mode];
            with.extend_from_slice(&bytes);
            all_three(&with);
        }
        files += 1;
    }
    assert!(files > 0);
}

#[test]
fn no_panic_logic_survives_random_buffers() {
    let cfg = Config::load().expect("config");
    let mut p = Prng::new(cfg.fuzz.seed);
    for _ in 0..cfg.conformance.random_block_cases {
        let len = p.below(4097) as usize;
        let mut buf = vec![0u8; len];
        p.fill(&mut buf);
        all_three(&buf);
    }
}

#[test]
fn no_panic_logic_survives_valid_frames_with_every_prefix_byte() {
    let data: Vec<u8> = (0..3000u32).map(|i| (i % 41) as u8).collect();
    let frame = compress_frame(&[&data], &Prefs::default());
    for first in 0..=255u8 {
        let mut with = vec![first, first.wrapping_mul(7)];
        with.extend_from_slice(&frame);
        all_three(&with);
        let mut one = vec![first];
        one.extend_from_slice(&frame);
        no_panic_frame(&one);
    }
}
