use cleanlz4::xxh32::{xxh32, Xxh32};
use oracle::config::Config;
use oracle::frame::{compress_frame, content_checksum_of, Prefs};
use oracle::prng::Prng;

fn random(n: usize, seed: u64) -> Vec<u8> {
    let mut v = vec![0u8; n];
    Prng::new(seed).fill(&mut v);
    v
}

#[test]
fn matches_oracle_content_checksum_for_lengths_0_to_300() {
    let seed = Config::load().expect("config").fuzz.seed;
    let all = random(300, seed);
    for n in 0..=300usize {
        assert_eq!(
            xxh32(&all[..n], 0),
            content_checksum_of(&all[..n]),
            "len {n}"
        );
    }
}

#[test]
fn matches_oracle_for_large_random_inputs() {
    let seed = Config::load().expect("config").fuzz.seed;
    for (i, n) in [1024usize, 4099, 65_536, 100_003, (1 << 20) + 3]
        .into_iter()
        .enumerate()
    {
        let d = random(n, seed + i as u64);
        assert_eq!(xxh32(&d, 0), content_checksum_of(&d), "len {n}");
    }
}

#[test]
fn streaming_in_odd_chunks_matches_oracle() {
    let seed = Config::load().expect("config").fuzz.seed;
    let d = random(50_000, seed);
    let want = content_checksum_of(&d);
    for chunk in [1usize, 3, 7, 15, 17, 100, 4093] {
        let mut h = Xxh32::new(0);
        for c in d.chunks(chunk) {
            h.update(c);
        }
        assert_eq!(h.digest(), want, "chunk {chunk}");
    }
}

#[test]
fn header_checksum_byte_matches_oracle_frames() {
    let seed = Config::load().expect("config").fuzz.seed;
    let d = random(2000, seed);
    for prefs in [
        Prefs::default(),
        Prefs {
            block_size_id: 7,
            linked: true,
            content_checksum: true,
            block_checksum: true,
            ..Prefs::default()
        },
        Prefs {
            block_size_id: 4,
            ..Prefs::default()
        },
    ] {
        let f = compress_frame(&[&d], &prefs);
        // Magic (4 bytes), FLG, BD, then the checksum byte: a 7-byte header.
        let descriptor = &f[4..6];
        assert_eq!(f[6], ((xxh32(descriptor, 0) >> 8) & 0xFF) as u8);
    }
}

#[test]
fn block_checksum_matches_oracle_frames() {
    let seed = Config::load().expect("config").fuzz.seed;
    let d = random(5000, seed);
    let prefs = Prefs {
        block_checksum: true,
        ..Prefs::default()
    };
    let f = compress_frame(&[&d], &prefs);
    let size_word = u32::from_le_bytes([f[7], f[8], f[9], f[10]]);
    let len = (size_word & 0x7FFF_FFFF) as usize;
    let data = &f[11..11 + len];
    let stored = u32::from_le_bytes([f[11 + len], f[12 + len], f[13 + len], f[14 + len]]);
    assert_eq!(xxh32(data, 0), stored);
}
