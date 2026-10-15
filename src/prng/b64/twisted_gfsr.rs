use wrapn::{wrap, wusize};

use crate::{
    SplitMix64,
    rng::Rng,
};

// --- TwistedGFSR ---

/// Twisted GFSR (Generalized Feedback Shift Register) 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, TwistedGFSR};
///
/// let mut rng = TwistedGFSR::new(0);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct TwistedGFSR {
    seed: [u32; N_GFSR],
    index: wusize,
}

const N_GFSR: usize = 25;
const M_GFSR: usize = 7;
const MAG01: [u32; 2] = [0x0, 0x8ebf_d028];

impl TwistedGFSR {
    /// Creates a new `TwistedGFSR` instance with the given seed.
    pub fn new(seed: u64) -> Self {
        // All 64 seed bits are used (the previous `seed as u32` ignored the upper half).
        let mut seedgen = SplitMix64::new(seed);
        let mut words = [0u32; N_GFSR + 1];
        for pair in words.chunks_exact_mut(2) {
            let v = seedgen.nextu_const();
            pair[0] = v as u32;
            pair[1] = (v >> 32) as u32;
        }
        let mut seed = [0u32; N_GFSR];
        seed.copy_from_slice(&words[..N_GFSR]);
        if seed.iter().all(|&w| w == 0) {
            seed[0] = 1; // the all-zero state is a fixed point of TT800
        }
        Self {
            seed,
            index: wrap!(N_GFSR),
        }
    }

    fn twist(&mut self) {
        for k in 0..(N_GFSR - M_GFSR) {
            self.seed[k] =
                self.seed[k + M_GFSR] ^ (self.seed[k] >> 1) ^ MAG01[(self.seed[k] & 1) as usize];
        }
        for k in (N_GFSR - M_GFSR)..N_GFSR {
            self.seed[k] = self.seed[k + M_GFSR - N_GFSR]
                ^ (self.seed[k] >> 1)
                ^ MAG01[(self.seed[k] & 1) as usize];
        }
        self.index = 0.into();
    }
}

impl TwistedGFSR {
    /// One raw 32-bit TT800 output (Matsumoto & Kurita 1996, with the
    /// `y ^= y >> 16` tempering of the 1996 version).
    #[inline]
    pub(crate) fn next_u32_raw(&mut self) -> u32 {
        if self.index >= N_GFSR {
            self.twist();
        }
        let mut y = self.seed[*self.index];
        y ^= (y << 7) & 0x2b5b_2500;
        y ^= (y << 15) & 0xdb8b_0000;
        y ^= y >> 16;
        self.index += 1;
        y
    }
}

impl Rng for TwistedGFSR {
    type Word = u64;

    /// Two consecutive TT800 outputs, first one in the high half.
    ///
    /// Previously only one 32-bit output was returned in a `u64`, which broke the
    /// `Rng` contract ("uniform over `[0, 2^64)`"): `choice` always returned
    /// index 0 and `shuffle` gave the same permutation for every seed.
    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let hi = self.next_u32_raw() as u64;
        let lo = self.next_u32_raw() as u64;
        (hi << 32) | lo
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Choice, Shuffle};

    crate::safe_test! { TwistedGFSR }

    /// TT800 step function checked against an independent transcription of
    /// Matsumoto & Kurita's `tt800.c` (1996) on an injected state.
    #[test]
    fn tt800_matches_reference() {
        fn reference(x: &mut [u32; 25], k: &mut usize) -> u32 {
            const MAG01: [u32; 2] = [0x0, 0x8ebfd028];
            if *k == 25 {
                for kk in 0..25 {
                    let src = if kk < 18 { kk + 7 } else { kk + 7 - 25 };
                    x[kk] = x[src] ^ (x[kk] >> 1) ^ MAG01[(x[kk] % 2) as usize];
                }
                *k = 0;
            }
            let mut y = x[*k];
            y ^= (y << 7) & 0x2b5b2500;
            y ^= (y << 15) & 0xdb8b0000;
            y ^= y >> 16;
            *k += 1;
            y
        }
        // arbitrary non-zero state (any state works for this comparison)
        let init: [u32; 25] = [
            0x95f24dab, 0x0b685215, 0xe76ccae7, 0xaf3ec239, 0x715fad23, 0x24a590ad, 0x69e4b5ef,
            0xbf456141, 0x96bc1b7b, 0xa7bdf825, 0xc1de75b7, 0x8858a9c9, 0x2da87693, 0xb657f9dd,
            0xffdc8a9f, 0x8121da71, 0x8b823ecb, 0x885d05f5, 0x4e20cd47, 0x5a9ad5d9, 0x512c0c03,
            0xea857ccd, 0x4cc1d30f, 0x8891a8a1, 0xa6b7aadb,
        ];
        let mut g = TwistedGFSR { seed: init, index: wrap!(N_GFSR) };
        let (mut x, mut k) = (init, 25usize);
        for _ in 0..10_000 {
            assert_eq!(g.next_u32_raw(), reference(&mut x, &mut k));
        }
    }

    /// REVIEW B6 regression: the generic `Word` helpers must see full-width words.
    #[test]
    fn full_width_words_make_choice_and_shuffle_work() {
        let mut g = TwistedGFSR::new(3);
        assert!((0..1000).any(|_| g.nextu() >> 32 != 0), "high half must be used");
        let items: Vec<u32> = (0..10).collect();
        let mut hist = [0u32; 10];
        for _ in 0..100_000 {
            hist[*g.choice(&items) as usize] += 1;
        }
        assert!(hist.iter().all(|&c| (9_000..11_000).contains(&c)), "{hist:?}");
        let perms: std::collections::HashSet<Vec<u32>> = (0..50u64)
            .map(|s| {
                let mut v: Vec<u32> = (0..10).collect();
                TwistedGFSR::new(s).shuffle(&mut v).unwrap();
                v
            })
            .collect();
        assert!(perms.len() > 45, "shuffle must depend on the seed");
        let f = (0..10_000).map(|_| g.nextf()).fold(0.0f64, f64::max);
        assert!(f < 1.0 && f > 0.99);
    }
}