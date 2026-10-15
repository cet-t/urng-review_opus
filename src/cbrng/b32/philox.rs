#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use std::arch::x86_64::*;

use wrapn::{wrap, wu32, wusize};

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
use crate::_internal::{i2f_bits, u2f_01};
use crate::prng::b32::SplitMix32;
use crate::rng::Rng;

// --- Philox32 ---

/// A Philox 4x32 random number generator.
///
/// This is a counter-based RNG suitable for parallel applications. Implements
/// [`Rng`] directly: each call to [`Rng::nextu`] hands out one `u32` from
/// an internal 4-word buffer, recomputing a fresh block every 4th call.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Philox32};
///
/// let mut rng = Philox32::new(1);
/// let _: u32 = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Philox32 {
    pub(crate) c: [wu32; 4],
    pub(crate) k: [wu32; 2],
    pub(crate) buf: [wu32; 4],
    pub(crate) pos: wusize,
}

impl Philox32 {
    /// Creates a new `Philox32` instance seeded with the given value.
    pub fn new(seed: u32) -> Self {
        let mut seedgen = SplitMix32::new(seed);
        Self {
            c: wrap![
                seedgen.nextu(),
                seedgen.nextu(),
                seedgen.nextu(),
                seedgen.nextu(),
            ],
            k: wrap![seedgen.nextu(), seedgen.nextu()],
            buf: wrap![0; 4],
            pos: 4.into(),
        }
    }

    /// Computes Philox4x32-10 output from counter and key values (pure function).
    ///
    /// Ten rounds with the Weyl key schedule `k += (W0, W1)` between rounds, as in
    /// Random123 (REVIEW B12: the previous code ran 20 rounds and multiplied the
    /// key by the Weyl constants, so it was not Philox).
    #[inline(always)]
    pub(crate) fn compute(c: [wu32; 4], k: [wu32; 2]) -> [wu32; 4] {
        const M0: u64 = 0xD2511F53;
        const M1: u64 = 0xCD9E8D57;
        const W0: u32 = 0x9E3779B9;
        const W1: u32 = 0xBB67AE85;

        let mut x = c.map(|w| *w);
        let mut key = k.map(|w| *w);
        for r in 0..10 {
            if r > 0 {
                key[0] = key[0].wrapping_add(W0);
                key[1] = key[1].wrapping_add(W1);
            }
            let p0 = x[0] as u64 * M0;
            let p1 = x[2] as u64 * M1;
            x = [
                (p1 >> 32) as u32 ^ x[1] ^ key[0],
                p1 as u32,
                (p0 >> 32) as u32 ^ x[3] ^ key[1],
                p0 as u32,
            ];
        }
        wrap![x[0], x[1], x[2], x[3]]
    }
    /// Generates the next block of 4 random `u32` values in one call.
    ///
    /// This is the raw bulk-generation path (used internally to refill the
    /// scalar [`Rng::nextu`] buffer, and available directly for
    /// throughput-sensitive callers that want the whole block at once).
    #[inline(always)]
    pub fn next_raw(&mut self) -> [wu32; 4] {
        let out = Self::compute(self.c, self.k);
        self.c[0] += 1;
        if self.c[0] == 0 {
            self.c[1] += 1;
            if self.c[1] == 0 {
                self.c[2] += 1;
                if self.c[2] == 0 {
                    self.c[3] += 1;
                }
            }
        }
        out
    }
}

crate::_internal::impl_ring_rng32!(Philox32, 4, next_raw);

// --- Philox32x4-10 x4 ---

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[allow(non_upper_case_globals)]
pub const PHILOX32x16: usize = 16;

