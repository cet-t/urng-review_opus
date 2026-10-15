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

/// REVIEW B13(d): every C ABI `rand_f32s` / `rand_f64s` must stay in
/// `[min, max)`. Near 1e6 (f32) and 1e15 (f64) one ulp is large compared with
/// the range width, so `u * (max - min) + min` rounds up to `max` for a few
/// percent of draws unless the result is clamped.
#[test]
fn c_abi_float_ranges_exclude_max() {
    use urng::*;
    const N: usize = 200_000;
    let mut bad: Vec<String> = Vec::new();
    macro_rules! run {
        (32, $h:expr, $randf:ident) => {{
            let (lo, hi) = (1.0e6f32, 1.0e6f32 + 1.0);
            let mut buf = vec![0f32; N];
            unsafe { $randf($h, buf.as_mut_ptr(), N, lo, hi) };
            let n = buf.iter().filter(|&&v| !(v >= lo && v < hi)).count();
            if n > 0 {
                bad.push(format!("{}: {n} of {N} outside [min, max)", stringify!($randf)));
            }
        }};
        (64, $h:expr, $randf:ident) => {{
            let (lo, hi) = (1.0e15f64, 1.0e15f64 + 1.0);
            let mut buf = vec![0f64; N];
            unsafe { $randf($h, buf.as_mut_ptr(), N, lo, hi) };
            let n = buf.iter().filter(|&&v| !(v >= lo && v < hi)).count();
            if n > 0 {
                bad.push(format!("{}: {n} of {N} outside [min, max)", stringify!($randf)));
            }
        }};
    }
    macro_rules! check {
        ($b:tt, $ctor:ident, $randf:ident, $free:ident, $seed:expr) => {{
            pastey::paste! {
                let h = [<$ctor _new>]($seed);
                run!($b, h, $randf);
                unsafe { $free(h) };
            }
        }};
    }
    macro_rules! check_simd {
        ($b:tt, $ctor:ident, $randf:ident, $free:ident, $seed:expr) => {{
            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            pastey::paste! {
                let h = [<$ctor _new>]($seed);
                if !h.is_null() {
                    run!($b, h, $randf);
                    unsafe { $free(h) };
                }
            }
        }};
    }
        check!(32, philox32, philox32_rand_f32s, philox32_free, 1u32);
        check_simd!(32, philox32x4x4, philox32x4x4_rand_f32s, philox32x4x4_free, 1u32);
        check!(32, squares32, squares32_rand_f32s, squares32_free, 1u32);
        check_simd!(32, squares32x8, squares32x8_rand_f32s, squares32x8_free, 1u32);
        check!(64, philox64, philox64_rand_f64s, philox64_free, 1u64);
        check!(64, threefish256, threefish256_rand_f64s, threefish256_free, 1u64);
        check!(32, threefry32x4, threefry32x4_rand_f32s, threefry32x4_free, 1u32);
        check!(32, threefry32x2, threefry32x2_rand_f32s, threefry32x2_free, 1u32);
        check!(32, jsf32, jsf32_rand_f32s, jsf32_free, 1u32);
        check_simd!(32, jsf32x16, jsf32x16_rand_f32s, jsf32x16_free, 1u32);
        check!(32, mt19937, mt19937_rand_f32s, mt19937_free, 1u32);
        check!(32, sfmt19937, sfmt19937_rand_f32s, sfmt19937_free, 1u64);
        check!(32, sfmt607, sfmt607_rand_f32s, sfmt607_free, 1u64);
        check!(32, sfmt216091, sfmt216091_rand_f32s, sfmt216091_free, 1u64);
        check!(32, pcg32, pcg32_rand_f32s, pcg32_free, 1u32);
        check_simd!(32, pcg32x8, pcg32x8_rand_f32s, pcg32x8_free, 1u32);
        check!(32, sfc32, sfc32_rand_f32s, sfc32_free, 1u32);
        check_simd!(32, sfc32x4, sfc32x4_rand_f32s, sfc32x4_free, 1u32);
        check_simd!(32, sfc32x8, sfc32x8_rand_f32s, sfc32x8_free, 1u32);
        check_simd!(32, sfc32x16, sfc32x16_rand_f32s, sfc32x16_free, 1u32);
        check!(32, splitmix32, splitmix32_rand_f32s, splitmix32_free, 1u32);
        check!(32, xoroshiro64ss, xoroshiro64ss_rand_f32s, xoroshiro64ss_free, 1u32);
        check_simd!(32, xoroshiro64ssx8, xoroshiro64ssx8_rand_f32s, xoroshiro64ssx8_free, 1u32);
        check_simd!(32, xoroshiro64ssx16, xoroshiro64ssx16_rand_f32s, xoroshiro64ssx16_free, 1u32);
        check!(32, xorshift32, xorshift32_rand_f32s, xorshift32_free, 1u32);
        check!(32, xorshift128, xorshift128_rand_f32s, xorshift128_free, 1u32);
        check!(32, xoshiro128pp, xoshiro128pp_rand_f32s, xoshiro128pp_free, 1u32);
        check!(32, xoshiro128ss, xoshiro128ss_rand_f32s, xoshiro128ss_free, 1u32);
        check_simd!(32, xoshiro128ppx16, xoshiro128ppx16_rand_f32s, xoshiro128ppx16_free, 1u32);
        check_simd!(32, xoshiro128ssx16, xoshiro128ssx16_rand_f32s, xoshiro128ssx16_free, 1u32);
        check!(64, biski64, biski64_rand_f64s, biski64_free, 1u64);
        check_simd!(64, biski64x8, biski64x8_rand_f64s, biski64x8_free, 1u64);
        check!(64, cet64, cet64_rand_f64s, cet64_free, 1u64);
        check!(64, cet256, cet256_rand_f64s, cet256_free, 1u64);
        check!(64, mt1993764, mt1993764_rand_f64s, mt1993764_free, 1u64);
        check!(64, sfmt1993764, sfmt_rand_f64s, sfmt1993764_free, 1u64);
        check!(64, sfc64, sfc64_rand_f64s, sfc64_free, 1u64);
        check!(64, splitmix64, splitmix64_rand_f64s, splitmix64_free, 1u64);
        check!(64, twisted_gfsr, twisted_gfsr_rand_f64s, twisted_gfsr_free, 1u64);
        check!(64, xoroshiro128pp, xoroshiro128pp_rand_f64s, xoroshiro128pp_free, 1u64);
        check!(64, xoroshiro128ss, xoroshiro128ss_rand_f64s, xoroshiro128ss_free, 1u64);
        check!(64, xorshift64, xorshift64_rand_f64s, xorshift64_free, 1u64);
        check!(64, xoshiro256pp, xoshiro256pp_rand_f64s, xoshiro256pp_free, 1u64);
        check!(64, xoshiro256ss, xoshiro256ss_rand_f64s, xoshiro256ss_free, 1u64);
    assert!(bad.is_empty(), "{bad:#?}");
}

