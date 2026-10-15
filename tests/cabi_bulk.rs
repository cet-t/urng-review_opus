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
