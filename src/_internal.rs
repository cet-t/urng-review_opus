#![allow(unused, dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};


/// SIMD bit-trick converters mapping an integer lane vector uniformly onto
/// `[0, 1)`, matching the scalar [`u2f_01`] exactly lane-by-lane (top mantissa
/// bits via `(x >> bias) | exponent`, then `- 1.0`). Used by every SIMD
/// generator so its `nextf`/`randf` reproduce the scalar sequence bit-for-bit.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub(crate) mod simd_f01 {
    use std::arch::x86_64::*;

    /// 4×`u32` → 4×`f32` in `[0, 1)`.
    #[inline(always)]
    pub(crate) unsafe fn u32x4(v: __m128i) -> __m128 {
        unsafe {
            let bits = _mm_or_si128(_mm_srli_epi32(v, 9), _mm_set1_epi32(0x3F80_0000));
            _mm_sub_ps(_mm_castsi128_ps(bits), _mm_set1_ps(1.0))
        }
    }

    /// 8×`u32` → 8×`f32` in `[0, 1)`.
    #[inline]
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn u32x8(v: __m256i) -> __m256 {
        let bits = _mm256_or_si256(_mm256_srli_epi32(v, 9), _mm256_set1_epi32(0x3F80_0000));
        _mm256_sub_ps(_mm256_castsi256_ps(bits), _mm256_set1_ps(1.0))
    }

    /// 16×`u32` → 16×`f32` in `[0, 1)`.
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub(crate) unsafe fn u32x16(v: __m512i) -> __m512 {
        let bits = _mm512_or_si512(_mm512_srli_epi32(v, 9), _mm512_set1_epi32(0x3F80_0000));
        _mm512_sub_ps(_mm512_castsi512_ps(bits), _mm512_set1_ps(1.0))
    }

    /// 2×`u64` → 2×`f64` in `[0, 1)`.
    #[inline(always)]
    pub(crate) unsafe fn u64x2(v: __m128i) -> __m128d {
        unsafe {
            let bits = _mm_or_si128(
                _mm_srli_epi64(v, 11),
                _mm_set1_epi64x(0x3FF0_0000_0000_0000),
            );
            _mm_sub_pd(_mm_castsi128_pd(bits), _mm_set1_pd(1.0))
        }
    }

    /// 4×`u64` → 4×`f64` in `[0, 1)`.
    #[inline]
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn u64x4(v: __m256i) -> __m256d {
        let bits = _mm256_or_si256(
            _mm256_srli_epi64(v, 11),
            _mm256_set1_epi64x(0x3FF0_0000_0000_0000),
        );
        _mm256_sub_pd(_mm256_castsi256_pd(bits), _mm256_set1_pd(1.0))
    }

    /// 8×`u64` → 8×`f64` in `[0, 1)`.
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub(crate) unsafe fn u64x8(v: __m512i) -> __m512d {
        let bits = _mm512_or_si512(
            _mm512_srli_epi64(v, 11),
            _mm512_set1_epi64(0x3FF0_0000_0000_0000),
        );
        _mm512_sub_pd(_mm512_castsi512_pd(bits), _mm512_set1_pd(1.0))
    }
}

macro_rules! randi_wide {
    (i 32) => {
        i64
    };
    (i 64) => {
        i128
    };
    (u 32) => {
        u64
    };
    (u 64) => {
        u128
    };
}

pub(crate) use randi_wide;

macro_rules! i2f_bits {
    (32 bits) => {
        0x3F800000
    };
    (32 bias) => {
        9
    };
    (64 bits) => {
        0x3FF0000000000000
    };
    (64 bias) => {
        11
    };
}

pub(crate) use i2f_bits;

macro_rules! u2f_01 {
    ($ft:ty, $bits:tt, $x:expr) => {{
        <$ft>::from_bits(($x >> i2f_bits!($bits bias)) | i2f_bits!($bits bits)) - 1.0
    }};
}

pub(crate) use u2f_01;