/// REVIEW B13(a, b, f): every C ABI `rand_i32s` / `rand_i64s` handles the full
/// range without overflow (a debug-build overflow would abort the process)
/// and without collapsing to a constant, stays inside narrow ranges, and
/// writes nothing for `min > max`.
#[test]
fn c_abi_integer_ranges() {
    use urng::*;
    const N: usize = 50_000;
    let mut bad: Vec<String> = Vec::new();
    macro_rules! run {
        (32, $h:expr, $randi:ident) => {{
            let mut buf = vec![0i32; N];
            unsafe { $randi($h, buf.as_mut_ptr(), N, i32::MIN, i32::MAX) };
            let distinct = buf.iter().collect::<HashSet<_>>().len();
            if distinct < N * 9 / 10 {
                bad.push(format!("{}: full range gave only {distinct} distinct values", stringify!($randi)));
            }
            unsafe { $randi($h, buf.as_mut_ptr(), N, -3, 4) };
            if !buf.iter().all(|v| (-3..=4).contains(v)) {
                bad.push(format!("{}: value outside [-3, 4]", stringify!($randi)));
            }
            let mut untouched = [11i32; 64];
            unsafe { $randi($h, untouched.as_mut_ptr(), 64, 1, 0) };
            if untouched != [11; 64] {
                bad.push(format!("{}: wrote for min > max", stringify!($randi)));
            }
        }};
        (64, $h:expr, $randi:ident) => {{
            let mut buf = vec![0i64; N];
            unsafe { $randi($h, buf.as_mut_ptr(), N, i64::MIN, i64::MAX) };
            let distinct = buf.iter().collect::<HashSet<_>>().len();
            if distinct < N * 9 / 10 {
                bad.push(format!("{}: full range gave only {distinct} distinct values", stringify!($randi)));
            }
            unsafe { $randi($h, buf.as_mut_ptr(), N, -3, 4) };
            if !buf.iter().all(|v| (-3..=4).contains(v)) {
                bad.push(format!("{}: value outside [-3, 4]", stringify!($randi)));
            }
            let mut untouched = [11i64; 64];
            unsafe { $randi($h, untouched.as_mut_ptr(), 64, 1, 0) };
            if untouched != [11; 64] {
                bad.push(format!("{}: wrote for min > max", stringify!($randi)));
            }
        }};
    }
    macro_rules! check {
        ($b:tt, $ctor:ident, $randi:ident, $free:ident, $seed:expr) => {{
            pastey::paste! {
                let h = [<$ctor _new>]($seed);
                run!($b, h, $randi);
                unsafe { $free(h) };
            }
        }};
    }
    macro_rules! check_simd {
        ($b:tt, $ctor:ident, $randi:ident, $free:ident, $seed:expr) => {{
            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            pastey::paste! {
                let h = [<$ctor _new>]($seed);
                if !h.is_null() {
                    run!($b, h, $randi);
                    unsafe { $free(h) };
                }
            }
        }};
    }
        check!(32, philox32, philox32_rand_i32s, philox32_free, 1u32);
        check_simd!(32, philox32x4x4, philox32x4x4_rand_i32s, philox32x4x4_free, 1u32);
        check!(32, squares32, squares32_rand_i32s, squares32_free, 1u32);
        check_simd!(32, squares32x8, squares32x8_rand_i32s, squares32x8_free, 1u32);
        check!(64, philox64, philox64_rand_i64s, philox64_free, 1u64);
        check!(64, threefish256, threefish256_rand_i64s, threefish256_free, 1u64);
        check!(32, threefry32x4, threefry32x4_rand_i32s, threefry32x4_free, 1u32);
        check!(32, threefry32x2, threefry32x2_rand_i32s, threefry32x2_free, 1u32);
        check!(32, jsf32, jsf32_rand_i32s, jsf32_free, 1u32);
        check_simd!(32, jsf32x16, jsf32x16_rand_i32s, jsf32x16_free, 1u32);
        check!(32, mt19937, mt19937_rand_i32s, mt19937_free, 1u32);
        check!(32, sfmt19937, sfmt19937_rand_i32s, sfmt19937_free, 1u64);
        check!(32, sfmt607, sfmt607_rand_i32s, sfmt607_free, 1u64);
        check!(32, sfmt216091, sfmt216091_rand_i32s, sfmt216091_free, 1u64);
        check!(32, pcg32, pcg32_rand_i32s, pcg32_free, 1u32);
        check_simd!(32, pcg32x8, pcg32x8_rand_i32s, pcg32x8_free, 1u32);
        check!(32, sfc32, sfc32_rand_i32s, sfc32_free, 1u32);
        check_simd!(32, sfc32x4, sfc32x4_rand_i32s, sfc32x4_free, 1u32);
        check_simd!(32, sfc32x8, sfc32x8_rand_i32s, sfc32x8_free, 1u32);
        check_simd!(32, sfc32x16, sfc32x16_rand_i32s, sfc32x16_free, 1u32);
        check!(32, splitmix32, splitmix32_rand_i32s, splitmix32_free, 1u32);
        check!(32, xoroshiro64ss, xoroshiro64ss_rand_i32s, xoroshiro64ss_free, 1u32);
        check_simd!(32, xoroshiro64ssx8, xoroshiro64ssx8_rand_i32s, xoroshiro64ssx8_free, 1u32);
        check_simd!(32, xoroshiro64ssx16, xoroshiro64ssx16_rand_i32s, xoroshiro64ssx16_free, 1u32);
        check!(32, xorshift32, xorshift32_rand_i32s, xorshift32_free, 1u32);
        check!(32, xorshift128, xorshift128_rand_i32s, xorshift128_free, 1u32);
        check!(32, xoshiro128pp, xoshiro128pp_rand_i32s, xoshiro128pp_free, 1u32);
        check!(32, xoshiro128ss, xoshiro128ss_rand_i32s, xoshiro128ss_free, 1u32);
        check_simd!(32, xoshiro128ppx16, xoshiro128ppx16_rand_i32s, xoshiro128ppx16_free, 1u32);
        check_simd!(32, xoshiro128ssx16, xoshiro128ssx16_rand_i32s, xoshiro128ssx16_free, 1u32);
        check!(64, biski64, biski64_rand_i64s, biski64_free, 1u64);
        check_simd!(64, biski64x8, biski64x8_rand_i64s, biski64x8_free, 1u64);
        check!(64, cet64, cet64_rand_i64s, cet64_free, 1u64);
        check!(64, cet256, cet256_rand_i64s, cet256_free, 1u64);
        check!(64, mt1993764, mt1993764_rand_i64s, mt1993764_free, 1u64);
        check!(64, sfmt1993764, sfmt1993764_rand_i64s, sfmt1993764_free, 1u64);
        check!(64, sfc64, sfc64_rand_i64s, sfc64_free, 1u64);
        check!(64, splitmix64, splitmix64_rand_i64s, splitmix64_free, 1u64);
        check!(64, twisted_gfsr, twisted_gfsr_rand_i64s, twisted_gfsr_free, 1u64);
        check!(64, xoroshiro128pp, xoroshiro128pp_rand_i64s, xoroshiro128pp_free, 1u64);
        check!(64, xoroshiro128ss, xoroshiro128ss_rand_i64s, xoroshiro128ss_free, 1u64);
        check!(64, xorshift64, xorshift64_rand_i64s, xorshift64_free, 1u64);
        check!(64, xoshiro256pp, xoshiro256pp_rand_i64s, xoshiro256pp_free, 1u64);
        check!(64, xoshiro256ss, xoshiro256ss_rand_i64s, xoshiro256ss_free, 1u64);
    assert!(bad.is_empty(), "{bad:#?}");
}

