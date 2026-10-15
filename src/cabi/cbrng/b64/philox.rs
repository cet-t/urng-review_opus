//! C ABI for [`Philox64`]. A bulk call writes exactly what the same number of
//! sequential [`crate::Rng::nextu`] calls would return (REVIEW B14; see
//! [`crate::_internal::par_fill_ctr`]). `philox64_rand_i64s` with `min > max`
//! writes nothing.

use wrapn::wrap;

use crate::_internal::{CtrRing, par_fill_ctr};
use crate::cbrng::b64::Philox64;
use crate::rng::Word;

impl CtrRing for Philox64 {
    const N: usize = 2;
    fn buffered(&self) -> bool {
        *self.pos < 2
    }
    fn ctr(&self) -> u128 {
        (*self.c[0] as u128) | ((*self.c[1] as u128) << 64)
    }
    fn set_ctr(&mut self, c: u128) {
        self.c = wrap![c as u64, (c >> 64) as u64];
    }
    fn block(&self, c: u128, out: &mut [u64]) {
        let r = Philox64::compute(wrap![c as u64, (c >> 64) as u64], self.k);
        out[0] = *r[0];
        out[1] = *r[1];
    }
}

/// Creates a new heap-allocated `Philox64` and returns a raw pointer to it.
/// The caller is responsible for freeing it with [`philox64_free`].
#[unsafe(no_mangle)]
pub extern "C" fn philox64_new(seed: u64) -> *mut Philox64 {
    Box::into_raw(Box::new(Philox64::new(seed)))
}
/// Frees a `Philox64` instance previously created by [`philox64_new`].
/// Does nothing if `ptr` is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox64_free(ptr: *mut Philox64) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

/// Fills `out[0..count]` with raw `u64` random values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox64_next_u64s(ptr: *mut Philox64, out: *mut u64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x);
    }
}

/// Fills `out[0..count]` with `f64` values in `[0, 1)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox64_next_f64s(ptr: *mut Philox64, out: *mut f64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x.to_f01());
    }
}

/// Fills `out[0..count]` with `i64` values in `[min, max]`. Writes nothing if `min > max`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox64_rand_i64s(
    ptr: *mut Philox64,
    out: *mut i64,
    count: usize,
    min: i64,
    max: i64,
) {
    if min > max {
        return;
    }
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x.to_randi(min, max));
    }
}

/// Fills `out[0..count]` with `f64` values in `[min, max)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox64_rand_f64s(
    ptr: *mut Philox64,
    out: *mut f64,
    count: usize,
    min: f64,
    max: f64,
) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x.to_randf(min, max));
    }
}