macro_rules! u2f_01w {
    ($ft:ty, $bits:tt, $x:expr) => {
        ::wrapn::wrap!(<$ft>::from_bits(($x.0.0 >> i2f_bits!($bits bias)) | i2f_bits!($bits bits)) - 1.0)
    };
}

pub(crate) use u2f_01w;

macro_rules! sm64_from_seed32 {
    ($seed:expr) => {{
        let mut s = $crate::SplitMix32::new($seed);
        let sg =
            $crate::SplitMix64::new(((s.nextu_const() as u64) << 32) | (s.nextu_const() as u64));
        sg
    }};
}

pub(crate) use sm64_from_seed32;

/// Implements [`crate::rng::Rng`] (with `Word = u32`) for a counter-based block
/// generator that owns `buf: [wu32; N]` and `pos: wusize` fields, by
/// buffering blocks produced by an existing `fn $raw(&mut self) -> [u32; N]`
/// method and handing out one scalar per call (recomputing a fresh block every
/// `N`th call).
///
/// Crate-internal only — call as `crate::_internal::impl_ring_rng32!`.
macro_rules! impl_ring_rng32 {
    ($ty:ty, $n:expr, $raw:ident) => {
        impl $crate::rng::Rng for $ty {
            type Word = u32;

            #[inline]
            fn nextu(&mut self) -> Self::Word {
                if self.pos >= $n {
                    self.buf = self.$raw();
                    self.pos = ::wrapn::wrap!(0);
                }
                let v = self.buf[*self.pos];
                self.pos += 1;
                *v
            }
        }
    };
}
pub(crate) use impl_ring_rng32;

/// Implements [`crate::rng::Rng`] (with `Word = u64`) for a counter-based block
/// generator; see [`impl_ring_rng32`].
///
/// Crate-internal only — call as `crate::_internal::impl_ring_rng64!`.
macro_rules! impl_ring_rng64 {
    ($ty:ty, $n:expr, $raw:ident) => {
        impl $crate::rng::Rng for $ty {
            type Word = u64;

            #[inline]
            fn nextu(&mut self) -> Self::Word {
                if self.pos >= $n {
                    // self.buf = self.$raw().map(::core::convert::Into::into);
                    self.buf = self.$raw();
                    self.pos = ::wrapn::wrap!(0);
                }
                let v = self.buf[*self.pos];
                self.pos += 1;
                *v
            }
        }
    };
}
pub(crate) use impl_ring_rng64;

static DEFAULT_SEED_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Derives a time-based 64-bit seed for `Default` impls.
///
/// Mixes wall-clock nanoseconds with a call counter (to avoid identical
/// seeds for back-to-back calls within the same timer tick) through the
/// existing [`crate::prng::b64::SplitMix64::compute`] finalizer.
pub(crate) fn default_seed64() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let count = DEFAULT_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    crate::prng::b64::SplitMix64::compute(nanos ^ count.wrapping_mul(0x9E3779B97F4A7C15))
}

/// 32-bit counterpart of [`default_seed64`]: folds the 64-bit mix down via xor.
pub(crate) fn default_seed32() -> u32 {
    let z = default_seed64();
    (z ^ (z >> 32)) as u32
}

/// Turns a C output buffer into a slice. `(NULL, 0)` and any `count == 0` are
/// accepted and yield `None` (`slice::from_raw_parts_mut` requires a non-null
/// pointer even for empty slices, so passing them through was UB).
///
/// # Safety
///
/// If `count > 0`, `out` must be valid for writes of `count` elements, aligned,
/// and not aliased for the returned lifetime. A null `out` with `count > 0` also
/// yields `None`.
#[inline(always)]
pub(crate) unsafe fn out_slice<'a, T>(out: *mut T, count: usize) -> Option<&'a mut [T]> {
    if out.is_null() || count == 0 {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts_mut(out, count) })
    }
}

