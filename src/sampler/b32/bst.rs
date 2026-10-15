use crate::rng::Rng;
use crate::sampler::Sampler;

/// Weighted sampler using cumulative sums and binary search (O(log n) sample, O(n) build).
///
/// # Examples
///
/// ```
/// use urng::Sampler;
/// use urng::Bst32;
/// use urng::Mt19937;
///
/// let mut rng = Mt19937::new(1);
/// let mut sampler = Bst32::new(&mut rng, &[1.0f32, 9.0]);
/// assert!(sampler.sample() < 2);
/// ```
#[derive(Debug)]
pub struct Bst32<'a, R: Rng<Word = u32> + 'a> {
    rng: &'a mut R,
    cumulative: Vec<f32>,
}

impl<'a, R: Rng<Word = u32> + 'a> Bst32<'a, R> {
    /// Creates a new sampler with the given random number generator and weights.
    ///
    /// # Panics
    ///
    /// If the weights are empty, negative, non-finite or sum to zero.
    pub fn new(rng: &'a mut R, weights: &[f32]) -> Self {
        Self {
            rng,
            cumulative: Self::build_cumulative(weights),
        }
    }

    fn build_cumulative(weights: &[f32]) -> Vec<f32> {
        crate::sampler::validate_weights_f32(weights);
        let mut cumulative = Vec::with_capacity(weights.len());
        let mut sum = 0.0f32;
        for &w in weights {
            sum += w;
            cumulative.push(sum);
        }
        cumulative
    }
}

impl<'a, R: Rng<Word = u32> + 'a> Sampler<'a, R> for Bst32<'a, R> {
    fn weights(&mut self, weights: &[f32]) {
        self.cumulative = Self::build_cumulative(weights);
    }

    fn sample(&mut self) -> usize {
        let total = *self.cumulative.last().unwrap_or(&0.0);
        let r = self.rng.randf(0.0, total);
        // first entry strictly above r: a zero-weight entry repeats the previous
        // cumulative value and is skipped (REVIEW B15: binary_search_by could stop
        // on it when r equalled that value exactly)
        self.cumulative
            .partition_point(|&c| c <= r)
            .min(self.cumulative.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prng::b32::Mt19937;
    use crate::sampler::Sampler;

    #[test]
    fn bst32_works() {
        let mut rng = Mt19937::new(1);
        let mut sampler = Bst32::new(&mut rng, &[1.0f32, 9.0f32]);
        let ones = (0..100).filter(|_| sampler.sample() == 1).count();
        assert_eq!(ones, 87);
    }

    /// Always returns the same word (0x80.. maps to exactly 0.5 in [0, 1)).
    struct Const(u32);
    impl Rng for Const {
        type Word = u32;
        fn nextu(&mut self) -> u32 {
            self.0
        }
    }

    /// REVIEW B15: with weights [1, 0, 1] and r == 1.0 exactly, the old binary
    /// search could return the zero-weight index 1.
    #[test]
    fn bst32_never_selects_zero_weight_at_a_boundary() {
        let mut c = Const(0x8000_0000);
        let mut s = Bst32::new(&mut c, &[1.0, 0.0, 1.0]);
        assert_eq!(s.sample(), 2);
        let mut c = Const(0);
        let mut s = Bst32::new(&mut c, &[0.0, 0.0, 3.0]);
        assert_eq!(s.sample(), 2, "leading zero weights are skipped too");

        let mut rng = <crate::prng::b32::Mt19937>::new(3);
        let mut s = Bst32::new(&mut rng, &[1.0, 0.0, 3.0, 0.0]);
        let mut counts = [0usize; 4];
        for _ in 0..40_000 {
            counts[s.sample()] += 1;
        }
        assert_eq!((counts[1], counts[3]), (0, 0), "{counts:?}");
        assert!((counts[2] as f64 / counts[0] as f64 - 3.0).abs() < 0.2, "{counts:?}");
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn bst32_rejects_empty_weights() {
        let mut c = Const(0);
        let _ = Bst32::new(&mut c, &[]);
    }

    #[test]
    #[should_panic(expected = "positive, finite sum")]
    fn bst32_rejects_all_zero_weights() {
        let mut c = Const(0);
        let _ = Bst32::new(&mut c, &[0.0, 0.0]);
    }

    #[test]
    #[should_panic(expected = "finite and non-negative")]
    fn bst32_rejects_negative_weights() {
        let mut c = Const(0);
        let mut s = Bst32::new(&mut c, &[1.0]);
        s.weights(&[1.0, -1.0]);
    }}
