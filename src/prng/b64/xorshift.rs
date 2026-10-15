use wrapn::{wrap, wu64};

use crate::prng::b64::SplitMix64;
use crate::rng::Rng;

// --- Xorshift64 ---

/// Xorshift 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Xorshift64};
///
/// let mut rng = Xorshift64::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xorshift64 {
    a: wu64,
}

impl Xorshift64 {
    /// Creates a new `Xorshift64` instance with the given seed.
    ///
    /// A seed that SplitMix64 maps to the all-zero state (a fixed point of
    /// xorshift) is remapped to a fixed non-zero state (REVIEW B21).
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        let a = seedgen.nextu_const();
        Self {
            a: wrap!(if a == 0 { 0x9E37_79B9_7F4A_7C15 } else { a }),
        }
    }
}

impl Rng for Xorshift64 {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let mut x = self.a;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.a = x;
        *x
    }
}

#[cfg(test)]
mod tests {
    /// REVIEW B21: SplitMix64 maps the seed -gamma to the all-zero state.
    #[test]
    fn xorshift64_has_no_all_zero_seed() {
        let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_neg();
        assert_eq!(crate::SplitMix64::new(seed).nextu(), 0);
        let mut g = Xorshift64::new(seed);
        assert!((0..8).any(|_| g.nextu() != 0));
        assert_eq!(*Xorshift64::new(1).a, crate::SplitMix64::new(1).nextu());
    }
    use super::*;

    crate::safe_test! { Xorshift64 }
}