/// Fills `out[..count]` from repeated calls to `next`.
///
/// # Safety
///
/// `out` must be valid for writes of `count` elements.
#[inline(always)]
pub(crate) unsafe fn fill_with<T, F: FnMut() -> T>(out: *mut T, count: usize, mut next: F) {
    let Some(buffer) = (unsafe { out_slice(out, count) }) else {
        return;
    };
    for v in buffer {
        *v = next();
    }
}

/// Fills `chunk` from repeated calls to `gen`, 4x unrolled.
/// The tail is filled from one extra `gen` call truncated to the remainder.
#[inline(always)]
pub(crate) unsafe fn fill_chunk<T: Copy, const N: usize, F: FnMut() -> [T; N]>(
    chunk: &mut [T],
    mut generate: F,
) {
    let mut out_ptr = chunk.as_mut_ptr();
    let mut remaining = chunk.len();
    while remaining >= N * 4 {
        let v0 = generate();
        let v1 = generate();
        let v2 = generate();
        let v3 = generate();
        unsafe {
            std::ptr::copy_nonoverlapping(v0.as_ptr(), out_ptr, N);
            std::ptr::copy_nonoverlapping(v1.as_ptr(), out_ptr.add(N), N);
            std::ptr::copy_nonoverlapping(v2.as_ptr(), out_ptr.add(N * 2), N);
            std::ptr::copy_nonoverlapping(v3.as_ptr(), out_ptr.add(N * 3), N);
            out_ptr = out_ptr.add(N * 4);
        }
        remaining -= N * 4;
    }
    while remaining >= N {
        let v = generate();
        unsafe {
            std::ptr::copy_nonoverlapping(v.as_ptr(), out_ptr, N);
            out_ptr = out_ptr.add(N);
        }
        remaining -= N;
    }
    if remaining > 0 {
        let v = generate();
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), out_ptr, remaining) };
    }
}

/// Non-temporal variant of [`fill_chunk`]: streams each generated batch
/// straight to memory, bypassing the cache (no RFO read traffic).
/// Requires `N * size_of::<T>()` to be a multiple of 32 bytes.
/// Falls back to cached stores when `chunk` is not 32-byte aligned.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[allow(unsafe_op_in_unsafe_fn)]
pub(crate) unsafe fn fill_chunk_nt<T: Copy, const N: usize, F: FnMut() -> [T; N]>(
    chunk: &mut [T],
    mut generate: F,
) {
    use std::arch::x86_64::*;
    let words = (N * size_of::<T>()) / 32;
    // A batch smaller than one 32-byte store cannot be streamed; silently
    // dropping stores would corrupt the output, so fall back instead.
    if words == 0 || !(N * size_of::<T>()).is_multiple_of(32) {
        return fill_chunk(chunk, generate);
    }
    let mut p = chunk.as_mut_ptr();
    let mut rem = chunk.len();
    if (p as usize) & 31 == 0 {
        while rem >= N {
            let v = generate();
            let src = v.as_ptr() as *const __m256i;
            for i in 0..words {
                _mm256_stream_si256(
                    (p as *mut u8).add(i * 32) as *mut __m256i,
                    _mm256_loadu_si256(src.add(i)),
                );
            }
            p = p.add(N);
            rem -= N;
        }
        _mm_sfence();
    } else {
        while rem >= N {
            let v = generate();
            std::ptr::copy_nonoverlapping(v.as_ptr(), p, N);
            p = p.add(N);
            rem -= N;
        }
    }
    if rem > 0 {
        let v = generate();
        std::ptr::copy_nonoverlapping(v.as_ptr(), p, rem);
    }
}

/// Buffers larger than this are streamed past the cache (NT stores);
/// smaller ones use cached stores so an L3-resident working set never
/// touches DRAM. Compare against the *whole* destination buffer size,
/// not the per-thread chunk.
pub(crate) const NT_THRESHOLD_BYTES: usize = 24 << 20;

/// Returns whether a fill of `total_bytes` should use non-temporal stores.
#[inline(always)]
pub(crate) fn prefer_nt<T>(total_elems: usize) -> bool {
    total_elems * size_of::<T>() > NT_THRESHOLD_BYTES
}

