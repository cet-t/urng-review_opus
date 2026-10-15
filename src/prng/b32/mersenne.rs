use std::ptr;

use bytemuck::cast_slice;
use wrapn::{wrap, wu32, wusize};

use crate::{_internal::sm64_from_seed32, prng::b32::SplitMix32, rng::Rng};

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

// --- Mt19937 ---

/// A 32-bit Mersenne Twister (MT19937) random number generator.
///
/// # Example
/// ```
/// use urng::{Rng, Mt19937};
///
/// let mut rng = Mt19937::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Mt19937 {
    mt: [wu32; MT32_N],
    mti: wusize,
}

const MT32_N: usize = 624;
const MT32_M: usize = 397;
const MT32_MATRIX_A: u32 = 0x9908B0DF;
const MT32_UPPER_MASK: u32 = 0x80000000;
const MT32_LOWER_MASK: u32 = 0x7FFFFFFF;

impl Mt19937 {
    /// Creates a new `Mt19937` instance with the given seed.
    ///
    /// The seed is first mixed through SplitMix32, so this is **not** the
    /// reference seeding: `Mt19937::new(s)` equals
    /// `Mt19937::from_init_genrand(SplitMix32::new(s).nextu())`. Use
    /// [`Mt19937::from_init_genrand`] to match `init_genrand` / C++
    /// `std::mt19937` / NumPy's `RandomState` legacy seeding.
    pub fn new(seed: u32) -> Self {
        let mut seedgen = SplitMix32::new(seed);
        Self::from_init_genrand(seedgen.nextu())
    }

    /// Creates an `Mt19937` seeded exactly like the reference `init_genrand`
    /// (Matsumoto & Nishimura, mt19937ar.c), i.e. like C++ `std::mt19937(seed)`.
    ///
    /// ```
    /// use urng::{Mt19937, Rng};
    /// let mut mt = Mt19937::from_init_genrand(5489); // std::mt19937 default seed
    /// assert_eq!(mt.nextu(), 3499211612);
    /// ```
    pub fn from_init_genrand(seed: u32) -> Self {
        let mut mt = [wrap!(0u32); MT32_N];
        mt[0] = wrap!(seed);
        for i in 1..MT32_N {
            let prev = mt[i - 1];
            mt[i] = (prev ^ (prev >> 30)) * 1812433253u32 + i as u32;
        }
        Self {
            mt,
            mti: wrap!(MT32_N),
        }
    }

    /// Kept for sequential single-threaded block fills (cabi now fills in parallel).
    #[inline]
    pub(crate) fn _fill_next_u32s(&mut self, out: &mut [u32]) {
        let mut written = 0;
        while written < out.len() {
            if self.mti >= MT32_N {
                self.twist();
            }

            let idx = self.mti;
            let available = wrap!(MT32_N) - idx;
            let take = available.min(wrap!(out.len()) - written);
            let src = &self.mt[*idx..*(idx + take)];
            let dst = &mut out[written..written + take.raw()];

            for (d, s) in dst.iter_mut().zip(src.iter()) {
                let mut y = *s;
                y ^= y >> 11;
                y ^= (y << 7) & 0x9D2C5680;
                y ^= (y << 15) & 0xEFC60000;
                y ^= y >> 18;
                *d = *y;
            }

            self.mti += take;
            written += take.raw();
        }
    }

    fn twist(&mut self) {
        for i in 0..MT32_N - MT32_M {
            let x = (self.mt[i] & MT32_UPPER_MASK) | (self.mt[i + 1] & MT32_LOWER_MASK);
            let mut x_a = x >> 1;
            if x & 1 != 0 {
                x_a ^= MT32_MATRIX_A;
            }
            self.mt[i] = self.mt[i + MT32_M] ^ x_a;
        }
        for i in MT32_N - MT32_M..MT32_N - 1 {
            let x = (self.mt[i] & MT32_UPPER_MASK) | (self.mt[i + 1] & MT32_LOWER_MASK);
            let mut x_a = x >> 1;
            if x & 1 != 0 {
                x_a ^= MT32_MATRIX_A;
            }
            self.mt[i] = self.mt[i + MT32_M - MT32_N] ^ x_a;
        }
        let x = (self.mt[MT32_N - 1] & MT32_UPPER_MASK) | (self.mt[0] & MT32_LOWER_MASK);
        let mut x_a = x >> 1;
        if x & 1 != 0 {
            x_a ^= MT32_MATRIX_A;
        }
        self.mt[MT32_N - 1] = self.mt[MT32_M - 1] ^ x_a;
        self.mti = wrap!(0);
    }
}

