use wrapn::{wrap, wu32};

use crate::prng::b32::SplitMix32;
use crate::rng::Rng;

// --- Xorshift32 ---

/// A 32-bit Xorshift random number generator.
///
/// This generator uses a shift-register based algorithm.
///
/// # Example
/// ```
/// use urng::{Rng, Xorshift32};
///
/// let mut rng = Xorshift32::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xorshift32 {
    a: wu32,
}

impl Xorshift32 {
    /// Creates a new `Xorshift32` instance with the given seed.
    pub const fn new(seed: u32) -> Self {
        let mut sm = SplitMix32::new(seed);
        Self {
            a: wrap!(sm.nextu_const()),
        }
    }
}

impl Rng for Xorshift32 {
    type Word = u32;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let x = self.a;
        self.a = x ^ (x << 13);
        self.a ^= self.a >> 17;
        self.a ^= self.a << 5;
        *self.a
    }
}

// --- Xorshift128 ---

/// A 128-bit Xorshift random number generator.
///
/// Produces 32-bit output from a 128-bit internal state.
/// Period: 2^128 - 1.
///
/// # Example
/// ```
/// use urng::{Rng, Xorshift128};
///
/// let mut rng = Xorshift128::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xorshift128 {
    x: [wu32; 4],
}

impl Xorshift128 {
    /// Creates a new `Xorshift128` instance.
    ///
    /// Each seed element is OR-ed with 1 to prevent an all-zero state.
    pub const fn new(seed: u32) -> Self {
        let mut sm = SplitMix32::new(seed);
        Self {
            x: wrap![
                sm.nextu_const(),
                sm.nextu_const(),
                sm.nextu_const(),
                sm.nextu_const()
            ],
        }
    }
}

impl Rng for Xorshift128 {
    type Word = u32;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        let mut t = self.x[3];
        t ^= t << 11;
        t ^= t >> 8;
        let s = self.x[0];
        (self.x[1], self.x[2], self.x[3]) = (s, self.x[1], self.x[2]);
        self.x[0] = t ^ s ^ (s >> 19);
        *self.x[0]
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    use std::arch::x86_64::*;

    use crate::{RngV, SplitMix32};

    const XORSHIFT32X8: usize = 8;

    /// 8-way Xorshift32 (AVX2). Lane `i` is the scalar [`crate::Xorshift32`]
    /// seeded with SplitMix32 output `i`.
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, Xorshift32x8};
    ///
    /// if std::arch::is_x86_feature_detected!("avx2") {
    ///     let mut rng = unsafe { Xorshift32x8::new(1) };
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Xorshift32x8 {
        a: __m256i,
    }

    impl Xorshift32x8 {
        /// Creates a new generator.
        ///
        /// # Safety
        ///
        /// The CPU must support AVX2. (This constructor used to be a plain safe
        /// `fn` that executed AVX2/AVX-512VL instructions without any check.)
        #[target_feature(enable = "avx2")]
        pub fn new(seed: u32) -> Self {
            let mut sm = SplitMix32::new(seed);
            let mut a = [0u32; XORSHIFT32X8];
            for v in a.iter_mut() {
                *v = sm.nextu_const();
                if *v == 0 {
                    *v = 0x6C07_8965; // xorshift's all-zero state is a fixed point
                }
            }
            Self { a: unsafe { _mm256_loadu_si256(a.as_ptr() as *const __m256i) } }
        }

        #[target_feature(enable = "avx2")]
        #[inline]
        fn step(&mut self) -> __m256i {
            let x = self.a;
            let x = _mm256_xor_si256(x, _mm256_slli_epi32::<13>(x));
            let x = _mm256_xor_si256(x, _mm256_srli_epi32::<17>(x));
            let x = _mm256_xor_si256(x, _mm256_slli_epi32::<5>(x));
            self.a = x;
            x
        }
    }

    // SAFETY: values are only created by `new` (AVX2).
    unsafe impl RngV for Xorshift32x8 {
        type Word = __m256i;

        #[inline(always)]
        fn nextuv(&mut self) -> Self::Word {
            // SAFETY: AVX2 guaranteed by construction.
            unsafe { self.step() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! {
        Xorshift32,
        Xorshift128
    }

    #[test]
    fn xorshift128_matches_reference() {
        let mut g = Xorshift128::new(3);
        let mut st = g.x.map(|w| *w);
        for _ in 0..100_000 {
            assert_eq!(g.nextu(), crate::_reftest::xorshift128(&mut st));
        }
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn xorshift32x8_lanes_match_scalar() {
        use crate::RngV;
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }
        let mut sg = SplitMix32::new(9);
        let mut lanes: Vec<Xorshift32> = (0..8).map(|_| Xorshift32 { a: wrap!(sg.nextu_const()) }).collect();
        let mut v = unsafe { Xorshift32x8::new(9) };
        for _ in 0..10_000 {
            let got: [u32; 8] = bytemuck::cast(v.nextuv());
            for i in 0..8 {
                assert_eq!(got[i], lanes[i].nextu());
            }
        }
    }
}