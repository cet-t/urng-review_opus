use wrapn::{wrap, wu32, wusize};

use crate::{SplitMix64, impl_ring_rng32};

/// Salsa20 stream cipher (D. J. Bernstein) used as a random number generator,
/// generic over the round count (`Salsa8` = Salsa20/8, `Salsa20` = Salsa20/20).
///
/// Uses the specification's 256-bit-key state layout (constants on the
/// diagonal, 64-bit nonce in words 6–7, 64-bit block counter in words 8–9) and
/// feed-forward. The keystream is returned as consecutive 32-bit LE words.
///
/// # Security
///
/// [`Salsa::new`] derives the key from a 32-bit seed and **must not be used for
/// cryptographic purposes**; use [`Salsa::from_key`] with a secret 256-bit key.
///
/// # Example
/// ```
/// use urng::{Rng, Salsa20};
///
/// let mut rng = Salsa20::new(12345);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Clone)]
pub struct Salsa<const ROUNDS: usize> {
    x: [wu32; 16],

    pub(crate) buf: [wu32; 16],
    pub(crate) pos: wusize,
}

impl<const ROUNDS: usize> core::fmt::Debug for Salsa<ROUNDS> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Salsa").field("rounds", &ROUNDS).finish_non_exhaustive()
    }
}

impl<const ROUNDS: usize> Salsa<ROUNDS> {
    const _ROUNDS_EVEN: () = assert!(ROUNDS % 2 == 0 && ROUNDS > 0, "ROUNDS must be even");

    /// Creates a new instance whose key is expanded from a 32-bit `seed`
    /// (not suitable for cryptography).
    pub fn new(seed: u32) -> Self {
        let mut sg = SplitMix64::new(seed as u64);
        let mut key = [0u32; 8];
        for pair in key.chunks_exact_mut(2) {
            let v = sg.nextu_const();
            pair[0] = v as u32;
            pair[1] = (v >> 32) as u32;
        }
        Self::from_key(key, 0)
    }

    /// Creates an instance from a 256-bit key (eight LE words) and a 64-bit
    /// nonce; the block counter starts at 0.
    pub fn from_key(key: [u32; 8], nonce: u64) -> Self {
        let _ = Self::_ROUNDS_EVEN;
        let mut x = [0u32; 16];
        // "expand 32-byte k"
        x[0] = 0x61707865;
        x[5] = 0x3320646e;
        x[10] = 0x79622d32;
        x[15] = 0x6b206574;
        x[1..5].copy_from_slice(&key[..4]);
        x[11..15].copy_from_slice(&key[4..]);
        x[6] = nonce as u32;
        x[7] = (nonce >> 32) as u32;
        Self {
            x: x.map(|w| wrap!(w)),
            buf: wrap![0_u32; 16],
            pos: wrap!(16),
        }
    }

    fn qr(x: &mut [wu32; 16], a: usize, b: usize, c: usize, d: usize) {
        x[b] ^= (x[a] + x[d]).rotate_left(7);
        x[c] ^= (x[b] + x[a]).rotate_left(9);
        x[d] ^= (x[c] + x[b]).rotate_left(13); // was 9
        x[a] ^= (x[d] + x[c]).rotate_left(18);
    }

    fn next_raw(&mut self) -> [wu32; 16] {
        let mut x = self.x;
        for _ in 0..ROUNDS / 2 {
            Self::qr(&mut x, 0, 4, 8, 12);
            Self::qr(&mut x, 5, 9, 13, 1);
            Self::qr(&mut x, 10, 14, 2, 6);
            Self::qr(&mut x, 15, 3, 7, 11);

            Self::qr(&mut x, 0, 1, 2, 3);
            Self::qr(&mut x, 5, 6, 7, 4);
            Self::qr(&mut x, 10, 11, 8, 9);
            Self::qr(&mut x, 15, 12, 13, 14);
        }
        for (o, i) in x.iter_mut().zip(self.x.iter()) {
            *o += *i;
        }
        self.x[8] += 1;
        if self.x[8] == 0 {
            self.x[9] += 1;
        }
        x
    }
}

impl<const ROUNDS: usize> Default for Salsa<ROUNDS> {
    fn default() -> Self {
        Self::new(crate::_internal::default_seed32())
    }
}

impl<const ROUNDS: usize> crate::Seed for Salsa<ROUNDS> {
    type Seed = u32;

    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        Self::new(seed)
    }
}

/// Salsa20/8.
pub type Salsa8 = Salsa<8>;
impl_ring_rng32!(Salsa8, 16, next_raw);

/// Salsa20/20.
pub type Salsa20 = Salsa<20>;
impl_ring_rng32!(Salsa20, 16, next_raw);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;
    use crate::_reftest as r;

    crate::safe_test! {
        Salsa8,
        Salsa20
    }

    fn keystream_bytes<const R: usize>(g: &mut Salsa<R>, n: usize) -> Vec<u8>
    where
        Salsa<R>: Rng<Word = u32>,
    {
        (0..n / 4).flat_map(|_| g.nextu().to_le_bytes()).collect()
    }

    /// eSTREAM Salsa20/20 256-bit key, Set 1 vector 0 (key = 80 00 .. 00,
    /// IV = 0): first 64 keystream bytes.
    #[test]
    fn salsa20_estream_set1_vector0() {
        let mut g = Salsa20::from_key([0x80, 0, 0, 0, 0, 0, 0, 0], 0);
        let hex = "E3BE8FDD8BECA2E3EA8EF9475B29A6E7003951E1097A5C38D23B7A5FAD9F6844\
                   B22C97559E2723C7CBBD3FE4FC8D9A0744652A83E72A9C461876AF4D7EF1A117";
        let want: Vec<u8> =
            (0..64).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        assert_eq!(keystream_bytes(&mut g, 64), want);
    }

    /// Quarter-round values from the Salsa20 specification, and Salsa20/8
    /// against an independent reference with feed-forward and counter.
    #[test]
    fn salsa_spec_quarterround_and_reference() {
        assert_eq!(r::salsa_qr([1, 0, 0, 0], 13), [0x08008145, 0x00000080, 0x00010200, 0x20500000]);
        let mut x: [wu32; 16] = wrap![0; 16];
        x[0] = wrap!(1);
        Salsa20::qr(&mut x, 0, 1, 2, 3);
        assert_eq!([*x[0], *x[1], *x[2], *x[3]], [0x08008145, 0x00000080, 0x00010200, 0x20500000]);

        let key = [9, 8, 7, 6, 5, 4, 3, 2];
        let mut g = Salsa8::from_key(key, 0x1122_3344_5566_7788);
        let mut input = [0u32; 16];
        input[0] = 0x61707865;
        input[5] = 0x3320646e;
        input[10] = 0x79622d32;
        input[15] = 0x6b206574;
        input[1..5].copy_from_slice(&key[..4]);
        input[11..15].copy_from_slice(&key[4..]);
        input[6] = 0x5566_7788;
        input[7] = 0x1122_3344;
        for counter in 0..32u32 {
            input[8] = counter;
            let mut y = input;
            for _ in 0..4 {
                r::salsa_double_round(&mut y, 13);
            }
            for i in 0..16 {
                y[i] = y[i].wrapping_add(input[i]);
            }
            let got: [u32; 16] = std::array::from_fn(|_| g.nextu());
            assert_eq!(got, y, "block {counter}");
        }
    }
}
