//! Regression tests for REVIEW B4: `SplitMix32/64::new` forced the seed odd
//! (`seed | 1`), so seeds 2k and 2k+1 produced identical streams for every
//! generator seeded through SplitMix, and `TwistedGFSR::new` ignored the upper
//! 32 seed bits.

use urng::*;

fn first<R: Rng>(mut r: R, n: usize) -> Vec<u64>
where
    R::Word: Into<u64>,
{
    (0..n).map(|_| r.nextu().into()).collect()
}

macro_rules! assert_adjacent_seeds_differ {
    ($($ty:ty),+ $(,)?) => {$(
        for k in 0..64u64 {
            let a = first(<$ty>::new((2 * k) as _), 16);
            let b = first(<$ty>::new((2 * k + 1) as _), 16);
            assert_ne!(a, b, "{}: seeds {} and {} give the same stream", stringify!($ty), 2 * k, 2 * k + 1);
        }
    )+};
}

#[test]
fn adjacent_seeds_give_different_streams_32() {
    assert_adjacent_seeds_differ!(
        SplitMix32, Sfc32, Jsf32, Pcg32, Mt19937, Sfmt19937, Sfmt607, Sfmt216091, Xoshiro128Pp,
        Xoshiro128Ss, Xoroshiro64Ss, Xorshift32, Xorshift128, Xorwow, Philox32, Threefry32x2,
        Threefry32x4, Squares32, ChaCha8, ChaCha20, Salsa8, Salsa20,
    );
}

#[test]
fn adjacent_seeds_give_different_streams_64() {
    assert_adjacent_seeds_differ!(
        SplitMix64, Xoshiro256Pp, Xoshiro256Ss, Xoroshiro128Pp, Xoroshiro128Ss, Sfc64, Biski64,
        Cet64, Cet256, Mt1993764, Sfmt1993764, Philox64, Philox4x64, Threefish256, WyHash64,
        Xorshift64, TwistedGFSR,
    );
}

#[test]
fn twisted_gfsr_uses_all_64_seed_bits() {
    assert_ne!(
        first(TwistedGFSR::new(5), 64),
        first(TwistedGFSR::new(5 + (1u64 << 32)), 64)
    );
}

/// With the forced-odd seed removed, `SplitMix64::new(s)` is exactly the
/// reference SplitMix64 started from state `s` (Steele, Lea & Flood 2014).
#[test]
fn splitmix64_matches_reference_for_every_seed() {
    fn reference(mut x: u64) -> impl FnMut() -> u64 {
        move || {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
    }
    for seed in [0u64, 1, 2, 1234567, u64::MAX] {
        let mut r = reference(seed);
        let mut g = SplitMix64::new(seed);
        for _ in 0..100 {
            assert_eq!(g.nextu(), r());
        }
    }
}

#[cfg(feature = "wide")]
#[test]
fn adjacent_seeds_give_different_streams_wide() {
    use urng::wide::*;
    macro_rules! wide_differ {
        ($($ty:ty),+) => {$(
            for k in 0..32u64 {
                let mut a = <$ty>::new((2 * k) as _);
                let mut b = <$ty>::new((2 * k + 1) as _);
                assert_ne!(a.nextu(), b.nextu(), "{}: seeds {} / {}", stringify!($ty), 2 * k, 2 * k + 1);
            }
        )+};
    }
    wide_differ!(
        SplitMix32x4, Sfc32x8, Jsf32x8, Pcg32x8, Squares32x8, Xoshiro128Ppx8, Xoroshiro64Ssx8,
        Xorshift128x8, Xorwowx8, Threefry32x4x8
    );
}
