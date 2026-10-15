
use crate::{prng::b32::Sfc32, rng::Rng};

// --- Sfc32 ---

#[unsafe(no_mangle)]
pub extern "C" fn sfc32_new(seed: u32) -> *mut Sfc32 {
    Box::into_raw(Box::new(Sfc32::new(seed)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc32_free(ptr: *mut Sfc32) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc32_next_u32s(ptr: *mut Sfc32, out: *mut u32, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        crate::_internal::par_fill_reseed32(buffer, crate::_internal::key64(rng), Sfc32::from_seed64, |r| r.nextu());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc32_next_f32s(ptr: *mut Sfc32, out: *mut f32, count: usize) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        crate::_internal::par_fill_reseed32(buffer, crate::_internal::key64(rng), Sfc32::from_seed64, |r| r.nextf());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc32_rand_i32s(
    ptr: *mut Sfc32,
    out: *mut i32,
    count: usize,
    min: i32,
    max: i32,
) {
    if min > max {
        return; // empty range: write nothing (a panic cannot cross the C ABI)
    }
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        crate::_internal::par_fill_reseed32(buffer, crate::_internal::key64(rng), Sfc32::from_seed64, |r| r.randi(min, max));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sfc32_rand_f32s(
    ptr: *mut Sfc32,
    out: *mut f32,
    count: usize,
    min: f32,
    max: f32,
) {
    unsafe {
        let Some(rng) = ptr.as_mut() else { return };
        let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
        crate::_internal::par_fill_reseed32(buffer, crate::_internal::key64(rng), Sfc32::from_seed64, |r| r.randf(min, max));
    }
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
mod simd {
    use rayon::prelude::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    use std::ptr;

    use crate::prng::b32::sfc::{SFC32X4, SFC32X8, SFC32X16, Sfc32x4, Sfc32x8, Sfc32x16};

    // --- Sfc32x4 ---

    const SFC32X4_PAR_CHUNK: usize = 0x20000;

    #[unsafe(no_mangle)]
    pub extern "C" fn sfc32x4_new(seed: u32) -> *mut Sfc32x4 {
        Box::into_raw(Box::new(Sfc32x4::new(seed)))
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x4_free(ptr: *mut Sfc32x4) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    unsafe fn sfc32x4_next_u32s_chunk(rng: &mut Sfc32x4, chunk: &mut [u32], nt: bool) {
        crate::_internal::fill_chunk_auto(chunk, nt, || {
            bytemuck::cast::<_, [u32; SFC32X4 * 4]>([
                rng.nextuv(),
                rng.nextuv(),
                rng.nextuv(),
                rng.nextuv(),
            ])
        });
    }

    #[cfg(target_arch = "x86_64")]
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    unsafe fn sfc32x4_next_f32s_chunk(rng: &mut Sfc32x4, chunk: &mut [f32], nt: bool) {
        crate::_internal::fill_chunk_auto(chunk, nt, || {
            bytemuck::cast::<_, [f32; SFC32X4 * 4]>([
                rng.nextfv(),
                rng.nextfv(),
                rng.nextfv(),
                rng.nextfv(),
            ])
        });
    }

    #[cfg(target_arch = "x86_64")]
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    unsafe fn sfc32x4_rand_i32s_chunk(
        rng: &mut Sfc32x4,
        chunk: &mut [i32],
        nt: bool,
        v_range: __m128i,
        v_min: __m128i,
    ) {
        crate::_internal::fill_chunk_auto(chunk, nt, || {
            bytemuck::cast::<_, [i32; SFC32X4 * 4]>([
                rng.randiv(v_range, v_min),
                rng.randiv(v_range, v_min),
                rng.randiv(v_range, v_min),
                rng.randiv(v_range, v_min),
            ])
        });
    }

    #[cfg(target_arch = "x86_64")]
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    unsafe fn sfc32x4_rand_f32s_chunk(
        rng: &mut Sfc32x4,
        chunk: &mut [f32],
        nt: bool,
        v_mult: __m128,
        v_min: __m128, v_hi: __m128,
    ) {
        crate::_internal::fill_chunk_auto(chunk, nt, || {
            bytemuck::cast::<_, [f32; SFC32X4 * 4]>([
                crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi),
                crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi),
                crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi),
                crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi),
            ])
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x4_next_u32s(ptr: *mut Sfc32x4, out: *mut u32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let base_seed = { let v = rng.nextu(); ((v[0] as u64) << 32) | v[1] as u64 };
            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X4_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x4::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x4_next_u32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x4_next_f32s(ptr: *mut Sfc32x4, out: *mut f32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let base_seed = { let v = rng.nextu(); ((v[0] as u64) << 32) | v[1] as u64 };
            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X4_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x4::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x4_next_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x4_rand_i32s(
        ptr: *mut Sfc32x4,
        out: *mut i32,
        count: usize,
        min: i32,
        max: i32,
    ) {
        if min > max {
            return; // empty range: write nothing (a panic cannot cross the C ABI)
        }
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let base_seed = { let v = rng.nextu(); ((v[0] as u64) << 32) | v[1] as u64 };
            let v_range = _mm_set1_epi32(crate::rngv::scale32(min, max));
            let v_min = _mm_set1_epi32(min);
            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X4_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x4::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x4_rand_i32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_range,
                        v_min,
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x4_rand_f32s(
        ptr: *mut Sfc32x4,
        out: *mut f32,
        count: usize,
        min: f32,
        max: f32,
    ) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let base_seed = { let v = rng.nextu(); ((v[0] as u64) << 32) | v[1] as u64 };
            let v_mult = _mm_set1_ps(max - min);
            let v_hi = _mm_set1_ps(crate::_internal::hi_f32(min, max));
            let v_min = _mm_set1_ps(min);
            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X4_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x4::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x4_rand_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_mult,
                        v_min, v_hi);
                });
        }
    }

    // --- Sfc32x8 ---

    #[unsafe(no_mangle)]
    pub extern "C" fn sfc32x8_new(seed: u32) -> *mut Sfc32x8 {
        if !(std::arch::is_x86_feature_detected!("avx2")) {
            return std::ptr::null_mut(); // CPU lacks the required features
        }
        // SAFETY: the required CPU features were verified above.
        unsafe { Box::into_raw(Box::new(Sfc32x8::new(seed))) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x8_free(ptr: *mut Sfc32x8) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    #[cfg(target_arch = "x86_64")]
    const SFC32X8_PAR_CHUNK: usize = 0x20000;
    #[cfg(target_arch = "x86_64")]
    const SFC32X8_UNROLL: usize = SFC32X8 << 2;

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x8_next_u32s_chunk(rng: &mut Sfc32x8, chunk: &mut [u32], nt: bool) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X8_UNROLL {
                _mm256_stream_si256(out_ptr as *mut _, rng.nextuv());
                _mm256_stream_si256(out_ptr.add(SFC32X8) as *mut _, rng.nextuv());
                _mm256_stream_si256(out_ptr.add(SFC32X8 * 2) as *mut _, rng.nextuv());
                _mm256_stream_si256(out_ptr.add(SFC32X8 * 3) as *mut _, rng.nextuv());
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                let v = rng.nextuv();
                _mm256_stream_si256(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        } else {
            while remaining >= SFC32X8_UNROLL {
                _mm256_storeu_si256(out_ptr as *mut _, rng.nextuv());
                _mm256_storeu_si256(out_ptr.add(SFC32X8) as *mut _, rng.nextuv());
                _mm256_storeu_si256(out_ptr.add(SFC32X8 * 2) as *mut _, rng.nextuv());
                _mm256_storeu_si256(out_ptr.add(SFC32X8 * 3) as *mut _, rng.nextuv());
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                let v = rng.nextuv();
                _mm256_storeu_si256(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        }

        if remaining > 0 {
            let mut tmp = [0u32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.nextuv());
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x8_next_f32s_chunk(rng: &mut Sfc32x8, chunk: &mut [f32], nt: bool) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X8_UNROLL {
                _mm256_stream_ps(out_ptr, rng.nextfv());
                _mm256_stream_ps(out_ptr.add(SFC32X8), rng.nextfv());
                _mm256_stream_ps(out_ptr.add(SFC32X8 * 2), rng.nextfv());
                _mm256_stream_ps(out_ptr.add(SFC32X8 * 3), rng.nextfv());
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_stream_ps(out_ptr, rng.nextfv());
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        } else {
            while remaining >= SFC32X8_UNROLL {
                _mm256_storeu_ps(out_ptr, rng.nextfv());
                _mm256_storeu_ps(out_ptr.add(SFC32X8), rng.nextfv());
                _mm256_storeu_ps(out_ptr.add(SFC32X8 * 2), rng.nextfv());
                _mm256_storeu_ps(out_ptr.add(SFC32X8 * 3), rng.nextfv());
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_storeu_ps(out_ptr, rng.nextfv());
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        }

        if remaining > 0 {
            let mut tmp = [0f32; SFC32X8];
            _mm256_storeu_ps(tmp.as_mut_ptr(), rng.nextfv());
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x8_rand_i32s_chunk(
        rng: &mut Sfc32x8,
        chunk: &mut [i32],
        nt: bool,
        v_range: __m256i,
        v_min: __m256i,
    ) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X8_UNROLL {
                _mm256_stream_si256(out_ptr as *mut _, rng.randiv(v_range, v_min));
                _mm256_stream_si256(out_ptr.add(SFC32X8) as *mut _, rng.randiv(v_range, v_min));
                _mm256_stream_si256(
                    out_ptr.add(SFC32X8 * 2) as *mut _,
                    rng.randiv(v_range, v_min),
                );
                _mm256_stream_si256(
                    out_ptr.add(SFC32X8 * 3) as *mut _,
                    rng.randiv(v_range, v_min),
                );
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_stream_si256(out_ptr as *mut _, rng.randiv(v_range, v_min));
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        } else {
            while remaining >= SFC32X8_UNROLL {
                _mm256_storeu_si256(out_ptr as *mut _, rng.randiv(v_range, v_min));
                _mm256_storeu_si256(out_ptr.add(SFC32X8) as *mut _, rng.randiv(v_range, v_min));
                _mm256_storeu_si256(
                    out_ptr.add(SFC32X8 * 2) as *mut _,
                    rng.randiv(v_range, v_min),
                );
                _mm256_storeu_si256(
                    out_ptr.add(SFC32X8 * 3) as *mut _,
                    rng.randiv(v_range, v_min),
                );
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_storeu_si256(out_ptr as *mut _, rng.randiv(v_range, v_min));
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        }

        if remaining > 0 {
            let mut tmp = [0i32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.randiv(v_range, v_min));
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x8_rand_f32s_chunk(
        rng: &mut Sfc32x8,
        chunk: &mut [f32],
        nt: bool,
        v_mult: __m256,
        v_min: __m256, v_hi: __m256,
    ) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X8_UNROLL {
                _mm256_stream_ps(out_ptr, crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_stream_ps(out_ptr.add(SFC32X8), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_stream_ps(out_ptr.add(SFC32X8 * 2), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_stream_ps(out_ptr.add(SFC32X8 * 3), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_stream_ps(out_ptr, crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        } else {
            while remaining >= SFC32X8_UNROLL {
                _mm256_storeu_ps(out_ptr, crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_storeu_ps(out_ptr.add(SFC32X8), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_storeu_ps(out_ptr.add(SFC32X8 * 2), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                _mm256_storeu_ps(out_ptr.add(SFC32X8 * 3), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                out_ptr = out_ptr.add(SFC32X8_UNROLL);
                remaining -= SFC32X8_UNROLL;
            }
            while remaining >= SFC32X8 {
                _mm256_storeu_ps(out_ptr, crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
                out_ptr = out_ptr.add(SFC32X8);
                remaining -= SFC32X8;
            }
        }

        if remaining > 0 {
            let mut tmp = [0f32; SFC32X8];
            _mm256_storeu_ps(tmp.as_mut_ptr(), crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi));
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x8_next_u32s(ptr: *mut Sfc32x8, out: *mut u32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.nextuv());
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X8_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x8::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x8_next_u32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x8_next_f32s(ptr: *mut Sfc32x8, out: *mut f32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.nextuv());
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X8_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x8::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x8_next_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x8_rand_i32s(
        ptr: *mut Sfc32x8,
        out: *mut i32,
        count: usize,
        min: i32,
        max: i32,
    ) {
        if min > max {
            return; // empty range: write nothing (a panic cannot cross the C ABI)
        }
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.nextuv());
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let v_min = _mm256_set1_epi32(min);
            let v_range = _mm256_set1_epi32(crate::rngv::scale32(min, max));

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X8_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x8::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x8_rand_i32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_range,
                        v_min,
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x8_rand_f32s(
        ptr: *mut Sfc32x8,
        out: *mut f32,
        count: usize,
        min: f32,
        max: f32,
    ) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X8];
            _mm256_storeu_si256(tmp.as_mut_ptr() as *mut _, rng.nextuv());
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let v_mult = _mm256_set1_ps(max - min);
            let v_hi = _mm256_set1_ps(crate::_internal::hi_f32(min, max));
            let v_min = _mm256_set1_ps(min);

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X8_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x8::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x8_rand_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_mult,
                        v_min, v_hi);
                });
        }
    }

    // --- Sfc32x16 ---

    #[unsafe(no_mangle)]
    pub extern "C" fn sfc32x16_new(seed: u32) -> *mut Sfc32x16 {
        if !(std::arch::is_x86_feature_detected!("avx512f")) {
            return std::ptr::null_mut(); // CPU lacks the required features
        }
        // SAFETY: the required CPU features were verified above.
        unsafe { Box::into_raw(Box::new(Sfc32x16::new(seed))) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x16_free(ptr: *mut Sfc32x16) {
        if !ptr.is_null() {
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }

    #[cfg(target_arch = "x86_64")]
    const SFC32X16_PAR_CHUNK: usize = 0x20000;
    #[cfg(target_arch = "x86_64")]
    const SFC32X16_UNROLL: usize = SFC32X16 << 2;

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x16_next_u32s_chunk(rng: &mut Sfc32x16, chunk: &mut [u32], nt: bool) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.nextuv();
                let v1 = rng.nextuv();
                let v2 = rng.nextuv();
                let v3 = rng.nextuv();
                _mm512_stream_si512(out_ptr as *mut _, v0);
                _mm512_stream_si512(out_ptr.add(SFC32X16) as *mut _, v1);
                _mm512_stream_si512(out_ptr.add(SFC32X16 * 2) as *mut _, v2);
                _mm512_stream_si512(out_ptr.add(SFC32X16 * 3) as *mut _, v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.nextuv();
                _mm512_stream_si512(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        } else {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.nextuv();
                let v1 = rng.nextuv();
                let v2 = rng.nextuv();
                let v3 = rng.nextuv();
                _mm512_storeu_si512(out_ptr as *mut _, v0);
                _mm512_storeu_si512(out_ptr.add(SFC32X16) as *mut _, v1);
                _mm512_storeu_si512(out_ptr.add(SFC32X16 * 2) as *mut _, v2);
                _mm512_storeu_si512(out_ptr.add(SFC32X16 * 3) as *mut _, v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.nextuv();
                _mm512_storeu_si512(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        }

        if remaining > 0 {
            let mut tmp = [0u32; SFC32X16];
            let v = rng.nextuv();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x16_next_f32s_chunk(rng: &mut Sfc32x16, chunk: &mut [f32], nt: bool) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.nextfv();
                let v1 = rng.nextfv();
                let v2 = rng.nextfv();
                let v3 = rng.nextfv();
                _mm512_stream_ps(out_ptr, v0);
                _mm512_stream_ps(out_ptr.add(SFC32X16), v1);
                _mm512_stream_ps(out_ptr.add(SFC32X16 * 2), v2);
                _mm512_stream_ps(out_ptr.add(SFC32X16 * 3), v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.nextfv();
                _mm512_stream_ps(out_ptr, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        } else {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.nextfv();
                let v1 = rng.nextfv();
                let v2 = rng.nextfv();
                let v3 = rng.nextfv();
                _mm512_storeu_ps(out_ptr, v0);
                _mm512_storeu_ps(out_ptr.add(SFC32X16), v1);
                _mm512_storeu_ps(out_ptr.add(SFC32X16 * 2), v2);
                _mm512_storeu_ps(out_ptr.add(SFC32X16 * 3), v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.nextfv();
                _mm512_storeu_ps(out_ptr, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        }

        if remaining > 0 {
            let mut tmp = [0f32; SFC32X16];
            let v = rng.nextfv();
            _mm512_storeu_ps(tmp.as_mut_ptr(), v);
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x16_rand_i32s_chunk(
        rng: &mut Sfc32x16,
        chunk: &mut [i32],
        nt: bool,
        v_range: __m512i,
        v_min: __m512i,
    ) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.randiv(v_range, v_min);
                let v1 = rng.randiv(v_range, v_min);
                let v2 = rng.randiv(v_range, v_min);
                let v3 = rng.randiv(v_range, v_min);
                _mm512_stream_si512(out_ptr as *mut _, v0);
                _mm512_stream_si512(out_ptr.add(SFC32X16) as *mut _, v1);
                _mm512_stream_si512(out_ptr.add(SFC32X16 * 2) as *mut _, v2);
                _mm512_stream_si512(out_ptr.add(SFC32X16 * 3) as *mut _, v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.randiv(v_range, v_min);
                _mm512_stream_si512(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        } else {
            while remaining >= SFC32X16_UNROLL {
                let v0 = rng.randiv(v_range, v_min);
                let v1 = rng.randiv(v_range, v_min);
                let v2 = rng.randiv(v_range, v_min);
                let v3 = rng.randiv(v_range, v_min);
                _mm512_storeu_si512(out_ptr as *mut _, v0);
                _mm512_storeu_si512(out_ptr.add(SFC32X16) as *mut _, v1);
                _mm512_storeu_si512(out_ptr.add(SFC32X16 * 2) as *mut _, v2);
                _mm512_storeu_si512(out_ptr.add(SFC32X16 * 3) as *mut _, v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = rng.randiv(v_range, v_min);
                _mm512_storeu_si512(out_ptr as *mut _, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        }

        if remaining > 0 {
            let mut tmp = [0i32; SFC32X16];
            let v = rng.randiv(v_range, v_min);
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn sfc32x16_rand_f32s_chunk(
        rng: &mut Sfc32x16,
        chunk: &mut [f32],
        nt: bool,
        v_mult: __m512,
        v_min: __m512, v_hi: __m512,
    ) {
        let mut out_ptr = chunk.as_mut_ptr();
        let mut remaining = chunk.len();
        let aligned = nt && (out_ptr as usize & 63) == 0;

        if aligned {
            while remaining >= SFC32X16_UNROLL {
                let v0 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v1 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v2 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v3 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                _mm512_stream_ps(out_ptr, v0);
                _mm512_stream_ps(out_ptr.add(SFC32X16), v1);
                _mm512_stream_ps(out_ptr.add(SFC32X16 * 2), v2);
                _mm512_stream_ps(out_ptr.add(SFC32X16 * 3), v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                _mm512_stream_ps(out_ptr, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        } else {
            while remaining >= SFC32X16_UNROLL {
                let v0 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v1 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v2 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                let v3 = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                _mm512_storeu_ps(out_ptr, v0);
                _mm512_storeu_ps(out_ptr.add(SFC32X16), v1);
                _mm512_storeu_ps(out_ptr.add(SFC32X16 * 2), v2);
                _mm512_storeu_ps(out_ptr.add(SFC32X16 * 3), v3);
                out_ptr = out_ptr.add(SFC32X16_UNROLL);
                remaining -= SFC32X16_UNROLL;
            }
            while remaining >= SFC32X16 {
                let v = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
                _mm512_storeu_ps(out_ptr, v);
                out_ptr = out_ptr.add(SFC32X16);
                remaining -= SFC32X16;
            }
        }

        if remaining > 0 {
            let mut tmp = [0f32; SFC32X16];
            let v = crate::rngv::ClampBelow::clamp_below(rng.randfv(v_mult, v_min), v_hi);
            _mm512_storeu_ps(tmp.as_mut_ptr(), v);
            ptr::copy_nonoverlapping(tmp.as_ptr(), out_ptr, remaining);
        }
        // Non-temporal stores must be fenced on the issuing thread before the data is
    // read elsewhere (std::arch docs, "Safety of non-temporal stores").
    #[allow(unused_unsafe)]
    unsafe { ::core::arch::x86_64::_mm_sfence() };
}

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x16_next_u32s(ptr: *mut Sfc32x16, out: *mut u32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X16];
            let v = rng.nextuv();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X16_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x16::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x16_next_u32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x16_next_f32s(ptr: *mut Sfc32x16, out: *mut f32, count: usize) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X16];
            let v = rng.nextuv();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X16_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x16::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x16_next_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x16_rand_i32s(
        ptr: *mut Sfc32x16,
        out: *mut i32,
        count: usize,
        min: i32,
        max: i32,
    ) {
        if min > max {
            return; // empty range: write nothing (a panic cannot cross the C ABI)
        }
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X16];
            let v = rng.nextuv();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let v_min = _mm512_set1_epi32(min);
            let v_range = _mm512_set1_epi32(crate::rngv::scale32(min, max));

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X16_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x16::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x16_rand_i32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_range,
                        v_min,
                    );
                });
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sfc32x16_rand_f32s(
        ptr: *mut Sfc32x16,
        out: *mut f32,
        count: usize,
        min: f32,
        max: f32,
    ) {
        if count == 0 {
            return;
        }
        unsafe {
            let Some(rng) = ptr.as_mut() else { return };
            let mut tmp = [0u32; SFC32X16];
            let v = rng.nextuv();
            _mm512_storeu_si512(tmp.as_mut_ptr() as *mut _, v);
            let base_seed = ((tmp[0] as u64) << 32) | tmp[1] as u64;

            let v_mult = _mm512_set1_ps(max - min);
            let v_hi = _mm512_set1_ps(crate::_internal::hi_f32(min, max));
            let v_min = _mm512_set1_ps(min);

            let buffer = match crate::_internal::out_slice(out, count) { Some(b) => b, None => return };
            buffer
                .par_chunks_mut(SFC32X16_PAR_CHUNK)
                .enumerate()
                .for_each(|(chunk_idx, chunk)| {
                    let mut local_rng = Sfc32x16::from_seed64(crate::_internal::chunk_seed64(base_seed, chunk_idx));
                    sfc32x16_rand_f32s_chunk(
                        &mut local_rng,
                        chunk,
                        crate::_internal::prefer_nt_for(count, chunk),
                        v_mult,
                        v_min, v_hi);
                });
        }
    }
} // mod simd
