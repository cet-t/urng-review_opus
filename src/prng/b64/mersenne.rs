use std::ptr;

use wrapn::{wrap, wu64, wusize};

use crate::prng::b64::SplitMix64;
use crate::rng::Rng;

#[allow(non_camel_case_types)]
type u32x4 = [u32; 4];

#[inline(always)]
fn xor4(a: u32x4, b: u32x4) -> u32x4 {
    [a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]]
}

#[inline(always)]
fn and4(a: u32x4, b: u32x4) -> u32x4 {
    [a[0] & b[0], a[1] & b[1], a[2] & b[2], a[3] & b[3]]
}

#[inline(always)]
fn shl4(a: u32x4, shift: u32) -> u32x4 {
    [a[0] << shift, a[1] << shift, a[2] << shift, a[3] << shift]
}

#[inline(always)]
fn shr4(a: u32x4, shift: u32) -> u32x4 {
    [a[0] >> shift, a[1] >> shift, a[2] >> shift, a[3] >> shift]
}

#[inline(always)]
fn lshift128(a: u32x4, bytes: u32) -> u32x4 {
    bytemuck::cast(bytemuck::cast::<_, u128>(a) << (bytes * 8))
}

#[inline(always)]
fn rshift128(a: u32x4, bytes: u32) -> u32x4 {
    bytemuck::cast(bytemuck::cast::<_, u128>(a) >> (bytes * 8))
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn sfmt_recursion(
    a: u32x4,
    b: u32x4,
    r1: u32x4,
    r2: u32x4,
    mask: u32x4,
    sl1: u32,
    sl2: u32,
    sr1: u32,
    sr2: u32,
) -> u32x4 {
    xor4(
        xor4(xor4(a, lshift128(a, sl2)), and4(shr4(b, sr1), mask)),
        xor4(rshift128(r1, sr2), shl4(r2, sl1)),
    )
}

// --- Mt1993764 ---

/// Mersenne Twister (MT19937-64) 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Mt1993764};
///
/// let mut rng = Mt1993764::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Mt1993764 {
    mt: [wu64; N],
    mti: wusize,
}

const N: usize = 312;
const M: usize = 156;
const MATRIX_A: u64 = 0xB5026F5AA96619E9;
const UPPER_MASK: u64 = 0xFFFFFFFF80000000;
const LOWER_MASK: u64 = 0x7FFFFFFF;

impl Mt1993764 {
    /// Creates a new `Mt1993764` instance with the given seed.
    ///
    /// The seed is first mixed through SplitMix64, so this is **not** the
    /// reference seeding: `Mt1993764::new(s)` equals
    /// `Mt1993764::from_init_genrand64(SplitMix64::new(s).nextu())`. Use
    /// [`Mt1993764::from_init_genrand64`] to match C++ `std::mt19937_64`.
    pub fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self::from_init_genrand64(seedgen.nextu())
    }

    /// Creates an `Mt1993764` seeded exactly like the reference
    /// `init_genrand64` (mt19937-64.c), i.e. like C++ `std::mt19937_64(seed)`.
    ///
    /// ```
    /// use urng::{Mt1993764, Rng};
    /// let mut mt = Mt1993764::from_init_genrand64(5489);
    /// assert_eq!(mt.nextu(), 14514284786278117030);
    /// ```
    pub fn from_init_genrand64(seed: u64) -> Self {
        let mut mt = wrap![0; N];
        mt[0] = wrap!(seed);
        for i in 1..N {
            let prev = mt[i - 1];
            mt[i] = (prev ^ (prev >> 62)) * 6364136223846793005u64 + (i as u64);
        }
        Self { mt, mti: N.into() }
    }

    #[inline]
    pub fn fill_next_u64s(&mut self, out: &mut [u64]) {
        let mut written = 0;
        while written < out.len() {
            if self.mti >= N {
                self.twist();
            }

            let idx = self.mti;
            let available = N - idx.raw();
            let take = available.min(out.len() - written);
            let src = &self.mt[(*idx)..*(idx + take)];
            let dst = &mut out[written..written + take];

            for (d, &y) in dst.iter_mut().zip(src.iter()) {
                let mut v = y;
                v ^= (v >> 29) & 0x5555555555555555;
                v ^= (v << 17) & 0x71D67FFFEDA60000;
                v ^= (v << 37) & 0xFFF7EEE000000000;
                v ^= v >> 43;
                *d = *v;
            }

            self.mti += take;
            written += take;
        }
    }

    fn twist(&mut self) {
        for i in 0..N - M {
            let x = (self.mt[i] & UPPER_MASK) | (self.mt[i + 1] & LOWER_MASK);
            let mut x_a = x >> 1;
            if x & 1 != 0 {
                x_a ^= MATRIX_A;
            }
            self.mt[i] = self.mt[i + M] ^ x_a;
        }
        for i in N - M..N - 1 {
            let x = (self.mt[i] & UPPER_MASK) | (self.mt[i + 1] & LOWER_MASK);
            let mut x_a = x >> 1;
            if x & 1 != 0 {
                x_a ^= MATRIX_A;
            }
            self.mt[i] = self.mt[i + M - N] ^ x_a;
        }
        let x = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
        let mut x_a = x >> 1;
        if x & 1 != 0 {
            x_a ^= MATRIX_A;
        }
        self.mt[N - 1] = self.mt[M - 1] ^ x_a;
        self.mti = 0.into();
    }
}

