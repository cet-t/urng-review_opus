use wrapn::{wrap, wu32, wusize};

#[allow(unused_imports)]
use crate::{prng::b32::SplitMix32, rng::Rng};

// --- Threefry32x4 ---

const THREEFRY32_C240: u32 = 0x1BD11BDA;
/// A Threefry4x32 random number generator (Random123 family).
///
/// This is a counter-based RNG using a reduced-round (20 rounds) Threefish cipher
/// with 32-bit words and 4 output values per block. Implements [`Rng`] directly:
/// each call to [`Rng::nextu`] hands out one `u32` from an internal 4-word
/// buffer, recomputing a fresh block every 4th call.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Threefry32x4};
///
/// let mut rng = Threefry32x4::new(1);
/// let _: u32 = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Threefry32x4 {
    pub(crate) c: [wu32; 4],
    pub(crate) k: [wu32; 5],
    pub(crate) pos: wusize,
    pub(crate) buf: [wu32; 4],
}

impl Threefry32x4 {
    /// Creates a new `Threefry32x4` instance seeded with the given value.
    ///
    pub fn new(seed: u32) -> Self {
        let mut seedgen = SplitMix32::new(seed);
        let mut k = wrap![0u32; 5];
        for item in k.iter_mut().take(4) {
            *item = seedgen.nextu().into();
        }
        k[4] = k[0] ^ k[1] ^ k[2] ^ k[3] ^ THREEFRY32_C240;

        Self {
            c: wrap![0; 4],
            k,
            pos: wrap!(4),
            buf: wrap![0; 4],
        }
    }

    /// Pure counter-based computation: Random123 Threefry4x32-20.
    ///
    /// REVIEW B12: the previous version added Threefish-style tweak words in
    /// every key injection and XORed the counter into the output, so it was not
    /// the Random123 function its documentation names.
    ///
    /// # Arguments
    ///
    /// * `c`  - 4-word counter (the plaintext block).
    /// * `k`  - 5-word key schedule (k\[4\] = parity word).
    #[inline(always)]
    pub(crate) fn compute(c: [wu32; 4], k: &[wu32; 5]) -> [wu32; 4] {
        let mut v = c;

        macro_rules! round {
            ($r_sh_0:expr, $r_sh_1:expr) => {
                let y0 = v[0] + v[1];
                let f1 = v[1].rotate_left($r_sh_0) ^ y0;

                let y1 = v[2] + v[3];
                let f3 = v[3].rotate_left($r_sh_1) ^ y1;

                v[0] = y0;
                v[1] = f3;
                v[2] = y1;
                v[3] = f1;
            };
        }

        macro_rules! inject_key {
            ($s:expr) => {
                v[0] += k[$s % 5];
                v[1] += k[($s + 1) % 5];
                v[2] += k[($s + 2) % 5];
                v[3] += k[($s + 3) % 5] + $s as u32;
            };
        }

        inject_key!(0);
        round!(10, 26);
        round!(11, 21);
        round!(13, 27);
        round!(23, 5);

        inject_key!(1);
        round!(6, 20);
        round!(17, 11);
        round!(25, 10);
        round!(18, 20);

        inject_key!(2);
        round!(10, 26);
        round!(11, 21);
        round!(13, 27);
        round!(23, 5);

        inject_key!(3);
        round!(6, 20);
        round!(17, 11);
        round!(25, 10);
        round!(18, 20);

        inject_key!(4);
        round!(10, 26);
        round!(11, 21);
        round!(13, 27);
        round!(23, 5);

        inject_key!(5);
        v
    }

    /// Generates the next block of 4 random `u32` values in one call.
    ///
    /// This is the raw bulk-generation path (used internally to refill the
    /// scalar [`Rng::nextu`] buffer, and available directly for
    /// throughput-sensitive callers that want the whole block at once).
    #[inline(always)]
    pub fn next_raw(&mut self) -> [wu32; 4] {
        let dst = Self::compute(self.c, &self.k);

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

        dst
    }
}

crate::_internal::impl_ring_rng32!(Threefry32x4, 4, next_raw);

// --- Threefry32x2 ---

/// A Threefry 2x32 random number generator (Random123 family).
///
/// Counter-based PRNG using a reduced-round (20 rounds) Threefish cipher
/// with 32-bit words and 2 output values per block.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Threefry32x2};
///
/// let mut rng = Threefry32x2::new(1);
/// let _: u32 = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Threefry32x2 {
    pub(crate) c: [wu32; 2],
    pub(crate) k: [wu32; 3],
    pub(crate) buf: [wu32; 2],
    pub(crate) pos: wusize,
}

impl Threefry32x2 {
    /// Creates a new `Threefry32x2` instance seeded with the given value.
    ///
    #[inline]
    pub const fn new(seed: u32) -> Self {
        let mut sm = SplitMix32::new(seed);
        let k0 = sm.nextu_const();
        let k1 = sm.nextu_const();

        Self {
            c: wrap![0, 0],
            k: wrap![k0, k1, k0 ^ k1 ^ THREEFRY32_C240],
            buf: wrap![0; 2],
            pos: wrap!(2),
        }
    }

