use oracle::block::{compress_bound, compress_default, compress_hc, decompress_safe};
use oracle::prng::Prng;

fn sample(n: usize) -> Vec<u8> {
    let mut p = Prng::new(11);
    let mut v = vec![0u8; n];
    p.fill(&mut v);
    // Make half of it compressible.
    for (i, b) in v.iter_mut().enumerate() {
        if i % 2 == 0 {
            *b = (i / 64) as u8;
        }
    }
    v
}

#[test]
fn roundtrip_default_and_hc() {
    let data = sample(10_000);
    let c = compress_default(&data);
    assert_eq!(decompress_safe(&c, data.len()).expect("ok"), data);
    let h = compress_hc(&data, 9);
    assert_eq!(decompress_safe(&h, data.len()).expect("ok"), data);
}

#[test]
fn decompress_safe_rejects_when_capacity_is_too_small() {
    let data = sample(5_000);
    let c = compress_default(&data);
    assert!(decompress_safe(&c, data.len() - 1).is_err());
}

#[test]
fn compress_bound_is_at_least_the_input_length() {
    for n in [0usize, 1, 100, 65_536, 1 << 20] {
        assert!(compress_bound(n) >= n);
    }
}

#[test]
fn empty_input_roundtrips() {
    let c = compress_default(&[]);
    assert_eq!(decompress_safe(&c, 0).expect("ok"), Vec::<u8>::new());
}
