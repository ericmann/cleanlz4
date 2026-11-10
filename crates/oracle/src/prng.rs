//! A small deterministic generator (SplitMix64).

/// Deterministic 64-bit generator.
#[derive(Clone, Debug)]
pub struct Prng {
    state: u64,
}

impl Prng {
    /// Create a generator from a seed.
    pub fn new(seed: u64) -> Self {
        Prng { state: seed }
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n`; returns 0 when `n` is 0.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        self.next_u64() % n
    }

    /// Fill a buffer with random bytes.
    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            for (d, s) in chunk.iter_mut().zip(bytes.iter()) {
                *d = *s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Prng::new(7);
        let mut b = Prng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = Prng::new(8);
        assert_ne!(Prng::new(7).next_u64(), c.next_u64());
        let mut x = [0u8; 13];
        let mut y = [0u8; 13];
        Prng::new(3).fill(&mut x);
        Prng::new(3).fill(&mut y);
        assert_eq!(x, y);
        assert!(Prng::new(1).below(10) < 10);
    }
}
