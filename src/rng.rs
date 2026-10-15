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

    /// Maps the word onto the half-open float range `[min, max)`. The result is
    /// always `< max` when `min < max` (rounding could previously produce `max`).
    #[must_use]
    fn to_randf(self, min: Self::Float, max: Self::Float) -> Self::Float;

    /// Maps the word uniformly onto a slice index in `[0, len)` (Lemire's
    /// multiply-shift). Equivalent to `to_randi(0, len as Int - 1) as usize`.
    #[must_use]
    fn to_index(self, len: usize) -> usize;
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

    /// Generates an integer in the inclusive range `[min, max]` (see
    /// [`Word::to_randi`] for the bias bound).
    ///
    /// # Panics
    ///
    /// If `min > max`.
    #[must_use]
    #[inline(always)]
    fn randi(
        &mut self,
        min: <Self::Word as Word>::Int,
        max: <Self::Word as Word>::Int,
    ) -> <Self::Word as Word>::Int {
        self.nextu().to_randi(min, max)
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

    #[test]
    fn to_index_handles_huge_lengths() {
        let len = 1usize << 40;
        assert!(u32::MAX.to_index(len) < len);
        assert_eq!(0u32.to_index(len), 0);
        assert_eq!(u32::MAX.to_index(10), 9);
    }
}
