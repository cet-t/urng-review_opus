
use rayon::prelude::*;

use crate::prng::b64::{Sfc64, SplitMix64};
use crate::rng::Rng;

/// Creates a new heap-allocated `Sfc64` and returns a raw pointer to it.
/// The caller is responsible for freeing it with [`sfc64_free`].
#[unsafe(no_mangle)]
pub extern "C" fn sfc64_new(seed: u64) -> *mut Sfc64 {
    Box::into_raw(Box::new(Sfc64::new(seed)))
}

/// Frees a `Sfc64` instance previously created by [`sfc64_new`].
/// Does nothing if `ptr` is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc64_free(ptr: *mut Sfc64) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

const SFC64_PAR_CHUNK: usize = 0x10000;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc64_next_u64s(ptr: *mut Sfc64, out: *mut u64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        let seed = rng.nextu();

        buffer
            .par_chunks_mut(SFC64_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let seed = SplitMix64::compute(
                    seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut rng = Sfc64::new(seed);
                for v in chunk {
                    *v = rng.nextu();
                }
            });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc64_next_f64s(ptr: *mut Sfc64, out: *mut f64, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        let seed = rng.nextu();

        buffer
            .par_chunks_mut(SFC64_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let seed = SplitMix64::compute(
                    seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut rng = Sfc64::new(seed);
                for v in chunk {
                    *v = rng.nextf();
                }
            });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc64_rand_i64s(
    ptr: *mut Sfc64,
    out: *mut i64,
    count: usize,
    min: i64,
    max: i64,
) {
    if min > max {
        return; // empty range: write nothing (a panic cannot cross the C ABI)
    }
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        let seed = rng.nextu();

        buffer
            .par_chunks_mut(SFC64_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let seed = SplitMix64::compute(
                    seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut rng = Sfc64::new(seed);
                for v in chunk {
                    *v = rng.randi(min, max);
                }
            });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc64_rand_f64s(
    ptr: *mut Sfc64,
    out: *mut f64,
    count: usize,
    min: f64,
    max: f64,
) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        let seed = rng.nextu();

        buffer
            .par_chunks_mut(SFC64_PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let seed = SplitMix64::compute(
                    seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
                );
                let mut rng = Sfc64::new(seed);
                for v in chunk {
                    *v = rng.randf(min, max);
                }
            });
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd {
    use super::*;
    use crate::prng::b64::Sfc64x8;
    use std::arch::x86_64::*;

    #[unsafe(no_mangle)]
    pub extern "C" fn sfc64x8_new(seed: u64) -> *mut Sfc64x8 {
        if !(std::arch::is_x86_feature_detected!("avx512f")) {
            return std::ptr::null_mut(); // CPU lacks the required features
        }
        // SAFETY: the required CPU features were verified above.
        unsafe { Box::into_raw(Box::new(Sfc64x8::new(seed))) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc64x8_free(ptr: *mut Sfc64x8) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    const SFC64X8_PAR_CHUNK: usize = 0x1000;

    /// 6-way interleaved SFC64 with AVX-512 SoA layout.
    ///
    /// SFC64's critical path is 5 cycles (res竊団竊鍛竊池es), so 6 independent groups
    /// fully hide the latency. Each group holds 8 streams 竊・48 u64 per iteration.
    ///
    /// Register budget: 6ﾃ・=24 state ZMM + 6 res + 1 `one` = 31 ZMM (fits in 32).
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_op_in_unsafe_fn, unused_assignments)]
    pub(super) unsafe fn sfc64x8_next_u64s_chunk(chunk_idx: usize, chunk: &mut [u64], nt: bool, seed: u64) {
        const STRIDE: u64 = 0x9E3779B97F4A7C15u64;
        // Each chunk owns a disjoint block of 48 stream indices (REVIEW B3: the old
        // `chunk_idx + lane` indexing made adjacent chunks share all but one stream).
        let chunk_base = seed.wrapping_add(((chunk_idx as u64) * 48).wrapping_mul(STRIDE));

        macro_rules! make_state {
            ($group:expr) => {{
                let mut va = [0; 8];
                let mut vb = [0; 8];
                let mut vc = [0; 8];
                for i in 0usize..8 {
                    let seed = SplitMix64::compute(
                        chunk_base.wrapping_add(((($group << 3) + i) as u64).wrapping_mul(STRIDE)),
                    );
                    let mut sm = SplitMix64::new(seed);
                    va[i] = sm.nextu();
                    vb[i] = sm.nextu();
                    vc[i] = sm.nextu();
                }
                (
                    _mm512_loadu_si512(va.as_ptr() as *const __m512i),
                    _mm512_loadu_si512(vb.as_ptr() as *const __m512i),
                    _mm512_loadu_si512(vc.as_ptr() as *const __m512i),
                    _mm512_set1_epi64(1),
                )
            }};
        }

        let (mut a0, mut b0, mut c0, mut ctr0) = make_state!(0);
        let (mut a1, mut b1, mut c1, mut ctr1) = make_state!(1);
        let (mut a2, mut b2, mut c2, mut ctr2) = make_state!(2);
        let (mut a3, mut b3, mut c3, mut ctr3) = make_state!(3);
        let (mut a4, mut b4, mut c4, mut ctr4) = make_state!(4);
        let (mut a5, mut b5, mut c5, mut ctr5) = make_state!(5);

        let one = _mm512_set1_epi64(1i64);

        macro_rules! step6 {
            () => {{
                let res0 = _mm512_add_epi64(_mm512_add_epi64(a0, b0), ctr0);
                let res1 = _mm512_add_epi64(_mm512_add_epi64(a1, b1), ctr1);
                let res2 = _mm512_add_epi64(_mm512_add_epi64(a2, b2), ctr2);
                let res3 = _mm512_add_epi64(_mm512_add_epi64(a3, b3), ctr3);
                let res4 = _mm512_add_epi64(_mm512_add_epi64(a4, b4), ctr4);
                let res5 = _mm512_add_epi64(_mm512_add_epi64(a5, b5), ctr5);
                // a = b ^ (b >> 11)
                a0 = _mm512_xor_si512(b0, _mm512_srli_epi64(b0, 11));
                a1 = _mm512_xor_si512(b1, _mm512_srli_epi64(b1, 11));
                a2 = _mm512_xor_si512(b2, _mm512_srli_epi64(b2, 11));
                a3 = _mm512_xor_si512(b3, _mm512_srli_epi64(b3, 11));
                a4 = _mm512_xor_si512(b4, _mm512_srli_epi64(b4, 11));
                a5 = _mm512_xor_si512(b5, _mm512_srli_epi64(b5, 11));
                // b = c + (c << 3)
                b0 = _mm512_add_epi64(c0, _mm512_slli_epi64(c0, 3));
                b1 = _mm512_add_epi64(c1, _mm512_slli_epi64(c1, 3));
                b2 = _mm512_add_epi64(c2, _mm512_slli_epi64(c2, 3));
                b3 = _mm512_add_epi64(c3, _mm512_slli_epi64(c3, 3));
                b4 = _mm512_add_epi64(c4, _mm512_slli_epi64(c4, 3));
                b5 = _mm512_add_epi64(c5, _mm512_slli_epi64(c5, 3));
                // c = rotl(c, 24) + res (PractRand sfc64)
                c0 = _mm512_add_epi64(_mm512_rol_epi64(c0, 24), res0);
                c1 = _mm512_add_epi64(_mm512_rol_epi64(c1, 24), res1);
                c2 = _mm512_add_epi64(_mm512_rol_epi64(c2, 24), res2);
                c3 = _mm512_add_epi64(_mm512_rol_epi64(c3, 24), res3);
                c4 = _mm512_add_epi64(_mm512_rol_epi64(c4, 24), res4);
                c5 = _mm512_add_epi64(_mm512_rol_epi64(c5, 24), res5);
                // counter += 1
                ctr0 = _mm512_add_epi64(ctr0, one);
                ctr1 = _mm512_add_epi64(ctr1, one);
                ctr2 = _mm512_add_epi64(ctr2, one);
                ctr3 = _mm512_add_epi64(ctr3, one);
                ctr4 = _mm512_add_epi64(ctr4, one);
                ctr5 = _mm512_add_epi64(ctr5, one);
                (res0, res1, res2, res3, res4, res5)
            }};
        }

        let is_aligned = nt && (chunk.as_ptr() as usize) & 63 == 0;
        let mut chunks_exact = chunk.chunks_exact_mut(48);

        if is_aligned {
            for dst in chunks_exact.by_ref() {
                let (r0, r1, r2, r3, r4, r5) = step6!();
                let p = dst.as_mut_ptr();
                _mm512_stream_si512(p as *mut __m512i, r0);
                _mm512_stream_si512(p.add(8) as *mut __m512i, r1);
                _mm512_stream_si512(p.add(16) as *mut __m512i, r2);
                _mm512_stream_si512(p.add(24) as *mut __m512i, r3);
                _mm512_stream_si512(p.add(32) as *mut __m512i, r4);
                _mm512_stream_si512(p.add(40) as *mut __m512i, r5);
            }
        } else {
            for dst in chunks_exact.by_ref() {
                let (r0, r1, r2, r3, r4, r5) = step6!();
                let p = dst.as_mut_ptr();
                _mm512_storeu_si512(p as *mut __m512i, r0);
                _mm512_storeu_si512(p.add(8) as *mut __m512i, r1);
                _mm512_storeu_si512(p.add(16) as *mut __m512i, r2);
                _mm512_storeu_si512(p.add(24) as *mut __m512i, r3);
                _mm512_storeu_si512(p.add(32) as *mut __m512i, r4);
                _mm512_storeu_si512(p.add(40) as *mut __m512i, r5);
            }
        }

        let rem = chunks_exact.into_remainder();
        if !rem.is_empty() {
            let mut tmp = [0u64; 48];
            let (r0, r1, r2, r3, r4, r5) = step6!();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut __m512i, r0);
            _mm512_storeu_si512(tmp.as_mut_ptr().add(8) as *mut __m512i, r1);
            _mm512_storeu_si512(tmp.as_mut_ptr().add(16) as *mut __m512i, r2);
            _mm512_storeu_si512(tmp.as_mut_ptr().add(24) as *mut __m512i, r3);
            _mm512_storeu_si512(tmp.as_mut_ptr().add(32) as *mut __m512i, r4);
            _mm512_storeu_si512(tmp.as_mut_ptr().add(40) as *mut __m512i, r5);
            for (j, v) in rem.iter_mut().enumerate() {
                *v = tmp[j];
            }
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc64x8_next_u64s(ptr: *mut Sfc64x8, out: *mut u64, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let vals = rng.nextu();
            let seed = vals.iter().copied().fold(0, u64::wrapping_add);

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC64X8_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    sfc64x8_next_u64s_chunk(
                        chunk_idx,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        seed,
                    );
                });

            let seed = SplitMix64::compute(
                seed.wrapping_add((count as u64).wrapping_mul(0x9E3779B97F4A7C15)),
            );
            *rng = Sfc64x8::new(seed);
        }
    }
    #[cfg(test)]
    pub(super) use sfc64x8_next_u64s_chunk as sfc64x8_next_u64s_chunk_for_test;
} // mod simd

