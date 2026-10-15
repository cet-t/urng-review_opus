//! C ABI for [`Threefry32x4`] and [`Threefry32x2`].
//!
//! A bulk call writes exactly what the same number of sequential
//! [`crate::Rng::nextu`] calls would return (REVIEW B14; see
//! [`crate::_internal::par_fill_ctr`]). `*_rand_i32s` with `min > max` writes
//! nothing and leaves the state unchanged.

use wrapn::wrap;

use crate::_internal::{CtrRing, par_fill_ctr};
use crate::cbrng::b32::{Threefry32x2, Threefry32x4};
use crate::rng::Word;

impl CtrRing for Threefry32x4 {
    const N: usize = 4;
    fn buffered(&self) -> bool {
        *self.pos < 4
    }
    fn ctr(&self) -> u128 {
        self.c.iter().rev().fold(0u128, |acc, w| (acc << 32) | **w as u128)
    }
    fn set_ctr(&mut self, c: u128) {
        self.c = wrap![c as u32, (c >> 32) as u32, (c >> 64) as u32, (c >> 96) as u32];
    }
    fn block(&self, c: u128, out: &mut [u32]) {
        let ctr = wrap![c as u32, (c >> 32) as u32, (c >> 64) as u32, (c >> 96) as u32];
        for (o, w) in out.iter_mut().zip(Threefry32x4::compute(ctr, &self.k)) {
            *o = *w;
        }
    }
}

/// The 2x32 counter is 64 bits wide and wraps modulo 2^64, like `next_raw`.
impl CtrRing for Threefry32x2 {
    const N: usize = 2;
    fn buffered(&self) -> bool {
        *self.pos < 2
    }
    fn ctr(&self) -> u128 {
        (*self.c[0] as u128) | ((*self.c[1] as u128) << 32)
    }
    fn set_ctr(&mut self, c: u128) {
        self.c = wrap![c as u32, (c >> 32) as u32];
    }
    fn block(&self, c: u128, out: &mut [u32]) {
        let ctr = wrap![c as u32, (c >> 32) as u32];
        for (o, w) in out.iter_mut().zip(Threefry32x2::compute(ctr, &self.k)) {
            *o = *w;
        }
    }
}

macro_rules! threefry_cabi {
    ($ty:ty, $new:ident, $free:ident, $next_u:ident, $next_f:ident, $rand_i:ident, $rand_f:ident) => {
        #[doc = concat!("Creates a new `", stringify!($ty), "` instance; free it with `", stringify!($free), "`.")]
        #[unsafe(no_mangle)]
        pub extern "C" fn $new(seed: u32) -> *mut $ty {
            Box::into_raw(Box::new(<$ty>::new(seed)))
        }

        #[doc = concat!("Frees a `", stringify!($ty), "` instance. Does nothing for NULL.")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $free(ptr: *mut $ty) {
            if !ptr.is_null() {
                unsafe { drop(Box::from_raw(ptr)) };
            }
        }

        /// Fills the output buffer with the next random `u32` values.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $next_u(ptr: *mut $ty, out: *mut u32, count: usize) {
            unsafe {
                let Some(rng) = ptr.as_mut() else { return };
                let Some(buf) = crate::_internal::out_slice(out, count) else { return };
                par_fill_ctr(rng, buf, |x| x);
            }
        }

        /// Fills the output buffer with the next random `f32` values in the range [0, 1).
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $next_f(ptr: *mut $ty, out: *mut f32, count: usize) {
            unsafe {
                let Some(rng) = ptr.as_mut() else { return };
                let Some(buf) = crate::_internal::out_slice(out, count) else { return };
                par_fill_ctr(rng, buf, |x| x.to_f01());
            }
        }

        /// Fills the output buffer with random `i32` values in the range [min, max].
        /// Writes nothing if `min > max`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $rand_i(ptr: *mut $ty, out: *mut i32, count: usize, min: i32, max: i32) {
            if min > max {
                return;
            }
            unsafe {
                let Some(rng) = ptr.as_mut() else { return };
                let Some(buf) = crate::_internal::out_slice(out, count) else { return };
                par_fill_ctr(rng, buf, |x| x.to_randi(min, max));
            }
        }

        /// Fills the output buffer with random `f32` values in the range [min, max).
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $rand_f(ptr: *mut $ty, out: *mut f32, count: usize, min: f32, max: f32) {
            unsafe {
                let Some(rng) = ptr.as_mut() else { return };
                let Some(buf) = crate::_internal::out_slice(out, count) else { return };
                par_fill_ctr(rng, buf, |x| x.to_randf(min, max));
            }
        }
    };
}

threefry_cabi!(
    Threefry32x4,
    threefry32x4_new,
    threefry32x4_free,
    threefry32x4_next_u32s,
    threefry32x4_next_f32s,
    threefry32x4_rand_i32s,
    threefry32x4_rand_f32s
);
threefry_cabi!(
    Threefry32x2,
    threefry32x2_new,
    threefry32x2_free,
    threefry32x2_next_u32s,
    threefry32x2_next_f32s,
    threefry32x2_rand_i32s,
    threefry32x2_rand_f32s
);