impl Rng for Mt19937 {
    type Word = u32;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        if self.mti >= MT32_N {
            self.twist();
        }
        let mut y = self.mt[*self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9D2C5680;
        y ^= (y << 15) & 0xEFC60000;
        y ^= y >> 18;
        *y
    }
}

// --- Sfmt19937 ---

/// A SIMD oriented Fast Mersenne Twister (SFMT) random number generator (32-bit version).
///
/// # Example
/// ```
/// use urng::{Rng, Sfmt19937};
///
/// let mut rng = Sfmt19937::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(16))]
pub struct Sfmt19937 {
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

impl Sfmt19937 {
    /// Creates an `Sfmt19937` seeded exactly like the reference
    /// `sfmt_init_gen_rand` (Saito & Matsumoto, SFMT 1.5), including the
    /// period certification.
    ///
    /// ```
    /// use urng::{Rng, Sfmt19937};
    /// let mut s = Sfmt19937::from_init_gen_rand(1234); // SFMT.19937.out.txt
    /// assert_eq!(s.nextu(), 3440181298);
    /// ```
    pub fn from_init_gen_rand(seed: u32) -> Self {
        let mut raw = [0u32; SFMT_N << 2];
        raw[0] = seed;
        for i in 1..raw.len() {
            raw[i] = 1812433253u32.wrapping_mul(raw[i - 1] ^ (raw[i - 1] >> 30)).wrapping_add(i as u32);
        }
        let mut state = [[0u32; 4]; SFMT_N];
        for (i, s) in state.iter_mut().enumerate() {
            s.copy_from_slice(&raw[4 * i..4 * i + 4]);
        }
        let mut rng = Self {
            state,
            idx: wrap!(SFMT_N << 2),
        };
        rng.period_certification();
        rng
    }

    /// Creates a new `Sfmt19937` instance with the given seed.
    ///
    /// The state is filled from SplitMix64 rather than the reference
    /// recurrence, so outputs differ from the reference implementation; use
    /// [`Sfmt19937::from_init_gen_rand`] for reference-compatible seeding.
    pub fn new(seed: u32) -> Self {
        let mut seedgen = sm64_from_seed32!(seed);

        // Initialize state using u32 array for simplicity
        let mut raw_state = [0u32; SFMT_N << 2];
        for i in 0..(SFMT_N << 1) {
            // Fill with 64-bit values from SplitMix64
            let s = seedgen.nextu_const();
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
            idx: wrap!(SFMT_N << 2), // Force generate on first call. 156 * 4 = 624 u32s
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
            unsafe { std::slice::from_raw_parts(self.state.as_ptr() as *const u32, SFMT_N << 2) };
        const PARITY: [u32; 4] = [SFMT_PARITY1, SFMT_PARITY2, SFMT_PARITY3, SFMT_PARITY4];

        for i in 0..4 {
            inner ^= psfmt32[i] & PARITY[i];
        }
        let mut i = 16;
        while i > 0 {
            inner ^= inner >> i;
            i >>= 1;
        }
        inner &= 1;

        // Verification passed
        if inner == 1 {
            return;
        }

        // Modification for period certification
        let psfmt32_mut = unsafe {
            std::slice::from_raw_parts_mut(self.state.as_mut_ptr() as *mut u32, SFMT_N << 2)
        };

        for i in 0..4 {
            let mut work = 1;
            for _ in 0..32 {
                if (work & PARITY[i]) != 0 {
                    psfmt32_mut[i] ^= work;
                    return;
                }
                work <<= 1;
            }
        }
    }

    /// Kept for sequential single-threaded block fills (cabi now fills in parallel).
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn fill_next_u32s(&mut self, out: &mut [u32]) {
        let mut written = 0;
        while written < out.len() {
            if self.idx >= SFMT_N << 2 {
                self.gen_rand_all();
                self.idx = wrap!(0);
            }

            let available = (SFMT_N << 2) - *self.idx;
            let take = available.min(out.len() - written);

            unsafe {
                ptr::copy_nonoverlapping(
                    (self.state.as_ptr() as *const u32).add(*self.idx),
                    out.as_mut_ptr().add(written),
                    take,
                );
            }

            self.idx += take;
            written += take;
        }
    }
}

impl Rng for Sfmt19937 {
    type Word = u32;

