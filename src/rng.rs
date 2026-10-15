//! A 32/64-bit random number generator trait.

use crate::_internal::{i2f_bits, randi_wide, u2f_01};

mod sealed {
    /// Prevents `Word` from being implemented outside this crate, so the
    /// `u32 -> f32/i32` / `u64 -> f64/i64` width relationships stay closed.
    pub trait Sealed {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
}

/// The unsigned output word of a generator (`u32` or `u64`).
///
/// A `Word` fixes the whole numeric family of an RNG — its float type, its
/// signed-range integer type, and the bit width — and centralizes the
/// uniform-conversion formulas in one place so every [`Rng`] implementor
/// inherits them for free.
pub trait Word: sealed::Sealed + Copy {
    /// The float type produced from this word (`f32` for `u32`, `f64` for `u64`).
    type Float: Copy;
    /// The signed integer type used for ranged draws (`i32` / `i64`).
    type Int: Copy;

    /// Maps the word uniformly onto `[0, 1)`.
    #[must_use]
    fn to_f01(self) -> Self::Float;

    /// Maps the word onto the inclusive integer range `[min, max]` (multiply-shift
    /// without rejection: each value's probability differs from uniform by at
    /// most `range / 2^BITS`).
    ///
    /// # Panics
    ///
    /// If `min > max`.
    #[must_use]
    fn to_randi(self, min: Self::Int, max: Self::Int) -> Self::Int;

    /// Exactly uniform draw from `[min, max]` (Lemire's multiply-shift with
    /// rejection). Starts from `self` and pulls further words from `more` only
    /// when the first one falls in the rejection zone (probability below
    /// `range / 2^BITS`); the accepted result equals [`Word::to_randi`] of the
    /// accepted word.
    ///
    /// # Panics
    ///
    /// If `min > max`.
    #[must_use]
    fn to_randi_uniform<F: FnMut() -> Self>(self, min: Self::Int, max: Self::Int, more: F) -> Self::Int;

    /// Maps the word onto the half-open float range `[min, max)`. The result is
    /// always `< max` when `min < max` (rounding could previously produce `max`).
    #[must_use]
    fn to_randf(self, min: Self::Float, max: Self::Float) -> Self::Float;

    /// Maps the word uniformly onto a slice index in `[0, len)` (Lemire's
    /// multiply-shift). Equivalent to `to_randi(0, len as Int - 1) as usize`.
    #[must_use]
    fn to_index(self, len: usize) -> usize;

    /// Exactly uniform index in `[0, len)` (Lemire with rejection), pulling
    /// extra words from `more` only when needed. For `len > 2^BITS` a single
    /// word cannot be uniform and this falls back to [`Word::to_index`].
    /// Returns 0 for `len == 0`.
    #[must_use]
    fn to_index_uniform<F: FnMut() -> Self>(self, len: usize, more: F) -> usize;
}

macro_rules! impl_word {
    ($bits:tt) => {
        pastey::paste! {
            impl Word for [<u $bits>] {
                type Float = [<f $bits>];
                type Int = [<i $bits>];

                #[inline(always)]
                fn to_f01(self) -> Self::Float {
                    u2f_01!([<f $bits>], $bits, self)
                }

                #[inline(always)]
                fn to_randi(self, min: Self::Int, max: Self::Int) -> Self::Int {
                    assert!(min <= max, "to_randi: min must not exceed max");
                    let range = (max as randi_wide!(i $bits) - min as randi_wide!(i $bits) + 1)
                        as randi_wide!(u $bits);
                    // range <= 2^BITS, so the product fits the wide type; the final add
                    // wraps on purpose (k + min always lies in [min, max]). The old
                    // `as iN + min` overflowed in debug builds for ranges > 2^(BITS-1).
                    (((self as randi_wide!(u $bits) * range) >> $bits) as [<i $bits>]).wrapping_add(min)
                }

                #[inline]
                fn to_randi_uniform<F: FnMut() -> Self>(
                    self,
                    min: Self::Int,
                    max: Self::Int,
                    mut more: F,
                ) -> Self::Int {
                    assert!(min <= max, "to_randi: min must not exceed max");
                    let range = (max as randi_wide!(i $bits) - min as randi_wide!(i $bits) + 1)
                        as randi_wide!(u $bits);
                    if range >> $bits != 0 {
                        // full range: every word maps to exactly one value
                        return (self as [<i $bits>]).wrapping_add(min);
                    }
                    let s = range as [<u $bits>];
                    let mut m = self as randi_wide!(u $bits) * range;
                    if (m as [<u $bits>]) < s {
                        // 2^BITS mod s: low products below it would make some values
                        // one count more likely than others
                        let t = s.wrapping_neg() % s;
                        while (m as [<u $bits>]) < t {
                            m = more() as randi_wide!(u $bits) * range;
                        }
                    }
                    ((m >> $bits) as [<i $bits>]).wrapping_add(min)
                }

                #[inline(always)]
                fn to_randf(self, min: Self::Float, max: Self::Float) -> Self::Float {
                    let r = self.to_f01() * (max - min) + min;
                    if r >= max && max > min { max.next_down() } else { r }
                }

                #[inline(always)]
                fn to_index(self, len: usize) -> usize {
                    // u128 so that len > 2^32 cannot overflow with 32-bit words
                    ((self as u128 * len as u128) >> $bits) as usize
                }

                #[inline]
                fn to_index_uniform<F: FnMut() -> Self>(self, len: usize, mut more: F) -> usize {
                    let n = len as u128;
                    if len == 0 || n > (1u128 << $bits) {
                        return self.to_index(len);
                    }
                    if n == (1u128 << $bits) {
                        return self as usize;
                    }
                    let s = n as [<u $bits>];
                    let mut m = self as u128 * n;
                    if (m as [<u $bits>]) < s {
                        let t = s.wrapping_neg() % s;
                        while (m as [<u $bits>]) < t {
                            m = more() as u128 * n;
                        }
                    }
                    (m >> $bits) as usize
                }
            }
        }
    };
}

impl_word!(32);
impl_word!(64);

/// A random number generator characterized by a single output [`Word`].
///
/// Implementors provide only [`nextu`](Rng::nextu); the ranged and float
/// draws are supplied as defaults derived from the associated [`Word`], so the
/// numeric formulas live in exactly one place.
pub trait Rng {
    /// The unsigned output word this generator produces.
    type Word: Word;

