//! Cipher-based random number generators.
//!
//! ChaCha (RFC 8439 block function) and Salsa20 keystreams consumed as random
//! output. The `new(seed: u32)` constructors expand a 32-bit seed and are meant
//! for reproducible, high-quality *non-cryptographic* streams; for secret
//! keystreams use `from_key` with a 256-bit key from a secure source.

#![allow(ambiguous_glob_reexports)]

pub(crate) mod b32;

pub use b32::*;