    #[inline]
    fn nextu(&mut self) -> Self::Word {
        if self.idx >= SFMT_N * 4 {
            self.gen_rand_all();
            self.idx = wrap!(0);
        }

        let s: &[u32] = cast_slice(&self.state);
        let val = s[*self.idx];
        self.idx += 1;
        val
    }
}

macro_rules! define_sfmt_variant {
    (
        mexp = $mexp:literal,
        n = $n:literal,
        pos1 = $pos1:literal,
        sl1 = $sl1:literal,
        sl2 = $sl2:literal,
        sr1 = $sr1:literal,
        sr2 = $sr2:literal,
        msk1 = $msk1:expr,
        msk2 = $msk2:expr,
        msk3 = $msk3:expr,
        msk4 = $msk4:expr,
        parity1 = $parity1:expr,
        parity2 = $parity2:expr,
        parity3 = $parity3:expr,
        parity4 = $parity4:expr $(,)?
    ) => {
        pastey::paste! {
            /// A SIMD oriented Fast Mersenne Twister (SFMT) random number generator (32-bit version).
            ///
            /// # Example
            #[doc = concat!(
                "```\n",
                "use urng::{Rng, ",
                stringify!([<Sfmt $mexp>]),
                "};\n",
                "\n",
                "let mut rng = ",
                stringify!([<Sfmt $mexp>]),
                "::new(1);\n",
                "let _ = rng.nextu();\n",
                "```"
            )]
            #[repr(C)]
            #[repr(align(16))]
            pub struct [<Sfmt $mexp>] {
                state: [u32x4; $n],
                idx: ::wrapn::wusize,
            }

            impl [<Sfmt $mexp>] {
                #[doc = concat!("Creates a new `", stringify!([<Sfmt $mexp>]), "` instance with the given seed.")]
                pub fn new(seed: u32) -> Self {
                    let mut seedgen = sm64_from_seed32!(seed);
                    let mut raw_state = [0u32; $n << 2];
                    for i in 0..($n * 2) {
                        let s = seedgen.nextu();
                        raw_state[2 * i]     = s as u32;
                        raw_state[2 * i + 1] = (s >> 32) as u32;
                    }
                    let mut state = [[0u32; 4]; $n];
                    for i in 0..$n {
                        state[i] = [
                            raw_state[4 * i],
                            raw_state[4 * i + 1],
                            raw_state[4 * i + 2],
                            raw_state[4 * i + 3],
                        ];
                    }
                    let mut rng = Self { state, idx: ($n << 2).into() };
                    rng.period_certification();
                    rng
                }

                fn gen_rand_all(&mut self) {
                    unsafe {
                        let ptr = self.state.as_mut_ptr();
                        let mut r1 = *ptr.add($n - 2);
                        let mut r2 = *ptr.add($n - 1);
                        let mask = [$msk1, $msk2, $msk3, $msk4];

                        for i in 0..($n - $pos1) {
                            let p_i = ptr.add(i);
                            let a = *p_i;
                            let b = *ptr.add(i + $pos1);
                            let r = sfmt_recursion(
                                a,
                                b,
                                r1,
                                r2,
                                mask,
                                $sl1 as u32,
                                $sl2 as u32,
                                $sr1 as u32,
                                $sr2 as u32,
                            );
                            *p_i = r;
                            r1 = r2;
                            r2 = r;
                        }

                        for i in ($n - $pos1)..$n {
                            let p_i = ptr.add(i);
                            let a = *p_i;
                            let b = *ptr.add(i + $pos1 - $n);
                            let r = sfmt_recursion(
                                a,
                                b,
                                r1,
                                r2,
                                mask,
                                $sl1 as u32,
                                $sl2 as u32,
                                $sr1 as u32,
                                $sr2 as u32,
                            );
                            *p_i = r;
                            r1 = r2;
                            r2 = r;
                        }
                    }
                }

                fn period_certification(&mut self) {
                    let mut inner = 0u32;
                    let psfmt32 = unsafe {
                        std::slice::from_raw_parts(self.state.as_ptr() as *const u32, $n << 2)
                    };
                    let parity = [$parity1, $parity2, $parity3, $parity4];
                    for i in 0..4 {
                        inner ^= psfmt32[i] & parity[i];
                    }
                    let mut shift = 16u32;
                    while shift > 0 {
                        inner ^= inner >> shift;
                        shift >>= 1;
                    }
                    inner &= 1;
                    if inner == 1 {
                        return;
                    }
                    let psfmt32_mut = unsafe {
                        std::slice::from_raw_parts_mut(self.state.as_mut_ptr() as *mut u32, $n << 2)
                    };
                    for i in 0..4 {
                        let mut work = 1u32;
                        for _ in 0..32 {
                            if (work & parity[i]) != 0 {
                                psfmt32_mut[i] ^= work;
                                return;
                            }
                            work <<= 1;
                        }
                    }
                }

                /// Kept for sequential single-threaded block fills (cabi now fills in parallel).
                #[allow(dead_code)]
                #[inline]
                pub(crate) fn fill_next_u32s(&mut self, out: &mut [u32]) {
                    let mut written = 0;
                    while written < out.len() {
                        if self.idx >= $n << 2 {
                            self.gen_rand_all();
                            self.idx = wrap!(0);
                        }
                        let available = ($n << 2) - *self.idx;
                        let take = available.min(out.len() - written);
                        unsafe {
                            ptr::copy_nonoverlapping(
                                (self.state.as_ptr() as *const u32).add(*self.idx),
                                out.as_mut_ptr().add(written),
                                take,
                            );
                        }
                        self.idx += take;
                        written += take;
                    }
                }
            }


            impl Rng for [<Sfmt $mexp>] {
                type Word = u32;

                #[inline]
                fn nextu(&mut self) -> Self::Word {
                    if self.idx >= $n << 2 {
                        self.gen_rand_all();
                        self.idx = wrap!(0);
                    }
                    let s: &[u32] = cast_slice(&self.state);
                    let val = s[*self.idx];
                    self.idx += 1;
                    val
                }
            }
        }
    };
}

