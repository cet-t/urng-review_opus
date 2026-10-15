use wrapn::{wrap, wu64};

use crate::prng::b64::SplitMix64;
use crate::rng::Rng;

/// Xoshiro256++ 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Xoshiro256Pp};
///
/// let mut rng = Xoshiro256Pp::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xoshiro256Pp {
    s: [wu64; 4],
}

impl Xoshiro256Pp {
    /// Creates a new `Xoshiro256Pp` instance with the given seed.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            s: wrap![
                seedgen.nextu_const(),
                seedgen.nextu_const(),
                seedgen.nextu_const(),
                seedgen.nextu_const(),
            ],
        }
    }
}

impl Rng for Xoshiro256Pp {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let s = &mut self.s;
        let res = s[0] + (s[0] + s[3]).rotate_left(23);
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);

        *res
    }
}

/// Xoshiro256** 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Xoshiro256Ss};
///
/// let mut rng = Xoshiro256Ss::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xoshiro256Ss {
    s: [wu64; 4],
}

impl Xoshiro256Ss {
    /// Creates a new `Xoshiro256Ss` instance with the given seed.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            s: wrap![
                seedgen.nextu_const(),
                seedgen.nextu_const(),
                seedgen.nextu_const(),
                seedgen.nextu_const(),
            ],
        }
    }
}

impl Rng for Xoshiro256Ss {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let s = &mut self.s;
        let res = (s[1] * 5).rotate_left(7) * 9;
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);

        *res
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    use crate::prng::b64::SplitMix64;

    /// Two independent xoshiro256** streams evaluated side by side.
    ///
    /// Uses only SSE2 (part of the x86_64 baseline), so the API is safe on
    /// every x86_64 CPU. Lane 0 is exactly [`crate::Xoshiro256Ss::new`]`(seed)`;
    /// lane 1 is seeded from the next four SplitMix64 outputs.
    ///
    /// # Example
    /// ```
    /// use urng::Xoshiro256Ssx2;
    ///
    /// let mut rng = Xoshiro256Ssx2::new(1);
    /// let _pair: [u64; 2] = rng.nextu();
    /// ```
    #[cfg(target_arch = "x86_64")]
    #[repr(C, align(64))]
    #[derive(Clone)]
    pub struct Xoshiro256Ssx2 {
        /// `s[w]` holds state word `w` of stream 0 (low lane) and stream 1 (high lane).
        pub(crate) s: [__m128i; 4],
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn rotl64(x: __m128i, k: i32) -> __m128i {
        // SAFETY: SSE2 is part of the x86_64 baseline, so these intrinsics are
        // always available. The counts are passed in a register (`_mm_sll_epi64`).
        unsafe {
            let l = _mm_cvtsi32_si128(k);
            let r = _mm_cvtsi32_si128(64 - k);
            _mm_or_si128(_mm_sll_epi64(x, l), _mm_srl_epi64(x, r))
        }
    }

    #[cfg(target_arch = "x86_64")]
    impl Xoshiro256Ssx2 {
        /// Creates a new `Xoshiro256Ssx2` instance.
        pub fn new(seed: u64) -> Self {
            let mut seedgen = SplitMix64::new(seed);
            let v = [0u64; 8].map(|_| seedgen.nextu_const());
            Self::from_states([v[0], v[1], v[2], v[3]], [v[4], v[5], v[6], v[7]])
        }

        /// Builds the generator from two explicit xoshiro256 states.
        pub(crate) fn from_states(a: [u64; 4], b: [u64; 4]) -> Self {
            Self {
                // SAFETY: SSE2 is part of the x86_64 baseline.
                s: std::array::from_fn(|w| unsafe { _mm_set_epi64x(b[w] as i64, a[w] as i64) }),
            }
        }

        /// Returns the two states (`[stream0, stream1]`).
        #[cfg(test)]
        pub(crate) fn states(&self) -> [[u64; 4]; 2] {
            let w: [[u64; 2]; 4] = self.s.map(bytemuck::cast);
            [
                [w[0][0], w[1][0], w[2][0], w[3][0]],
                [w[0][1], w[1][1], w[2][1], w[3][1]],
            ]
        }

        /// Generates the next 2 random `u64` values (`[stream0, stream1]`).
        #[inline(always)]
        pub fn nextu(&mut self) -> [u64; 2] {
            let [s0, s1, s2, s3] = self.s;
            // SAFETY: SSE2 is part of the x86_64 baseline; no memory is accessed.
            let (res, next) = unsafe {
                // result = rotl(s1 * 5, 7) * 9
                let x5 = _mm_add_epi64(s1, _mm_slli_epi64(s1, 2));
                let r = rotl64(x5, 7);
                let res = _mm_add_epi64(r, _mm_slli_epi64(r, 3));

                let t = _mm_slli_epi64(s1, 17);
                let s2 = _mm_xor_si128(s2, s0);
                let s3 = _mm_xor_si128(s3, s1);
                let s1 = _mm_xor_si128(s1, s2);
                let s0 = _mm_xor_si128(s0, s3);
                let s2 = _mm_xor_si128(s2, t);
                let s3 = rotl64(s3, 45);
                (res, [s0, s1, s2, s3])
            };
            self.s = next;
            bytemuck::cast(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::_reftest::xoshiro256ss as xoshiro256ss_ref;

    crate::safe_test! {
        Xoshiro256Pp,
        Xoshiro256Ss
    }

    /// Published xoshiro256** / xoshiro256++ outputs for state {1, 2, 3, 4}
    /// (same vectors as the `rand_xoshiro` crate's reference tests).
    #[test]
    fn xoshiro256_reference_vectors() {
        let mut ss = Xoshiro256Ss { s: wrap![1, 2, 3, 4] };
        let want_ss: [u64; 4] = [11520, 0, 1509978240, 1215971899390074240];
        for w in want_ss {
            assert_eq!(ss.nextu(), w);
        }
        let mut pp = Xoshiro256Pp { s: wrap![1, 2, 3, 4] };
        let want_pp: [u64; 4] = [41943041, 58720359, 3588806011781223, 3591011842654386];
        for w in want_pp {
            assert_eq!(pp.nextu(), w);
        }
        // independent reference implementation agrees for a long run
        let mut ss = Xoshiro256Ss::new(77);
        let mut st = ss.s.map(|w| *w);
        for _ in 0..10_000 {
            assert_eq!(ss.nextu(), xoshiro256ss_ref(&mut st));
        }
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn xoshiro256ssx2_lanes_match_scalar() {
        for seed in [0u64, 1, 2, 12345, u64::MAX] {
            let mut v = Xoshiro256Ssx2::new(seed);
            let [a, b] = v.states();
            let mut lane0 = Xoshiro256Ss::new(seed);
            assert_eq!(lane0.s.map(|w| *w), a, "lane 0 must equal the scalar generator");
            let mut lane1 = Xoshiro256Ss { s: wrap![b[0], b[1], b[2], b[3]] };
            let mut zero_streak = 0;
            for _ in 0..10_000 {
                let got = v.nextu();
                assert_eq!(got, [lane0.nextu(), lane1.nextu()]);
                zero_streak = if got == [0, 0] { zero_streak + 1 } else { 0 };
                assert!(zero_streak < 4, "state collapsed to zero");
            }
        }
    }
}