impl Rng for Mt1993764 {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        if self.mti >= N {
            self.twist();
        }
        let mut y = self.mt[*self.mti];
        self.mti += 1;
        y ^= (y >> 29) & 0x5555555555555555;
        y ^= (y << 17) & 0x71D67FFFEDA60000;
        y ^= (y << 37) & 0xFFF7EEE000000000;
        y ^= y >> 43;
        *y
    }
}

// --- Sfmt1993764 ---

/// SIMD-oriented Fast Mersenne Twister (SFMT) 64-bit RNG implementation.
///
/// # Example
/// ```
/// use urng::{Rng, Sfmt1993764};
///
/// let mut rng = Sfmt1993764::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Sfmt1993764 {
    state: [u32x4; SFMT_N],
    idx: wusize,
}

const SFMT_N: usize = 156;
const SFMT_POS1: usize = 122;
const SFMT_SL1: u32 = 18;
const SFMT_SR1: u32 = 11;

const SFMT_MSK1: u32 = 0xdfffffef;
const SFMT_MSK2: u32 = 0xddfecb7f;
const SFMT_MSK3: u32 = 0xbffaffff;
const SFMT_MSK4: u32 = 0xbffffff6;
const SFMT_PARITY1: u32 = 0x00000001;
const SFMT_PARITY2: u32 = 0x00000000;
const SFMT_PARITY3: u32 = 0x00000000;
const SFMT_PARITY4: u32 = 0x13c9e684;