/// [`prefer_nt`] with the element type inferred from a sample slice
/// (typically the per-thread chunk; pass the *total* element count).
#[inline(always)]
pub(crate) fn prefer_nt_for<T>(total_elems: usize, _sample: &[T]) -> bool {
    total_elems * size_of::<T>() > NT_THRESHOLD_BYTES
}

/// Dispatches to [`fill_chunk_nt`] when requested and AVX2 is available,
/// else [`fill_chunk`]. Pass `nt = prefer_nt::<T>(buffer.len())` computed
/// on the whole destination buffer.
///
/// # Safety
///
/// Same contract as [`fill_chunk`].
#[inline(always)]
pub(crate) unsafe fn fill_chunk_auto<T: Copy, const N: usize, F: FnMut() -> [T; N]>(
    chunk: &mut [T],
    nt: bool,
    generate: F,
) {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    if nt && std::arch::is_x86_feature_detected!("avx2") {
        return unsafe { fill_chunk_nt(chunk, generate) };
    }
    let _ = nt;
    unsafe { fill_chunk(chunk, generate) }
}

/// The largest value a draw in `[min, max)` may take: `max.next_down()` when
/// `max > min`, otherwise `+inf` (no clamping; matches [`crate::Word::to_randf`]).
#[inline]
#[cfg_attr(not(feature = "cabi"), allow(dead_code))]
pub(crate) fn hi_f32(min: f32, max: f32) -> f32 {
    if max > min { max.next_down() } else { f32::INFINITY }
}

/// `f64` version of [`hi_f32`].
#[inline]
#[cfg_attr(not(feature = "cabi"), allow(dead_code))]
pub(crate) fn hi_f64(min: f64, max: f64) -> f64 {
    if max > min { max.next_down() } else { f64::INFINITY }
}

/// The 32-bit word source used by the 32-bit generators' constructors.
///
/// `from_u32(seed)` yields exactly the `SplitMix32::new(seed)` stream, so every
/// `new(seed)` keeps its outputs. `from_u64(seed)` yields both halves of a
/// `SplitMix64::new(seed)` stream, so `from_seed64` constructors reach far more
/// than 2^32 initial states. The C ABI bulk fills seed their per-chunk
/// generators that way (REVIEW B14: with 32-bit chunk seeds, repeated calls
/// regenerated identical blocks after about 2^16 chunk instantiations).
#[derive(Clone, Copy)]
pub(crate) struct SeedWords {
    sm32: crate::prng::b32::SplitMix32,
    sm64: crate::prng::b64::SplitMix64,
    wide: bool,
    /// The unused high half of the last 64-bit output.
    pending: Option<u32>,
}

impl SeedWords {
    pub(crate) const fn from_u32(seed: u32) -> Self {
        Self {
            sm32: crate::prng::b32::SplitMix32::new(seed),
            sm64: crate::prng::b64::SplitMix64::new(0),
            wide: false,
            pending: None,
        }
    }

    pub(crate) const fn from_u64(seed: u64) -> Self {
        Self {
            sm32: crate::prng::b32::SplitMix32::new(0),
            sm64: crate::prng::b64::SplitMix64::new(seed),
            wide: true,
            pending: None,
        }
    }

    #[inline]
    pub(crate) const fn nextu_const(&mut self) -> u32 {
        if !self.wide {
            return self.sm32.nextu_const();
        }
        if let Some(hi) = self.pending.take() {
            return hi;
        }
        let x = self.sm64.nextu_const();
        self.pending = Some((x >> 32) as u32);
        x as u32
    }

    #[inline]
    pub(crate) const fn nextu(&mut self) -> u32 {
        self.nextu_const()
    }
}

/// A 64-bit key drawn from a 32-bit generator (two words, high word first).
#[cfg(feature = "cabi")]
#[inline]
pub(crate) fn key64<R: crate::rng::Rng<Word = u32>>(rng: &mut R) -> u64 {
    let hi = rng.nextu() as u64;
    (hi << 32) | rng.nextu() as u64
}