/// A Philox 4x32x4 random number generator.
///
/// This is a counter-based RNG suitable for parallel applications.
///
/// # Example
/// ```no_run
/// use urng::Philox32x4x4;
///
/// unsafe {
///     let mut rng = Philox32x4x4::new(12345);
///     let _ = rng.nextu();
/// }
/// ```
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[repr(C, align(64))]
pub struct Philox32x4x4 {
    pub(crate) c: __m512i,
    pub(crate) k: __m512i,
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
impl Philox32x4x4 {
    /// Creates a new `Philox32x4x4` instance seeded with the given value.
    /// Requires AVX-512F support.
    ///
    /// # Safety
    ///
    /// Must only be called on a CPU that supports AVX-512F.
    #[target_feature(enable = "avx512f")]
    pub fn new(seed: u32) -> Self {
        let mut c = [0; PHILOX32x16];
        let mut k = [0; PHILOX32x16];

        let mut seedgen = SplitMix32::new(seed);
        c.iter_mut().for_each(|c| *c = seedgen.nextu());

        // [k0, 0, k1, 0]
        (0..PHILOX32x16).step_by(2).for_each(|i| {
            k[i] = seedgen.nextu();
        });

        unsafe {
            Self {
                c: _mm512_loadu_si512(c.as_ptr() as *const _),
                k: _mm512_loadu_si512(k.as_ptr() as *const _),
            }
        }
    }

    /// Philox4x32-10 on four independent blocks at once. Each 128-bit lane holds
    /// one block `[x0, x1, x2, x3]`; `k` holds `[k0, 0, k1, 0]` per lane.
    ///
    /// REVIEW B12: the previous code multiplied `x2` by M0 instead of M1 (the
    /// multiplier was `set1_epi64(M1:M0)`, whose low half is M0 in every 64-bit
    /// lane), bumped the zero slot and `k1` with the wrong Weyl constants, and
    /// ran 20 rounds.
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub(crate) fn block(c: __m512i, k: __m512i) -> __m512i {
        // even 64-bit lanes multiply x0 by M0, odd lanes multiply x2 by M1
        let m = _mm512_set_epi64(
            0xCD9E8D57, 0xD2511F53, 0xCD9E8D57, 0xD2511F53, 0xCD9E8D57, 0xD2511F53, 0xCD9E8D57,
            0xD2511F53,
        );
        // Weyl increments land on k0 (word 0) and k1 (word 2) only
        let w = _mm512_set4_epi32(0, 0xBB67AE85u32 as i32, 0, 0x9E3779B9u32 as i32);
        let mut x = c;
        let mut key = k;
        for r in 0..10 {
            if r > 0 {
                key = _mm512_add_epi32(key, w);
            }
            // [lo0, hi0, lo1, hi1] -> [hi1, lo1, hi0, lo0]
            let prod = _mm512_shuffle_epi32::<0x1B>(_mm512_mul_epu32(x, m));
            // [x1, 0, x3, 0]
            let x_shift = _mm512_srli_epi64::<32>(x);
            x = _mm512_xor_si512(prod, _mm512_xor_si512(x_shift, key));
        }
        x
    }

    /// Adds `n` to each lane's 128-bit block counter.
    #[target_feature(enable = "avx512f")]
    #[inline]
    pub(crate) fn add_blocks(c: __m512i, n: u128) -> __m512i {
        // SAFETY: both types are 64 plain bytes; every bit pattern is valid.
        let lanes: [u128; 4] = unsafe { std::mem::transmute(c) };
        unsafe { std::mem::transmute(lanes.map(|l| l.wrapping_add(n))) }
    }

    #[target_feature(enable = "avx512f")]
    pub(crate) fn compute(&mut self) -> [u32; PHILOX32x16] {
        // SAFETY: 64 plain bytes either way.
        unsafe { std::mem::transmute(Self::block(self.c, self.k)) }
    }

    /// Generates the next block of random numbers.
    ///
    /// # Safety
    ///
    /// The caller must ensure the CPU supports the `avx512f` target feature.
    #[target_feature(enable = "avx512f")]
    pub fn nextu(&mut self) -> [u32; PHILOX32x16] {
        let out = self.compute();
        self.c = Self::add_blocks(self.c, 1);
        out
    }
    /// Generates 16 random `f32` values in the range [0, 1) using AVX-512.
    ///
    /// # Safety
    ///
    /// The caller must ensure the CPU supports the `avx512f` target feature.
    #[target_feature(enable = "avx512f")]
    pub fn nextf(&mut self) -> [f32; PHILOX32x16] {
        self.nextu().map(|x| u2f_01!(f32, 32, x))
    }

