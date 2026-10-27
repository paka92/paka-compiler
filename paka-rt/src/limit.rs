//! A run's time limit (docs/decisions/M4-04.md): a watcher thread sets the run's interrupt once
//! the limit passes, and the run polls it.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// Runs `f`, calling `interrupt` from a watcher thread once `limit` passes; and whether it did.
pub fn until<R>(
    limit: Duration,
    interrupt: impl FnOnce() + Send,
    f: impl FnOnce() -> R,
) -> (R, bool) {
    let (done, wait) = mpsc::channel::<()>();
    thread::scope(|s| {
        let watcher = s.spawn(move || {
            let late = wait.recv_timeout(limit) == Err(RecvTimeoutError::Timeout);
            if late {
                interrupt();
            }
            late
        });
        let r = f();
        drop(done);
        (r, watcher.join().expect("the watcher does not panic"))
    })
}
