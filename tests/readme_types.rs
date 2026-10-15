//! REVIEW B20: the types listed in README.md exist under the documented paths.

#[test]
fn readme_types_exist() {
    let _ = urng::Philox4x64::new(1);
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        let _ = urng::Sfc32x4::new(1);
        let _ = urng::Xoshiro256Ssx2::new(1);
        if std::arch::is_x86_feature_detected!("avx2") {
            let _ = unsafe { urng::Xorshift32x8::new(1) };
        }
    }
}
