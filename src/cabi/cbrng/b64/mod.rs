//! C-compatible ABI wrappers for the 64-bit counter-based RNGs.


pub(crate) mod philox;
pub(crate) mod threefish;

pub use philox::*;
pub use threefish::*;
