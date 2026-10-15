use wrapn::{wrap, wu32};

use crate::Rng;

/// SFC (Small Fast Chaotic) 32-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Sfc32};
///
/// let mut rng = Sfc32::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Sfc32 {
    pub(crate) a: wu32,
    pub(crate) b: wu32,
    pub(crate) c: wu32,
    pub(crate) counter: wu32,
}

impl Sfc32 {
    /// Creates a new `Sfc32` instance with the given seed.
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
            a: wrap!(seedgen.nextu_const()),
            b: wrap!(seedgen.nextu_const()),
            c: wrap!(seedgen.nextu_const()),
            counter: wrap!(1),
        }
    }
}

impl Rng for Sfc32 {
    type Word = u32;

    #[inline(always)]
    fn nextu(&mut self) -> Self::Word {
        let tmp = self.a + self.b + self.counter;
        self.counter += 1;
        self.a = self.b ^ (self.b >> 9);
        self.b = self.c + (self.c << 3);
        self.c = self.c.rotate_right(11);
        self.c += tmp;
        *tmp
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    use std::arch::x86_64::*;

    use crate::prng::b32::jsf::simd::rotl8;
    use crate::{WordV, i2f_bits, u2f_01};

    pub(crate) const SFC32X4: usize = 4;

    /// 4-way SIMD implementation of SFC (Small Fast Chaotic) 32-bit RNG (SSE2,
    /// part of the x86_64 baseline, so the API is safe).
    ///
    /// Lane `i` is the scalar SFC32 with `(a, b, c)` = SplitMix32 outputs
    /// `3i, 3i+1, 3i+2` and counter 1.
    ///
    /// # Example
    /// ```
    /// use urng::Sfc32x4;
    ///
    /// let mut rng = Sfc32x4::new(1);
    /// let _ = rng.nextu();
    /// ```
    #[repr(C, align(64))]
    pub struct Sfc32x4 {
        pub(crate) a: __m128i,
        pub(crate) b: __m128i,
        pub(crate) c: __m128i,
        pub(crate) counter: __m128i,
    }

    impl Sfc32x4 {
        /// Creates a new `Sfc32x4` instance with the given seed.
        pub fn new(seed: u32) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u32(seed))
        }

        /// Seeds the whole state from a 64-bit seed via SplitMix64 (used by the C ABI
        /// bulk fills; REVIEW B14).
        #[allow(dead_code)]
        pub(crate) fn from_seed64(seed: u64) -> Self {
            Self::from_seed_words(crate::_internal::SeedWords::from_u64(seed))
        }

        fn from_seed_words(mut seedgen: crate::_internal::SeedWords) -> Self {
            let mut a = [0u32; SFC32X4];
            let mut b = [0u32; SFC32X4];
            let mut c = [0u32; SFC32X4];
            for i in 0..SFC32X4 {
                a[i] = seedgen.nextu();
                b[i] = seedgen.nextu();
                c[i] = seedgen.nextu();
            }
            // SAFETY: SSE2 is part of the x86_64 baseline.
            unsafe {
                Self {
                    a: _mm_loadu_si128(a.as_ptr() as *const _),
                    b: _mm_loadu_si128(b.as_ptr() as *const _),
                    c: _mm_loadu_si128(c.as_ptr() as *const _),
                    counter: _mm_set1_epi32(1),
                }
            }
        }

        #[inline(always)]
        pub(crate) fn nextuv(&mut self) -> __m128i {
            // SAFETY: SSE2 is part of the x86_64 baseline.
            unsafe {
                let tmp = _mm_add_epi32(_mm_add_epi32(self.a, self.b), self.counter);
                self.counter = _mm_add_epi32(self.counter, _mm_set1_epi32(1));
                self.a = _mm_xor_si128(self.b, _mm_srli_epi32(self.b, 9));
                self.b = _mm_add_epi32(self.c, _mm_slli_epi32(self.c, 3));
                self.c = _mm_add_epi32(
                    _mm_or_si128(_mm_slli_epi32(self.c, 21), _mm_srli_epi32(self.c, 11)),
                    tmp,
                );
                tmp
            }
        }

        #[inline(always)]
        pub(crate) fn nextfv(&mut self) -> __m128 {
            // SAFETY: SSE2 baseline.
            unsafe { self.nextuv().to_f01() }
        }

