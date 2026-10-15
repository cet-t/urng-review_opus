use wrapn::{wrap, wu32};

use crate::Rng;

/// JSF (Jenkins Small Fast) 32-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Jsf32};
///
/// let mut rng = Jsf32::new(12345);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Jsf32 {
    pub(crate) a: wu32,
    pub(crate) b: wu32,
    pub(crate) c: wu32,
    pub(crate) d: wu32,
}

impl Jsf32 {
    /// Creates a new `Jsf32` instance with the given seed.
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
            a: wrap!(0xf1ea5eed),
            b: wrap!(seedgen.nextu_const()),
            c: wrap!(seedgen.nextu_const()),
            d: wrap!(seedgen.nextu_const()),
        }
    }
}

impl Rng for Jsf32 {
    type Word = u32;

    #[inline(always)]
    fn nextu(&mut self) -> Self::Word {
        let e = self.a - self.b.rotate_left(27);
        self.a = self.b ^ self.c.rotate_left(17);
        self.b = self.c + self.d;
        self.c = self.d + e;
        self.d = e + self.a;
        *self.d
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    use std::arch::x86_64::*;

    use crate::RngV;

    /// 8-way SIMD implementation of JSF (Jenkins Small Fast) 32-bit RNG (AVX2).
    ///
    /// Lane `i` is the scalar JSF32 with `b, c, d` = SplitMix32 outputs
    /// `i`, `8 + i`, `16 + i`.
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, Jsf32x8};
    ///
    /// if std::arch::is_x86_feature_detected!("avx2") {
    ///     let mut rng = unsafe { Jsf32x8::new(12345) };
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Jsf32x8 {
        pub(crate) a: __m256i,
        pub(crate) b: __m256i,
        pub(crate) c: __m256i,
        pub(crate) d: __m256i,
    }

    pub(crate) const JSF32X8: usize = 8;

    /// 32-bit lane rotate with AVX2 shifts (`R` must be `32 - L`).
    #[target_feature(enable = "avx2")]
    #[inline]
    pub(crate) fn rotl8<const L: i32, const R: i32>(x: __m256i) -> __m256i {
        _mm256_or_si256(_mm256_slli_epi32::<L>(x), _mm256_srli_epi32::<R>(x))
    }

    impl Jsf32x8 {
        /// Creates a new generator.
        ///
        /// # Safety
        ///
        /// The CPU must support AVX2 (this is everything the type uses; it no
        /// longer needs AVX-512VL).
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
            let mut sv = [[0u32; JSF32X8]; 3];
            for vals in sv.iter_mut() {
                for v in vals.iter_mut() {
                    *v = seedgen.nextu_const();
                }
            }
            unsafe {
                Self {
                    a: _mm256_set1_epi32(0xf1ea5eed_u32 as i32),
                    b: _mm256_loadu_si256(sv[0].as_ptr() as *const __m256i),
                    c: _mm256_loadu_si256(sv[1].as_ptr() as *const __m256i),
                    d: _mm256_loadu_si256(sv[2].as_ptr() as *const __m256i),
                }
            }
        }