/// Decorrelated per-chunk 64-bit seed: distinct chunk indices give distinct
/// seeds for a fixed key (the Weyl step is odd and the finalizer bijective).
#[cfg(feature = "cabi")]
#[inline]
pub(crate) fn chunk_seed64(key: u64, chunk_idx: usize) -> u64 {
    crate::prng::b64::SplitMix64::compute(key.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// A counter-mode block generator whose [`crate::rng::Rng::nextu`] stream is
/// `block(ctr), block(ctr + 1), ...` served word by word through a ring buffer.
#[cfg(feature = "cabi")]
pub(crate) trait CtrRing: crate::rng::Rng + Sync {
    /// Words per block (must divide 64).
    const N: usize;
    /// Whether words of the current block are still buffered.
    fn buffered(&self) -> bool;
    /// The counter of the next block to compute.
    fn ctr(&self) -> u128;
    fn set_ctr(&mut self, ctr: u128);
    /// Writes `block(ctr)` into `out[..N]`.
    fn block(&self, ctr: u128, out: &mut [Self::Word]);
}

/// Bulk fill for [`CtrRing`] generators (REVIEW B14). The output is exactly
/// what `buf.len()` successive `nextu` calls return: buffered words first,
/// then whole blocks generated in parallel from the counter, then a final
/// partial block through the ring buffer so the next call continues with the
/// unused words. Independent of buffer alignment and thread scheduling.
#[cfg(feature = "cabi")]
pub(crate) fn par_fill_ctr<R, T, M>(rng: &mut R, buf: &mut [T], map: M)
where
    R: CtrRing,
    R::Word: Copy + Default,
    T: Copy + Default + Send,
    M: Fn(R::Word) -> T + Sync,
{
    use rayon::iter::{IndexedParallelIterator, ParallelIterator};
    use rayon::slice::ParallelSliceMut;
    const PAR_CHUNK: usize = 0x20000;
    const BATCH: usize = 64;
    debug_assert!(BATCH % R::N == 0 && PAR_CHUNK % R::N == 0);

    let mut head = 0;
    while rng.buffered() && head < buf.len() {
        buf[head] = map(rng.nextu());
        head += 1;
    }
    let rest = &mut buf[head..];
    let whole = rest.len() / R::N * R::N;
    let (body, tail) = rest.split_at_mut(whole);
    let c0 = rng.ctr();
    let nt = prefer_nt::<T>(body.len());
    {
        let r: &R = rng;
        body.par_chunks_mut(PAR_CHUNK)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let mut ctr = c0.wrapping_add((chunk_idx * (PAR_CHUNK / R::N)) as u128);
                let mut words = [R::Word::default(); BATCH];
                unsafe {
                    fill_chunk_auto(chunk, nt, || {
                        let mut out = [T::default(); BATCH];
                        for blk in words.chunks_exact_mut(R::N) {
                            r.block(ctr, blk);
                            ctr = ctr.wrapping_add(1);
                        }
                        for (o, w) in out.iter_mut().zip(words) {
                            *o = map(w);
                        }
                        out
                    });
                }
            });
    }
    rng.set_ctr(c0.wrapping_add((whole / R::N) as u128));
    for o in tail {
        *o = map(rng.nextu());
    }
}