define_sfmt_variant!(
    mexp = 607,
    n = 5,
    pos1 = 2,
    sl1 = 15,
    sl2 = 3,
    sr1 = 13,
    sr2 = 3,
    msk1 = 0xfdff37ffu32,
    msk2 = 0xef7f3f7du32,
    msk3 = 0xff777b7du32,
    msk4 = 0x7ff7fb2fu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0x00000000u32,
    parity4 = 0x5986f054u32,
);

define_sfmt_variant!(
    mexp = 1279,
    n = 10,
    pos1 = 7,
    sl1 = 14,
    sl2 = 3,
    sr1 = 5,
    sr2 = 1,
    msk1 = 0xf7fefffdu32,
    msk2 = 0x7fefcfffu32,
    msk3 = 0xaff3ef3fu32,
    msk4 = 0xb5ffff7fu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0x00000000u32,
    parity4 = 0x20000000u32,
);

define_sfmt_variant!(
    mexp = 2281,
    n = 18,
    pos1 = 12,
    sl1 = 19,
    sl2 = 1,
    sr1 = 5,
    sr2 = 1,
    msk1 = 0xbff7ffbfu32,
    msk2 = 0xfdfffffeu32,
    msk3 = 0xf7ffef7fu32,
    msk4 = 0xf2f7cbbfu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0x00000000u32,
    parity4 = 0x41dfa600u32,
);

define_sfmt_variant!(
    mexp = 4253,
    n = 34,
    pos1 = 17,
    sl1 = 20,
    sl2 = 1,
    sr1 = 7,
    sr2 = 1,
    msk1 = 0x9f7bffffu32,
    msk2 = 0x9fffff5fu32,
    msk3 = 0x3efffffbu32,
    msk4 = 0xfffff7bbu32,
    parity1 = 0xa8000001u32,
    parity2 = 0xaf5390a3u32,
    parity3 = 0xb740b3f8u32,
    parity4 = 0x6c11486du32,
);

define_sfmt_variant!(
    mexp = 11213,
    n = 88,
    pos1 = 68,
    sl1 = 14,
    sl2 = 3,
    sr1 = 7,
    sr2 = 3,
    msk1 = 0xeffff7fbu32,
    msk2 = 0xffffffefu32,
    msk3 = 0xdfdfbfffu32,
    msk4 = 0x7fffdbfdu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0xe8148000u32,
    parity4 = 0xd0c7afa3u32,
);