#[cfg(all(test, feature = "simd", target_arch = "x86_64"))]
mod tests {
    use super::simd::sfc64x8_next_u64s_chunk_for_test as chunk_fn;
    use crate::prng::b64::{Sfc64, SplitMix64};
    use crate::rng::Rng;

    /// The bulk kernel must be 48 interleaved scalar SFC64 streams whose seeds are
    /// derived from disjoint (chunk, lane) indices (REVIEW B3, B10).
    #[test]
    fn sfc64x8_chunk_equals_scalar_streams() {
        if !(std::arch::is_x86_feature_detected!("avx512f")) {
            return;
        }
        const STRIDE: u64 = 0x9E3779B97F4A7C15;
        let (seed, chunk_idx) = (0x1234_5678u64, 3usize);
        let mut out = vec![0u64; 48 * 50 + 5];
        unsafe { chunk_fn(chunk_idx, &mut out, false, seed) };
        let base = seed.wrapping_add(((chunk_idx as u64) * 48).wrapping_mul(STRIDE));
        for lane in 0..48usize {
            let s = SplitMix64::compute(base.wrapping_add((lane as u64).wrapping_mul(STRIDE)));
            let mut sm = SplitMix64::new(s);
            let mut r = Sfc64::from_state([sm.nextu(), sm.nextu(), sm.nextu(), 1]);
            for t in 0..50 {
                assert_eq!(out[t * 48 + lane], r.nextu(), "lane {lane} step {t}");
            }
            if lane < 5 {
                assert_eq!(out[50 * 48 + lane], r.nextu(), "tail lane {lane}");
            }
        }
    }
}
