use wrapn::{wrap, wu64};

use crate::{prng::b64::SplitMix64, rng::Rng};

/// Vigna's polynomial jump: XOR-accumulates the states selected by the bits of
/// `table` while stepping the generator, then installs the accumulated state.
#[inline]
fn xoroshiro128_jump<R: Rng>(rng: &mut R, state: impl Fn(&R) -> [wu64; 2], set: impl Fn(&mut R, [wu64; 2]), table: [u64; 2]) {
    let mut acc = wrap![0u64; 2];
    for word in table {
        for b in 0..64 {
            if word & (1u64 << b) != 0 {
                let s = state(rng);
                acc[0] ^= s[0];
                acc[1] ^= s[1];
            }
            let _ = rng.nextu();
        }
    }
    set(rng, acc);
}

/// Xoroshiro128++ 64-bit RNG implementation (Blackman & Vigna 2019).
///
/// # Example
/// ```
/// use urng::{Rng, Xoroshiro128Pp};
///
/// let mut rng = Xoroshiro128Pp::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xoroshiro128Pp {
    s: [wu64; 2],
}

impl Xoroshiro128Pp {
    /// Creates a new `Xoroshiro128Pp` instance with the given seed.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            s: wrap![seedgen.nextu_const(), seedgen.nextu_const()],
        }
    }

    /// Applies the jump function for advancing by $2^{64}$ steps.
    #[inline]
    pub fn jump(&mut self) {
        xoroshiro128_jump(self, |r| r.s, |r, s| r.s = s, [0x2bd7a6a6e99c2ddc, 0x0992ccaf6a6fca05]);
    }

    /// Applies the long-jump function for advancing by $2^{96}$ steps.
    #[inline]
    pub fn long_jump(&mut self) {
        xoroshiro128_jump(self, |r| r.s, |r, s| r.s = s, [0x360fd5f2cf8d5d99, 0x9c6e6877736c46e3]);
    }
}

impl Rng for Xoroshiro128Pp {
    type Word = u64;

    /// `rotl(s0 + s1, 17) + s0` with the (49, 21, 28) engine.
    ///
    /// Previously this returned `s0 + s1` (the `+` scrambler) and used a state
    /// transition that matched neither xoroshiro128+ nor ++ and did not have the
    /// maximal period.
    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let s0 = self.s[0];
        let mut s1 = self.s[1];
        let result = (s0 + s1).rotate_left(17) + s0;

        s1 ^= s0;
        self.s[0] = s0.rotate_left(49) ^ s1 ^ (s1 << 21);
        self.s[1] = s1.rotate_left(28);

        *result
    }
}

/// Xoroshiro128** 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Xoroshiro128Ss};
///
/// let mut rng = Xoroshiro128Ss::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xoroshiro128Ss {
    s: [wu64; 2],
}

impl Xoroshiro128Ss {
    /// Creates a new `Xoroshiro128Ss` instance with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            s: wrap![seedgen.nextu(), seedgen.nextu()],
        }
    }

    /// Applies the jump function for advancing by $2^{64}$ steps.
    #[inline]
    pub fn jump(&mut self) {
        xoroshiro128_jump(self, |r| r.s, |r, s| r.s = s, [0xdf900294d8f554a5, 0x170865df4b3201fc]);
    }

    /// Applies the long-jump function for advancing by $2^{96}$ steps.
    ///
    /// (The previous version reset the state between the two table words and ran
    /// the table twice, so it did not advance by 2^96.)
    #[inline]
    pub fn long_jump(&mut self) {
        xoroshiro128_jump(self, |r| r.s, |r, s| r.s = s, [0xd2a98b26625eee7b, 0xdddf9b1090aa7ac1]);
    }
}

impl Rng for Xoroshiro128Ss {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let result = (self.s[0] * 5).rotate_left(7) * 9;

        self.s[1] ^= self.s[0];
        self.s[0] = self.s[1] ^ self.s[0].rotate_left(24) ^ (self.s[1] << 16);
        self.s[1] = self.s[1].rotate_left(37);

        *result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::_reftest as r;
    use crate::_reftest::gf2;

    crate::safe_test! {
        Xoroshiro128Pp,
        Xoroshiro128Ss
    }

    fn pack(s: [u64; 2]) -> u128 {
        ((s[1] as u128) << 64) | s[0] as u128
    }
    fn unpack(x: u128) -> [u64; 2] {
        [x as u64, (x >> 64) as u64]
    }

    /// Step functions agree with Vigna's reference code; first ++ output for
    /// state [1, 2] is rotl(3, 17) + 1.
    #[test]
    fn steps_match_reference() {
        let mut pp = Xoroshiro128Pp { s: wrap![1, 2] };
        assert_eq!(pp.nextu(), (3u64 << 17) + 1);
        let x0 = [0x0123456789abcdef, 0xfedcba9876543210];
        let (mut a, mut b) = (x0, x0);
        let mut pp = Xoroshiro128Pp { s: wrap![x0[0], x0[1]] };
        let mut ss = Xoroshiro128Ss { s: wrap![x0[0], x0[1]] };
        for _ in 0..100_000 {
            assert_eq!(pp.nextu(), r::xoroshiro128pp(&mut a));
            assert_eq!(ss.nextu(), r::xoroshiro128ss(&mut b));
        }
    }

    /// The linear engines have maximal period 2^128 - 1 (checked exactly with
    /// GF(2) matrix exponentiation), and jump/long_jump advance by exactly 2^64 /
    /// 2^96 steps of the *implemented* engine.
    #[test]
    fn engines_have_full_period_and_jumps_are_exact() {
        fn engine_pp(x: u128) -> u128 {
            let mut g = Xoroshiro128Pp { s: wrap![unpack(x)[0], unpack(x)[1]] };
            let _ = g.nextu();
            pack(g.s.map(|w| *w))
        }
        fn engine_ss(x: u128) -> u128 {
            let mut g = Xoroshiro128Ss { s: wrap![unpack(x)[0], unpack(x)[1]] };
            let _ = g.nextu();
            pack(g.s.map(|w| *w))
        }
        let x0 = [0x0123456789abcdef, 0xfedcba9876543210];
        for (name, f) in [("pp", engine_pp as fn(u128) -> u128), ("ss", engine_ss)] {
            let t = gf2::from_fn(f);
            assert!(gf2::has_max_order(&t), "{name}: not maximal period");
            let j64 = gf2::apply(&gf2::pow2(&t, 64), pack(x0));
            let j96 = gf2::apply(&gf2::pow2(&t, 96), pack(x0));
            let (got64, got96) = if name == "pp" {
                let mut g = Xoroshiro128Pp { s: wrap![x0[0], x0[1]] };
                g.jump();
                let a = pack(g.s.map(|w| *w));
                let mut g = Xoroshiro128Pp { s: wrap![x0[0], x0[1]] };
                g.long_jump();
                (a, pack(g.s.map(|w| *w)))
            } else {
                let mut g = Xoroshiro128Ss { s: wrap![x0[0], x0[1]] };
                g.jump();
                let a = pack(g.s.map(|w| *w));
                let mut g = Xoroshiro128Ss { s: wrap![x0[0], x0[1]] };
                g.long_jump();
                (a, pack(g.s.map(|w| *w)))
            };
            assert_eq!(got64, j64, "{name}: jump != T^(2^64)");
            assert_eq!(got96, j96, "{name}: long_jump != T^(2^96)");
        }
    }
}
