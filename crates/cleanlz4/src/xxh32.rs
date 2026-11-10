//! XXH32, one-shot and streaming (xxHash spec, "XXH32 Algorithm Description").

const PRIME32_1: u32 = 0x9E37_79B1;
const PRIME32_2: u32 = 0x85EB_CA77;
const PRIME32_3: u32 = 0xC2B2_AE3D;
const PRIME32_4: u32 = 0x27D4_EB2F;
const PRIME32_5: u32 = 0x1656_67B1;

/// Number of bytes in a stripe (Step 2).
const STRIPE: usize = 16;

/// Step 2: one round of one accumulator.
fn round(acc: u32, lane: u32) -> u32 {
    acc.wrapping_add(lane.wrapping_mul(PRIME32_2))
        .rotate_left(13)
        .wrapping_mul(PRIME32_1)
}

/// Step 2: consume one full stripe, one lane per accumulator.
fn consume_stripe(acc: &mut [u32; 4], stripe: &[u8; STRIPE]) {
    let (lanes, _) = stripe.as_chunks::<4>();
    for (a, lane) in acc.iter_mut().zip(lanes.iter()) {
        // Lanes are read little-endian ("Operation notations").
        *a = round(*a, u32::from_le_bytes(*lane));
    }
}

/// Steps 5 to 7 over the remaining bytes, starting from an accumulator that
/// already holds steps 1 to 4.
fn finish(mut acc: u32, rest: &[u8]) -> u32 {
    // Step 5: consume remaining input.
    let (words, tail) = rest.as_chunks::<4>();
    for w in words {
        acc = acc
            .wrapping_add(u32::from_le_bytes(*w).wrapping_mul(PRIME32_3))
            .rotate_left(17)
            .wrapping_mul(PRIME32_4);
    }
    for b in tail {
        acc = acc
            .wrapping_add(u32::from(*b).wrapping_mul(PRIME32_5))
            .rotate_left(11)
            .wrapping_mul(PRIME32_1);
    }
    // Step 6: final mix (avalanche).
    acc ^= acc >> 15;
    acc = acc.wrapping_mul(PRIME32_2);
    acc ^= acc >> 13;
    acc = acc.wrapping_mul(PRIME32_3);
    acc ^= acc >> 16;
    // Step 7: output.
    acc
}

/// XXH32 of `input` with `seed`.
pub fn xxh32(input: &[u8], seed: u32) -> u32 {
    let mut h = Xxh32::new(seed);
    h.update(input);
    h.digest()
}

/// Streaming XXH32.
#[derive(Debug, Clone)]
pub struct Xxh32 {
    seed: u32,
    acc: [u32; 4],
    buf: [u8; STRIPE],
    buf_len: usize,
    /// Total input length; Step 4 adds its low 32 bits.
    total: u64,
}

impl Xxh32 {
    /// Step 1: initialize the accumulators from `seed`.
    pub fn new(seed: u32) -> Self {
        Xxh32 {
            seed,
            acc: [
                seed.wrapping_add(PRIME32_1).wrapping_add(PRIME32_2),
                seed.wrapping_add(PRIME32_2),
                seed,
                seed.wrapping_sub(PRIME32_1),
            ],
            buf: [0; STRIPE],
            buf_len: 0,
            total: 0,
        }
    }

    /// Feed more input.
    pub fn update(&mut self, input: &[u8]) {
        self.total = self.total.wrapping_add(input.len() as u64);
        let mut input = input;
        if self.buf_len > 0 {
            let need = STRIPE.saturating_sub(self.buf_len);
            let take = need.min(input.len());
            let (head, rest) = input.split_at_checked(take).unwrap_or((input, &[]));
            for (d, s) in self.buf.iter_mut().skip(self.buf_len).zip(head.iter()) {
                *d = *s;
            }
            self.buf_len = self.buf_len.saturating_add(head.len());
            input = rest;
            if self.buf_len == STRIPE {
                let stripe = self.buf;
                consume_stripe(&mut self.acc, &stripe);
                self.buf_len = 0;
            }
        }
        if self.buf_len > 0 {
            // The input was exhausted filling the buffer.
            return;
        }
        let (stripes, rest) = input.as_chunks::<STRIPE>();
        for s in stripes {
            consume_stripe(&mut self.acc, s);
        }
        for (d, s) in self.buf.iter_mut().zip(rest.iter()) {
            *d = *s;
        }
        self.buf_len = rest.len();
    }

    /// The digest of everything fed so far; the state is not consumed.
    pub fn digest(&self) -> u32 {
        let rest = self.buf.get(..self.buf_len).unwrap_or(&[]);
        let [a1, a2, a3, a4] = self.acc;
        let acc = if self.total >= STRIPE as u64 {
            // Step 3: accumulator convergence.
            a1.rotate_left(1)
                .wrapping_add(a2.rotate_left(7))
                .wrapping_add(a3.rotate_left(12))
                .wrapping_add(a4.rotate_left(18))
        } else {
            // Step 1, special case: input shorter than one stripe.
            self.seed.wrapping_add(PRIME32_5)
        };
        // Step 4: add the low 32 bits of the input length.
        let acc = acc.wrapping_add(self.total as u32);
        finish(acc, rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 + 7) as u8).collect()
    }

    #[test]
    fn streaming_equals_one_shot_for_every_split() {
        for len in 0..=64usize {
            let d = data(len);
            let want = xxh32(&d, 0);
            for split in 0..=len {
                let mut h = Xxh32::new(0);
                h.update(&d[..split]);
                h.update(&d[split..]);
                assert_eq!(h.digest(), want, "len {len} split {split}");
            }
            let mut h = Xxh32::new(0);
            for b in &d {
                h.update(std::slice::from_ref(b));
            }
            assert_eq!(h.digest(), want, "byte-wise len {len}");
        }
    }

    #[test]
    fn seed_changes_the_digest() {
        let d = data(40);
        assert_ne!(xxh32(&d, 0), xxh32(&d, 1));
        assert_ne!(xxh32(&[], 0), xxh32(&[], 1));
    }

    #[test]
    fn lengths_around_stripe_boundaries_differ() {
        let d = data(40);
        let mut seen = std::collections::HashSet::new();
        for len in [0usize, 1, 3, 4, 15, 16, 17, 31, 32, 33] {
            assert!(seen.insert(xxh32(&d[..len], 0)), "len {len}");
        }
    }
}
