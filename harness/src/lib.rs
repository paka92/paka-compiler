//! The experiment harness. Standard library only, and apart from the compiler.
//!
//! - `client`: one `claude -p` process per run; send a turn, get the reply and its token counts.
//! - `usage`: the token caps and the running total kept in a file.
//! - `json`: a strict JSON reader.

pub mod client;
mod error;
pub mod json;
mod stream;
pub mod usage;

pub use error::Error;