        #[target_feature(enable = "avx2")]
        #[inline]
        fn step(&mut self) -> __m256i {
            let e = _mm256_sub_epi32(self.a, rotl8::<27, 5>(self.b));
            self.a = _mm256_xor_si256(self.b, rotl8::<17, 15>(self.c));
            self.b = _mm256_add_epi32(self.c, self.d);
            self.c = _mm256_add_epi32(self.d, e);
            self.d = _mm256_add_epi32(e, self.a);
            self.d
        }
    }

    // SAFETY: values are only created by `new`, which requires AVX2; `step`
    // and `__m256i: WordV` need nothing beyond AVX2.
    unsafe impl RngV for Jsf32x8 {
        type Word = __m256i;

        #[inline(always)]
        fn nextuv(&mut self) -> __m256i {
            // SAFETY: AVX2 is guaranteed by construction.
            unsafe { self.step() }
        }
    }

    /// 16-way SIMD implementation of JSF (Jenkins Small Fast) 32-bit RNG (AVX-512F).
    ///
    /// Lane `i` is the scalar JSF32 with `b, c, d` = SplitMix32 outputs
    /// `i`, `16 + i`, `32 + i`.
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, Jsf32x16};
    ///
    /// if std::arch::is_x86_feature_detected!("avx512f") {
    ///     let mut rng = unsafe { Jsf32x16::new(12345) };
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Jsf32x16 {
        pub(crate) a: __m512i,
        pub(crate) b: __m512i,
        pub(crate) c: __m512i,
        pub(crate) d: __m512i,
    }

    pub(crate) const JSF32X16: usize = 16;

    impl Jsf32x16 {
        /// Creates a new generator.
        ///
        /// # Safety
        ///
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
            let mut sv = [[0u32; JSF32X16]; 3];
            for vals in sv.iter_mut() {
                for v in vals.iter_mut() {
                    *v = seedgen.nextu_const();
                }
            }
            const A: [u32; JSF32X16] = [0xf1ea5eedu32; JSF32X16];
            unsafe {
                Self {
                    a: _mm512_loadu_si512(A.as_ptr() as _),
                    b: _mm512_loadu_si512(sv[0].as_ptr() as _),
                    c: _mm512_loadu_si512(sv[1].as_ptr() as _),
                    d: _mm512_loadu_si512(sv[2].as_ptr() as _),
                }
            }
        }

        #[target_feature(enable = "avx512f")]
        #[inline]
        fn step(&mut self) -> __m512i {
            let e = _mm512_sub_epi32(self.a, _mm512_rol_epi32::<27>(self.b));
            self.a = _mm512_xor_si512(self.b, _mm512_rol_epi32::<17>(self.c));
            self.b = _mm512_add_epi32(self.c, self.d);
            self.c = _mm512_add_epi32(self.d, e);
            self.d = _mm512_add_epi32(e, self.a);
            self.d
        }
    }

    // SAFETY: values are only created by `new`, which requires AVX-512F.
    unsafe impl RngV for Jsf32x16 {
        type Word = __m512i;

        #[inline(always)]
        fn nextuv(&mut self) -> __m512i {
            // SAFETY: AVX-512F is guaranteed by construction.
            unsafe { self.step() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    use crate::SplitMix32;

    crate::safe_test! { Jsf32 }

    #[test]
    fn jsf32_matches_reference() {
        let mut g = Jsf32::new(42);
        let mut st = [*g.a, *g.b, *g.c, *g.d];
        for _ in 0..100_000 {
            assert_eq!(g.nextu(), crate::_reftest::jsf32(&mut st));
        }
    }

    /// Lanes equal the scalar generator. (REVIEW B8: Jsf32x8 used the AVX-512VL
    /// rotate; it is now pure AVX2 and runs wherever AVX2 is available.)
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn jsf32_simd_lanes_match_scalar() {
        use crate::RngV;
        fn lanes(seed: u32, n: usize) -> Vec<Jsf32> {
            let mut sg = SplitMix32::new(seed);
            let v: Vec<u32> = (0..3 * n).map(|_| sg.nextu_const()).collect();
            (0..n)
                .map(|i| Jsf32 { a: wrap!(0xf1ea5eed), b: wrap!(v[i]), c: wrap!(v[n + i]), d: wrap!(v[2 * n + i]) })
                .collect()
        }
        if std::arch::is_x86_feature_detected!("avx2") {
            let mut v = unsafe { Jsf32x8::new(3) };
            let mut s = lanes(3, 8);
            for _ in 0..10_000 {
                let got: [u32; 8] = bytemuck::cast(v.nextuv());
                for i in 0..8 {
                    assert_eq!(got[i], s[i].nextu());
                }
            }
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            let mut v = unsafe { Jsf32x16::new(3) };
            let mut s = lanes(3, 16);
            for _ in 0..10_000 {
                let got: [u32; 16] = unsafe { std::mem::transmute(v.nextuv()) };
                for i in 0..16 {
                    assert_eq!(got[i], s[i].nextu());
                }
            }
        }
    }
}