define_sfmt_variant!(
    mexp = 44497,
    n = 348,
    pos1 = 330,
    sl1 = 5,
    sl2 = 3,
    sr1 = 9,
    sr2 = 3,
    msk1 = 0xeffffffbu32,
    msk2 = 0xdfbebfffu32,
    msk3 = 0xbfbf7befu32,
    msk4 = 0x9ffd7bffu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0xa3ac4000u32,
    parity4 = 0xecc1327au32,
);

define_sfmt_variant!(
    mexp = 86243,
    n = 674,
    pos1 = 366,
    sl1 = 6,
    sl2 = 7,
    sr1 = 19,
    sr2 = 1,
    msk1 = 0xfdbffbffu32,
    msk2 = 0xbff7ff3fu32,
    msk3 = 0xfd77efffu32,
    msk4 = 0xbf9ff3ffu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0x00000000u32,
    parity4 = 0xe9528d85u32,
);

define_sfmt_variant!(
    mexp = 132049,
    n = 1032,
    pos1 = 110,
    sl1 = 19,
    sl2 = 1,
    sr1 = 21,
    sr2 = 1,
    msk1 = 0xffffbb5fu32,
    msk2 = 0xfb6ebf95u32,
    msk3 = 0xfffefffau32,
    msk4 = 0xcff77fffu32,
    parity1 = 0x00000001u32,
    parity2 = 0x00000000u32,
    parity3 = 0xcb520000u32,
    parity4 = 0xc7e91c7du32,
);

define_sfmt_variant!(
    mexp = 216091,
    n = 1689,
    pos1 = 627,
    sl1 = 11,
    sl2 = 3,
    sr1 = 10,
    sr2 = 1,
    msk1 = 0xbff7bff7u32,
    msk2 = 0xbfffffffu32,
    msk3 = 0xbffffa7fu32,
    msk4 = 0xffddfbfbu32,
    parity1 = 0xf8000001u32,
    parity2 = 0x89e80709u32,
    parity3 = 0x3bd2b64bu32,
    parity4 = 0x0c64b1e4u32,
);

#[cfg(test)]
mod tests {

    use super::*;

    /// REVIEW B12: reference-compatible seeding. 4123659995 is the 10000th
    /// output that the C++ standard requires of a default-constructed
    /// `std::mt19937` ([rand.predef]); 3499211612 is its first output.
    #[test]
    fn mt19937_reference_seeding() {
        let mut mt = Mt19937::from_init_genrand(5489);
        assert_eq!(mt.nextu(), 3499211612);
        for _ in 1..9999 {
            let _ = mt.nextu();
        }
        assert_eq!(mt.nextu(), 4123659995);
        // the reference state recurrence
        let r = Mt19937::from_init_genrand(42);
        assert_eq!(r.mt.map(|w| *w), crate::_reftest::mt_init(42));
        // `new` keeps its value-compatible meaning (SplitMix32 pre-mix)
        let mut a = Mt19937::new(7);
        let mut b = Mt19937::from_init_genrand(SplitMix32::new(7).nextu());
        for _ in 0..1000 {
            assert_eq!(a.nextu(), b.nextu());
        }
    }

    /// SFMT19937 with `sfmt_init_gen_rand(1234)` (first value of the official
    /// SFMT.19937.out.txt), and the state equals an independent
    /// implementation of the reference initialisation.
    #[test]
    fn sfmt19937_reference_seeding() {
        let s = Sfmt19937::from_init_gen_rand(1234);
        let st: Vec<u32> = s.state.iter().flatten().copied().collect();
        assert_eq!(st, crate::_reftest::sfmt19937_init(1234));
        let mut s = s;
        assert_eq!(s.nextu(), 3440181298);
        // period certification modifies the state when the parity check fails
        let fixed = (0..64u32)
            .filter(|&seed| Sfmt19937::from_init_gen_rand(seed).state[0][0] != crate::_reftest::mt_init(seed)[0])
            .count();
        let reference_fixed = (0..64u32)
            .filter(|&seed| crate::_reftest::sfmt19937_init(seed)[0] != crate::_reftest::mt_init(seed)[0])
            .count();
        assert_eq!(fixed, reference_fixed);
    }

    crate::safe_test! {
        Mt19937,
        Sfmt19937,
        Sfmt607,
        Sfmt1279,
        Sfmt2281,
        Sfmt4253,
        Sfmt11213,
        Sfmt44497,
        Sfmt86243,
        Sfmt132049,
        Sfmt216091
    }
}
