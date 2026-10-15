use wrapn::{wrap, wu64};

use crate::prng::b64::SplitMix64;
use crate::rng::Rng;

// --- Sfc64 ---

/// SFC (Small Fast Chaotic) 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Sfc64};
///
/// let mut rng = Sfc64::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Sfc64 {
    a: wu64,
    b: wu64,
    c: wu64,
    counter: wu64,
}

impl Sfc64 {
    /// Creates a new `Sfc64` instance with the given seed.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            a: wrap!(seedgen.nextu_const()),
            b: wrap!(seedgen.nextu_const()),
            c: wrap!(seedgen.nextu_const()),
            counter: wrap!(1),
        }
    }
}

impl Rng for Sfc64 {
    type Word = u64;

    #[inline(always)]
    fn nextu(&mut self) -> Self::Word {
        let res = self.a + self.b + self.counter;
        self.a = self.b ^ (self.b >> 11);
        self.b = self.c + (self.c << 3);
        // PractRand sfc64: c = rotl(c, 24) + tmp (the old code dropped c entirely)
        self.c = self.c.rotate_left(24) + res;

        self.counter += 1;
        *res
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub mod simd {
    use std::arch::x86_64::*;

    use crate::_internal::{i2f_bits, u2f_01};
    use crate::prng::b64::SplitMix64;

    // --- Sfc64x4 (AVX2) ---

    /// An 8-way SIMD SFC64 generator using AVX-512F (lane i = scalar Sfc64 seeded
    /// with SplitMix64 outputs 3i, 3i+1, 3i+2).
    ///
    /// # Example
    /// ```no_run
    /// use urng::Sfc64x8;
    ///
    /// if !std::arch::is_x86_feature_detected!("avx512f") { return; }
    /// let mut rng = unsafe { Sfc64x8::new(0) };
    /// let _ = unsafe { rng.nextu() };
    /// ```
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[repr(C, align(64))]
    pub struct Sfc64x8 {
        a: __m512i,
        b: __m512i,
        c: __m512i,
        counter: __m512i,
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    impl Sfc64x8 {
        /// Creates a new `Sfc64x8` from 8 independent seeds.
        ///
        /// # Safety
        /// The CPU must support AVX-512F (this is a `#[target_feature]` constructor,
        /// so calling it from ordinary code requires `unsafe`).
        #[inline]
        #[target_feature(enable = "avx512f")]
        pub fn new(seed: u64) -> Self {
            let mut a = [0u64; 8];
            let mut b = [0u64; 8];
            let mut c = [0u64; 8];
            let mut sg = SplitMix64::new(seed);
            for i in 0..8 {
                a[i] = sg.nextu_const();
                b[i] = sg.nextu_const();
                c[i] = sg.nextu_const();
            }
            unsafe {
                Self {
                    a: _mm512_loadu_si512(a.as_ptr() as *const __m512i),
                    b: _mm512_loadu_si512(b.as_ptr() as *const __m512i),
                    c: _mm512_loadu_si512(c.as_ptr() as *const __m512i),
                    counter: _mm512_set1_epi64(1),
                }
            }
        }

        /// Generates 8 random `u64` values (one per lane).
        ///
        /// # Safety
                /// Requires AVX512 support.
        #[inline]
        #[target_feature(enable = "avx512f")]
        pub unsafe fn nextu(&mut self) -> [u64; 8] {
            unsafe {
                let one = _mm512_set1_epi64(1);

                // res = a + b + counter
                let res = _mm512_add_epi64(_mm512_add_epi64(self.a, self.b), self.counter);

                // a = b ^ (b >> 11)
                self.a = _mm512_xor_si512(self.b, _mm512_srli_epi64(self.b, 11));

                // b = c + (c << 3)
                self.b = _mm512_add_epi64(self.c, _mm512_slli_epi64(self.c, 3));

                // c = rotl(c, 24) + res
                self.c = _mm512_add_epi64(_mm512_rol_epi64(self.c, 24), res);

                // counter += 1
                self.counter = _mm512_add_epi64(self.counter, one);

                // Store 8 results
                let mut out = [0u64; 8];
                _mm512_storeu_si512(out.as_mut_ptr() as *mut __m512i, res);
                out
            }
        }

        /// Generates 8 random `f64` values in [0, 1) and writes them to `out`.
        ///
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
        #[inline]
        #[target_feature(enable = "avx512f")]
        pub unsafe fn nextf(&mut self) -> [f64; 8] {
            unsafe {
                let u = self.nextu();
                let mut out = [0f64; 8];
                for i in 0..8 {
                    out[i] = u2f_01!(f64, 64, u[i]);
                }
                out
            }
        }

        /// Generates 8 random `i64` values in [min, max].
        ///
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
        #[inline]
        #[target_feature(enable = "avx512f")]
        pub unsafe fn randi(&mut self, min: i64, max: i64) -> [i64; 8] {
            unsafe {
                let u = self.nextu();
                u.map(|w| crate::rng::Word::to_randi(w, min, max))
            }
        }

        /// Generates 8 random `f64` values in [min, max) and writes them to `out`.
        ///
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
        #[inline]
        #[target_feature(enable = "avx512f")]
        pub unsafe fn randf(&mut self, min: f64, max: f64) -> [f64; 8] {
            unsafe {
                let u = self.nextu();
                u.map(|w| crate::rng::Word::to_randf(w, min, max))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! { Sfc64 }

    /// PractRand reference vector (as published in `rand_sfc`): a = b = c = 1,
    /// counter = 1, 18 warm-up steps, then these 16 outputs.
    #[test]
    fn sfc64_practrand_vector() {
        let mut r = Sfc64 { a: wrap!(1), b: wrap!(1), c: wrap!(1), counter: wrap!(1) };
        for _ in 0..18 {
            let _ = r.nextu();
        }
        let expected: [u64; 16] = [
            0xAD6FDC729FEEF3C1, 0x2A20433D733F77D5, 0x0310E21369647420, 0x331A176BC71DCABC,
            0x53118F35C2494D94, 0xA3A99DE7E77E16BF, 0xA7B1B70A3E59A1FF, 0x8E1127B28667EB3C,
            0x3FC589DC124CF6E8, 0x81E0EAAACEB81D81, 0x79F534652D262DF6, 0x87F70C8214E186C5,
            0x67AF9C007B825917, 0x5134AEC9998D8629, 0x205AA24994068634, 0x1C762918DBA3E139,
        ];
        for e in expected {
            assert_eq!(r.nextu(), e);
        }
        let mut g = Sfc64::new(9);
        let mut st = [*g.a, *g.b, *g.c, *g.counter];
        for _ in 0..100_000 {
            assert_eq!(g.nextu(), crate::_reftest::sfc64(&mut st));
        }
    }

    /// Every lane of the AVX-512 version equals the scalar generator.
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn sfc64x8_lanes_match_scalar() {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return;
        }
        let mut sg = SplitMix64::new(5);
        let mut lanes: Vec<Sfc64> = (0..8)
            .map(|_| Sfc64 {
                a: wrap!(sg.nextu_const()),
                b: wrap!(sg.nextu_const()),
                c: wrap!(sg.nextu_const()),
                counter: wrap!(1),
            })
            .collect();
        let mut v = unsafe { Sfc64x8::new(5) };
        for _ in 0..10_000 {
            let got = unsafe { v.nextu() };
            for (i, l) in lanes.iter_mut().enumerate() {
                assert_eq!(got[i], l.nextu());
            }
        }
    }
}

impl Sfc64 {
    /// Builds the generator from a raw `(a, b, c, counter)` state (test helper).
    #[cfg(all(test, feature = "cabi"))]
    pub(crate) fn from_state(s: [u64; 4]) -> Self {
        Self { a: wrap!(s[0]), b: wrap!(s[1]), c: wrap!(s[2]), counter: wrap!(s[3]) }
    }
}
