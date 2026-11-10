use cleanlz4::decompress_block_with_dict;
use oracle::block::decompress_safe;
use oracle::cli::decompress_block_with_dict as oracle_decode;
use oracle::config::Config;
use oracle::dict::{dict_block, DictBlock};
use oracle::synth::word_salad;

const CAP: usize = 4 << 20;

/// A dictionary and data that shares text with it, so matches reach back.
fn sample(seed: u64, size: usize) -> (Vec<u8>, Vec<u8>) {
    let dict = word_salad(seed, size);
    let mut data = dict[size - 3000..].to_vec();
    data.extend_from_slice(&word_salad(seed + 1, 2000));
    data.extend_from_slice(&dict[size / 2..size / 2 + 1500]);
    (dict, data)
}

fn make(size: usize, seed: u64) -> (Vec<u8>, Vec<u8>, DictBlock) {
    let (dict, data) = sample(seed, size);
    let b = dict_block(&dict, &data).expect("compressed second block");
    (dict, data, b)
}

#[test]
fn dict_block_decodes_with_the_oracle_using_the_dictionary() {
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        let (dict, data, b) = make(*size as usize, cfg.fuzz.seed);
        assert_eq!(oracle_decode(&b.block, &dict).expect("oracle"), data);
    }
}

#[test]
fn cleanlz4_matches_the_oracle_for_every_configured_dictionary_size() {
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        for seed in 0..4 {
            let (dict, data, b) = make(*size as usize, cfg.fuzz.seed + seed);
            let ours = decompress_block_with_dict(&b.block, &dict, data.len()).expect("ours");
            assert_eq!(ours, data, "size {size}");
        }
    }
}

#[test]
fn at_least_one_block_per_size_is_dictionary_dependent() {
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        let any = (0..8).any(|s| make(*size as usize, cfg.fuzz.seed + s).2.dict_dependent);
        assert!(any, "size {size}");
        let (_, data, b) = make(*size as usize, cfg.fuzz.seed);
        if b.dict_dependent {
            let plain = decompress_safe(&b.block, data.len());
            assert!(plain.map(|p| p != data).unwrap_or(true));
        }
    }
}

#[test]
fn wrong_dictionary_never_panics_and_agrees_with_the_oracle() {
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        let (dict, _data, b) = make(*size as usize, cfg.fuzz.seed);
        let wrongs = [
            word_salad(999, dict.len()),
            dict[..dict.len() / 2].to_vec(),
            dict[..17].to_vec(),
            dict.iter().rev().copied().collect::<Vec<u8>>(),
        ];
        for wrong in &wrongs {
            let ours = decompress_block_with_dict(&b.block, wrong, CAP);
            let theirs = oracle_decode(&b.block, wrong);
            match (ours, theirs) {
                (Ok(a), Ok(t)) => assert_eq!(a, t, "size {size}, wrong dict {}", wrong.len()),
                (Err(_), Err(_)) => {}
                (a, t) => panic!(
                    "size {size}, wrong dict {}: ours ok={} oracle ok={}",
                    wrong.len(),
                    a.is_ok(),
                    t.is_ok()
                ),
            }
        }
    }
}

#[test]
fn a_block_sharing_nothing_with_the_dictionary_is_not_dict_dependent() {
    let cfg = Config::load().expect("config");
    // ASCII dictionary, high-byte data: no four-byte match can reach into it.
    let data: Vec<u8> = (0..16 * 1024).map(|i| 0x80 + (i % 0x30) as u8).collect();
    assert!(data.iter().all(|b| *b >= 0x80));
    for size in &cfg.dict.sizes_bytes {
        let size = *size as usize;
        let dict = word_salad(cfg.fuzz.seed, size);
        assert!(dict.iter().all(|b| *b < 0x80), "size {size}");
        let b = dict_block(&dict, &data).expect("compressed second block");
        assert!(!b.dict_dependent, "size {size}: labelled dict_dependent");
        assert_eq!(
            decompress_safe(&b.block, data.len()).expect("oracle decodes"),
            data,
            "size {size}: oracle output"
        );
        assert_eq!(
            cleanlz4::decompress_block(&b.block, data.len()).expect("cleanlz4 decodes"),
            data,
            "size {size}: cleanlz4 output"
        );
    }
}
