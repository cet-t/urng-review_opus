//! `rand_core` trait implementations for the 64-bit generators.

use crate::{cbrng::b64::*, prng::b64::*};

crate::_internal::impl_rand64! {
    Biski64,
    Cet64,
    Cet256,
    Mt1993764,
    Sfmt1993764,
    Philox64,
    Philox4x64,
    Sfc64,
    SplitMix64,
    Threefish256,
    TwistedGFSR,
    Xoroshiro128Pp,
    Xoroshiro128Ss,
    Xorshift64,
    Xoshiro256Pp,
    Xoshiro256Ss,
    WyHash64
}

#[cfg(test)]
mod tests {
    use rand::{Rng as _, SeedableRng};

    use super::*;
    use crate::Rng;

    /// REVIEW B17: 64-bit generators implement `TryRng` (and therefore
    /// `rand::Rng`), take an 8-byte little-endian seed and keep all 64 bits.
    #[test]
    fn rand64_conventions() {
        let mut a = Xoshiro256Pp::from_seed(0x0123_4567_89AB_CDEFu64.to_le_bytes());
        let mut b = Xoshiro256Pp::new(0x0123_4567_89AB_CDEF);
        assert_eq!(rand::Rng::next_u64(&mut a), b.nextu());
        assert_eq!(rand::Rng::next_u32(&mut a), (b.nextu() >> 32) as u32);
        let mut bytes = [0u8; 12];
        a.fill_bytes(&mut bytes);
        let w0 = b.nextu().to_le_bytes();
        let w1 = b.nextu().to_le_bytes();
        assert_eq!(bytes[..8], w0);
        assert_eq!(bytes[8..], w1[..4]);
        // seeds differing only in the high 32 bits give different streams
        let mut lo = Sfc64::from_seed(1u64.to_le_bytes());
        let mut hi = Sfc64::from_seed((1u64 | 1 << 40).to_le_bytes());
        assert_ne!(rand::Rng::next_u64(&mut lo), rand::Rng::next_u64(&mut hi));
        let x: f64 = rand::RngExt::random(&mut Philox4x64::seed_from_u64(3));
        assert!((0.0..1.0).contains(&x));
    }
}