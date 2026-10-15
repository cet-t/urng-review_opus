use wrapn::{wrap, wu32};

use crate::rng::Rng;

/// A xoroshiro64** random number generator.
///
/// A fast, high-quality 32-bit generator with a 64-bit state.
///
/// # Example
/// ```
/// use urng::{Rng, Xoroshiro64Ss};
///
/// let mut rng = Xoroshiro64Ss::new(12345);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Xoroshiro64Ss {
    s: [wu32; 2],
}

impl Xoroshiro64Ss {
    /// Creates a new `Xoroshiro64Ss` instance with the given seed.
    pub const fn new(seed: u32) -> Self {
        Self::from_seed_words(crate::_internal::SeedWords::from_u32(seed))
    }

    /// Seeds the whole state from a 64-bit seed via SplitMix64 (used by the C ABI
    /// bulk fills; REVIEW B14).
    #[allow(dead_code)]
    pub(crate) const fn from_seed64(seed: u64) -> Self {
        Self::from_seed_words(crate::_internal::SeedWords::from_u64(seed))
    }

    const fn from_seed_words(mut seedgen: crate::_internal::SeedWords) -> Self {

        Self {
            s: wrap![seedgen.nextu_const(), seedgen.nextu_const()],
        }
    }
}

impl Rng for Xoroshiro64Ss {
    type Word = u32;

    #[inline(always)]
    fn nextu(&mut self) -> Self::Word {
        let s0 = self.s[0];
        let mut s1 = self.s[1];
        let result = (s0 * 0x9E3779BB).rotate_left(5) * 5;

        s1 ^= s0;
        self.s[0] = s0.rotate_left(26) ^ s1 ^ (s1 << 9);
        self.s[1] = s1.rotate_left(13);

        *result
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    use std::arch::x86_64::*;

    
    use crate::prng::b32::jsf::simd::rotl8;
    use crate::rngv::RngV;

    pub(crate) const XOROSHIRO64SSX8: usize = 8;

    /// 8-way SIMD implementation of xoroshiro64** (AVX2).
    ///
    /// Lane `i` is the scalar [`crate::Xoroshiro64Ss`] with `(s0, s1)` =
    /// SplitMix32 outputs `2i, 2i + 1` (lane 0 equals `Xoroshiro64Ss::new(seed)`).
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, Xoroshiro64Ssx8};
    ///
    /// if std::arch::is_x86_feature_detected!("avx2") {
    ///     let mut rng = unsafe { Xoroshiro64Ssx8::new(12345) };
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Xoroshiro64Ssx8 {
        s0: __m256i,
        s1: __m256i,
    }

