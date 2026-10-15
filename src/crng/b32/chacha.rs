use wrapn::{wrap, wu32, wusize};

use crate::{SplitMix64, impl_ring_rng32};

/// ChaCha stream cipher (D. J. Bernstein) used as a random number generator,
/// generic over the round count (`ChaCha8` = 8 rounds, `ChaCha20` = 20 rounds).
///
/// The block function follows RFC 8439 (constants, key, feed-forward). The
/// state uses the original 64-bit block counter (words 12–13) and 64-bit
/// stream/nonce (words 14–15) layout, as in `rand_chacha`. The keystream is
/// returned as consecutive 32-bit little-endian words.
///
/// # Security
///
/// [`ChaCha::new`] derives the key from a 32-bit seed, so its output is
/// trivially brute-forceable and **must not be used for cryptographic
/// purposes**. For secret keystreams, use [`ChaCha::from_key`] with a 256-bit
/// key from a secure source. `Debug` does not print the key.
///
/// # Example
/// ```
/// use urng::{Rng, ChaCha20};
///
/// let mut rng = ChaCha20::new(12345);
/// let _ = rng.nextu();
/// ```
#[repr(C, align(64))]
#[derive(Clone)]
pub struct ChaCha<const ROUNDS: usize> {
    /// Block input: constants, key, counter, nonce.
    x: [wu32; 16],

    pub(crate) buf: [wu32; 16],
    pub(crate) pos: wusize,
}

impl<const ROUNDS: usize> core::fmt::Debug for ChaCha<ROUNDS> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChaCha").field("rounds", &ROUNDS).finish_non_exhaustive()
    }
}

/// "expand 32-byte k"
const SIGMA: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

impl<const ROUNDS: usize> ChaCha<ROUNDS> {
    const _ROUNDS_EVEN: () = assert!(ROUNDS % 2 == 0 && ROUNDS > 0, "ROUNDS must be even");

    /// Creates a new instance whose key is expanded from a 32-bit `seed`
    /// (not suitable for cryptography, see the type-level docs).
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

    /// Creates an instance from a 256-bit key (eight little-endian words) and a
    /// 64-bit stream/nonce; the block counter starts at 0.
    pub fn from_key(key: [u32; 8], stream: u64) -> Self {
        let _ = Self::_ROUNDS_EVEN;
        let mut x = [0u32; 16];
        x[..4].copy_from_slice(&SIGMA);
        x[4..12].copy_from_slice(&key);
        x[14] = stream as u32;
        x[15] = (stream >> 32) as u32;
        Self {
            x: x.map(|w| wrap!(w)),
            buf: wrap![0_u32; 16],
            pos: wrap!(16),
        }
    }

    /// Sets the 64-bit block counter (words 12–13) and discards buffered output.
    pub fn set_block_counter(&mut self, counter: u64) {
        self.x[12] = wrap!(counter as u32);
        self.x[13] = wrap!((counter >> 32) as u32);
        self.pos = wrap!(16);
    }

    fn qr(x: &mut [wu32; 16], a: usize, b: usize, c: usize, d: usize) {
        x[a] += x[b];
        x[d] ^= x[a];
        x[d] = x[d].rotate_left(16);
        x[c] += x[d];
        x[b] ^= x[c];
        x[b] = x[b].rotate_left(12);
        x[a] += x[b];
        x[d] ^= x[a];
        x[d] = x[d].rotate_left(8);
        x[c] += x[d];
        x[b] ^= x[c];
        x[b] = x[b].rotate_left(7);
    }

    /// Computes the keystream block for the current counter, then increments
    /// the 64-bit counter.
    fn next_raw(&mut self) -> [wu32; 16] {
        let mut x = self.x;
        for _ in 0..ROUNDS / 2 {
            Self::qr(&mut x, 0, 4, 8, 12);
            Self::qr(&mut x, 1, 5, 9, 13);
            Self::qr(&mut x, 2, 6, 10, 14);
            Self::qr(&mut x, 3, 7, 11, 15);
            Self::qr(&mut x, 0, 5, 10, 15);
            Self::qr(&mut x, 1, 6, 11, 12);
            Self::qr(&mut x, 2, 7, 8, 13);
            Self::qr(&mut x, 3, 4, 9, 14);
        }
        for (o, i) in x.iter_mut().zip(self.x.iter()) {
            *o += *i;
        }
        self.x[12] += 1;
        if self.x[12] == 0 {
            self.x[13] += 1;
        }
        x
    }
}

