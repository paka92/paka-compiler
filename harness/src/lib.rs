//! Tools for the M3 experiment, apart from the compiler and using only the standard library.
//!
//! - `trial`: the trial loop; runs a batch of runs with raw logs and resumes after a kill.
//! - `client`: one `claude -p` process per run; send a turn, get the reply and its token counts,
//!   killed by a watchdog past its time limits.
//! - `prompt`: the system text, the first message and the feedback a run sends.
//! - `log`: a run's append-only raw log.
//! - `usage`: the token caps and the running total kept in a file.
//! - `metrics`: raw logs and a rulings file to the three metrics and the verdict.
//! - `json`: a strict JSON reader.
//! - `task`: loads a task folder (statement, visible cases, hidden cases).
//! - `runner`: builds a submission and judges it on a task's cases.

pub mod client;
mod error;
pub mod json;
pub mod log;
pub mod metrics;
pub mod prompt;
pub mod runner;
mod stream;
pub mod task;
pub mod trial;
pub mod usage;

pub use error::Error;
