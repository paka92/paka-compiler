//! A binary's function examples (M7-11b, docs/decisions/M7-03.md 1.1): started with
//! `--paka-examples NAME`, it runs the one named at depth 1, with no app, no state and no effects,
//! and prints `NAME pass` or `NAME fail`; a trap ends the run with its line.

use crate::abi::heap::fresh;
use crate::abi::{Example, Str, take_trap};
use crate::rt::{Fd1, INTERNAL_ERROR, STOPPED, on_runner};
use std::ffi::OsString;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

const SWITCH: &str = "--paka-examples";
const USAGE: i32 = 2;

/// Runs the example the process's arguments name, when they start with the switch, and writes
/// its line; the status, or -1 when the app runs instead. `names` are the examples built in,
/// space-separated, `call` running each by its position.
pub fn run(app: Str, names: Str, call: Option<Example>) -> i32 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let text = |s: Str| s.bytes().and_then(|b| str::from_utf8(b).ok());
    let (app, names) = (text(app).unwrap_or("?"), text(names).unwrap_or(""));
    let Some((status, line)) = outcome(app, names, call, &args) else {
        return -1;
    };
    let _ = match status {
        0 => Fd1::default().write_all(line.as_bytes()),
        _ => io::stderr().write_all(line.as_bytes()),
    };
    status
}

/// The status and line of the run `args` ask for; `None` without the switch.
pub(crate) fn outcome(
    app: &str,
    names: &str,
    call: Option<Example>,
    args: &[OsString],
) -> Option<(i32, String)> {
    if args.first().is_none_or(|a| a != SWITCH) {
        return None;
    }
    let usage = |why: String| Some((USAGE, format!("{app}: {why}\n")));
    let name = match args {
        [_, name] if !name.is_empty() => name.to_string_lossy(),
        _ => return usage(format!("{SWITCH} takes one example name")),
    };
    let Some(id) = names.split(' ').position(|n| n == name) else {
        return usage(format!("{SWITCH}: no function example named {name}"));
    };
    let internal = |why| Some((INTERNAL_ERROR, format!("{app}: internal error: {why}\n")));
    let Some(call) = call else {
        return internal("examples call is null");
    };
    let mut held = 0;
    let example = || fresh(|| (call(id as u64, &mut held), take_trap()));
    let Ok(((status, trap), heap, false)) = catch_unwind(AssertUnwindSafe(|| on_runner(example)))
    else {
        return internal("runtime panicked");
    };
    match (status, trap) {
        (0, _) if !(heap.leaks().is_empty() && heap.faults().is_empty()) => {
            internal("memory ledger not clean")
        }
        (0, _) => {
            let verdict = if held & 1 == 1 { "pass" } else { "fail" };
            Some((0, format!("{name} {verdict}\n")))
        }
        (1, Some((kind, text))) => {
            let [kind, text] = [kind, text].map(String::from_utf8_lossy);
            Some((STOPPED, format!("{app}: trap {kind}{text}\n")))
        }
        _ => internal("example returned a non-zero status"),
    }
}

#[cfg(test)]
mod tests;
