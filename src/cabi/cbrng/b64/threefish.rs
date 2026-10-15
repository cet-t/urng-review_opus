//! C ABI for [`Threefish256`]. A bulk call writes exactly what the same number
//! of sequential [`Rng::nextu`] calls would return (REVIEW B14: the previous
//! version discarded the unused words of the last block and ignored words
//! still buffered from earlier calls). `threefish256_rand_i64s` with
//! `min > max` writes nothing.

use crate::cbrng::b64::Threefish256;
use crate::rng::{Rng, Word};

/// Creates a new heap-allocated `Threefish256` and returns a raw pointer to it.
/// The caller is responsible for freeing it with [`threefish256_free`].
#[unsafe(no_mangle)]
pub extern "C" fn threefish256_new(seed: u64) -> *mut Threefish256 {
    Box::into_raw(Box::new(Threefish256::new(seed)))
}
/// Frees a `Threefish256` instance previously created by [`threefish256_new`].
/// Does nothing if `ptr` is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn threefish256_free(ptr: *mut Threefish256) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

/// Fills `out[0..count]` with raw `u64` random values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn threefish256_next_u64s(ptr: *mut Threefish256, out: *mut u64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        crate::_internal::fill_with(out, count, || rng.nextu());
    }
}
/// Fills `out[0..count]` with `f64` values in `[0, 1)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn threefish256_next_f64s(ptr: *mut Threefish256, out: *mut f64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        crate::_internal::fill_with(out, count, || rng.nextf());
    }
}
/// Fills `out[0..count]` with `i64` values in `[min, max]`. Writes nothing if `min > max`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn threefish256_rand_i64s(
    ptr: *mut Threefish256,
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
        crate::_internal::fill_with(out, count, || rng.nextu().to_randi(min, max));
    }
}
/// Fills `out[0..count]` with `f64` values in `[min, max)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn threefish256_rand_f64s(
    ptr: *mut Threefish256,
    out: *mut f64,
    count: usize,
    min: f64,
    max: f64,
) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        crate::_internal::fill_with(out, count, || rng.nextu().to_randf(min, max));
    }
}