    impl Xoroshiro64Ssx8 {
        /// # Safety
        /// The CPU must support AVX2 (nothing beyond AVX2 is used).
        #[target_feature(enable = "avx2")]
        pub fn new(seed: u32) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u32(seed))
        }

        /// Seeds the whole state from a 64-bit seed via SplitMix64 (used by the C ABI
        /// bulk fills; REVIEW B14).
        #[target_feature(enable = "avx2")]
        #[allow(dead_code)]
        pub(crate) fn from_seed64(seed: u64) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u64(seed))
        }

        #[target_feature(enable = "avx2")]
        fn from_seed_words(mut seedgen: crate::_internal::SeedWords) -> Self {
            let mut s0 = [0u32; XOROSHIRO64SSX8];
            let mut s1 = [0u32; XOROSHIRO64SSX8];
            for i in 0..XOROSHIRO64SSX8 {
                s0[i] = seedgen.nextu_const();
                s1[i] = seedgen.nextu_const();
            }
            unsafe {
                Self {
                    s0: _mm256_loadu_si256(s0.as_ptr() as *const __m256i),
                    s1: _mm256_loadu_si256(s1.as_ptr() as *const __m256i),
                }
            }
        }

        #[target_feature(enable = "avx2")]
        #[inline]
        fn step(&mut self) -> __m256i {
            let s0 = self.s0;
            let mut s1 = self.s1;
            // result = rotl(s0 * 0x9E3779BB, 5) * 5  (the old code returned only
            // s0 * 0x9E3779BB, i.e. xoroshiro64*)
            let m = _mm256_mullo_epi32(s0, _mm256_set1_epi32(0x9E3779BBu32 as i32));
            let r = rotl8::<5, 27>(m);
            let result = _mm256_add_epi32(r, _mm256_slli_epi32::<2>(r));

            s1 = _mm256_xor_si256(s1, s0);
            self.s0 = _mm256_xor_si256(_mm256_xor_si256(rotl8::<26, 6>(s0), s1), _mm256_slli_epi32::<9>(s1));
            self.s1 = rotl8::<13, 19>(s1);
            result
        }
    }

    // SAFETY: values are only created by `new` (AVX2).
    unsafe impl RngV for Xoroshiro64Ssx8 {
        type Word = __m256i;

        #[inline(always)]
        fn nextuv(&mut self) -> Self::Word {
            // SAFETY: AVX2 guaranteed by construction.
            unsafe { self.step() }
        }
    }

    pub(crate) const XOROSHIRO64SSX16: usize = 16;

    /// 16-way SIMD implementation of xoroshiro64** (AVX-512F). Lane layout as
    /// for [`Xoroshiro64Ssx8`].
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, Xoroshiro64Ssx16};
    ///
    /// if std::arch::is_x86_feature_detected!("avx512f") {
    ///     let mut rng = unsafe { Xoroshiro64Ssx16::new(12345) };
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Xoroshiro64Ssx16 {
        s0: __m512i,
        s1: __m512i,
    }

    impl Xoroshiro64Ssx16 {
        /// # Safety
        /// The CPU must support AVX-512F.
        #[target_feature(enable = "avx512f")]
        pub fn new(seed: u32) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u32(seed))
        }

        /// Seeds the whole state from a 64-bit seed via SplitMix64 (used by the C ABI
        /// bulk fills; REVIEW B14).
        #[target_feature(enable = "avx512f")]
        #[allow(dead_code)]
        pub(crate) fn from_seed64(seed: u64) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u64(seed))
        }

        #[target_feature(enable = "avx512f")]
        fn from_seed_words(mut seedgen: crate::_internal::SeedWords) -> Self {
            let mut s0 = [0u32; XOROSHIRO64SSX16];
            let mut s1 = [0u32; XOROSHIRO64SSX16];
            for i in 0..XOROSHIRO64SSX16 {
                s0[i] = seedgen.nextu_const();
                s1[i] = seedgen.nextu_const();
            }
            unsafe {
                Self {
                    s0: _mm512_loadu_si512(s0.as_ptr() as _),
                    s1: _mm512_loadu_si512(s1.as_ptr() as _),
                }
            }
        }

        #[target_feature(enable = "avx512f")]
        #[inline]
        fn step(&mut self) -> __m512i {
            let s0 = self.s0;
            let mut s1 = self.s1;
            let m = _mm512_mullo_epi32(s0, _mm512_set1_epi32(0x9E3779BBu32 as i32));
            let r = _mm512_rol_epi32::<5>(m);
            let result = _mm512_add_epi32(r, _mm512_slli_epi32::<2>(r));

            s1 = _mm512_xor_si512(s1, s0);
            self.s0 = _mm512_xor_si512(
                _mm512_xor_si512(_mm512_rol_epi32::<26>(s0), s1),
                _mm512_slli_epi32::<9>(s1),
            );
            self.s1 = _mm512_rol_epi32::<13>(s1);
            result
        }
    }

    // SAFETY: values are only created by `new` (AVX-512F).
    unsafe impl RngV for Xoroshiro64Ssx16 {
        type Word = __m512i;

        #[inline(always)]
        fn nextuv(&mut self) -> Self::Word {
            // SAFETY: AVX-512F guaranteed by construction.
            unsafe { self.step() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    use crate::SplitMix32;

    crate::safe_test! { Xoroshiro64Ss }

    #[test]
    fn xoroshiro64ss_matches_reference() {
        let mut g = Xoroshiro64Ss::new(7);
        let mut st = g.s.map(|w| *w);
        for _ in 0..100_000 {
            assert_eq!(g.nextu(), crate::_reftest::xoroshiro64ss(&mut st));
        }
    }

    /// REVIEW B12/B8: SIMD lanes returned `s0 * 0x9E3779BB` (xoroshiro64*) and the
    /// x8 version used the AVX-512VL rotate. Every lane must equal the scalar.
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn xoroshiro64ss_simd_lanes_match_scalar() {
        use crate::RngV;
        fn lanes(seed: u32, n: usize) -> Vec<Xoroshiro64Ss> {
            let mut sg = SplitMix32::new(seed);
            (0..n).map(|_| Xoroshiro64Ss { s: wrap![sg.nextu_const(), sg.nextu_const()] }).collect()
        }
        if std::arch::is_x86_feature_detected!("avx2") {
            let mut v = unsafe { Xoroshiro64Ssx8::new(12345) };
            let mut s = lanes(12345, 8);
            for _ in 0..10_000 {
                let got: [u32; 8] = bytemuck::cast(v.nextuv());
                for i in 0..8 {
                    assert_eq!(got[i], s[i].nextu());
                }
            }
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            let mut v = unsafe { Xoroshiro64Ssx16::new(12345) };
            let mut s = lanes(12345, 16);
            for _ in 0..10_000 {
                let got: [u32; 16] = unsafe { std::mem::transmute(v.nextuv()) };
                for i in 0..16 {
                    assert_eq!(got[i], s[i].nextu());
                }
            }
        }
    }
}