/// REVIEW B14: generators with a jump-ahead (SplitMix32 Weyl step, PCG LCG
/// advance) now fill in parallel yet return exactly the sequential stream.
#[test]
fn advanceable_bulk_equals_sequential_stream() {
    use urng::*;
    let sizes = [0usize, 1, 7, 16, 131_071, 131_072, 131_073, 300_000];
    let mut store = vec![0u32; 300_000 + 4];
    let mut a = SplitMix32::new(21);
    let ha = splitmix32_new(21);
    let mut b = Pcg32::new(22);
    let hb = pcg32_new(22);
    for (i, &n) in sizes.iter().enumerate() {
        let off = i % 3;
        let buf = &mut store[off..off + n];
        unsafe { splitmix32_next_u32s(ha, buf.as_mut_ptr(), n) };
        assert!(buf.iter().all(|&x| x == a.nextu()), "splitmix32 call {i}");
        unsafe { pcg32_next_u32s(hb, buf.as_mut_ptr(), n) };
        assert!(buf.iter().all(|&x| x == b.nextu()), "pcg32 call {i}");
    }
    let mut f = vec![0f32; 1000];
    unsafe { pcg32_rand_f32s(hb, f.as_mut_ptr(), 1000, -2.0, 2.0) };
    assert!(f.iter().all(|&x| x == b.randf(-2.0, 2.0)));
    unsafe {
        splitmix32_free(ha);
        pcg32_free(hb);
    }
}

