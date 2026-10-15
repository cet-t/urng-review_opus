use ::wide::{u32x4, u32x8, u32x16};

use crate::wide::RngW;

macro_rules! impl_variants {
    ($size:expr) => {
        pastey::paste! {
            #[doc = concat!("SplitMix32 producing ", stringify!($size), " values per call via `wide` SIMD vectors.")]
            #[doc = ""]
            #[doc = "Portable-SIMD counterpart of [`crate::prng::b32::SplitMix32`]. A fast finalizer-based"]
            #[doc = "generator, commonly used to seed the other `wide` generators. Each `nextu` call"]
            #[doc = "returns an array of `u32`, one per lane."]
            #[doc = ""]
            #[doc = "# Example"]
            #[doc = "```"]
            #[doc = "use urng::wide::RngW;"]
            #[doc = concat!("use urng::wide::SplitMix32x", stringify!($size), ";")]
            #[doc = ""]
            #[doc = concat!("let mut rng = SplitMix32x", stringify!($size), "::new(1);")]
            #[doc = concat!("let _ = rng.nextu();")]
            #[doc = "```"]
            #[allow(dead_code)]
            #[repr(C, align(64))]
            pub struct [<SplitMix32x $size>] {
                state: [<u32x $size>],
            }

            #[allow(dead_code)]
            impl [<SplitMix32x $size>] {
                #[doc = "Creates a new generator. Lane `i` starts at `seed + (i + 1) * gamma` and"]
                #[doc = "every call advances all lanes by `N * gamma`, so the lanes interleave one"]
                #[doc = "Weyl sequence: output element `e` is `fmix32(seed + (e + 1) * gamma)`."]
                pub fn new(seed: u32) -> Self {
                    let state: [u32; $size] =
                        std::array::from_fn(|i| seed.wrapping_add(0x9E3779B9u32.wrapping_mul(i as u32 + 1)));
                    Self {
                        state: [<u32x $size>]::from(state),
                    }
                }

                #[inline(always)]
                pub(crate) fn compute(state: [<u32x $size>]) -> [<u32x $size>] {
                    let mut z = state;
                    z = (z ^ (z >> 16)) * [<u32x $size>]::splat(0x85EBCA6B);
                    z = (z ^ (z >> 13)) * [<u32x $size>]::splat(0xC2B2AE35);
                    z ^ (z >> 16)
                }
            }

            impl RngW<$size> for [<SplitMix32x $size>] {
                type Word = u32;

                #[doc = "Generates the next block of `u32` values, one per SIMD lane."]
                #[doc = ""]
                #[doc = "Applies the murmur3 `fmix32` finalizer to every lane, then advances each"]
                #[doc = "lane by `N * 0x9E3779B9`. (The previous version advanced by one gamma, so"]
                #[doc = "every call returned the previous call's lanes shifted by one position.)"]
                #[inline(always)]
                fn nextu(&mut self) -> [Self::Word; $size] {
                    let z = Self::compute(self.state);
                    self.state += [<u32x $size>]::splat(0x9E3779B9u32.wrapping_mul($size));
                    bytemuck::cast(z)
                }
            }
        }
    };
    ($($size:expr),+ $(,)*) => {
        $(impl_variants!($size);)+
    };
}

impl_variants!(4, 8, 16);

#[cfg(test)]
mod tests {
    use super::*;

    crate::safe_test! {
        SplitMix32x4,
        SplitMix32x8,
        SplitMix32x16
    }

    /// REVIEW B10: lanes advanced by one gamma per call, so each call returned the
    /// previous call's lanes shifted by one. Lanes must interleave one stream.
    #[test]
    fn wide_splitmix_lanes_interleave_one_stream() {
        let mut a = SplitMix32x4::new(5);
        let mut b = SplitMix32x8::new(5);
        let mut c = SplitMix32x16::new(5);
        for t in 0..2000u32 {
            let (x, y, z) = (a.nextu(), b.nextu(), c.nextu());
            for i in 0..4 {
                assert_eq!(x[i], crate::_reftest::fmix32_stream(5, 4 * t + i as u32));
            }
            for i in 0..8 {
                assert_eq!(y[i], crate::_reftest::fmix32_stream(5, 8 * t + i as u32));
            }
            for i in 0..16 {
                assert_eq!(z[i], crate::_reftest::fmix32_stream(5, 16 * t + i as u32));
            }
        }
    }
}
