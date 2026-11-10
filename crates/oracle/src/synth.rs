//! Deterministic synthetic inputs (SPEC section 6.3). Every byte is a function
//! of the seed through `Prng`.

use crate::prng::Prng;

fn random_bytes(p: &mut Prng, n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    p.fill(&mut v);
    v
}

/// Compressible text of exactly `len` bytes built from a small vocabulary.
pub fn word_salad(seed: u64, len: usize) -> Vec<u8> {
    const WORDS: [&str; 24] = [
        "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "alpha", "beta", "gamma",
        "delta", "stream", "block", "frame", "offset", "match", "literal", "token", "window",
        "history", "output", "decoder", "checksum",
    ];
    let mut p = Prng::new(seed);
    let mut out = Vec::with_capacity(len + 16);
    while out.len() < len {
        let w = WORDS[p.below(WORDS.len() as u64) as usize];
        out.extend_from_slice(w.as_bytes());
        out.push(if p.below(12) == 0 { b'\n' } else { b' ' });
    }
    out.truncate(len);
    out
}

fn repeat_to(unit: &[u8], len: usize) -> Vec<u8> {
    unit.iter().copied().cycle().take(len).collect()
}

fn window_boundary(p: &mut Prng) -> Vec<u8> {
    let marker = random_bytes(p, 512);
    let mut out = marker.clone();
    for distance in [65_534usize, 65_535, 65_536, 65_537] {
        // The next marker starts `distance` bytes after the previous one.
        let filler = random_bytes(p, distance - marker.len());
        out.extend_from_slice(&filler);
        out.extend_from_slice(&marker);
    }
    out
}

fn min_match(p: &mut Prng) -> Vec<u8> {
    let unit = random_bytes(p, 4);
    let mut out = Vec::new();
    for _ in 0..8_000 {
        let n = 1 + p.below(8) as usize;
        out.extend(random_bytes(p, n));
        out.extend_from_slice(&unit);
    }
    out
}

/// The named inputs, deterministic in `seed`.
pub fn inputs(seed: u64) -> Vec<(String, Vec<u8>)> {
    let mut p = Prng::new(seed);
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), Vec::new()));
    v.push(("one-byte".into(), random_bytes(&mut p, 1)));
    for n in 2..=12usize {
        v.push((format!("tiny-{n}"), random_bytes(&mut p, n)));
    }
    v.push(("incompressible-64k".into(), random_bytes(&mut p, 65_553)));
    v.push(("incompressible-1m".into(), random_bytes(&mut p, 1 << 20)));
    v.push(("zeros-1m".into(), vec![0u8; 1 << 20]));
    v.push((
        "repetitive-text".into(),
        repeat_to(b"The quick brown fox jumps over 13 dogs", 300_000),
    ));
    v.push(("window-boundary".into(), window_boundary(&mut p)));
    v.push(("min-match".into(), min_match(&mut p)));
    let unit = random_bytes(&mut p, 300);
    v.push(("long-match".into(), repeat_to(&unit, 2 << 20)));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_deterministic_and_named_uniquely() {
        let a = inputs(1);
        let b = inputs(1);
        assert_eq!(a, b);
        assert_ne!(inputs(2)[1].1, a[1].1);
        let mut names: Vec<&String> = a.iter().map(|(n, _)| n).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
        let get = |n: &str| a.iter().find(|(m, _)| m == n).map(|(_, d)| d.len());
        assert_eq!(get("empty"), Some(0));
        assert_eq!(get("incompressible-64k"), Some(65_553));
        assert_eq!(get("repetitive-text"), Some(300_000));
        assert_eq!(get("long-match"), Some(2 << 20));
        assert_eq!(word_salad(3, 1000).len(), 1000);
    }
}
