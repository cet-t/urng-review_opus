//! `rand_core` trait implementations for the 32-bit generators.

use crate::{cbrng::b32::*, prng::b32::*};

crate::_internal::impl_rand32! {
    Jsf32,
    Mt19937,
    Sfmt607,
    Sfmt1279,
    Sfmt2281,
    Sfmt4253,
    Sfmt11213,
    Sfmt19937,
    Sfmt44497,
    Sfmt86243,
    Sfmt132049,
    Sfmt216091,
    Pcg32,
    Philox32,
    Sfc32,
    SplitMix32,
    Squares32,
    Threefry32x2,
    Threefry32x4,
    Xoroshiro64Ss,
    Xorshift32,
    Xorshift128,
    Xorwow,
    Xoshiro128Pp,
    Xoshiro128Ss,
}

/// The cipher generators take the 256-bit key as their seed (eight
/// little-endian words, stream/nonce 0), so the full key space is reachable.
macro_rules! impl_rand_cipher {
    ($($ty:ident),*) => {
        $(
            impl rand_core::TryRng for crate::crng::b32::$ty {
                type Error = std::convert::Infallible;

                #[inline]
                fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
                    Ok(crate::Rng::nextu(self))
                }

                #[inline]
                fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
                    let lo = crate::Rng::nextu(self) as u64;
                    let hi = crate::Rng::nextu(self) as u64;
                    Ok((hi << 32) | lo)
                }

                fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
                    for chunk in dst.chunks_mut(4) {
                        let v = crate::Rng::nextu(self).to_le_bytes();
                        chunk.copy_from_slice(&v[..chunk.len()]);
                    }
                    Ok(())
                }
            }

            impl rand_core::SeedableRng for crate::crng::b32::$ty {
                type Seed = [u8; 32];

                fn from_seed(seed: Self::Seed) -> Self {
                    let mut key = [0u32; 8];
                    for (k, b) in key.iter_mut().zip(seed.chunks_exact(4)) {
                        *k = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    }
                    Self::from_key(key, 0)
                }
            }
        )*
    };
}
impl_rand_cipher!(ChaCha8, ChaCha20, Salsa8, Salsa20);

#[cfg(test)]
mod tests {
    use rand::{Rng as _, SeedableRng};

    use super::*;
    use crate::Rng;

    #[test]
    fn sfmt19937_works() {
        let mut rng0 = Sfmt19937::seed_from_u64(0);
        let mut rng1 = Sfmt19937::seed_from_u64(0);
        assert_eq!(rng0.next_u32(), rng1.next_u32());
    }

    #[test]
    fn philox32_works() {
        let mut rng0 = Philox32::seed_from_u64(0);
        let mut rng1 = Philox32::seed_from_u64(0);
        assert_eq!(rng0.next_u32(), rng1.next_u32());
    }

    /// REVIEW B17: seeds are little-endian on every host, `next_u64` takes the
    /// first word as its low half, and `fill_bytes` agrees with `next_u64`.
    #[test]
    fn rand32_conventions() {
        let mut a = Jsf32::from_seed([0x78, 0x56, 0x34, 0x12]);
        let mut b = Jsf32::new(0x1234_5678);
        assert_eq!(rand::Rng::next_u32(&mut a), b.nextu());

        let mut a = Sfc32::new(5);
        let mut b = Sfc32::new(5);
        let (w0, w1) = (b.nextu() as u64, b.nextu() as u64);
        assert_eq!(rand::Rng::next_u64(&mut a), (w1 << 32) | w0);

        let mut a = Xoshiro128Pp::new(6);
        let mut b = Xoshiro128Pp::new(6);
        let mut bytes = [0u8; 8];
        a.fill_bytes(&mut bytes);
        assert_eq!(u64::from_le_bytes(bytes), rand::Rng::next_u64(&mut b));
        // a partial tail uses the low bytes of the next word
        let mut a = Threefry32x4::new(7);
        let mut b = Threefry32x4::new(7);
        let mut bytes = [0u8; 6];
        a.fill_bytes(&mut bytes);
        let w0 = b.nextu().to_le_bytes();
        let w1 = b.nextu().to_le_bytes();
        assert_eq!(bytes, [w0[0], w0[1], w0[2], w0[3], w1[0], w1[1]]);
    }

    #[test]
    fn cipher_seed_is_the_key() {
        let mut seed = [0u8; 32];
        seed[0] = 1;
        seed[31] = 0x80;
        let mut key = [0u32; 8];
        key[0] = 1;
        key[7] = 0x8000_0000;
        let mut a = crate::ChaCha20::from_seed(seed);
        let mut b = crate::ChaCha20::from_key(key, 0);
        let mut c = crate::Salsa20::from_seed(seed);
        let mut d = crate::Salsa20::from_key(key, 0);
        for _ in 0..40 {
            assert_eq!(rand::Rng::next_u32(&mut a), b.nextu());
            assert_eq!(rand::Rng::next_u32(&mut c), d.nextu());
        }
        // usable through the rand API
        let x: f64 = rand::RngExt::random(&mut crate::ChaCha8::seed_from_u64(1));
        assert!((0.0..1.0).contains(&x));
    }
}