impl Sfmt1993764 {
    /// Creates a new `Sfmt1993764` instance with the given seed.
    pub fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);

        // Initialize state using u32 array for simplicity
        let mut raw_state = [0u32; SFMT_N * 4];
        for i in 0..SFMT_N * 2 {
            // Fill with 64-bit values from SplitMix64
            let s = seedgen.nextu();
            raw_state[2 * i] = s as u32;
            raw_state[2 * i + 1] = (s >> 32) as u32;
        }

        let mut state = [[0u32; 4]; SFMT_N];
        for i in 0..SFMT_N {
            state[i] = [
                raw_state[4 * i],
                raw_state[4 * i + 1],
                raw_state[4 * i + 2],
                raw_state[4 * i + 3],
            ];
        }

        let mut rng = Self {
            state,
            idx: (SFMT_N * 2).into(), // Force generate on first call
        };
        rng.period_certification();
        rng
    }

    fn gen_rand_all(&mut self) {
        unsafe {
            let ptr = self.state.as_mut_ptr();
            let mut r1 = *ptr.add(SFMT_N - 2);
            let mut r2 = *ptr.add(SFMT_N - 1);

            let mask = [SFMT_MSK1, SFMT_MSK2, SFMT_MSK3, SFMT_MSK4];

            for i in 0..(SFMT_N - SFMT_POS1) {
                let p_i = ptr.add(i);
                let a = *p_i;
                let b = *ptr.add(i + SFMT_POS1);
                let r = sfmt_recursion(a, b, r1, r2, mask, SFMT_SL1, 1, SFMT_SR1, 1);

                *p_i = r;
                r1 = r2;
                r2 = r;
            }

            for i in (SFMT_N - SFMT_POS1)..SFMT_N {
                let p_i = ptr.add(i);
                let a = *p_i;
                let b = *ptr.add(i + SFMT_POS1 - SFMT_N);
                let r = sfmt_recursion(a, b, r1, r2, mask, SFMT_SL1, 1, SFMT_SR1, 1);

                *p_i = r;
                r1 = r2;
                r2 = r;
            }
        }
    }

    fn period_certification(&mut self) {
        let mut inner = 0;
        let psfmt32 =
            unsafe { std::slice::from_raw_parts(self.state.as_ptr() as *const u32, SFMT_N * 4) };
        let parity = [SFMT_PARITY1, SFMT_PARITY2, SFMT_PARITY3, SFMT_PARITY4];

        for i in 0..4 {
            inner ^= psfmt32[i] & parity[i];
        }
        let mut i = 16;
        while i > 0 {
            inner ^= inner >> i;
            i >>= 1;
        }
        inner &= 1;

        if inner == 1 {
            return;
        }

        let psfmt32_mut = unsafe {
            std::slice::from_raw_parts_mut(self.state.as_mut_ptr() as *mut u32, SFMT_N * 4)
        };

        for i in 0..4 {
            let mut work = 1;
            for _ in 0..32 {
                if (work & parity[i]) != 0 {
                    psfmt32_mut[i] ^= work;
                    return;
                }
                work <<= 1;
            }
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub(crate) fn fill_next_u64s(&mut self, out: &mut [u64]) {
        let mut written = 0;
        while written < out.len() {
            if self.idx >= SFMT_N * 2 {
                self.gen_rand_all();
                self.idx = 0.into();
            }

            let available = wrap!(SFMT_N * 2) - self.idx;
            let take = available.min(wrap!(out.len() - written));

            unsafe {
                ptr::copy_nonoverlapping(
                    (self.state.as_ptr() as *const u64).add(*self.idx),
                    out.as_mut_ptr().add(written),
                    *take,
                );
            }

            self.idx += take;
            written += *take;
        }
    }
}

impl Rng for Sfmt1993764 {
    type Word = u64;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        if self.idx >= SFMT_N * 2 {
            self.gen_rand_all();
            self.idx = 0.into();
        }

        let s: &[u64] = bytemuck::cast_slice(&self.state);
        let val = s[*self.idx];
        self.idx += 1;
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REVIEW B12: 9981545732273789042 is the 10000th output the C++ standard
    /// requires of a default-constructed `std::mt19937_64` ([rand.predef]).
    #[test]
    fn mt1993764_reference_seeding() {
        let mut mt = Mt1993764::from_init_genrand64(5489);
        assert_eq!(mt.nextu(), 14514284786278117030);
        for _ in 1..9999 {
            let _ = mt.nextu();
        }
        assert_eq!(mt.nextu(), 9981545732273789042);
        let r = Mt1993764::from_init_genrand64(42);
        assert_eq!(r.mt.map(|w| *w), crate::_reftest::mt64_init(42));
        let mut a = Mt1993764::new(7);
        let mut b = Mt1993764::from_init_genrand64(SplitMix64::new(7).nextu());
        for _ in 0..1000 {
            assert_eq!(a.nextu(), b.nextu());
        }
    }

    crate::safe_test! {
        Mt1993764,
        Sfmt1993764
    }
}