        /// Lanes in `[v_min, v_min + v_range)`; a `v_range` lane of 0 means 2^32.
        #[inline(always)]
        pub(crate) fn randiv(&mut self, v_range: __m128i, v_min: __m128i) -> __m128i {
            // SAFETY: SSE2 baseline.
            unsafe { self.nextuv().to_randi(v_range, v_min) }
        }

        #[inline(always)]
        pub(crate) fn randfv(&mut self, v_mult: __m128, v_min: __m128) -> __m128 {
            // SAFETY: SSE2 baseline.
            unsafe { self.nextuv().to_randf(v_mult, v_min) }
        }

        #[inline(always)]
        pub fn nextu(&mut self) -> [u32; SFC32X4] {
            bytemuck::cast(self.nextuv())
        }

        #[inline(always)]
        pub fn nextf(&mut self) -> [f32; SFC32X4] {
            self.nextu().map(|x| u2f_01!(f32, 32, x))
        }
    }

    pub(crate) const SFC32X8: usize = 8;

    /// 8-way SIMD implementation of SFC (Small Fast Chaotic) 32-bit RNG (AVX2).
    ///
    /// Lane layout as for [`Sfc32x4`].
    ///
    /// # Example
    /// ```no_run
    /// use urng::Sfc32x8;
    ///
    /// if std::arch::is_x86_feature_detected!("avx2") {
    ///     let mut rng = unsafe { Sfc32x8::new(1) };
    ///     let _ = rng.nextu();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Sfc32x8 {
        pub(crate) a: __m256i,
        pub(crate) b: __m256i,
        pub(crate) c: __m256i,
        pub(crate) counter: __m256i,
    }

    impl Sfc32x8 {
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx2` target feature
        /// (the type no longer needs AVX-512VL).
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
            let mut a = [0u32; SFC32X8];
            let mut b = [0u32; SFC32X8];
            let mut c = [0u32; SFC32X8];
            for i in 0..SFC32X8 {
                a[i] = seedgen.nextu();
                b[i] = seedgen.nextu();
                c[i] = seedgen.nextu();
            }
            unsafe {
                Self {
                    a: _mm256_loadu_si256(a.as_ptr() as *const _),
                    b: _mm256_loadu_si256(b.as_ptr() as *const _),
                    c: _mm256_loadu_si256(c.as_ptr() as *const _),
                    counter: _mm256_set1_epi32(1),
                }
            }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        pub(crate) fn nextuv(&mut self) -> __m256i {
            let tmp = _mm256_add_epi32(_mm256_add_epi32(self.a, self.b), self.counter);
            self.counter = _mm256_add_epi32(self.counter, _mm256_set1_epi32(1));
            self.a = _mm256_xor_si256(self.b, _mm256_srli_epi32(self.b, 9));
            self.b = _mm256_add_epi32(self.c, _mm256_slli_epi32(self.c, 3));
            self.c = _mm256_add_epi32(rotl8::<21, 11>(self.c), tmp);
            tmp
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        pub(crate) fn nextfv(&mut self) -> __m256 {
            unsafe { crate::_internal::simd_f01::u32x8(self.nextuv()) }
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        pub(crate) fn randiv(&mut self, v_range: __m256i, v_min: __m256i) -> __m256i {
            crate::rngv::to_randi_u32x8(self.nextuv(), v_range, v_min)
        }

        #[inline]
        #[target_feature(enable = "avx2")]
        pub(crate) fn randfv(&mut self, v_mult: __m256, v_min: __m256) -> __m256 {
            crate::rngv::to_randf_u32x8(self.nextuv(), v_mult, v_min)
        }

        #[inline(always)]
        pub fn nextu(&mut self) -> [u32; SFC32X8] {
            // SAFETY: a value exists only if `new`'s AVX2 requirement was met.
            bytemuck::cast(unsafe { self.nextuv() })
        }

        #[inline(always)]
        pub fn nextf(&mut self) -> [f32; SFC32X8] {
            self.nextu().map(|x| u2f_01!(f32, 32, x))
        }
    }

    pub(crate) const SFC32X16: usize = 16;

    /// 16-way SIMD implementation of SFC (Small Fast Chaotic) 32-bit RNG (AVX-512F).
    ///
    /// # Example
    /// ```no_run
    /// use urng::Sfc32x16;
    ///
    /// if std::arch::is_x86_feature_detected!("avx512f") {
    ///     let mut rng = unsafe { Sfc32x16::new(1) };
    ///     let _ = rng.nextu();
    /// }
    /// ```
    #[repr(C, align(64))]
    pub struct Sfc32x16 {
        pub(crate) a: __m512i,
        pub(crate) b: __m512i,
        pub(crate) c: __m512i,
        pub(crate) counter: __m512i,
    }

    impl Sfc32x16 {
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
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
            let mut a = [0u32; SFC32X16];
            let mut b = [0u32; SFC32X16];
            let mut c = [0u32; SFC32X16];
            for i in 0..SFC32X16 {
                a[i] = seedgen.nextu();
                b[i] = seedgen.nextu();
                c[i] = seedgen.nextu();
            }
            unsafe {
                Self {
                    a: _mm512_loadu_si512(a.as_ptr() as *const _),
                    b: _mm512_loadu_si512(b.as_ptr() as *const _),
                    c: _mm512_loadu_si512(c.as_ptr() as *const _),
                    counter: _mm512_set1_epi32(1),
                }
            }
        }

        #[inline]
        #[target_feature(enable = "avx512f")]
        pub(crate) fn nextuv(&mut self) -> __m512i {
            let tmp = _mm512_add_epi32(_mm512_add_epi32(self.a, self.b), self.counter);
            self.counter = _mm512_add_epi32(self.counter, _mm512_set1_epi32(1));
            self.a = _mm512_xor_si512(self.b, _mm512_srli_epi32(self.b, 9));
            self.b = _mm512_add_epi32(self.c, _mm512_slli_epi32(self.c, 3));
            self.c = _mm512_add_epi32(_mm512_rol_epi32::<21>(self.c), tmp);
            tmp
        }

        #[inline]
        #[target_feature(enable = "avx512f")]
        pub(crate) fn nextfv(&mut self) -> __m512 {
            unsafe { crate::_internal::simd_f01::u32x16(self.nextuv()) }
        }

        #[inline]
        #[target_feature(enable = "avx512f")]
        pub(crate) fn randiv(&mut self, v_range: __m512i, v_min: __m512i) -> __m512i {
            crate::rngv::to_randi_u32x16(self.nextuv(), v_range, v_min)
        }

        #[inline]
        #[target_feature(enable = "avx512f")]
        pub(crate) fn randfv(&mut self, v_mult: __m512, v_min: __m512) -> __m512 {
            crate::rngv::to_randf_u32x16(self.nextuv(), v_mult, v_min)
        }

        #[inline(always)]
        pub fn nextu(&mut self) -> [u32; SFC32X16] {
            // SAFETY: a value exists only if `new`'s AVX-512F requirement was met.
            unsafe { std::mem::transmute(self.nextuv()) }
        }

        pub fn nextf(&mut self) -> [f32; SFC32X16] {
            self.nextu().map(|x| u2f_01!(f32, 32, x))
        }
    }
}
#[cfg(test)]
mod tests {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    use std::arch::x86_64::*;