    /// Generates the next raw word in `[0, 2^BITS)`.
    #[must_use]
    fn nextu(&mut self) -> Self::Word;

    /// Generates the next float in `[0, 1)`.
    #[must_use]
    #[inline(always)]
    fn nextf(&mut self) -> <Self::Word as Word>::Float {
        self.nextu().to_f01()
    }

    /// Generates an integer uniformly in the inclusive range `[min, max]`
    /// (REVIEW B13: rejection step added; usually consumes one word, rarely
    /// more — see [`Word::to_randi_uniform`]).
    ///
    /// # Panics
    ///
    /// If `min > max`.
    #[must_use]
    #[inline]
    fn randi(
        &mut self,
        min: <Self::Word as Word>::Int,
        max: <Self::Word as Word>::Int,
    ) -> <Self::Word as Word>::Int {
        let first = self.nextu();
        first.to_randi_uniform(min, max, || self.nextu())
    }

    /// Generates a float in the half-open range `[min, max)` (strictly below `max`).
    #[must_use]
    #[inline(always)]
    fn randf(
        &mut self,
        min: <Self::Word as Word>::Float,
        max: <Self::Word as Word>::Float,
    ) -> <Self::Word as Word>::Float {
        self.nextu().to_randf(min, max)
    }
}

/// Constructs a generator from a raw seed of its native word width.
pub trait Seed {
    /// The seed type (`u32` or `u64`, matching the generator's word width).
    type Seed;

    /// Builds `Self` from a seed.
    #[must_use]
    fn from_seed(seed: Self::Seed) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REVIEW B13: `randi(i32::MIN, i32::MAX)` overflowed in debug builds.
    #[test]
    fn full_range_randi_does_not_overflow() {
        for x in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
            assert_eq!(x.to_randi(i32::MIN, i32::MAX), (x as i32).wrapping_add(i32::MIN));
        }
        for x in [0u64, 1, u64::MAX / 2 + 1, u64::MAX] {
            assert_eq!(x.to_randi(i64::MIN, i64::MAX), (x as i64).wrapping_add(i64::MIN));
        }
        assert_eq!(u32::MAX.to_randi(-3, -3), -3);
        assert_eq!(u32::MAX.to_randi(i32::MAX - 1, i32::MAX), i32::MAX);
    }

    #[test]
    #[should_panic(expected = "min must not exceed max")]
    fn randi_rejects_inverted_range() {
        let _ = 5u32.to_randi(10, 0);
    }

