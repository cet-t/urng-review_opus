use wrapn::{wrap, wu64, wusize};

use crate::{impl_ring_rng64, prng::b64::SplitMix64};

// --- Threefish256 ---

const THREEFISH_C240: u64 = 0x1BD11BDAA9FC1A22;
const THREE_FISH_N_ROUNDS: usize = 72;
const THREEFISH_PI: [usize; 4] = [0, 3, 2, 1];

const KS_N: usize = THREE_FISH_N_ROUNDS / 4 + 1; // 19
const KS_K_IDX: [[usize; 4]; KS_N] = {
    let mut t = [[0usize; 4]; KS_N];
    let mut s = 0;
    while s < KS_N {
        t[s] = [s % 5, (s + 1) % 5, (s + 2) % 5, (s + 3) % 5];
        s += 1;
    }
    t
};
const KS_TW_IDX: [[usize; 2]; KS_N] = {
    let mut t = [[0usize; 2]; KS_N];
    let mut s = 0;
    while s < KS_N {
        t[s] = [s % 3, (s + 1) % 3];
        s += 1;
    }
    t
};

const THREEFISH_R_256: [[u32; 2]; 8] = [
    [14, 16],
    [52, 57],
    [23, 40],
    [5, 37],
    [25, 33],
    [46, 12],
    [58, 22],
    [32, 32],
];

/// A Threefish-256 random number generator.
///
/// # Examples
///
/// ```
/// use urng::{Rng, Threefish256};
///
/// let mut rng = Threefish256::new(1);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Threefish256 {
    c: [wu64; 4],
    k: [wu64; 5],
    tw: [wu64; 3],
    index: wusize,
    buf: [wu64; 4],
    pos: wusize,
}

impl Threefish256 {
    /// Creates a new `Threefish256` instance.
    pub fn new(seed: u64) -> Self {
        let mut seedgen = SplitMix64::new(seed);
        let mut k = wrap![0u64; 5];
        k[0] = wrap!(seedgen.nextu_const());
        k[1] = wrap!(seedgen.nextu_const());
        k[2] = wrap!(seedgen.nextu_const());
        k[3] = wrap!(seedgen.nextu_const());
        k[4] = k[0] ^ k[1] ^ k[2] ^ k[3] ^ THREEFISH_C240;

        let tw0 = wrap!(seedgen.nextu_const());
        let tw1 = wrap!(seedgen.nextu_const());

        Self {
            c: wrap![0; 4],
            k,
            tw: [tw0, tw1, tw0 ^ tw1],
            index: wrap!(4),
            buf: wrap![0; 4],
            // must equal the buffer length (it was 0: the first four outputs were 0)
            pos: wrap!(4),
        }
    }

    #[inline(always)]
    fn mix(x0: wu64, x1: wu64, r: u32) -> [wu64; 2] {
        let y0 = x0 + x1;
        [y0, x1.rotate_left(r) ^ y0]
    }

    #[inline(always)]
    fn key_schedule(k: &[wu64; 5], tw: &[wu64; 3], s: usize) -> [wu64; 4] {
        let ki = KS_K_IDX[s];
        let ti = KS_TW_IDX[s];
        [
            k[ki[0]],
            k[ki[1]] + tw[ti[0]],
            k[ki[2]] + tw[ti[1]],
            k[ki[3]] + s as u64,
        ]
    }

    #[inline(always)]
    fn next_block(&mut self) -> [wu64; 4] {
        let mut v = self.c;

        for r in 0..THREE_FISH_N_ROUNDS {
            let mut e = wrap![0u64; 4];
            if (r & 0b011) == 0 {
                let ksi = Self::key_schedule(&self.k, &self.tw, r >> 2);
                e[0] = v[0] + ksi[0];
                e[1] = v[1] + ksi[1];
                e[2] = v[2] + ksi[2];
                e[3] = v[3] + ksi[3];
            } else {
                e = v;
            }

            let mut f = wrap! [0u64; 4];
            let r_sh = THREEFISH_R_256[r & 7]; // r % 8
            let mx0 = Self::mix(e[0], e[1], r_sh[0]);
            f[0] = mx0[0];
            f[1] = mx0[1];
            let mx1 = Self::mix(e[2], e[3], r_sh[1]);
            f[2] = mx1[0];
            f[3] = mx1[1];

            for i in 0..v.len() {
                v[i] = f[THREEFISH_PI[i]];
            }
        }

        let ksi = Self::key_schedule(&self.k, &self.tw, THREE_FISH_N_ROUNDS.div_ceil(4));
        let dst = [
            (v[0] + ksi[0]) ^ self.c[0],
            (v[1] + ksi[1]) ^ self.c[1],
            (v[2] + ksi[2]) ^ self.c[2],
            (v[3] + ksi[3]) ^ self.c[3],
        ];

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

    /// Generates the next random `u64` values.
    #[inline]
    pub fn next_raw(&mut self) -> [wu64; 4] {
        if self.index >= 4 {
            self.buf = self.next_block();
            self.index = 0.into();
        }
        let val = self.buf;
        self.index += 4;
        val
    }
}

impl_ring_rng64! { Threefish256, 4, next_raw }

#[cfg(test)]
mod tests {
    use crate::Rng;

    use super::*;

    crate::safe_test! { Threefish256 }

    fn with_key(key: [u64; 4], tweak: [u64; 2]) -> Threefish256 {
        let mut k = wrap![0u64; 5];
        for i in 0..4 {
            k[i] = wrap!(key[i]);
        }
        k[4] = k[0] ^ k[1] ^ k[2] ^ k[3] ^ THREEFISH_C240;
        let (t0, t1) = (wrap!(tweak[0]), wrap!(tweak[1]));
        Threefish256 {
            c: wrap![0; 4],
            k,
            tw: [t0, t1, t0 ^ t1],
            index: wrap!(4),
            buf: wrap![0; 4],
            pos: wrap!(4),
        }
    }

    /// Skein 1.3 Threefish-256 KAT (key = tweak = plaintext = 0). The generator
    /// outputs E(ctr) XOR ctr, which equals E(0) for the first block.
    #[test]
    fn threefish256_skein_kat_and_reference() {
        let mut g = with_key([0; 4], [0; 2]);
        let block: Vec<u64> = (0..4).map(|_| g.nextu()).collect();
        assert_eq!(
            block,
            [0x94EEEA8B1F2ADA84, 0xADF103313EAE6670, 0x952419A1F4B16D53, 0xD83F13E63C9F6B11]
        );
        let key = [1, 2, 3, 4];
        let tweak = [5, 6];
        let mut g = with_key(key, tweak);
        for ctr in 0..8u64 {
            let mut want = crate::_reftest::threefish256(key, tweak, [ctr, 0, 0, 0]);
            want[0] ^= ctr;
            let got: Vec<u64> = (0..4).map(|_| g.nextu()).collect();
            assert_eq!(got, want, "ctr {ctr}");
        }
    }

    /// REVIEW B11: `Threefish256::new` started with `pos = 0`, so the first four
    /// outputs were always 0.
    #[test]
    fn threefish256_first_outputs_are_not_the_zero_buffer() {
        for seed in [0u64, 1, 123] {
            let mut g = Threefish256::new(seed);
            let first: Vec<u64> = (0..4).map(|_| g.nextu()).collect();
            assert!(first.iter().any(|&v| v != 0), "seed {seed}: {first:x?}");
        }
    }
}
