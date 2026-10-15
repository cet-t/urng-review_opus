//! Raw-vector RNG traits (`std::arch` SIMD generators).
//!
//! # Safety model
//!
//! The 256/512-bit vector types need CPU features beyond the x86_64 baseline
//! (`__m256i`: AVX2, `__m512i`: AVX-512F), and values of these types can be
//! created in safe code (e.g. with `bytemuck`). The conversion methods of
//! [`WordV`] are therefore `unsafe`: the caller must guarantee the features.
//! [`RngV`] is an `unsafe trait`: an implementor guarantees that a value of the
//! implementing type can only exist after the required features were asserted
//! (its constructors are `unsafe` or `#[target_feature]`), which is what makes
//! the safe provided methods sound.

use std::arch::x86_64::*;

use crate::_internal::simd_f01;

mod sealed {
    use std::arch::x86_64::{__m128i, __m256i, __m512i};

    pub trait Sealed {}
    impl Sealed for __m128i {}
    impl Sealed for __m256i {}
    impl Sealed for __m512i {}
}

/// Lane-wise conversions of a raw 32-bit-lane vector.
pub trait WordV: sealed::Sealed + Copy + Sized {
    /// The integer vector type (same as `Self`).
    type Int;
    /// The matching `f32` vector type.
    type Float;

    /// Maps every lane onto `[0, 1)`.
    ///
    /// # Safety
    ///
    /// The CPU must support the vector width (`__m256i`: AVX2, `__m512i`: AVX-512F).
    #[must_use]
    unsafe fn to_f01(self) -> Self::Float;

    /// Maps every lane `x` to `min + ((x * scale) >> 32)`, i.e. onto
    /// `[min, min + scale)`. A `scale` lane of `0` means `2^32` (the full range).
    /// Multiply-shift without rejection: the bias is at most `scale / 2^32`.
    ///
    /// # Safety
    ///
    /// Same as [`WordV::to_f01`].
    #[must_use]
    unsafe fn to_randi(self, scale: Self::Int, min: Self::Int) -> Self::Int;

    /// Maps every lane onto `min + scale * [0, 1)`.
    ///
    /// # Safety
    ///
    /// Same as [`WordV::to_f01`].
    #[must_use]
    unsafe fn to_randf(self, scale: Self::Float, min: Self::Float) -> Self::Float;
}

impl WordV for __m128i {
    type Int = Self;
    type Float = __m128;

    #[inline(always)]
    unsafe fn to_f01(self) -> Self::Float {
        unsafe { simd_f01::u32x4(self) }
    }

    /// (The previous version returned the low half of the product in lanes 1
    /// and 3 because of a wrong `_mm_shuffle_ps` immediate.)
    #[inline(always)]
    unsafe fn to_randi(self, scale: Self::Int, min: Self::Int) -> Self::Int {
        unsafe {
            let hi_mask = _mm_set1_epi64x(0xFFFF_FFFF_0000_0000u64 as i64);
            let even = _mm_srli_epi64(_mm_mul_epu32(self, scale), 32);
            let odd = _mm_and_si128(
                _mm_mul_epu32(_mm_srli_epi64(self, 32), _mm_srli_epi64(scale, 32)),
                hi_mask,
            );
            let prod = _mm_or_si128(even, odd);
            // scale == 0 encodes 2^32: (x * 2^32) >> 32 == x
            let full = _mm_cmpeq_epi32(scale, _mm_setzero_si128());
            let r = _mm_or_si128(_mm_and_si128(full, self), _mm_andnot_si128(full, prod));
            _mm_add_epi32(r, min)
        }
    }

    #[inline(always)]
    unsafe fn to_randf(self, scale: Self::Float, min: Self::Float) -> Self::Float {
        unsafe { _mm_add_ps(_mm_mul_ps(simd_f01::u32x4(self), scale), min) }
    }
}

/// AVX2 only (the previous version used `_mm256_mask_blend_epi32`, AVX-512VL).
#[target_feature(enable = "avx2")]
#[inline]
pub(crate) fn to_randi_u32x8(x: __m256i, scale: __m256i, min: __m256i) -> __m256i {
    let even = _mm256_srli_epi64(_mm256_mul_epu32(x, scale), 32);
    let odd = _mm256_mul_epu32(_mm256_srli_epi64(x, 32), _mm256_srli_epi64(scale, 32));
    let prod = _mm256_blend_epi32::<0b1010_1010>(even, odd);
    let full = _mm256_cmpeq_epi32(scale, _mm256_setzero_si256());
    _mm256_add_epi32(_mm256_blendv_epi8(prod, x, full), min)
}

#[target_feature(enable = "avx2")]
#[inline]
pub(crate) fn to_randf_u32x8(x: __m256i, scale: __m256, min: __m256) -> __m256 {
    let f = unsafe { simd_f01::u32x8(x) };
    _mm256_add_ps(_mm256_mul_ps(f, scale), min)
}

impl WordV for __m256i {
    type Int = Self;
    type Float = __m256;

    #[inline(always)]
    unsafe fn to_f01(self) -> Self::Float {
        unsafe { simd_f01::u32x8(self) }
    }

    #[inline(always)]
    unsafe fn to_randi(self, scale: Self::Int, min: Self::Int) -> Self::Int {
        unsafe { to_randi_u32x8(self, scale, min) }
    }

    #[inline(always)]
    unsafe fn to_randf(self, scale: Self::Float, min: Self::Float) -> Self::Float {
        unsafe { to_randf_u32x8(self, scale, min) }
    }
}

