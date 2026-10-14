//! Tools for the M3 experiment, apart from the compiler and using only the standard library.
//!
//! - `client`: one `claude -p` process per run; send a turn, get the reply and its token counts.
//! - `usage`: the token caps and the running total kept in a file. For the use of one batch, the
//!   trial loop takes `Ledger::used()` before and after it.
//! - `json`: a strict JSON reader.

pub mod client;
mod error;
pub mod json;
mod stream;
pub mod usage;

pub use error::Error;