    /// Generates a random `i32` value in the range [min, max].
    ///
    /// # Safety
    ///
    /// The caller must ensure the CPU supports the `avx512f` target feature.
    #[target_feature(enable = "avx512f")]
    pub fn randi(&mut self, min: i32, max: i32) -> [i32; PHILOX32x16] {
        self.nextu().map(|x| crate::rng::Word::to_randi(x, min, max))

    }

    /// Generates a random `f32` value in the range [min, max).
    ///
    /// # Safety
    ///
    /// The caller must ensure the CPU supports the `avx512f` target feature.
    #[target_feature(enable = "avx512f")]
    pub fn randf(&mut self, min: f32, max: f32) -> [f32; PHILOX32x16] {
        self.nextu().map(|x| crate::rng::Word::to_randf(x, min, max))

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! { Philox32 }

    /// Random123 known-answer tests for philox4x32_10 (kat_vectors).
    #[test]
    fn philox4x32_random123_kats() {
        let kats: [([u32; 4], [u32; 2], [u32; 4]); 3] = [
            ([0; 4], [0; 2], [0x6627e8d5, 0xe169c58d, 0xbc57ac4c, 0x9b00dbd8]),
            ([u32::MAX; 4], [u32::MAX; 2], [0x408f276d, 0x41c83b0e, 0xa20bc7c6, 0x6d5451fd]),
            (
                [0x243f6a88, 0x85a308d3, 0x13198a2e, 0x03707344],
                [0xa4093822, 0x299f31d0],
                [0xd16cfe09, 0x94fdcceb, 0x5001e420, 0x24126ea1],
            ),
        ];
        for (c, k, want) in kats {
            assert_eq!(crate::_reftest::philox4x32_10(c, k), want, "reference");
            let got = Philox32::compute(c.map(|w| wrap!(w)), k.map(|w| wrap!(w))).map(|w| *w);
            assert_eq!(got, want, "Philox32");
        }
        // the Rng stream walks consecutive counters (with 128-bit carry)
        let mut g = Philox32::new(4);
        g.c = wrap![u32::MAX, u32::MAX, 7, 0];
        let k = g.k.map(|w| *w);
        let blocks = [[u32::MAX, u32::MAX, 7, 0], [0, 0, 8, 0], [1, 0, 8, 0]];
        for c in blocks {
            let want = crate::_reftest::philox4x32_10(c, k);
            for w in want {
                assert_eq!(g.nextu(), w);
            }
        }
    }

    /// REVIEW B7/B12: replaces an `unsafe_test!` that only compiled with
    /// `-C target-feature=+avx512f` (so it never ran). Each 128-bit lane `b`
    /// must be Philox4x32-10 with counter `c_b + n` and key `(k_b0, k_b1)`.
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn philox32x4x4_lanes_match_reference() {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return;
        }
        for seed in [0u32, 1, 0xFFFF_FFFF] {
            let mut v = unsafe { Philox32x4x4::new(seed) };
            // force a 64-bit carry in lane 0 within the first steps
            let mut c: [u32; 16] = unsafe { std::mem::transmute(v.c) };
            c[0..4].copy_from_slice(&[u32::MAX - 1, u32::MAX, 5, 0]);
            v.c = unsafe { std::mem::transmute(c) };
            let k: [u32; 16] = unsafe { std::mem::transmute(v.k) };
            let ctr0: [u128; 4] = unsafe { std::mem::transmute(v.c) };
            for n in 0..2000u128 {
                let got = unsafe { v.nextu() };
                for b in 0..4 {
                    let ctr = ctr0[b].wrapping_add(n);
                    let cw = [ctr as u32, (ctr >> 32) as u32, (ctr >> 64) as u32, (ctr >> 96) as u32];
                    let want = crate::_reftest::philox4x32_10(cw, [k[4 * b], k[4 * b + 2]]);
                    assert_eq!(&got[4 * b..4 * b + 4], &want, "lane {b}, step {n}");
                }
            }
            assert!(k.iter().skip(1).step_by(2).all(|&z| z == 0), "odd key words are zero");
        }
    }
}