#[target_feature(enable = "avx512f")]
#[inline]
pub(crate) fn to_randi_u32x16(x: __m512i, scale: __m512i, min: __m512i) -> __m512i {
    let even = _mm512_srli_epi64(_mm512_mul_epu32(x, scale), 32);
    let odd = _mm512_mul_epu32(_mm512_srli_epi64(x, 32), _mm512_srli_epi64(scale, 32));
    let prod = _mm512_mask_blend_epi32(0b1010_1010_1010_1010, even, odd);
    let full = _mm512_cmpeq_epi32_mask(scale, _mm512_setzero_si512());
    _mm512_add_epi32(_mm512_mask_blend_epi32(full, prod, x), min)
}

#[target_feature(enable = "avx512f")]
#[inline]
pub(crate) fn to_randf_u32x16(x: __m512i, scale: __m512, min: __m512) -> __m512 {
    let f = unsafe { simd_f01::u32x16(x) };
    _mm512_add_ps(_mm512_mul_ps(f, scale), min)
}

impl WordV for __m512i {
    type Int = Self;
    type Float = __m512;

    #[inline(always)]
    unsafe fn to_f01(self) -> Self::Float {
        unsafe { simd_f01::u32x16(self) }
    }

    #[inline(always)]
    unsafe fn to_randi(self, scale: Self::Int, min: Self::Int) -> Self::Int {
        unsafe { to_randi_u32x16(self, scale, min) }
    }

    #[inline(always)]
    unsafe fn to_randf(self, scale: Self::Float, min: Self::Float) -> Self::Float {
        unsafe { to_randf_u32x16(self, scale, min) }
    }
}

/// The 32-bit lane scale for an inclusive range `[min, max]` as used by
/// [`WordV::to_randi`] (`0` encodes the full 2^32 range).
#[inline]
#[cfg_attr(not(feature = "cabi"), allow(dead_code))] // used by the C ABI kernels
pub(crate) fn scale32(min: i32, max: i32) -> i32 {
    (max as i64 - min as i64 + 1) as u32 as i32
}

/// A SIMD generator producing one raw vector per call.
///
/// # Safety
///
/// Implementors guarantee that every value of the implementing type was created
/// only after the CPU features needed by its `Word` and by `nextuv` were
/// verified or asserted by the caller (through an `unsafe` or
/// `#[target_feature]` constructor), and that no safe API creates one otherwise.
pub unsafe trait RngV {
    /// The raw vector type produced per call.
    type Word: WordV;

    /// Generates the next raw vector.
    fn nextuv(&mut self) -> Self::Word;

    /// Generates a vector of floats in `[0, 1)`.
    #[inline(always)]
    fn nextfv(&mut self) -> <Self::Word as WordV>::Float {
        // SAFETY: the `RngV` contract guarantees the features of `Self::Word`.
        unsafe { self.nextuv().to_f01() }
    }

    /// Generates lanes in `[min, min + scale)` (see [`WordV::to_randi`]).
    #[inline(always)]
    fn randiv(
        &mut self,
        scale: <Self::Word as WordV>::Int,
        min: <Self::Word as WordV>::Int,
    ) -> <Self::Word as WordV>::Int {
        // SAFETY: as above.
        unsafe { self.nextuv().to_randi(scale, min) }
    }

    /// Generates lanes in `min + scale * [0, 1)`.
    #[inline(always)]
    fn randfv(
        &mut self,
        scale: <Self::Word as WordV>::Float,
        min: <Self::Word as WordV>::Float,
    ) -> <Self::Word as WordV>::Float {
        // SAFETY: as above.
        unsafe { self.nextuv().to_randf(scale, min) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(x: u32, scale: u32, min: i32) -> i32 {
        let s = if scale == 0 { 1u64 << 32 } else { scale as u64 };
        (((x as u64 * s) >> 32) as i32).wrapping_add(min)
    }

    /// REVIEW B13: lanes 1 and 3 of the `__m128i` version returned the low half
    /// of the product, and a full-range scale (2^32) could not be expressed.
    #[test]
    fn to_randi_matches_scalar_for_all_widths() {
        let xs: [u32; 16] =
            std::array::from_fn(|i| (i as u32).wrapping_mul(0x9E3779B9) ^ 0xFFFF_0000);
        for &(scale, min) in &[(10u32, 0i32), (13, -5), (0, i32::MIN), (u32::MAX, 7), (1, 3)] {
            let want: [i32; 16] = std::array::from_fn(|i| scalar(xs[i], scale, min));
            unsafe {
                let v4: __m128i = bytemuck::cast([xs[0], xs[1], xs[2], xs[3]]);
                let r4: [i32; 4] =
                    bytemuck::cast(v4.to_randi(_mm_set1_epi32(scale as i32), _mm_set1_epi32(min)));
                assert_eq!(r4[..], want[..4], "x4 scale {scale}");
                if std::arch::is_x86_feature_detected!("avx2") {
                    let v8: __m256i = bytemuck::cast(<[u32; 8]>::try_from(&xs[..8]).unwrap());
                    let r8: [i32; 8] = bytemuck::cast(
                        v8.to_randi(_mm256_set1_epi32(scale as i32), _mm256_set1_epi32(min)),
                    );
                    assert_eq!(r8[..], want[..8], "x8 scale {scale}");
                }
                if std::arch::is_x86_feature_detected!("avx512f") {
                    let v16: __m512i = std::mem::transmute(xs);
                    let r16: [i32; 16] = std::mem::transmute(
                        v16.to_randi(_mm512_set1_epi32(scale as i32), _mm512_set1_epi32(min)),
                    );
                    assert_eq!(r16, want, "x16 scale {scale}");
                }
            }
        }
        assert_eq!(scale32(i32::MIN, i32::MAX), 0);
        assert_eq!(scale32(-5, 7), 13);
    }
}