    use super::*;
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    use crate::SplitMix32;

    // (a stray `#[cfg(all(feature = "simd", target_arch = "x86_64"))]` used to hide this test in default builds)
    crate::safe_test! { Sfc32 }
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    crate::safe_test! { Sfc32x4 }

    /// PractRand reference vector (as published in `rand_sfc`): a = 0, b = 2,
    /// c = 1, counter = 1, 15 warm-up steps, then these 16 outputs.
    #[test]
    fn sfc32_practrand_vector() {
        let mut r = Sfc32 { a: wrap!(0), b: wrap!(2), c: wrap!(1), counter: wrap!(1) };
        for _ in 0..15 {
            let _ = r.nextu();
        }
        let expected: [u32; 16] = [
            0x03B80BB8, 0xA87DBC7E, 0x1787178C, 0x4C7B7234, 0xC65DADE2, 0x2C692349, 0xF52C2153,
            0xDF098072, 0x9D49B03C, 0x9562381A, 0xC9B41738, 0x64B75E54, 0x36CE9B32, 0xF106947E,
            0x0AFC726B, 0x549BBC87,
        ];
        for e in expected {
            assert_eq!(r.nextu(), e);
        }
    }
    /// Lane-vs-scalar equality for the AVX2 / AVX-512 versions, including the
    /// ranged paths. (Replaces a determinism-only test that ran AVX-512 code
    /// without checking the CPU.) REVIEW B8: Sfc32x8 now needs only AVX2.
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn sfc32_wide_simd_lanes_match_scalar() {
        fn lanes(seed: u32, n: usize) -> Vec<Sfc32> {
            let mut sg = SplitMix32::new(seed);
            (0..n)
                .map(|_| Sfc32 { a: sg.nextu().into(), b: sg.nextu().into(), c: sg.nextu().into(), counter: 1.into() })
                .collect()
        }
        let (min, max) = (i32::MIN, i32::MAX - 5);
        let scale = crate::rngv::scale32(min, max);
        if std::arch::is_x86_feature_detected!("avx2") {
            let mut v = unsafe { Sfc32x8::new(4) };
            let mut s = lanes(4, 8);
            for _ in 0..5000 {
                let got = v.nextu();
                for i in 0..8 {
                    assert_eq!(got[i], s[i].nextu());
                }
                let r: [i32; 8] = unsafe {
                    bytemuck::cast(v.randiv(_mm256_set1_epi32(scale), _mm256_set1_epi32(min)))
                };
                for i in 0..8 {
                    assert_eq!(r[i], s[i].randi(min, max));
                }
            }
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            let mut v = unsafe { Sfc32x16::new(4) };
            let mut s = lanes(4, 16);
            for _ in 0..5000 {
                let got = v.nextu();
                for i in 0..16 {
                    assert_eq!(got[i], s[i].nextu());
                }
                let r: [i32; 16] = unsafe {
                    std::mem::transmute(v.randiv(_mm512_set1_epi32(scale), _mm512_set1_epi32(min)))
                };
                for i in 0..16 {
                    assert_eq!(r[i], s[i].randi(min, max));
                }
            }
        }
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    fn scalar_lanes(seed: u32) -> [Sfc32; SFC32X4] {
        let mut seedgen = SplitMix32::new(seed);
        std::array::from_fn(|_| Sfc32 {
            a: seedgen.nextu().into(),
            b: seedgen.nextu().into(),
            c: seedgen.nextu().into(),
            counter: 1.into(),
        })
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn sfc32x4_lanes_match_scalar() {
        let mut vector = Sfc32x4::new(0);
        let mut scalars = scalar_lanes(0);

        for _ in 0..8 {
            let got = vector.nextu();
            let want: [u32; SFC32X4] = std::array::from_fn(|i| scalars[i].nextu());
            assert_eq!(got, want);
        }
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn sfc32x4_randiv_randfv_match_scalar() {
        let (min_i, max_i) = (-5i32, 7i32);
        let (min_f, max_f) = (-2.0f32, 3.0f32);
        let v_range = unsafe { _mm_set1_epi32((max_i as i64 - min_i as i64 + 1) as u32 as i32) };
        let v_min = unsafe { _mm_set1_epi32(min_i) };
        let v_mult = unsafe { _mm_set1_ps(max_f - min_f) };
        let v_minf = unsafe { _mm_set1_ps(min_f) };

        let mut vector = Sfc32x4::new(1);
        let mut scalars = scalar_lanes(1);
        for _ in 0..8 {
            let got: [i32; SFC32X4] = unsafe { std::mem::transmute(vector.randiv(v_range, v_min)) };
            let want: [i32; SFC32X4] = std::array::from_fn(|i| scalars[i].randi(min_i, max_i));
            assert_eq!(got, want);
        }

        let mut vector = Sfc32x4::new(2);
        let mut scalars = scalar_lanes(2);
        for _ in 0..8 {
            let got: [f32; SFC32X4] = unsafe { std::mem::transmute(vector.randfv(v_mult, v_minf)) };
            let want: [f32; SFC32X4] = std::array::from_fn(|i| scalars[i].randf(min_f, max_f));
            assert_eq!(got, want);
        }

        let mut vector = Sfc32x4::new(3);
        let mut scalars = scalar_lanes(3);
        for _ in 0..8 {
            let got: [f32; SFC32X4] = unsafe { std::mem::transmute(vector.nextfv()) };
            let want: [f32; SFC32X4] = std::array::from_fn(|i| scalars[i].nextf());
            assert_eq!(got, want);
        }
    }
}
