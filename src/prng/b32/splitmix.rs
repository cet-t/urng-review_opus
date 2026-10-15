use wrapn::{wrap, wu32};

use crate::rng::Rng;

/// SplitMix32 32-bit RNG implementation.
///
/// Fast 32-bit finalizer-based PRNG commonly used to seed other generators.
/// Uses a single 32-bit state word advanced by the golden-ratio constant.
///
/// # Example
/// ```
/// use urng::{Rng, SplitMix32};
///
/// let mut rng = SplitMix32::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct SplitMix32 {
    state: wu32,
}

const A: u64 = 0xFF51_AFD7_ED55_8CCD;
const B: u64 = 0xC4CE_B9FE_1A85_EC53;

impl SplitMix32 {
    /// Creates a new `SplitMix32` instance with the given seed.
    pub const fn new(seed: u32) -> Self {
        Self {
            state: wrap!(seed),
        }
    }

    #[inline(always)]
    pub(crate) const fn nextu_const(&mut self) -> u32 {
        self.state.0.0 = self.state.0.0.wrapping_add(0x9E3779B9);

        let mut z = self.state.0.0 as u64;
        z = (z ^ (z >> 16)).wrapping_mul(A);
        z = (z ^ (z >> 16)).wrapping_mul(B);
        (z ^ (z >> 16)) as u32
    }
}

impl Rng for SplitMix32 {
    type Word = u32;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        self.state += 0x9E3779B9;

        let mut z = self.state.cast::<u64>();
        z = (z ^ (z >> 16)) * A;
        z = (z ^ (z >> 16)) * B;
        *(z ^ (z >> 16)).cast::<u32>()
    }
}

#[cfg(feature = "simd")]
pub use simd::*;

#[cfg(feature = "simd")]
pub mod simd {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    #[cfg(target_arch = "x86_64")]
    use crate::RngV;

    pub const SPLITMIX32X16: usize = 16;
    pub const SPLITMIX32X16_PAR_CHUNK: usize = 8192;
    pub const SPLITMIX32_GAMMA: u32 = 0x9E37_79B9;

    /// 16-way SIMD implementation of SplitMix32 32-bit RNG.
    /// This implementation uses AVX-512F instructions to generate 16 random numbers in parallel.
    ///
    /// # Example
    /// ```no_run
    /// use urng::{RngV, SplitMix32x16};
    /// unsafe {
    ///     let mut rng = SplitMix32x16::new(1);
    ///     let _ = rng.nextuv();
    /// }
    /// ```
    #[cfg(target_arch = "x86_64")]
    #[repr(C, align(64))]
    pub struct SplitMix32x16 {
        pub(crate) state: __m512i,
    }

    #[cfg(target_arch = "x86_64")]
    impl SplitMix32x16 {
        /// Creates a new `SplitMix32x16` instance with the given seed.
        ///
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
        #[target_feature(enable = "avx512f")]
        pub unsafe fn new(seed: u32) -> Self {
            // Lane i starts at seed + (i + 1) * gamma and every lane advances by
            // 16 * gamma, so the 16 lanes interleave one Weyl sequence without
            // overlap: output element e is fmix32(seed + (e + 1) * gamma), exactly
            // the stream of `urng::wide::SplitMix32x16`. (Previously the lanes
            // started at unrelated values on overlapping cycles.)
            let s: [u32; SPLITMIX32X16] = std::array::from_fn(|i| {
                seed.wrapping_add(SPLITMIX32_GAMMA.wrapping_mul(i as u32 + 1))
            });

            unsafe {
                Self {
                    state: _mm512_loadu_si512(s.as_ptr() as _),
                }
            }
        }

        /// Murmur3 `fmix32` finalizer on 16 lanes.
        ///
        /// The previous version replaced both multiplications by additions, which
        /// left the high bits of consecutive outputs almost perfectly predictable.
        ///
        /// # Safety
        ///
        /// The caller must ensure the CPU supports the `avx512f` target feature.
        #[target_feature(enable = "avx512f")]
        pub unsafe fn compute(state: __m512i) -> __m512i {
            let c1 = _mm512_set1_epi32(0x85EB_CA6Bu32 as i32);
            let c2 = _mm512_set1_epi32(0xC2B2_AE35u32 as i32);

            let mut z = state;
            z = _mm512_xor_si512(z, _mm512_srli_epi32(z, 16));
            z = _mm512_mullo_epi32(z, c1);
            z = _mm512_xor_si512(z, _mm512_srli_epi32(z, 13));
            z = _mm512_mullo_epi32(z, c2);
            _mm512_xor_si512(z, _mm512_srli_epi32(z, 16))
        }
    }

    #[cfg(target_arch = "x86_64")]
    impl RngV for SplitMix32x16 {
        type Word = __m512i;

        fn nextuv(&mut self) -> Self::Word {
            unsafe {
                let v = Self::compute(self.state);
                self.state = _mm512_add_epi32(
                    self.state,
                    _mm512_set1_epi32(SPLITMIX32_GAMMA.wrapping_mul(SPLITMIX32X16 as u32) as i32),
                );
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! { SplitMix32 }

    /// REVIEW B10: the AVX-512 finalizer used additions instead of
    /// multiplications and the lanes overlapped. Lanes must now reproduce the
    /// interleaved reference stream, and the top bits of consecutive outputs
    /// must not follow a fixed increment.
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn splitmix32x16_matches_reference_stream() {
        use crate::RngV;
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return;
        }
        let mut v = unsafe { SplitMix32x16::new(77) };
        let mut lane0 = Vec::new();
        for t in 0..20_000u32 {
            let got: [u32; 16] = unsafe { std::mem::transmute(v.nextuv()) };
            for i in 0..16 {
                assert_eq!(got[i], crate::_reftest::fmix32_stream(77, 16 * t + i as u32));
            }
            lane0.push(got[0]);
        }
        let diffs: std::collections::HashSet<u32> = lane0
            .windows(2)
            .map(|w| ((w[1] >> 19).wrapping_sub(w[0] >> 19)) & 0x1FFF)
            .collect();
        assert!(diffs.len() > 7000, "top-13-bit increments take only {} values", diffs.len());
    }
}