impl<const ROUNDS: usize> Default for ChaCha<ROUNDS> {
    fn default() -> Self {
        Self::new(crate::_internal::default_seed32())
    }
}

impl<const ROUNDS: usize> crate::Seed for ChaCha<ROUNDS> {
    type Seed = u32;

    #[inline]
    fn from_seed(seed: Self::Seed) -> Self {
        Self::new(seed)
    }
}

/// ChaCha with 8 rounds.
pub type ChaCha8 = ChaCha<8>;
impl_ring_rng32!(ChaCha8, 16, next_raw);

/// ChaCha with 20 rounds.
pub type ChaCha20 = ChaCha<20>;
impl_ring_rng32!(ChaCha20, 16, next_raw);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rng;
    use crate::_reftest as r;

    crate::safe_test! {
        ChaCha8,
        ChaCha20
    }

    /// RFC 8439 §2.3.2 block test vector (key 00..1f, nonce 00:00:00:09:
    /// 00:00:00:4a:00:00:00:00, block count 1). In the 64-bit counter layout the
    /// RFC's words 12..15 are counter = 0x09000000_00000001, stream = 0x4a000000.
    #[test]
    fn chacha20_rfc8439_block() {
        let key: [u32; 8] = std::array::from_fn(|i| {
            u32::from_le_bytes([4 * i as u8, 4 * i as u8 + 1, 4 * i as u8 + 2, 4 * i as u8 + 3])
        });
        let mut c = ChaCha20::from_key(key, 0x4a000000);
        c.set_block_counter(0x0900_0000_0000_0001);
        let got: Vec<u32> = (0..16).map(|_| c.nextu()).collect();
        let want: [u32; 16] = [
            0xe4e7f110, 0x15593bd1, 0x1fdd0f50, 0xc47120a3, 0xc7f4d1c7, 0x0368c033, 0x9aaa2204,
            0x4e6cd4c3, 0x466482d2, 0x09aa9f07, 0x05d7c214, 0xa2028bd9, 0xd19c12b5, 0xb94e16de,
            0xe883d0cb, 0x4e3c50a2,
        ];
        assert_eq!(got, want);
    }

    /// ChaCha8 against an independent reference, and the counter advances
    /// (REVIEW B9: the old code fed each output block back as the next input, so
    /// one block revealed the whole state and every later block).
    #[test]
    fn chacha8_matches_reference_and_is_not_self_feeding() {
        let key = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut c = ChaCha8::from_key(key, 99);
        let mut input = [0u32; 16];
        input[..4].copy_from_slice(&SIGMA);
        input[4..12].copy_from_slice(&key);
        input[14] = 99;
        let mut prev: Option<[u32; 16]> = None;
        for counter in 0..64u32 {
            input[12] = counter;
            let mut x = input;
            for _ in 0..4 {
                r::chacha_double_round(&mut x);
            }
            for i in 0..16 {
                x[i] = x[i].wrapping_add(input[i]);
            }
            let got: [u32; 16] = std::array::from_fn(|_| c.nextu());
            assert_eq!(got, x, "block {counter}");
            if let Some(p) = prev {
                let mut y = p;
                for _ in 0..4 {
                    r::chacha_double_round(&mut y);
                }
                assert_ne!(got, y, "next block must not be a permutation of the previous output");
            }
            prev = Some(got);
        }
    }

    #[test]
    fn debug_does_not_leak_key() {
        let s = format!("{:?}", ChaCha20::from_key([0xdeadbeef; 8], 0));
        assert!(!s.contains("deadbeef") && !s.contains("3735928559"), "{s}");
    }
}
