//! What a runtime built with the `measure` feature adds (M6-30): a banner on standard error as a
//! run starts, and as it ends the time the copy routine took. Without the feature it adds nothing.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

const ON: bool = cfg!(feature = "measure");
static NANOS: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);

/// Runs `run`, one whole entry, between the banner and the copy time.
pub(crate) fn around(run: impl FnOnce() -> i32) -> i32 {
    if !ON {
        return run();
    }
    let _ = writeln!(io::stderr(), "MEASUREMENT BUILD: not a Paka program");
    let status = run();
    let (nanos, calls) = (NANOS.load(Relaxed), CALLS.load(Relaxed));
    let line = format!("MEASUREMENT BUILD: copy routine {nanos} ns in {calls} calls");
    let _ = writeln!(io::stderr(), "{line}");
    status
}

/// Adds the time from its making to its drop to the copy routine's.
pub(crate) struct Timed(Option<Instant>);

impl Timed {
    pub(crate) fn start() -> Timed {
        Timed(ON.then(Instant::now))
    }
}

impl Drop for Timed {
    fn drop(&mut self) {
        if let Some(start) = self.0 {
            let nanos = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            NANOS.fetch_add(nanos, Relaxed);
            CALLS.fetch_add(1, Relaxed);
        }
    }
}
