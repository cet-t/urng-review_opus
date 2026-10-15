//! C ABI for [`Philox32`] and [`Philox32x4x4`].
//!
//! Contract (REVIEW B12/B14): a bulk call writes exactly what the same number
//! of sequential Rust calls would produce, independent of the buffer address
//! and of thread scheduling.
//!
//! - `philox32_*`: `count` successive [`Rng::nextu`] words, including words
//!   still buffered from an earlier call, so bulk and single draws interleave.
//! - `philox32x4x4_*`: the concatenation of successive 16-word
//!   `Philox32x4x4::nextu` blocks; the unused tail of a final partial block is
//!   discarded (the state advances by `ceil(count / 16)` blocks).
//! - `*_rand_i32s` with `min > max` writes nothing and leaves the state
//!   unchanged (a Rust panic cannot cross the C boundary).

use wrapn::wrap;

use crate::_internal::{CtrRing, par_fill_ctr};
use crate::cbrng::b32::Philox32;
use crate::rng::Word;

/// Creates a new `Philox32` instance.
/// The caller is responsible for freeing the memory using `philox32_free`.
#[unsafe(no_mangle)]
pub extern "C" fn philox32_new(seed: u32) -> *mut Philox32 {
    Box::into_raw(Box::new(Philox32::new(seed)))
}

/// Frees the memory of a `Philox32` instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox32_free(ptr: *mut Philox32) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

impl CtrRing for Philox32 {
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
        for (o, w) in out.iter_mut().zip(Philox32::compute(ctr, self.k)) {
            *o = *w;
        }
    }
}
/// Fills the output buffer with the next random `u32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox32_next_u32s(ptr: *mut Philox32, out: *mut u32, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x);
    }
}

/// Fills the output buffer with the next random `f32` values in the range [0, 1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox32_next_f32s(ptr: *mut Philox32, out: *mut f32, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x.to_f01());
    }
}

/// Fills the output buffer with random `i32` values in the range [min, max].
/// Writes nothing if `min > max`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox32_rand_i32s(
    ptr: *mut Philox32,
    out: *mut i32,
    count: usize,
    min: i32,
    max: i32,
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