/// REVIEW B14: generators without a jump-ahead fill in parallel from
/// substreams. Every call now draws a 64-bit key and seeds the whole state, so
/// many small calls no longer regenerate earlier blocks (with 32-bit keys,
/// 300k calls of 4 words repeated a block 25 times for xoshiro128pp and 51
/// times for mt19937).
#[test]
fn repeated_small_calls_do_not_repeat_blocks() {
    use urng::*;
    const CALLS: usize = 200_000;
    macro_rules! dup_blocks {
        ($new:ident, $next:ident, $free:ident, $seed:expr) => {{
            let h = $new($seed);
            let mut seen = HashSet::with_capacity(CALLS);
            let mut dups = 0;
            for _ in 0..CALLS {
                let mut b = [0u32; 4];
                unsafe { $next(h, b.as_mut_ptr(), 4) };
                if !seen.insert(b) {
                    dups += 1;
                }
            }
            unsafe { $free(h) };
            dups
        }};
    }
    let results = [
        ("jsf32", dup_blocks!(jsf32_new, jsf32_next_u32s, jsf32_free, 1)),
        ("sfc32", dup_blocks!(sfc32_new, sfc32_next_u32s, sfc32_free, 1)),
        ("xoshiro128pp", dup_blocks!(xoshiro128pp_new, xoshiro128pp_next_u32s, xoshiro128pp_free, 1)),
        ("xoshiro128ss", dup_blocks!(xoshiro128ss_new, xoshiro128ss_next_u32s, xoshiro128ss_free, 1)),
        ("xoroshiro64ss", dup_blocks!(xoroshiro64ss_new, xoroshiro64ss_next_u32s, xoroshiro64ss_free, 1)),
        ("mt19937", dup_blocks!(mt19937_new, mt19937_next_u32s, mt19937_free, 1)),
        ("sfmt607", dup_blocks!(sfmt607_new, sfmt607_next_u32s, sfmt607_free, 1)),
    ];
    let bad: Vec<_> = results.iter().filter(|(_, d)| *d > 0).collect();
    assert!(bad.is_empty(), "repeated 4-word blocks: {bad:?}");

    // the 32-bit SIMD kernels derive their chunk generators the same way
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        macro_rules! dup_simd {
            ($new:ident, $next:ident, $free:ident) => {{
                let h = $new(1);
                if h.is_null() {
                    0
                } else {
                    let mut seen = HashSet::with_capacity(CALLS);
                    let mut dups = 0;
                    for _ in 0..CALLS {
                        let mut b = [0u32; 16];
                        unsafe { $next(h, b.as_mut_ptr(), 16) };
                        if !seen.insert(b) {
                            dups += 1;
                        }
                    }
                    unsafe { $free(h) };
                    dups
                }
            }};
        }
        let simd = [
            ("sfc32x4", dup_simd!(sfc32x4_new, sfc32x4_next_u32s, sfc32x4_free)),
            ("sfc32x8", dup_simd!(sfc32x8_new, sfc32x8_next_u32s, sfc32x8_free)),
            ("jsf32x16", dup_simd!(jsf32x16_new, jsf32x16_next_u32s, jsf32x16_free)),
            ("xoroshiro64ssx8", dup_simd!(xoroshiro64ssx8_new, xoroshiro64ssx8_next_u32s, xoroshiro64ssx8_free)),
            ("xoshiro128ppx16", dup_simd!(xoshiro128ppx16_new, xoshiro128ppx16_next_u32s, xoshiro128ppx16_free)),
        ];
        let bad: Vec<_> = simd.iter().filter(|(_, d)| *d > 0).collect();
        assert!(bad.is_empty(), "repeated 16-word blocks: {bad:?}");
    }

    // deterministic: the same seed gives the same bulk output
    let (h1, h2) = (jsf32_new(9), jsf32_new(9));
    let (mut x, mut y) = (vec![0u32; 300_000], vec![0u32; 300_000]);
    unsafe {
        jsf32_next_u32s(h1, x.as_mut_ptr(), x.len());
        jsf32_next_u32s(h2, y.as_mut_ptr(), y.len());
        jsf32_free(h1);
        jsf32_free(h2);
    }
    assert_eq!(x, y);
}

