//! The Paka runtime of milestone 1, written to `docs/abi/m1.md`.
//!
//! Unsafe code lives in `abi` alone. `rt` forbids it, and a forbid cannot be lifted by an
//! `allow` further in.

#![deny(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::float_arithmetic)]

#[allow(unsafe_code)]
pub mod abi;
#[forbid(unsafe_code)]
pub mod ops;
#[forbid(unsafe_code)]
mod rt;

pub use rt::*;
