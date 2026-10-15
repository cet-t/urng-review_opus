//! C-compatible ABI wrappers for the RNGs.
//!
//! # Calling convention
//!
//! - `*_new(seed)` returns an owned handle, or NULL for a SIMD type whose CPU
//!   features (AVX2 / AVX-512F / AVX-512DQ) are not available at run time.
//!   Release it with the matching `*_free`, which accepts NULL.
//! - Every function taking a handle is `unsafe` for Rust callers. A NULL
//!   handle, a NULL output buffer or `count == 0` writes nothing. Otherwise
//!   `out` must be valid for `count` writes of its element type and must not
//!   be accessed concurrently; any alignment of the element type is accepted
//!   and the values do not depend on it.
//! - `*_rand_i32s` / `*_rand_i64s` with `min > max` write nothing and leave the
//!   handle unchanged. `*_rand_f32s` / `*_rand_f64s` return values in
//!   `[min, max)` when `min < max`.
//! - The bulk functions are not reentrant on one handle (they mutate it).
//!
//! # What a bulk call returns (REVIEW B14)
//!
//! - **Sequential stream.** For the counter-based generators (`philox32`,
//!   `philox64`, `threefry32x4`, `threefry32x2`, `threefish256`, `squares32`), the
//!   generators with a jump-ahead (`splitmix32`, `splitmix64`, `pcg32`,
//!   `pcg32x8`, `splitmix32x16`) and the sequentially filled ones (`xorshift*`,
//!   `twisted_gfsr`), a bulk call returns exactly what the same number of
//!   `nextu` calls on the Rust type would return, so bulk and single draws can
//!   be interleaved. `philox32x4x4` and `squares32x8` return whole 16- / 8-word
//!   `nextu` blocks and discard the unused tail of the last one. `*_rand_i32s` maps one word per element
//!   (multiply-shift; each value's probability is within `range / 2^32` of
//!   uniform) for these generators.
//! - **Substreams.** The other generators have no cheap jump-ahead, so a
//!   bulk call splits the buffer into chunks and runs an independent generator
//!   per chunk, seeded over its whole state from a 64-bit key drawn from the
//!   handle and the chunk index. The result is deterministic for a given seed
//!   and call sequence but is *not* the handle's `nextu` stream. Each call
//!   draws a fresh key, so successive calls do not repeat each other's output.

#![allow(ambiguous_glob_reexports)]

pub mod cbrng;
pub mod prng;

pub use cbrng::*;
pub use prng::*;
