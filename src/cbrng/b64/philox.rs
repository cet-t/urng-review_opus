use wrapn::{wrap, wu64, wusize};

use crate::_internal::impl_ring_rng64;
use crate::prng::b64::SplitMix64;

// --- Philox64 ---

/// A Philox 2x64 random number generator.
///
/// This is a counter-based RNG suitable for parallel applications. Implements
/// [`Rng`] directly: each call to [`Rng::nextu`] hands out one `u64` from
/// an internal 2-word buffer, recomputing a fresh block every 2nd call.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Philox64};
///
/// let mut rng = Philox64::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
pub struct Philox64 {
    pub(crate) c: [wu64; 2],
    pub(crate) k: [wu64; 2],
    pub(crate) buf: [wu64; 2],
    pub(crate) pos: wusize,
}

impl Philox64 {
    /// Creates a new `Philox64` instance.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            c: wrap![1, 0],
            k: wrap![seedgen.nextu_const(), seedgen.nextu_const()],
            buf: wrap![0; 2],
            pos: wrap!(2),
        }
    }

    /// Computes Philox output from counter and key values (pure function).
    #[inline]
    pub(crate) fn compute(mut c: [wu64; 2], k: [wu64; 2]) -> [wu64; 2] {
        let mut key = k[0];

        const M0: u128 = 0xD2B74407B1CE6E93;
        const W0: u64 = 0x9E3779B97F4A7C15;

        macro_rules! step {
            () => {
                step!(fin);
                key += W0;
            };
            (fin) => {
                let prod = c[0].cast::<u128>() * M0;
                let hi = (prod >> 64).cast::<u64>();
                let lo = prod.cast::<u64>();

                c[0] = hi ^ c[1] ^ key;
                c[1] = lo;
            };
        }

        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!(fin);

        c
    }

    /// Generates the next block of 2 random `u64` values in one call.
    ///
    /// This is the raw bulk-generation path (used internally to refill the
    /// scalar [`Rng::nextu`] buffer, and available directly for
    /// throughput-sensitive callers that want the whole block at once).
    #[inline]
    pub fn next_raw(&mut self) -> [wu64; 2] {
        let out = Self::compute(self.c, self.k);
        self.c[0] += 1;
        if self.c[0] == 0 {
            self.c[1] += 1;
        }
        out
    }
}

impl_ring_rng64!(Philox64, 2, next_raw);

// --- Philox 4x64 ---

/// A Philox 4x64 random number generator.
///
/// This is a counter-based RNG suitable for parallel applications. Implements
/// [`Rng`] directly: each call to [`Rng::nextu`] hands out one `u64` from
/// an internal 4-word buffer, recomputing a fresh block every 4th call.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Philox4x64};
///
/// let mut rng = Philox4x64::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
pub struct Philox4x64 {
    c: [wu64; 4],
    k: [wu64; 2],
    buf: [wu64; 4],
    pos: wusize,
}

impl Philox4x64 {
    /// Creates a new `Philox4x64` instance.
    pub const fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        Self {
            c: wrap![3, 2, 1, 0],
            k: wrap![seedgen.nextu_const(), seedgen.nextu_const()],
            buf: wrap![0; 4],
            // must equal the buffer length so the first call computes a block
            // (it was 2, so the first two outputs were the zero-filled buffer)
            pos: wrap!(4),
        }
    }

    /// Computes Philox output from counter and key values (pure function).
    #[inline]
    pub(crate) fn compute(mut c: [wu64; 4], mut k: [wu64; 2]) -> [wu64; 4] {
        const M0: u128 = 0xD2E7470EE14C6C93;
        const M1: u128 = 0xCA5A826395121157;
        const W0: u64 = 0x9E3779B97F4A7C15;
        const W1: u64 = 0xBB67AE8584CAA73B;

        macro_rules! step {
            () => {
                step!(fin);
                k[0] += W0;
                k[1] += W1;
            };
            (fin) => {
                let prod0 = c[0].cast::<u128>() * M0;
                let hi0 = (prod0 >> 64).cast::<u64>();
                let lo0 = prod0.cast::<u64>();

                let prod1 = c[2].cast::<u128>() * M1;
                let hi1 = (prod1 >> 64).cast::<u64>();
                let lo1 = prod1.cast::<u64>();

                let c0 = hi1 ^ c[1] ^ k[0];
                let c1 = lo1;
                let c2 = hi0 ^ c[3] ^ k[1];
                let c3 = lo0;

                c[0] = c0;
                c[1] = c1;
                c[2] = c2;
                c[3] = c3;
            };
        }

        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!();
        step!(fin);

        c
    }

    #[inline]
    pub fn next_raw(&mut self) -> [wu64; 4] {
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

impl_ring_rng64!(Philox4x64, 4, next_raw);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::rng::Rng;

    crate::safe_test! {
        Philox64,
        Philox4x64
    }

    /// Random123 known-answer tests (kat_vectors, philox2x64_10 / philox4x64_10).
    #[test]
    fn philox_random123_kats() {
        let z = Philox64::compute(wrap![0, 0], wrap![0, 0]).map(|w| *w);
        assert_eq!(z, [0xca00a0459843d731, 0x66c24222c9a845b5]);
        let z = Philox4x64::compute(wrap![0; 4], wrap![0; 2]).map(|w| *w);
        assert_eq!(
            z,
            [0x16554d9eca36314c, 0xdb20fe9d672d0fdc, 0xd7e772cee186176b, 0x7e68b68aec7ba23b]
        );
        // independent reference agrees on non-trivial inputs
        let c = [1u64, 2, 3, 4];
        let k = [0xdeadbeef, 0x12345678];
        assert_eq!(
            Philox4x64::compute(wrap![1, 2, 3, 4], wrap![0xdeadbeef, 0x12345678]).map(|w| *w),
            crate::_reftest::philox4x64_10(c, k)
        );
    }

    /// REVIEW B11: `Philox4x64::new` started with `pos = 2`, so the first two
    /// outputs were the zero-filled buffer.
    #[test]
    fn philox4x64_first_outputs_come_from_the_first_block() {
        for seed in [0u64, 1, 123, u64::MAX] {
            let mut r = Philox4x64::new(seed);
            let expect = Philox4x64::compute(r.c, r.k).map(|w| *w);
            let got = [r.nextu(), r.nextu(), r.nextu(), r.nextu()];
            assert_eq!(got, expect, "seed {seed}");
        }
    }
}
