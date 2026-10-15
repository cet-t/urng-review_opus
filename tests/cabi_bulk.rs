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

/// REVIEW B12/B14: Philox32 bulk output equals the sequential `Rng::nextu`
/// stream for any buffer address, interleaving and size (including words left
/// in the ring buffer and sizes crossing the parallel chunk boundary).
#[test]
fn philox32_bulk_equals_sequential_stream() {
    use urng::*;
    let sizes = [0usize, 1, 3, 4, 5, 17, 131_071, 131_072, 131_073, 300_001];
    for seed in [0u32, 7] {
        let mut want_rng = Philox32::new(seed);
        let h = philox32_new(seed);
        let mut store = vec![0u32; 300_001 + 16];
        for (i, &n) in sizes.iter().enumerate() {
            let off = i % 5; // misaligned by 0..4 words
            let buf = &mut store[off..off + n];
            unsafe { philox32_next_u32s(h, buf.as_mut_ptr(), n) };
            for (j, &got) in buf.iter().enumerate() {
                assert_eq!(got, want_rng.nextu(), "seed {seed}, call {i} (n = {n}), element {j}");
            }
            // single draws through the same handle keep the stream continuous
            let mut one = 0u32;
            unsafe { philox32_next_u32s(h, &mut one, 1) };
            assert_eq!(one, want_rng.nextu());
        }
        // the mapped variants consume the same stream
        let mut f = vec![0f32; 1000];
        let mut r = vec![0i32; 1000];
        unsafe {
            philox32_next_f32s(h, f.as_mut_ptr(), f.len());
            philox32_rand_i32s(h, r.as_mut_ptr(), r.len(), i32::MIN, i32::MAX);
        }
        for &x in &f {
            assert_eq!(x, want_rng.nextf());
        }
        for &x in &r {
            assert_eq!(x, want_rng.randi(i32::MIN, i32::MAX));
        }
        // min > max writes nothing and consumes nothing
        let mut untouched = [7i32; 8];
        unsafe { philox32_rand_i32s(h, untouched.as_mut_ptr(), 8, 5, 4) };
        assert_eq!(untouched, [7; 8]);
        let mut next = 0u32;
        unsafe { philox32_next_u32s(h, &mut next, 1) };
        assert_eq!(next, want_rng.nextu());
        unsafe { philox32_free(h) };
    }
}

/// REVIEW B12/B14: Philox32x4x4 bulk output is the concatenation of
/// `Philox32x4x4::nextu` blocks and no longer depends on the buffer alignment
/// (the old unaligned-head path reused block `c` and then started at `c+1`).
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[test]
fn philox32x4x4_bulk_equals_sequential_blocks() {
    use urng::*;
    if !std::arch::is_x86_feature_detected!("avx512f") {
        assert!(philox32x4x4_new(1).is_null());
        return;
    }
    let sizes = [1usize, 15, 16, 17, 64, 1000, 131_072, 131_089, 400_000];
    let mut want_rng = unsafe { Philox32x4x4::new(3) };
    let h = philox32x4x4_new(3);
    let mut store = vec![0u32; 400_000 + 16];
    for (i, &n) in sizes.iter().enumerate() {
        let off = (i * 3) % 16;
        let buf = &mut store[off..off + n];
        unsafe { philox32x4x4_next_u32s(h, buf.as_mut_ptr(), n) };
        let mut want = Vec::with_capacity(n + 16);
        while want.len() < n {
            want.extend_from_slice(&unsafe { want_rng.nextu() });
        }
        assert!(buf == &want[..n], "call {i} (n = {n}, offset {off}) differs");
    }
    // the same request at two alignments gives the same values
    let (a, b) = (philox32x4x4_new(9), philox32x4x4_new(9));
    let mut x = vec![0u32; 5000];
    let mut y = vec![0u32; 5001];
    unsafe {
        philox32x4x4_next_u32s(a, x.as_mut_ptr(), 5000);
        philox32x4x4_next_u32s(b, y.as_mut_ptr().add(1), 5000);
    }
    assert_eq!(x[..], y[1..]);
    unsafe {
        philox32x4x4_free(a);
        philox32x4x4_free(b);
        philox32x4x4_free(h);
    }
}

/// REVIEW B14: every counter-mode C ABI bulk call equals the sequential
/// `Rng::nextu` stream (buffered words included, no discarded tail, any
/// alignment, across parallel chunk boundaries).
#[test]
fn counter_mode_bulk_equals_sequential_stream() {
    use urng::*;
    macro_rules! check {
        ($ty:ident, $w:ty, $seed:expr, $new:ident, $next:ident, $free:ident) => {{
            let sizes = [0usize, 1, 2, 3, 5, 63, 64, 65, 131_073, 262_145];
            let mut want = $ty::new($seed);
            let h = $new($seed);
            let mut store = vec![<$w>::default(); 262_145 + 8];
            for (i, &n) in sizes.iter().enumerate() {
                let off = i % 3;
                let buf = &mut store[off..off + n];
                unsafe { $next(h, buf.as_mut_ptr(), n) };
                for (j, &got) in buf.iter().enumerate() {
                    assert_eq!(got, want.nextu(), "{} call {i} (n = {n}) element {j}", stringify!($ty));
                }
            }
            unsafe { $free(h) };
        }};
    }
    check!(Threefry32x4, u32, 11, threefry32x4_new, threefry32x4_next_u32s, threefry32x4_free);
    check!(Threefry32x2, u32, 12, threefry32x2_new, threefry32x2_next_u32s, threefry32x2_free);
    check!(Philox64, u64, 13, philox64_new, philox64_next_u64s, philox64_free);
    check!(Threefish256, u64, 14, threefish256_new, threefish256_next_u64s, threefish256_free);

    // mapped variants use the same stream and the shared range mapping
    let mut want = Threefry32x2::new(5);
    let h = threefry32x2_new(5);
    let mut r = vec![0i32; 999];
    let mut f = vec![0f32; 999];
    unsafe {
        threefry32x2_rand_i32s(h, r.as_mut_ptr(), r.len(), i32::MIN, i32::MAX);
        threefry32x2_rand_f32s(h, f.as_mut_ptr(), f.len(), -1.0, 1.0);
        threefry32x2_free(h);
    }
    for &x in &r {
        assert_eq!(x, want.randi(i32::MIN, i32::MAX));
    }
    for &x in &f {
        assert_eq!(x, want.randf(-1.0, 1.0));
    }
    let mut want = Philox64::new(6);
    let h = philox64_new(6);
    let mut r = vec![0i64; 77];
    let mut untouched = [9i64; 4];
    unsafe {
        philox64_rand_i64s(h, untouched.as_mut_ptr(), 4, 1, 0);
        philox64_rand_i64s(h, r.as_mut_ptr(), r.len(), i64::MIN, i64::MAX);
        philox64_free(h);
    }
    assert_eq!(untouched, [9; 4], "min > max writes nothing");
    for &x in &r {
        assert_eq!(x, want.randi(i64::MIN, i64::MAX));
    }
}