/// Fills the output buffer with random `f32` values in the range [min, max).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn philox32_rand_f32s(
    ptr: *mut Philox32,
    out: *mut f32,
    count: usize,
    min: f32,
    max: f32,
) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let Some(buf) = crate::_internal::out_slice(out, count) else { return };
        par_fill_ctr(rng, buf, |x| x.to_randf(min, max));
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd {
    use rayon::prelude::*;
    use std::arch::x86_64::*;

    use crate::_internal::{fill_chunk_auto, prefer_nt};
    use crate::cbrng::b32::{PHILOX32x16, Philox32x4x4};
    use crate::rng::Word;

    /// Elements per parallel task (a multiple of the 64-word batch).
    const PAR_CHUNK: usize = 0x20000;
    /// Blocks (16 words each) generated per batch: four independent Philox
    /// evaluations keep the multiplier pipeline busy.
    const BATCH: usize = 4 * PHILOX32x16;

    /// Creates a new `Philox32x4x4` instance, or returns NULL if the CPU lacks
    /// AVX-512F. The caller frees it with `philox32x4x4_free`.
    #[unsafe(no_mangle)]
    pub extern "C" fn philox32x4x4_new(seed: u32) -> *mut Philox32x4x4 {
        if !std::arch::is_x86_feature_detected!("avx512f") {
            return std::ptr::null_mut(); // CPU lacks the required features
        }
        // SAFETY: the required CPU features were verified above.
        unsafe { Box::into_raw(Box::new(Philox32x4x4::new(seed))) }
    }

    /// Frees the memory of a `Philox32x4x4` instance.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn philox32x4x4_free(ptr: *mut Philox32x4x4) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    /// Fills one parallel chunk starting at block counter `c`.
    #[target_feature(enable = "avx512f")]
    unsafe fn fill_chunk_x4x4<T, M>(chunk: &mut [T], c: __m512i, k: __m512i, nt: bool, map: &M)
    where
        T: Copy + Default,
        M: Fn(u32) -> T,
    {
        let mut c = c;
        let gen_batch = || {
            let cs = [
                c,
                Philox32x4x4::add_blocks(c, 1),
                Philox32x4x4::add_blocks(c, 2),
                Philox32x4x4::add_blocks(c, 3),
            ];
            c = Philox32x4x4::add_blocks(c, 4);
            let mut out = [T::default(); BATCH];
            for (dst, cb) in out.chunks_exact_mut(PHILOX32x16).zip(cs) {
                // SAFETY: 64 plain bytes either way.
                let words: [u32; PHILOX32x16] =
                    unsafe { std::mem::transmute(Philox32x4x4::block(cb, k)) };
                for (o, w) in dst.iter_mut().zip(words) {
                    *o = map(w);
                }
            }
            out
        };
        unsafe { fill_chunk_auto(chunk, nt, gen_batch) };
    }

    /// Fills `buf` with successive 16-word blocks and advances the state by
    /// `ceil(buf.len() / 16)` blocks.
    ///
    /// # Safety
    ///
    /// The CPU must support AVX-512F (guaranteed for handles made by
    /// `philox32x4x4_new`).
    unsafe fn philox32x4x4_fill<T, M>(rng: &mut Philox32x4x4, buf: &mut [T], map: M)
    where
        T: Copy + Default + Send,
        M: Fn(u32) -> T + Sync,
    {
        let c0 = rng.c;
        let k = rng.k;
        let nt = prefer_nt::<T>(buf.len());
        // SAFETY: __m512i is 64 plain bytes; it is sent to the workers by value.
        let (c0b, kb): ([u8; 64], [u8; 64]) = unsafe { (std::mem::transmute(c0), std::mem::transmute(k)) };
        buf.par_chunks_mut(PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| unsafe {
                let c0: __m512i = std::mem::transmute(c0b);
                let k: __m512i = std::mem::transmute(kb);
                let c = Philox32x4x4::add_blocks(c0, (chunk_idx * (PAR_CHUNK / PHILOX32x16)) as u128);
                fill_chunk_x4x4(chunk, c, k, nt, &map);
            });
        let blocks = buf.len().div_ceil(PHILOX32x16) as u128;
        rng.c = unsafe { Philox32x4x4::add_blocks(c0, blocks) };
    }

    /// Fills the output buffer with the next random `u32` values.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn philox32x4x4_next_u32s(ptr: *mut Philox32x4x4, out: *mut u32, count: usize) {
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let Some(buf) = crate::_internal::out_slice(out, count) else { return };
            philox32x4x4_fill(rng, buf, |x| x);
        }
    }

    /// Fills the output buffer with the next random `f32` values in the range [0, 1).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn philox32x4x4_next_f32s(ptr: *mut Philox32x4x4, out: *mut f32, count: usize) {
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let Some(buf) = crate::_internal::out_slice(out, count) else { return };
            philox32x4x4_fill(rng, buf, |x| x.to_f01());
        }
    }

    /// Fills the output buffer with random `i32` values in the range [min, max].
    /// Writes nothing if `min > max`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn philox32x4x4_rand_i32s(
        ptr: *mut Philox32x4x4,
        out: *mut i32,
        count: usize,
        min: i32,
        max: i32,
    ) {
        if min > max {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let Some(buf) = crate::_internal::out_slice(out, count) else { return };
            philox32x4x4_fill(rng, buf, |x| x.to_randi(min, max));
        }
    }

    /// Fills the output buffer with random `f32` values in the range [min, max).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn philox32x4x4_rand_f32s(
        ptr: *mut Philox32x4x4,
        out: *mut f32,
        count: usize,
        min: f32,
        max: f32,
    ) {
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let Some(buf) = crate::_internal::out_slice(out, count) else { return };
            philox32x4x4_fill(rng, buf, |x| x.to_randf(min, max));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// Bulk fill matches `nextu` across a carry through all four counter words.
    #[test]
    fn bulk_fill_carries_through_128_bit_counter() {
        let mut a = Philox32::new(1);
        a.c = wrap![u32::MAX - 2, u32::MAX, u32::MAX, 0];
        let mut b = a;
        let mut buf = vec![0u32; 64];
        par_fill_ctr(&mut a, &mut buf, |x| x);
        for &x in &buf {
            assert_eq!(x, b.nextu());
        }
        assert_eq!(a.c.map(|w| *w), [13, 0, 0, 1]);
        a.set_ctr(u128::MAX - 5);
        assert_eq!(a.ctr(), u128::MAX - 5);
    }
}