    /// Pure counter-based computation: applies 20 rounds of Threefish to produce 2 output words.
    ///
    /// # Arguments
    ///
    /// * `c` - 2-word counter (the plaintext block).
    /// * `k` - 3-word key schedule (k[2] = k[0] ^ k[1] ^ C240).
    #[inline(always)]
    pub(crate) fn compute(c: [wu32; 2], k: &[wu32; 3]) -> [wu32; 2] {
        let mut v = c;

        macro_rules! round {
            ($r_sh:expr) => {
                let y = v[0] + v[1];
                v[0] = y;
                v[1] = v[1].rotate_left($r_sh) ^ y;
            };
        }

        macro_rules! inject_key {
            ($s:expr) => {
                v[0] += k[$s % 3];
                v[1] = v[1] + k[($s + 1) % 3] + $s as u32;
            };
        }

        inject_key!(0);
        round!(13);
        round!(15);
        round!(26);
        round!(6);

        inject_key!(1);
        round!(17);
        round!(29);
        round!(16);
        round!(24);

        inject_key!(2);
        round!(13);
        round!(15);
        round!(26);
        round!(6);

        inject_key!(3);
        round!(17);
        round!(29);
        round!(16);
        round!(24);

        inject_key!(4);
        round!(13);
        round!(15);
        round!(26);
        round!(6);

        let ksi5_0 = k[2];
        let ksi5_1 = k[0] + 5;

        [v[0] + ksi5_0, v[1] + ksi5_1]
    }

    /// Generates the next block of 2 random `u32` values in one call.
    ///
    /// This is the raw bulk-generation path (used internally to refill the
    /// scalar [`Rng::nextu`] buffer, and available directly for
    /// throughput-sensitive callers that want the whole block at once).
    #[inline(always)]
    pub fn next_raw(&mut self) -> [wu32; 2] {
        let dst = Self::compute(self.c, &self.k);
        let (n_c0, overflow) = self.c[0].overflowing_add(1);
        self.c[0] = n_c0.into();
        if overflow {
            self.c[1] += 1;
        }
        dst
    }
}

crate::_internal::impl_ring_rng32!(Threefry32x2, 2, next_raw);

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! {
        Threefry32x4,
        Threefry32x2
    }

    /// Random123 known-answer tests (kat_vectors: threefry2x32_20, threefry4x32_20).
    #[test]
    fn threefry_random123_kats() {
        let kats2: [([u32; 2], [u32; 2], [u32; 2]); 3] = [
            ([0; 2], [0; 2], [0x6b200159, 0x99ba4efe]),
            ([u32::MAX; 2], [u32::MAX; 2], [0x1cb996fc, 0xbb002be7]),
            ([0x243f6a88, 0x85a308d3], [0x13198a2e, 0x03707344], [0xc4923a9c, 0x483df7a0]),
        ];
        for (c, k, want) in kats2 {
            assert_eq!(crate::_reftest::threefry2x32_20(c, k), want, "reference 2x32");
            let ks = wrap![k[0], k[1], k[0] ^ k[1] ^ THREEFRY32_C240];
            let got = Threefry32x2::compute(wrap![c[0], c[1]], &ks).map(|w| *w);
            assert_eq!(got, want, "Threefry32x2");
        }
        let kats4: [([u32; 4], [u32; 4], [u32; 4]); 3] = [
            ([0; 4], [0; 4], [0x9c6ca96a, 0xe17eae66, 0xfc10ecd4, 0x5256a7d8]),
            ([u32::MAX; 4], [u32::MAX; 4], [0x2a881696, 0x57012287, 0xf6c7446e, 0xa16a6732]),
            (
                [0x243f6a88, 0x85a308d3, 0x13198a2e, 0x03707344],
                [0xa4093822, 0x299f31d0, 0x082efa98, 0xec4e6c89],
                [0x59cd1dbb, 0xb8879579, 0x86b5d00c, 0xac8b6d84],
            ),
        ];
        for (c, k, want) in kats4 {
            assert_eq!(crate::_reftest::threefry4x32_20(c, k), want, "reference 4x32");
            let ks = wrap![k[0], k[1], k[2], k[3], k[0] ^ k[1] ^ k[2] ^ k[3] ^ THREEFRY32_C240];
            let got = Threefry32x4::compute(wrap![c[0], c[1], c[2], c[3]], &ks).map(|w| *w);
            assert_eq!(got, want, "Threefry32x4");
        }
        // the Rng stream is consecutive counters through the ring buffer
        let mut g = Threefry32x4::new(5);
        let k = g.k;
        for n in 0..3u32 {
            let want = Threefry32x4::compute(wrap![n, 0, 0, 0], &k);
            for w in want {
                assert_eq!(g.nextu(), *w);
            }
        }
    }
}
