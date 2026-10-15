use std::slice::from_raw_parts_mut;

use rayon::prelude::*;

use crate::prng::b64::{SplitMix64, Xoshiro256Pp, Xoshiro256Ss};
use crate::rng::Rng;

/// Creates a new heap-allocated `Xoshiro256Pp` and returns a raw pointer to it.
/// The caller is responsible for freeing it with [`xoshiro256pp_free`].
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_new(seed: u64) -> *mut Xoshiro256Pp {
    Box::into_raw(Box::new(Xoshiro256Pp::new(seed)))
}
/// Frees a `Xoshiro256Pp` instance previously created by [`xoshiro256pp_new`].
/// Does nothing if `ptr` is null.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_free(ptr: *mut Xoshiro256Pp) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

const XOSHIRO256PP_PAR_CHUNK: usize = 4096;

/// Fills `out[0..count]` with raw `u64` random values using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_next_u64s(ptr: *mut Xoshiro256Pp, out: *mut u64, count: usize) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256PP_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Pp::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.nextu();
                }
            });
    }
}
/// Fills `out[0..count]` with `f64` values in `[0, 1)` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_next_f64s(ptr: *mut Xoshiro256Pp, out: *mut f64, count: usize) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256PP_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Pp::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.nextf();
                }
            });
    }
}
/// Fills `out[0..count]` with `i64` values in `[min, max]` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_rand_i64s(
    ptr: *mut Xoshiro256Pp,
    out: *mut i64,
    count: usize,
    min: i64,
    max: i64,
) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256PP_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Pp::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.randi(min, max);
                }
            });
    }
}
/// Fills `out[0..count]` with `f64` values in `[min, max)` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256pp_rand_f64s(
    ptr: *mut Xoshiro256Pp,
    out: *mut f64,
    count: usize,
    min: f64,
    max: f64,
) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256PP_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Pp::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.randf(min, max);
                }
            });
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd {
    use crate::prng::b64::Xoshiro256Ssx2;

    /// Creates a new heap-allocated `Xoshiro256Ssx2` and returns a raw pointer to it.
    /// The caller is responsible for freeing it with [`xoshiro256ssx2_free`].
    #[unsafe(no_mangle)]
    pub extern "C" fn xoshiro256ssx2_new(seed: u64) -> *mut Xoshiro256Ssx2 {
        Box::into_raw(Box::new(Xoshiro256Ssx2::new(seed)))
    }
    /// Frees a `Xoshiro256Ssx2` instance previously created by [`xoshiro256ssx2_new`].
    /// Does nothing if `ptr` is null.
    #[unsafe(no_mangle)]
    pub extern "C" fn xoshiro256ssx2_free(ptr: *mut Xoshiro256Ssx2) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    /// Fills `out[0..count]` with raw `u64` values, in exactly the order produced by
    /// repeated [`Xoshiro256Ssx2::nextu`] calls (stream 0, stream 1, stream 0, ...).
    ///
    /// The previous version derived per-lane seeds that overlapped between chunks
    /// (most of the output was duplicated); this one is sequential and therefore
    /// single-threaded.
    #[unsafe(no_mangle)]
    pub extern "C" fn xoshiro256ssx2_next_u64s(
        ptr: *mut Xoshiro256Ssx2,
        out: *mut u64,
        count: usize,
    ) {
        if ptr.is_null() || out.is_null() || count == 0 {
            return;
        }
        unsafe {
            let rng = &mut *ptr;
            let buffer = std::slice::from_raw_parts_mut(out, count);
            let mut pairs = buffer.chunks_exact_mut(2);
            for dst in pairs.by_ref() {
                dst.copy_from_slice(&rng.nextu());
            }
            if let [last] = pairs.into_remainder() {
                *last = rng.nextu()[0];
            }
        }
    }
} // mod simd

/// Creates a new heap-allocated `Xoshiro256Ss` and returns a raw pointer to it.
/// The caller is responsible for freeing it with [`xoshiro256ss_free`].
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_new(seed: u64) -> *mut Xoshiro256Ss {
    Box::into_raw(Box::new(Xoshiro256Ss::new(seed)))
}
/// Frees a `Xoshiro256Ss` instance previously created by [`xoshiro256ss_free`].
/// Does nothing if `ptr` is null.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_free(ptr: *mut Xoshiro256Ss) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

const XOSHIRO256SS_PAR_CHUNK: usize = 4096;

/// Fills `out[0..count]` with raw `u64` random values using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_next_u64s(ptr: *mut Xoshiro256Ss, out: *mut u64, count: usize) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256SS_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Ss::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.nextu();
                }
            });
    }
}
/// Fills `out[0..count]` with `f64` values in `[0, 1)` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_next_f64s(ptr: *mut Xoshiro256Ss, out: *mut f64, count: usize) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256SS_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Ss::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.nextf();
                }
            });
    }
}
/// Fills `out[0..count]` with `i64` values in `[min, max]` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_rand_i64s(
    ptr: *mut Xoshiro256Ss,
    out: *mut i64,
    count: usize,
    min: i64,
    max: i64,
) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256SS_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Ss::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.randi(min, max);
                }
            });
    }
}
/// Fills `out[0..count]` with `f64` values in `[min, max)` using parallel chunk generation.
#[unsafe(no_mangle)]
pub extern "C" fn xoshiro256ss_rand_f64s(
    ptr: *mut Xoshiro256Ss,
    out: *mut f64,
    count: usize,
    min: f64,
    max: f64,
) {
    unsafe {
        let rng = &mut *ptr;
        let buffer = from_raw_parts_mut(out, count);
        let base_seed = rng.nextu();

        buffer
            .par_chunks_mut(XOSHIRO256SS_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let chunk_seed = SplitMix64::compute(
                    base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut local_rng = Xoshiro256Ss::new(chunk_seed);
                for v in chunk {
                    *v = local_rng.randf(min, max);
                }
            });
    }
}
