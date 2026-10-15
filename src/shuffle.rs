/// Error returned when attempting to shuffle an empty slice.
#[derive(Debug)]
pub struct SliceShuffleError(pub(crate) ());
impl std::fmt::Display for SliceShuffleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cannot shuffle an empty slice.")
    }
}
impl std::error::Error for SliceShuffleError {}

/// Result type for shuffle operations, failing on an empty slice.
pub type ShuffleResult<T> = std::result::Result<T, self::SliceShuffleError>;

/// An iterator that yields references to the elements of a slice in a shuffled order.
///
/// Returned by [`Shuffle::shuffled`].
pub struct ShuffledIterator<'a, T> {
    pub(crate) slice: &'a [T],
    pub(crate) indices: Vec<usize>,
    pub(crate) pos: usize,
}

impl<'a, T> Iterator for ShuffledIterator<'a, T> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<&'a T> {
        let index = *self.indices.get(self.pos)?;
        self.pos += 1;
        Some(&self.slice[index])
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.indices.len() - self.pos;
        (remaining, Some(remaining))
    }
}

use crate::rng::{Rng, Word};

/// Shuffles a slice in-place or returns a shuffled iterator, for any [`Rng`].
pub trait Shuffle: Rng {
    /// Shuffles `src` in place.
    #[inline(always)]
    fn shuffle<T>(&mut self, src: &mut [T]) -> crate::ShuffleResult<()> {
        if src.is_empty() {
            Err(crate::SliceShuffleError(()))
        } else {
            // Fisher–Yates (Durstenfeld): j is drawn from [0, i], not [0, len).
            // The previous "swap with any index" loop produced biased permutations.
            for i in (1..src.len()).rev() {
                let j = self.nextu().to_index(i + 1);
                src.swap(i, j);
            }
            Ok(())
        }
    }

    /// Returns an iterator over `src` in a shuffled order.
    #[inline(always)]
    fn shuffled<'a, T>(
        &mut self,
        src: &'a [T],
    ) -> crate::ShuffleResult<crate::ShuffledIterator<'a, T>> {
        if src.is_empty() {
            return Err(crate::SliceShuffleError(()));
        }
        let mut indices: Vec<usize> = (0..src.len()).collect();
        self.shuffle(&mut indices)?;
        Ok(crate::ShuffledIterator {
            slice: src,
            indices,
            pos: 0,
        })
    }
}

impl<R: Rng + ?Sized> Shuffle for R {}

#[cfg(test)]
mod tests {
    use crate::{Shuffle, Xoshiro256Pp};
    use std::collections::HashMap;

    /// REVIEW B5: every permutation must be equally likely. The old loop gave
    /// probabilities 2/9 and 1/9 for n = 3.
    fn check_uniform(n: usize, trials: usize, use_iter: bool) {
        let mut rng = Xoshiro256Pp::new(42);
        let mut counts: HashMap<Vec<usize>, usize> = HashMap::new();
        let base: Vec<usize> = (0..n).collect();
        for _ in 0..trials {
            let p = if use_iter {
                rng.shuffled(&base).unwrap().copied().collect()
            } else {
                let mut v = base.clone();
                rng.shuffle(&mut v).unwrap();
                v
            };
            *counts.entry(p).or_default() += 1;
        }
        let perms: usize = (1..=n).product();
        assert_eq!(counts.len(), perms, "every permutation must occur");
        let expected = trials as f64 / perms as f64;
        let chi2: f64 = counts
            .values()
            .map(|&c| (c as f64 - expected).powi(2) / expected)
            .sum();
        // 99.99th percentile of chi-square: df=5 -> 25.7, df=23 -> 57.0
        let limit = if perms == 6 { 25.7 } else { 57.0 };
        assert!(chi2 < limit, "n={n}: chi2 = {chi2:.1} (limit {limit}), counts {counts:?}");
    }

    #[test]
    fn shuffle_is_uniform() {
        check_uniform(3, 600_000, false);
        check_uniform(4, 480_000, false);
    }

    #[test]
    fn shuffled_is_uniform() {
        check_uniform(3, 600_000, true);
    }

    #[test]
    fn shuffle_edge_lengths() {
        let mut rng = Xoshiro256Pp::new(1);
        let mut empty: [u8; 0] = [];
        assert!(rng.shuffle(&mut empty).is_err());
        let mut one = [9];
        rng.shuffle(&mut one).unwrap();
        assert_eq!(one, [9]);
    }
}
