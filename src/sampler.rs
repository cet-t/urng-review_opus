//! Weighted random selection.

pub mod b32;
pub mod b64;

pub use b32::*;
pub use b64::*;

use crate::rng::{Rng, Word};

/// Weighted random sampling trait, generic over the generator's [`Word`].
///
/// Implementors provide O(1) or O(log n) weighted index selection.
/// Available implementations: [`crate::sampler::b32::Bst32`], [`crate::sampler::b32::Alias32`],
/// [`crate::sampler::b64::Bst64`], [`crate::sampler::b64::Alias64`].
///
/// # Examples
///
/// ```
/// use urng::{Sampler, Alias32, Mt19937};
///
/// let mut rng = Mt19937::new(1);
/// let mut sampler = Alias32::new(&mut rng, &[1.0f32, 9.0]);
/// assert!(sampler.sample() < 2);
/// ```
///
/// # Panics
///
/// Constructors and [`Sampler::weights`] panic unless the weights are
/// non-empty, finite and non-negative with a positive, finite sum
/// (REVIEW B15: empty weights used to underflow in debug builds and return an
/// out-of-range index 0 in release builds).
pub trait Sampler<'a, R: Rng + 'a> {
    /// Samples a random index based on the weights.
    fn sample(&mut self) -> usize;
    /// Updates the weights of the sampler.
    fn weights(&mut self, weights: &[<R::Word as Word>::Float]);
}

macro_rules! impl_validate_weights {
    ($name:ident, $f:ty) => {
        /// Checks the sampler weight contract and returns the total weight.
        pub(crate) fn $name(weights: &[$f]) -> $f {
            assert!(!weights.is_empty(), "sampler weights must not be empty");
            assert!(
                weights.iter().all(|w| w.is_finite() && *w >= 0.0),
                "sampler weights must be finite and non-negative"
            );
            let total: $f = weights.iter().sum();
            assert!(total > 0.0 && total.is_finite(), "sampler weights must have a positive, finite sum");
            total
        }
    };
}
impl_validate_weights!(validate_weights_f32, f32);
impl_validate_weights!(validate_weights_f64, f64);