/// REVIEW B14: the Squares counter-based C ABI also returns the sequential
/// stream; the 8-lane version returns whole Squares32x8::nextu blocks and,
/// like philox32x4x4, discards the unused tail of the last block.
#[test]
fn squares_bulk_equals_sequential_stream() {
    use urng::*;
    let sizes = [0usize, 1, 7, 8, 9, 131_073, 300_001];
    let mut store = vec![0u32; 300_001 + 4];
    let mut want = Squares32::new(5);
    let h = squares32_new(5);
    for (i, &n) in sizes.iter().enumerate() {
        let buf = &mut store[i % 3..i % 3 + n];
        unsafe { squares32_next_u32s(h, buf.as_mut_ptr(), n) };
        assert!(buf.iter().all(|&x| x == want.nextu()), "squares32 call {i}");
    }
    unsafe { squares32_free(h) };

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    if std::arch::is_x86_feature_detected!("avx512f") && std::arch::is_x86_feature_detected!("avx512dq") {
        let mut v = unsafe { Squares32x8::new(6) };
        let mut stream = Vec::new();
        while stream.len() < 500_000 {
            stream.extend_from_slice(&unsafe { v.nextu() });
        }
        let h = squares32x8_new(6);
        let mut pos = 0;
        for (i, &n) in sizes.iter().enumerate() {
            let buf = &mut store[i % 3..i % 3 + n];
            unsafe { squares32x8_next_u32s(h, buf.as_mut_ptr(), n) };
            assert!(buf[..] == stream[pos..pos + n], "squares32x8 call {i} (n = {n})");
            pos += n.div_ceil(8) * 8; // the tail of the last block is discarded
        }
        unsafe { squares32x8_free(h) };
    }
}
