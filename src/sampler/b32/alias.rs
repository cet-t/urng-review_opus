use crate::rng::Rng;
use crate::sampler::Sampler;

/// Weighted sampler using Walker's Alias Method (O(1) sample, O(n) build).
///
/// Preferred over [`Bst32`](super::Bst32) when sampling repeatedly from the same weight distribution.
///
/// # Examples
///
/// ```
/// use urng::Sampler;
/// use urng::Alias32;
/// use urng::Mt19937;
///
/// let mut rng = Mt19937::new(1);
/// let mut sampler = Alias32::new(&mut rng, &[1.0f32, 9.0]);
/// assert!(sampler.sample() < 2);
///
/// // Update weights
/// sampler.weights(&[5.0, 5.0]);
/// assert!(sampler.sample() < 2);
/// ```
#[derive(Debug)]
pub struct Alias32<'a, R: Rng<Word = u32> + 'a> {
    rng: &'a mut R,
    prob: Vec<f32>,
    alias: Vec<usize>,
}

impl<'a, R: Rng<Word = u32> + 'a> Alias32<'a, R> {
    /// Creates a new sampler with the given random number generator and weights.
    ///
    /// # Panics
    ///
    /// If the weights are empty, negative, non-finite or sum to zero.
    pub fn new(rng: &'a mut R, weights: &[f32]) -> Self {
        let (prob, alias) = Self::build(weights);
        Self { rng, prob, alias }
    }

    fn build(weights: &[f32]) -> (Vec<f32>, Vec<usize>) {
        let total = crate::sampler::validate_weights_f32(weights);
        let n = weights.len();
        let scale = n as f32 / total;

        let mut prob = vec![0.0f32; n];
        let mut alias = vec![0usize; n];
        let mut small = Vec::with_capacity(n);
        let mut large = Vec::with_capacity(n);
        let mut p: Vec<f32> = weights.iter().map(|&w| w * scale).collect();

        for (i, &pi) in p.iter().enumerate() {
            if pi < 1.0 {
                small.push(i);
            } else {
                large.push(i);
            }
        }

        while let (Some(s), Some(l)) = (small.pop(), large.last().copied()) {
            prob[s] = p[s];
            alias[s] = l;
            p[l] -= 1.0 - p[s];
            if p[l] < 1.0 {
                large.pop();
                small.push(l);
            }
        }
        for i in large {
            prob[i] = 1.0;
        }
        for i in small {
            prob[i] = 1.0;
        }

        (prob, alias)
    }
}

impl<'a, R: Rng<Word = u32> + 'a> Sampler<'a, R> for Alias32<'a, R> {
    fn weights(&mut self, weights: &[f32]) {
        let (prob, alias) = Self::build(weights);
        self.prob = prob;
        self.alias = alias;
    }

    fn sample(&mut self) -> usize {
        let n = self.prob.len();
        let i = self.rng.randi(0, n as i32 - 1) as usize;
        let u = self.rng.randf(0.0, 1.0);
        if u < self.prob[i] { i } else { self.alias[i] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prng::b32::Mt19937;
    use crate::sampler::Sampler;

    #[test]
    fn alias32_works() {
        let mut rng = Mt19937::new(1);
        let mut sampler = Alias32::new(&mut rng, &[1.0f32, 9.0f32]);
        let ones = (0..100).filter(|_| sampler.sample() == 1).count();
        assert_eq!(ones, 91);
    }

    struct Const(u32);
    impl Rng for Const {
        type Word = u32;
        fn nextu(&mut self) -> u32 {
            self.0
        }
    }

    /// REVIEW B15: empty weights used to panic with an arithmetic error.
    #[test]
    #[should_panic(expected = "must not be empty")]
    fn alias32_rejects_empty_weights() {
        let mut c = Const(0);
        let _ = Alias32::new(&mut c, &[]);
    }

    #[test]
    fn alias32_never_selects_zero_weight() {
        let mut rng = <crate::prng::b32::Mt19937>::new(4);
        let mut s = Alias32::new(&mut rng, &[0.0, 2.0, 0.0, 6.0, 0.0]);
        let mut counts = [0usize; 5];
        for _ in 0..40_000 {
            counts[s.sample()] += 1;
        }
        assert_eq!(counts[0] + counts[2] + counts[4], 0, "{counts:?}");
        assert!((counts[3] as f64 / counts[1] as f64 - 3.0).abs() < 0.2, "{counts:?}");
    }}
