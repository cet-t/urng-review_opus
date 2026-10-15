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