    /// REVIEW B13: `randf(100, 101)` could round up to exactly 101.0. Checked
    /// exhaustively over the words that produce the largest unit floats.
    #[test]
    fn randf_is_strictly_below_max() {
        for k in (1u32 << 23) - 4096..(1u32 << 23) {
            let w = (k << 9) | 0x1FF;
            for (lo, hi) in [(100.0f32, 101.0f32), (-1.0, 1e-30), (1e6, 1e6 + 1.0), (0.0, 1.0)] {
                let r = w.to_randf(lo, hi);
                assert!(r >= lo && r < hi, "{lo}..{hi}: {r}");
            }
        }
        for k in (1u64 << 52) - 4096..(1u64 << 52) {
            let w = (k << 11) | 0x7FF;
            let lo = (1u64 << 40) as f64;
            let r = w.to_randf(lo, lo + 1.0);
            assert!(r >= lo && r < lo + 1.0, "{r}");
        }
    }

    /// REVIEW B13(c): with range 3*2^30 plain multiply-shift gives value
    /// residue 0 (mod 3) with probability 1/2 instead of 1/3. `randi` must be
    /// uniform.
    #[test]
    fn randi_is_uniform_for_awkward_ranges() {
        let min = i32::MIN;
        let max = (min as i64 + 3 * (1i64 << 30) - 1) as i32;
        let mut g = crate::SplitMix32::new(1);
        let n = 300_000;
        let mut counts = [0usize; 3];
        for _ in 0..n {
            let v = g.randi(min, max);
            assert!(v >= min && v <= max);
            counts[((v as i64 - min as i64) % 3) as usize] += 1;
        }
        for c in counts {
            let p = c as f64 / n as f64;
            assert!((p - 1.0 / 3.0).abs() < 0.005, "residue frequencies {counts:?}");
        }
        // the old mapping is measurably biased on the same stream
        let mut g = crate::SplitMix32::new(1);
        let zeros = (0..n).filter(|_| (g.nextu().to_randi(min, max) as i64 - min as i64) % 3 == 0).count();
        assert!((zeros as f64 / n as f64 - 0.5).abs() < 0.01);
    }

    /// The rejection path draws again and keeps the multiply-shift result of
    /// the accepted word.
    #[test]
    fn rejection_redraws_and_accepts_multiply_shift() {
        struct Script(Vec<u32>);
        impl Rng for Script {
            type Word = u32;
            fn nextu(&mut self) -> u32 {
                self.0.remove(0)
            }
        }
        let (min, max) = (0i32, 3 * (1 << 29) - 1); // s = 3*2^29, 2^32 mod s = 2^30
        // low product = 2^29 * (3x mod 8): rejected iff x = 0 or 3 (mod 8)
        let mut s = Script(vec![0, 3, 8, u32::MAX]);
        assert_eq!(s.randi(min, max), u32::MAX.to_randi(min, max));
        assert!(s.0.is_empty());
        // accepted on the first word: identical to to_randi
        let mut g = crate::SplitMix32::new(9);
        let mut h = crate::SplitMix32::new(9);
        for _ in 0..10_000 {
            assert_eq!(g.randi(-5, 17), h.nextu().to_randi(-5, 17));
        }
        // 64-bit, full and single-value ranges
        let mut g = crate::SplitMix64::new(3);
        for _ in 0..1000 {
            let _ = g.randi(i64::MIN, i64::MAX);
            assert_eq!(g.randi(42, 42), 42);
        }
    }

    #[test]
    fn to_index_uniform_rejects_like_randi() {
        let mut g = crate::SplitMix32::new(4);
        let mut h = crate::SplitMix32::new(4);
        for len in [1usize, 2, 3, 7, 1000, 3 << 29, 1 << 31] {
            for _ in 0..2000 {
                let a = { let x = g.nextu(); x.to_index_uniform(len, || g.nextu()) };
                let b = { let x = h.nextu(); x.to_randi_uniform(0, (len - 1) as i32, || h.nextu()) } as usize;
                assert_eq!(a, b, "len {len}");
            }
        }
        assert_eq!(7u32.to_index_uniform(0, || unreachable!()), 0);
        assert_eq!(u64::MAX.to_index_uniform(usize::MAX, || unreachable!()), usize::MAX - 1);
    }

    #[test]
    fn to_index_handles_huge_lengths() {
        let len = 1usize << 40;
        assert!(u32::MAX.to_index(len) < len);
        assert_eq!(0u32.to_index(len), 0);
        assert_eq!(u32::MAX.to_index(10), 9);
    }
}