/// Bulk fill for generators with an O(log n) jump-ahead (REVIEW B14): every
/// chunk starts from a copy of `rng` advanced to its offset, so the output is
/// exactly the sequential `nextu` stream; `rng` ends advanced past the buffer.
#[cfg(feature = "cabi")]
pub(crate) fn par_fill_advance<R, T, A, M>(rng: &mut R, buffer: &mut [T], advance: A, map: M)
where
    R: crate::rng::Rng + Clone + Sync,
    T: Copy + Default + Send,
    A: Fn(&mut R, u64) + Sync,
    M: Fn(R::Word) -> T + Sync,
{
    use rayon::iter::{IndexedParallelIterator, ParallelIterator};
    use rayon::slice::ParallelSliceMut;
    const PAR_CHUNK: usize = 0x20000;
    let nt = prefer_nt::<T>(buffer.len());
    let start: &R = rng;
    buffer
        .par_chunks_mut(PAR_CHUNK)
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let mut r = start.clone();
            advance(&mut r, (chunk_idx * PAR_CHUNK) as u64);
            unsafe {
                fill_chunk_auto(chunk, nt, || {
                    let mut out = [T::default(); 16];
                    for v in &mut out {
                        *v = map(r.nextu());
                    }
                    out
                });
            }
        });
    advance(rng, buffer.len() as u64);
}

/// Parallel bulk fill for 32-bit generators without a jump-ahead. Each 512KB
/// chunk runs its own generator built by `new_rng(chunk_seed64(key, idx))`,
/// which must seed the *whole* state from the 64-bit seed (`from_seed64`).
/// The output is a set of substreams, not the handle's sequential stream; the
/// caller draws a fresh 64-bit `key` from the handle on every call
/// (REVIEW B14: 32-bit keys and chunk seeds made repeated calls regenerate
/// identical blocks). 16 outputs are batched per generator call and stores
/// are size-adaptive (cached for L3-resident buffers, non-temporal above).
#[cfg(feature = "cabi")]
pub(crate) fn par_fill_reseed32<R, T, NF, SF>(
    buffer: &mut [T],
    key: u64,
    new_rng: NF,
    step: SF,
) where
    T: Copy + Default + Send,
    NF: Fn(u64) -> R + Sync,
    SF: Fn(&mut R) -> T + Sync,
{
    use rayon::iter::{IndexedParallelIterator, ParallelIterator};
    use rayon::slice::ParallelSliceMut;
    const PAR_CHUNK: usize = 0x20000;
    let nt = prefer_nt::<T>(buffer.len());
    buffer
        .par_chunks_mut(PAR_CHUNK)
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let mut rng = new_rng(chunk_seed64(key, chunk_idx));
            unsafe {
                fill_chunk_auto(chunk, nt, || {
                    let mut out = [T::default(); 16];
                    for v in &mut out {
                        *v = step(&mut rng);
                    }
                    out
                });
            }
        });
}
/// 64-bit counterpart of [`par_fill_reseed32`]: 8 outputs per batch
/// (64B for 8-byte `T`), chunk seeds decorrelated by golden-ratio steps
/// plus the SplitMix64 finalizer.
#[cfg(feature = "cabi")]
pub(crate) fn par_fill_reseed64<R, T, NF, SF>(
    buffer: &mut [T],
    base_seed: u64,
    new_rng: NF,
    step: SF,
) where
    T: Copy + Default + Send,
    NF: Fn(u64) -> R + Sync,
    SF: Fn(&mut R) -> T + Sync,
{
    use rayon::iter::{IndexedParallelIterator, ParallelIterator};
    use rayon::slice::ParallelSliceMut;
    const PAR_CHUNK: usize = 0x20000;
    let nt = prefer_nt::<T>(buffer.len());
    buffer
        .par_chunks_mut(PAR_CHUNK)
        .enumerate()
        .for_each(|(chunk_idx, chunk)| {
            let chunk_seed = crate::prng::b64::SplitMix64::compute(
                base_seed.wrapping_add((chunk_idx as u64).wrapping_mul(0x9E3779B97F4A7C15)),
            );
            let mut rng = new_rng(chunk_seed);
            unsafe {
                fill_chunk_auto(chunk, nt, || {
                    let mut out = [T::default(); 8];
                    for v in &mut out {
                        *v = step(&mut rng);
                    }
                    out
                });
            }
        });
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seed_calls_are_distinct() {
        assert_ne!(default_seed64(), default_seed64());
        assert_ne!(default_seed32(), default_seed32());
    }

    /// Sequential counter generator: makes dropped or duplicated batches
    /// detectable as exact-value mismatches.
    fn counter_batches<const N: usize>() -> impl FnMut() -> [u32; N] {
        let mut next = 0u32;
        move || {
            let mut out = [0u32; N];
            for v in &mut out {
                *v = next;
                next += 1;
            }
            out
        }
    }

    fn check_fill(buf: &[u32], len: usize) {
        for (i, &v) in buf[..len].iter().enumerate() {
            assert_eq!(v as usize, i, "element {i} wrong");
        }
    }

    #[test]
    fn fill_chunk_writes_every_element() {
        for len in [0usize, 1, 7, 16, 63, 64, 65, 1000] {
            let mut buf = vec![u32::MAX; len];
            unsafe { fill_chunk::<u32, 16, _>(&mut buf, counter_batches()) };
            check_fill(&buf, len);
        }
    }

    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn fill_chunk_nt_writes_every_element() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }
        for len in [0usize, 1, 7, 16, 63, 64, 65, 1000] {
            let mut buf = vec![u32::MAX; len];
            unsafe { fill_chunk_nt::<u32, 16, _>(&mut buf, counter_batches()) };
            check_fill(&buf, len);
        }
    }

    /// Batches smaller than one 32-byte store must fall back to cached
    /// stores instead of silently dropping every store (regression test:
    /// sfc32x4 once produced 4-element batches and wrote nothing).
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    #[test]
    fn fill_chunk_nt_small_batch_falls_back() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            return;
        }
        let mut buf = vec![u32::MAX; 100];
        unsafe { fill_chunk_nt::<u32, 4, _>(&mut buf, counter_batches()) };
        check_fill(&buf, 100);
    }
}

