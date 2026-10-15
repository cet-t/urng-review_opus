//! Regression tests for the C ABI bulk-fill functions (REVIEW B2, B3, B7, B14).
#![cfg(feature = "cabi")]
#![allow(unused_unsafe)]

use std::collections::HashSet;

/// Number of duplicated values in `v`.
fn dups<T: std::hash::Hash + Eq + Copy>(v: &[T]) -> usize {
    let mut s = HashSet::with_capacity(v.len());
    v.iter().filter(|x| !s.insert(**x)).count()
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[test]
fn xoshiro256ssx2_bulk_is_sequential_and_does_not_collapse() {
    use urng::*;
    let n = 4 * 131_072 + 3; // several former chunks + odd tail
    let mut buf = vec![0u64; n];
    unsafe {
        let h = xoshiro256ssx2_new(7);
        xoshiro256ssx2_next_u64s(h, buf.as_mut_ptr(), n);
        xoshiro256ssx2_free(h);
    }
    let mut r = Xoshiro256Ssx2::new(7);
    let mut want = Vec::with_capacity(n + 1);
    while want.len() < n {
        want.extend_from_slice(&r.nextu());
    }
    want.truncate(n);
    assert_eq!(buf, want, "bulk output must equal repeated nextu()");
    assert_eq!(dups(&buf), 0, "64-bit outputs must not repeat");
    assert!(buf[n - 100..].iter().any(|&x| x != 0));
}

/// AVX-512F + DQ available (the 64-bit x8 bulk fills need both).
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
fn avx512() -> bool {
    std::arch::is_x86_feature_detected!("avx512f") && std::arch::is_x86_feature_detected!("avx512dq")
}

/// REVIEW B3 (a): per-lane seeds of the 64-bit SIMD bulk fills overlapped between
/// chunks, so ~97% of a 2^20-element buffer was duplicated. 64-bit outputs of a
/// sound generator essentially never repeat in 2^20 draws.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[test]
fn simd64_bulk_fills_have_no_duplicates_across_chunks() {
    use urng::*;
    if !avx512() {
        eprintln!("skipped: AVX-512F/DQ not available");
        return;
    }
    let n = 1usize << 20;
    let mut buf = vec![0u64; n];
    unsafe {
        let h = sfc64x8_new(1);
        sfc64x8_next_u64s(h, buf.as_mut_ptr(), n);
        sfc64x8_free(h);
    }
    assert_eq!(dups(&buf), 0, "sfc64x8_next_u64s");
    unsafe {
        let h = biski64x8_new(1);
        biski64x8_next_u64s(h, buf.as_mut_ptr(), n);
        assert_eq!(dups(&buf), 0, "biski64x8_next_u64s");
        let mut f = vec![0f64; n];
        biski64x8_next_f64s(h, f.as_mut_ptr(), n);
        let bits: Vec<u64> = f.iter().map(|x| x.to_bits()).collect();
        // f64s carry 52 random bits: a handful of birthday collisions is expected
        assert!(dups(&bits) < 16, "biski64x8_next_f64s: {} dups", dups(&bits));
        biski64x8_free(h);
    }
    unsafe {
        let h = cet64x8_new(1);
        cet64x8_next_u64s(h, buf.as_mut_ptr(), n);
        cet64x8_free(h);
    }
    assert_eq!(dups(&buf), 0, "cet64x8_next_u64s");
}

/// REVIEW B3 (b): `xoroshiro64ss_*` seeded chunk k with `seed + k`, which together
/// with `seed | 1` made two adjacent 131072-element chunks identical.
#[test]
fn xoroshiro64ss_bulk_chunks_are_distinct() {
    use urng::*;
    let chunk = 0x20000;
    for seed in [1u32, 2, 3, 4] {
        let mut buf = vec![0u32; 4 * chunk];
        unsafe {
            let h = xoroshiro64ss_new(seed);
            xoroshiro64ss_next_u32s(h, buf.as_mut_ptr(), buf.len());
            xoroshiro64ss_free(h);
        }
        for k in 0..3 {
            assert!(buf[k * chunk..(k + 1) * chunk] != buf[(k + 1) * chunk..(k + 2) * chunk],
                "seed {seed}: chunk {k} == chunk {}",
                k + 1
            );
        }
        // 2^19 u32 draws: ~32 birthday collisions expected; duplicated chunks give 131072+
        assert!(dups(&buf) < 200, "seed {seed}: {} duplicates", dups(&buf));
    }
}

/// REVIEW B7(c): `(NULL, 0)` output buffers and null handles must be accepted
/// (they used to reach `slice::from_raw_parts_mut(NULL, 0)`, which is UB and
/// aborts debug builds).
#[test]
fn null_and_empty_buffers_are_accepted() {
    use urng::*;
    unsafe {
        let h = mt19937_new(1);
        mt19937_next_u32s(h, std::ptr::null_mut(), 0);
        mt19937_next_f32s(h, std::ptr::null_mut(), 0);
        mt19937_rand_i32s(h, std::ptr::null_mut(), 0, 0, 9);
        mt19937_rand_f32s(h, std::ptr::null_mut(), 0, 0.0, 1.0);
        mt19937_next_u32s(h, std::ptr::null_mut(), 16); // null buffer: ignored
        mt19937_free(h);
        let mut buf = [0u32; 4];
        mt19937_next_u32s(std::ptr::null_mut(), buf.as_mut_ptr(), 4); // null handle
        assert_eq!(buf, [0; 4]);
        xorshift32_next_f32s(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        let h = xorshift64_new(1);
        xorshift64_next_u64s(h, std::ptr::null_mut(), 0);
        xorshift64_free(h);
        let h = sfmt607_new(1);
        sfmt607_next_f32s(h, std::ptr::null_mut(), 0);
        sfmt607_free(h);
    }
}

/// REVIEW B13: SIMD `rand_i32s(i32::MIN, i32::MAX)` returned only `i32::MIN`
/// (the 2^32 range did not fit the 32-bit lane scale).
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[test]
fn simd_full_range_rand_i32s_is_not_constant() {
    use urng::*;
    let mut buf = vec![0i32; 4096];
    let distinct = |b: &[i32]| b.iter().collect::<HashSet<_>>().len();
    unsafe {
        let h = sfc32x4_new(1);
        sfc32x4_rand_i32s(h, buf.as_mut_ptr(), buf.len(), i32::MIN, i32::MAX);
        sfc32x4_free(h);
        assert!(distinct(&buf) > 4000, "sfc32x4");
        if std::arch::is_x86_feature_detected!("avx2") {
            let h = sfc32x8_new(1);
            sfc32x8_rand_i32s(h, buf.as_mut_ptr(), buf.len(), i32::MIN, i32::MAX);
            sfc32x8_free(h);
            assert!(distinct(&buf) > 4000, "sfc32x8");
            let h = xoroshiro64ssx8_new(1);
            xoroshiro64ssx8_rand_i32s(h, buf.as_mut_ptr(), buf.len(), i32::MIN, i32::MAX);
            xoroshiro64ssx8_free(h);
            assert!(distinct(&buf) > 4000, "xoroshiro64ssx8");
        }
        if std::arch::is_x86_feature_detected!("avx512f") {
            let h = jsf32x16_new(1);
            jsf32x16_rand_i32s(h, buf.as_mut_ptr(), buf.len(), i32::MIN, i32::MAX);
            jsf32x16_free(h);
            assert!(distinct(&buf) > 4000, "jsf32x16");
            let h = philox32x4x4_new(1);
            philox32x4x4_rand_i32s(h, buf.as_mut_ptr(), buf.len(), i32::MIN, i32::MAX);
            philox32x4x4_free(h);
            assert!(distinct(&buf) > 4000, "philox32x4x4");
        }
        // narrow ranges stay in range
        let h = sfc32x4_new(2);
        sfc32x4_rand_i32s(h, buf.as_mut_ptr(), buf.len(), -3, 5);
        sfc32x4_free(h);
        assert!(buf.iter().all(|&v| (-3..=5).contains(&v)));
    }
}
