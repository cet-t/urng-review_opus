[<img alt="crates.io" src="https://img.shields.io/crates/v/urng.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/urng)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-urng-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/urng)
[<img alt="github" src="https://img.shields.io/badge/github-cet--t%2Furng-5f5fff?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/cet-t/urng)

# Universal RNG

A collection of efficient pseudo-random number generators (PRNGs) implemented in pure Rust.
This crate provides a wide variety of algorithms, ranging from standard Mersenne Twister to modern high-performance generators like Xoshiro and Philox.

## Supported Generators

Generators are divided into standard generators, portable wide generators, and AVX-accelerated SIMD generators.
Standard generators implement the unified `Rng` trait (`Word = u32` or `Word = u64`; SIMD/counter-based variants return fixed-size arrays instead).
Portable wide generators require the `wide` feature and expose safe fixed-array bulk APIs.
AVX generators expose a bulk-generation API and are listed separately; they require the `simd` feature.

### 32-bit Generators (`urng::`)

| Struct          | Algorithm        | Period / State   |
| --------------- | ---------------- | ---------------- |
| `Mt19937`       | Mersenne Twister | $2^{19937}-1$    |
| `Sfmt607`       | SFMT             | $2^{607}-1$      |
| `Sfmt1279`      | SFMT             | $2^{1279}-1$     |
| `Sfmt2281`      | SFMT             | $2^{2281}-1$     |
| `Sfmt4253`      | SFMT             | $2^{4253}-1$     |
| `Sfmt11213`     | SFMT             | $2^{11213}-1$    |
| `Sfmt19937`     | SFMT             | $2^{19937}-1$    |
| `Sfmt44497`     | SFMT             | $2^{44497}-1$    |
| `Sfmt86243`     | SFMT             | $2^{86243}-1$    |
| `Sfmt132049`    | SFMT             | $2^{132049}-1$   |
| `Sfmt216091`    | SFMT             | $2^{216091}-1$   |
| `Sfc32`         | SFC32            | $2^{127}-1$      |
| `Pcg32`         | PCG-XSH-RR       | $2^{64}$         |
| `Philox32`      | Philox 4x32-10   | -                |
| `SplitMix32`    | SplitMix32       | $2^{32}$         |
| `Xorwow`        | XORWOW           | $2^{192}-2^{32}$ |
| `Xorshift32`    | Xorshift         | $2^{32}-1$       |
| `Xorshift128`   | Xorshift128      | $2^{128}-1$      |
| `Xoshiro128Pp`  | xoshiro128++     | $2^{128}-1$      |
| `Xoshiro128Ss`  | xoshiro128\*\*   | $2^{128}-1$      |
| `Xoroshiro64Ss` | xoroshiro64\*\*  | $2^{64}-1$       |
| `Threefry32x4`  | Threefry 4x32-20 | -                |
| `Threefry32x2`  | Threefry 2x32-20 | -                |
| `Squares32`     | Squares          | -                |
| `Jsf32`         | JSF32            | -                |

### 64-bit Generators (`urng::`)

| Struct           | Algorithm           | Period / State   |
| ---------------- | ------------------- | ---------------- |
| `Xoshiro256Pp`   | xoshiro256++        | $2^{256}-1$      |
| `Xoshiro256Ss`   | xoshiro256\*\*      | $2^{256}-1$      |
| `SplitMix64`     | SplitMix64          | $2^{64}$         |
| `Sfc64`          | SFC64               | $2^{256}$ approx |
| `Mt1993764`      | Mersenne Twister 64 | $2^{19937}-1$    |
| `Sfmt1993764`    | SFMT 64             | $2^{19937}-1$    |
| `Philox64`       | Philox 2x64-10      | -                |
| `Philox4x64`     | Philox 4x64-10      | -                |
| `Xorshift64`     | Xorshift64          | $2^{64}-1$       |
| `Xoroshiro128Pp` | xoroshiro128++      | $2^{128}-1$      |
| `Xoroshiro128Ss` | xoroshiro128\*\*    | $2^{128}-1$      |
| `TwistedGFSR`    | TGFSR               | $2^{800}$ approx |
| `Cet64`          | CET                 | $2^{64}$         |
| `Cet256`         | CET                 | $2^{256}$        |
| `Threefish256`   | Threefish-256       | -                |
| `Biski64`        | Biski64             | $2^{64}$         |

### Portable Wide Generators (`urng::wide`)

> Requires the `wide` feature.

These generators use the portable `wide` crate and expose safe `nextu`, `nextf`, `randi`, and `randf` methods returning fixed-size arrays.

| Struct family     | Algorithm       | Output variants |
| ----------------- | --------------- | --------------- |
| `SplitMix32x*`    | SplitMix32      | 4×/8×/16×`u32`  |
| `Jsf32x*`         | JSF32           | 4×/8×/16×`u32`  |
| `Pcg32x*`         | PCG-XSH-RR      | 4×/8×/16×`u32`  |
| `Sfc32x*`         | SFC32           | 4×/8×/16×`u32`  |
| `Xoroshiro64Ssx*` | xoroshiro64\*\* | 4×/8×/16×`u32`  |
| `Xorshift32x*`    | Xorshift32      | 4×/8×/16×`u32`  |
| `Xorshift128x*`   | Xorshift128     | 4×/8×/16×`u32`  |
| `Xorwowx*`        | XORWOW          | 4×/8×/16×`u32`  |
| `Xoshiro128Ppx*`  | xoshiro128++    | 4×/8×/16×`u32`  |
| `Xoshiro128Ssx*`  | xoshiro128\*\*  | 4×/8×/16×`u32`  |

Example:

```rust,ignore
// requires the `wide` feature (doctested in `urng::wide`)
use urng::wide::{RngW, Xoshiro128Ppx16};

let mut rng = Xoshiro128Ppx16::new(1);
let values: [u32; 16] = rng.nextu();
let floats: [f32; 16] = rng.nextf();
```

### SIMD Generators (x86_64)

> Requires the `simd` feature; available on `x86_64` only (the feature compiles to nothing elsewhere).

These generators expose a bulk-generation API. Every lane is an ordinary
scalar stream (tested lane by lane against the scalar generators). Types that
need instructions beyond the x86_64 baseline have `unsafe` / `#[target_feature]`
constructors: check the CPU first (`is_x86_feature_detected!`). The C ABI
`*_new` functions do this and return NULL when the features are missing.

#### SSE2 (always available on x86_64; safe API)

| Struct           | Algorithm         | Output  |
| ---------------- | ----------------- | ------- |
| `Sfc32x4`        | SFC32 x4          | 4×`u32` |
| `Xoshiro256Ssx2` | xoshiro256\*\* x2 | 2×`u64` |

#### AVX2 (`avx2`)

| Struct            | Algorithm          | Output  |
| ----------------- | ------------------ | ------- |
| `Sfc32x8`         | SFC32 x8           | 8×`u32` |
| `Jsf32x8`         | JSF32 x8           | 8×`u32` |
| `Xoroshiro64Ssx8` | xoroshiro64\*\* x8 | 8×`u32` |
| `Xorshift32x8`    | Xorshift32 x8      | 8×`u32` |

#### AVX-512 (`avx512f`, plus `avx512dq` where noted)

| Struct             | Algorithm            | Output   |
| ------------------ | -------------------- | -------- |
| `Pcg32x8`          | PCG-XSH-RR x8        | 8×`u32`  |
| `Philox32x4x4`     | Philox 4x32-10 x4    | 16×`u32` |
| `SplitMix32x16`    | SplitMix32 x16       | 16×`u32` |
| `Squares32x8`      | Squares x8 (+DQ)     | 8×`u32`  |
| `Xoshiro128Ppx16`  | xoshiro128++ x16     | 16×`u32` |
| `Xoshiro128Ssx16`  | xoshiro128\*\* x16   | 16×`u32` |
| `Jsf32x16`         | JSF32 x16            | 16×`u32` |
| `Sfc32x16`         | SFC32 x16            | 16×`u32` |
| `Xoroshiro64Ssx16` | xoroshiro64\*\* x16  | 16×`u32` |
| `Sfc64x8`          | SFC64 x8             | 8×`u64`  |
| `Cet64x8`          | CET64 x8 (+DQ)       | 8×`u64`  |
| `Cet256x2`         | CET256 x2 (+DQ)      | 2×`u64`  |
| `Biski64x8`        | Biski64 x8           | 8×`u64`  |

## Sampler

> Requires the `sampler` feature.

Weighted random index selection. Two implementations are provided for each bit-width, both implementing the `Sampler` trait (`urng::Sampler`).

| Struct    | Module   | Algorithm      | Build | Sample   |
| --------- | -------- | -------------- | ----- | -------- |
| `Bst32`   | `urng::` | Cumulative BST | O(n)  | O(log n) |
| `Alias32` | `urng::` | Walker's Alias | O(n)  | O(1)     |
| `Bst64`   | `urng::` | Cumulative BST | O(n)  | O(log n) |
| `Alias64` | `urng::` | Walker's Alias | O(n)  | O(1)     |

## SeedGen

> Requires the `seedgen` feature.

Hardware-noise-assisted seed generation. Wraps an existing `Rng` and mixes in hardware noise (RDSEED/RDRAND on x86/x86_64, timestamp fallback elsewhere) via a Murmur3-style hash.

| Struct    | Module          | Input RNG                             | Output                           |
| --------- | --------------- | ------------------------------------- | -------------------------------- |
| `SeedGen` | `urng::seedgen` | `Rng<Word = u32>` / `Rng<Word = u64>` | `(u32, u32)` / `(u64, u64)` pair |

`next_seed_pair()` returns `(raw, processed)` — the raw hardware value and the mixed seed.

## Testing

Statistical validation of RNG quality is handled by the external [`cribler`](https://crates.io/crates/cribler) crate, a batteries-included randomness-test toolkit. `cribler` ships every engine (chi-squared, Monte Carlo π, serial correlation, runs, Kolmogorov–Smirnov, birthday spacing, a NIST SP 800-22 subset, and a paranoid battery aggregator) and offers zero-feature integration: any generator plugs in via a plain `FnMut() -> f64` / `FnMut() -> u64` sampler closure.

Enable the `urng` feature on `cribler` for pre-built typed convenience that works directly against `urng`'s `Rng` generators:

```toml
[dependencies]
urng = "1.0.0"
cribler = { version = "0.3", features = ["urng"] }
```

The suites construct each named case from a seed, so no generator instance needs to be passed in:

```rust,ignore
use cribler::Suite;
use urng::Rng;

let results = cribler::Suite::default()
    .from_urng32::<urng::Sfc32>()?
    .from_rand::<rand_sfc::Sfc32>()?
    .run()?;

for r in results.iter() {
    println!("{}", serde_json::to_string_pretty(&r)?);
}
```

`from_urng32::<R>()` / `from_urng64::<R>()` register a `urng::Rng<Word = u32>` / `urng::Rng<Word = u64>` `R`, built from the suite's seed. The `rand` feature provides `from_rand::<R>()` for `rand_core::Rng` types, and `from_custom(source)` accepts anything else via a `WordSource` adapter.

## Usage Examples

Most generators expose the same basic workflow: create an instance with `new`, then use `nextu`, `nextf`, `randi`, `randf`, or `choice` depending on the output type you need. SIMD and counter-based generators return fixed-size arrays instead of single values.

Scalar generators (both 32-bit and 64-bit; SIMD variants are not included) also implement `Default`, seeding themselves from a time-based, per-call mix so no explicit seed is required:

```rust
use urng::*;

let mut rng = Sfc32::default();
let _ = Rng::nextu(&mut rng);
```

### Basic Usage

```rust
use urng::*;

fn main() {
    // 1. Initialize with a seed
    let mut rng = Xoshiro256Pp::new(12345);

    // 2. Generate random numbers
    let val_u64 = rng.nextu();
    println!("u64: {}", val_u64);

    let val_f64 = rng.nextf(); // [0.0, 1.0)
    println!("f64: {}", val_f64);

    // 3. Generate within a range
    let val_range = rng.randi(1, 100);
    println!("Integer (1-100): {}", val_range);

    // 4. Seeding with SplitMix64 (common pattern)
    // If you need to seed a large state generator from a single u64
    let mut sm = SplitMix64::new(9999);
    let seed_val = sm.nextu();
    let mut rng2 = Xoshiro256Pp::new(seed_val);
}
```

## C ABI

This crate exports a C-compatible ABI generic interface. Each generator has corresponding:

- `_new`
- `_free`
- `_next_uXXs` (bulk generation)
- `_next_fXXs` (bulk generation)
- `_rand_iXXs` (bulk generation)
- `_rand_fXXs` (bulk generation)

Example for `Mt19937`:

```c
void* mt19937_new(uint32_t seed);
void mt19937_next_u32s(void* ptr, uint32_t* out, size_t count);
void mt19937_next_f32s(void* ptr, float* out, size_t count);
void mt19937_rand_i32s(void* ptr, int32_t* out, size_t count, int32_t min, int32_t max);
void mt19937_rand_f32s(void* ptr, float* out, size_t count, float min, float max);
void mt19937_free(void* ptr);
```

Conventions (see the `cabi` module documentation for details):

- A NULL handle, a NULL buffer or `count == 0` writes nothing; any element
  alignment of `out` is accepted.
- `*_rand_i*s` writes nothing when `min > max`; `*_rand_f*s` returns values in
  `[min, max)`.
- SIMD `*_new` functions return NULL when the CPU lacks the required features.
- Counter-based generators and generators with a jump-ahead (`philox*`,
  `threefry*`, `threefish256`, `squares32`, `splitmix*`, `pcg32*`) and the
  sequential ones (`xorshift*`, `twisted_gfsr`) return exactly the Rust
  `nextu` stream. The others fill large buffers in parallel from
  independently seeded substreams (deterministic per seed, but not the
  `nextu` stream).

[SeedableRng]: https://docs.rs/rand_core/latest/rand_core/trait.SeedableRng.html
[TryRng]: https://docs.rs/rand_core/latest/rand_core/trait.TryRng.html
[try_fill_bytes]: https://docs.rs/rand_core/latest/rand_core/trait.TryRng.html#tymethod.try_fill_bytes
