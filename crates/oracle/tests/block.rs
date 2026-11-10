use cleanlz4::{decompress_block, Error};
use oracle::block::{compress_default, compress_hc, decompress_safe};
use oracle::config::Config;
use oracle::prng::Prng;
use oracle::synth::{inputs, word_salad};

#[test]
fn synthetic_inputs_roundtrip_default_and_hc() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed) {
        let mut blocks = vec![("default".to_string(), compress_default(&data))];
        for level in &cfg.conformance.hc_levels {
            blocks.push((format!("hc{level}"), compress_hc(&data, *level)));
        }
        for (mode, block) in blocks {
            let ours = decompress_block(&block, data.len())
                .unwrap_or_else(|e| panic!("{name} {mode}: {e}"));
            assert_eq!(ours, data, "{name} {mode}");
            assert_eq!(decompress_safe(&block, data.len()).expect("oracle"), data);
        }
    }
}

#[test]
fn random_blocks_roundtrip() {
    let cfg = Config::load().expect("config");
    let mut p = Prng::new(cfg.fuzz.seed);
    for case in 0..cfg.conformance.random_block_cases {
        let len = p.below(70_001) as usize;
        let data = match p.below(3) {
            0 => {
                let mut v = vec![0u8; len];
                p.fill(&mut v);
                v
            }
            1 => word_salad(p.next_u64(), len),
            _ => {
                // Mixed: alternating random and repeated stretches.
                let mut v = Vec::with_capacity(len);
                while v.len() < len {
                    let n = 1 + p.below(200) as usize;
                    if p.below(2) == 0 {
                        let mut r = vec![0u8; n];
                        p.fill(&mut r);
                        v.extend(r);
                    } else {
                        let b = p.next_u64() as u8;
                        v.extend(std::iter::repeat_n(b, n));
                    }
                }
                v.truncate(len);
                v
            }
        };
        let block = if case % 2 == 0 {
            compress_default(&data)
        } else {
            compress_hc(&data, 9)
        };
        assert_eq!(
            decompress_block(&block, data.len()).expect("decode"),
            data,
            "case {case} len {len}"
        );
    }
}

#[test]
fn exact_capacity_is_enough_and_one_less_is_rejected() {
    let data = word_salad(5, 20_000);
    let block = compress_default(&data);
    assert_eq!(decompress_block(&block, data.len()).expect("exact"), data);
    assert_eq!(
        decompress_block(&block, data.len() - 1),
        Err(Error::OutputLimitExceeded)
    );
    assert!(decompress_safe(&block, data.len()).is_ok());
    assert!(decompress_safe(&block, data.len() - 1).is_err());
}
