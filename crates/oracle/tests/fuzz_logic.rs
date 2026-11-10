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

use oracle::fuzz_logic::{diff_accept, diff_accept_in, diff_roundtrip, record_divergence};

#[test]
fn diff_roundtrip_logic_passes_on_fixed_inputs() {
    let cfg = Config::load().expect("config");
    let mut p = Prng::new(cfg.fuzz.seed);
    for _ in 0..256 {
        let len = p.below(65) as usize;
        let mut buf = vec![0u8; len];
        p.fill(&mut buf);
        diff_roundtrip(&buf);
    }
    // Longer targets, every preference byte.
    for b0 in 0..64u8 {
        diff_roundtrip(&[b0, b0, 0x40, 0x10, 1, 2, 3, 4, 5, 6, 7]);
    }
}

#[test]
fn diff_accept_logic_agrees_on_the_malformed_corpus() {
    let dir = malformed_dir();
    if !dir.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(&dir).expect("dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("file");
        diff_accept_in(&bytes, None);
    }
}

#[test]
fn diff_accept_logic_agrees_on_random_buffers() {
    let cfg = Config::load().expect("config");
    let mut p = Prng::new(cfg.fuzz.seed);
    for _ in 0..cfg.conformance.random_block_cases {
        let len = p.below(4097) as usize;
        let mut buf = vec![0u8; len];
        p.fill(&mut buf);
        diff_accept(&buf);
    }
}

fn temp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cleanlz4-div-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn a_divergence_is_written_to_the_directory_when_one_is_configured() {
    let d = temp("write");
    assert_eq!(record_divergence(&d, b"some input", 100), Ok(true));
    let name = format!("{}.bin", &oracle::manifest::sha256_hex(b"some input")[..16]);
    assert_eq!(std::fs::read(d.join(name)).expect("file"), b"some input");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_divergence_file_cap_is_honoured() {
    let d = temp("cap");
    for k in 0..5u8 {
        let wrote = record_divergence(&d, &[k], 3).expect("record");
        assert_eq!(wrote, k < 3, "input {k}");
    }
    let n = std::fs::read_dir(&d).expect("dir").count();
    assert_eq!(n, 3);
    let _ = std::fs::remove_dir_all(&d);
}