/// Dispatches to an AVX-512 optimized path on x86_64 when available, otherwise falls back.
///
/// Two forms:
/// - `dispatch_simd!(RetType, fallback_fn, avx512_fn, seed)` — allocate and return a raw pointer.
/// - `dispatch_simd!(Avx512T, FallbackT, fallback_fn, avx512_fn, ptr [, args])` — operate in-place.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
macro_rules! dispatch_simd {
    ($ret_type:ty, $fallback_fn:ident, $avx512_fn:ident, $seed:expr) => {{
        #[cfg(target_arch = "x86_64")]
        if std::arch::is_x86_feature_detected!("avx512f") {
            return $avx512_fn($seed) as *mut $ret_type;
        }
        $fallback_fn($seed) as *mut $ret_type
    }};
    ($avx512_type:ty, $fallback_type:ty, $fallback_fn:ident, $avx512_fn:ident, $ptr:expr $(, $arg:expr)*) => {{
        #[cfg(target_arch = "x86_64")]
        if std::arch::is_x86_feature_detected!("avx512f") {
            $avx512_fn($ptr as *mut $avx512_type $(, $arg)*);
            return;
        }
        $fallback_fn($ptr as *mut $fallback_type $(, $arg)*);
    }};
}
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
pub(crate) use dispatch_simd;

macro_rules! safe_test {
    ($($name:ident),+) => {
        $(pastey::paste! {
            #[test]
            fn [<test_ $name:snake>]() {
                let mut rng1 = $name::new(0);
                let mut rng2 = $name::new(0);
                assert_eq!(rng1.nextu(), rng2.nextu());
                assert_eq!(rng1.nextf(), rng2.nextf());
            }
        })+
    };
}
pub(crate) use safe_test;


/// Implements `Default` and [`crate::Seed`] for one or more `seed: u32`-constructed
/// RNGs. `Default` seeds via [`crate::_internal::default_seed32`] (a time-based,
/// per-call mix); `Seed::from_seed` forwards a raw `u32` to `new`.
macro_rules! impl_default_from_seed32 {
    ($($type:ty),* $(,)?) => {
        $(
            impl Default for $type {
                #[inline]
                fn default() -> Self {
                    Self::new($crate::_internal::default_seed32())
                }
            }

            impl $crate::Seed for $type {
                type Seed = u32;
                #[inline]
                fn from_seed(seed: u32) -> Self {
                    Self::new(seed)
                }
            }
        )*
    };
}
pub(crate) use impl_default_from_seed32;

/// Implements `Default` and [`crate::Seed`] for one or more `seed: u64`-constructed
/// RNGs. `Default` seeds via [`crate::_internal::default_seed64`] (a time-based,
/// per-call mix); `Seed::from_seed` forwards a raw `u64` to `new`.
macro_rules! impl_default_from_seed64 {
    ($($type:ty),* $(,)?) => {
        $(
            impl Default for $type {
                #[inline]
                fn default() -> Self {
                    Self::new($crate::_internal::default_seed64())
                }
            }

            impl $crate::Seed for $type {
                type Seed = u64;
                #[inline]
                fn from_seed(seed: u64) -> Self {
                    Self::new(seed)
                }
            }
        )*
    };
}
pub(crate) use impl_default_from_seed64;

/// `rand_core` integration for generators with a `u32` word and a `u32` seed
/// (REVIEW B17).
///
/// - `from_seed` reads the 4-byte seed little-endian (it used `from_ne_bytes`,
///   so results depended on the host byte order) and calls `new`.
/// - `next_u64` puts the first word in the low half, matching
///   `rand_core::utils::next_u64_via_u32` and the little-endian byte order of
///   `fill_bytes` (the first word used to be the high half).
#[cfg(feature = "rand")]
macro_rules! impl_rand32 {
    ($($type:ty),* $(,)?) => {
        $(
            impl rand_core::TryRng for $type {
                type Error = std::convert::Infallible;

                #[inline]
                fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
                    Ok($crate::rng::Rng::nextu(self))
                }

                #[inline]
                fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
                    let lo = $crate::rng::Rng::nextu(self) as u64;
                    let hi = $crate::rng::Rng::nextu(self) as u64;
                    Ok((hi << 32) | lo)
                }

                fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
                    for chunk in dst.chunks_mut(4) {
                        let v = $crate::rng::Rng::nextu(self).to_le_bytes();
                        chunk.copy_from_slice(&v[..chunk.len()]);
                    }
                    Ok(())
                }
            }

            impl rand_core::SeedableRng for $type {
                type Seed = [u8; 4];

                fn from_seed(seed: Self::Seed) -> Self {
                    Self::new(u32::from_le_bytes(seed))
                }
            }
        )*
    };
}
#[cfg(feature = "rand")]
pub(crate) use impl_rand32;

/// `rand_core` integration for generators with a `u64` word and a `u64` seed
/// (REVIEW B17: these had no `TryRng` impl at all, and their `Seed` was 4
/// bytes, so only 2^32 of their 2^64 seeds were reachable).
///
/// - `from_seed` reads the 8-byte seed little-endian and calls `new`.
/// - `next_u32` returns the high half of a word (the better-mixed bits for
///   the xorshift-family outputs).
/// - `fill_bytes` writes words little-endian, consistent with `next_u64`.
#[cfg(feature = "rand")]
macro_rules! impl_rand64 {
    ($($type:ty),* $(,)?) => {
        $(
            impl rand_core::TryRng for $type {
                type Error = std::convert::Infallible;

                #[inline]
                fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
                    Ok(($crate::rng::Rng::nextu(self) >> 32) as u32)
                }

                #[inline]
                fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
                    Ok($crate::rng::Rng::nextu(self))
                }

                fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
                    for chunk in dst.chunks_mut(8) {
                        let v = $crate::rng::Rng::nextu(self).to_le_bytes();
                        chunk.copy_from_slice(&v[..chunk.len()]);
                    }
                    Ok(())
                }
            }

            impl rand_core::SeedableRng for $type {
                type Seed = [u8; 8];

                fn from_seed(seed: Self::Seed) -> Self {
                    Self::new(u64::from_le_bytes(seed))
                }
            }
        )*
    };
}
#[cfg(feature = "rand")]
pub(crate) use impl_rand64;