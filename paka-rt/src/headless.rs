//! The run loop of a headless app in release (ABI section 13): the dev run loop, its calls going
//! to compiled code through the app's `call`, on the release heap.

use crate::abi::dev::app::{self, Stopped, serve};
use crate::abi::dev::{Context, MEMORY};
use crate::abi::heap::lend;
use crate::abi::{App, HANDLER, STATE, Trapped, Typed, take_trap};
use crate::effects::{End, Grant, Host, arguments};
use crate::rt::{Fd1, INTERNAL_ERROR, STOPPED, on_runner};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// Runs `app` with the process's arguments, writing its stop line, if any, to standard error; the
/// process status.
pub fn run_app(app: &App) -> i32 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (status, line) = match app.name.bytes().map(str::from_utf8) {
        Some(Ok(name)) => catch_unwind(AssertUnwindSafe(|| served(name, app, &args)))
            .unwrap_or_else(|_| internal(name, "runtime panicked")),
        _ => internal("?", "bad app name"),
    };
    let _ = io::stderr().write_all(line.as_bytes());
    status.into()
}

fn served(name: &str, app: &App, args: &[OsString]) -> End {
    let Some(grants) = grants(app.grants) else {
        return internal(name, "bad grants");
    };
    let Some(call) = app.call else {
        return internal(name, "call is null");
    };
    let Some(Ok(fields)) = app.fields.bytes().map(str::from_utf8) else {
        return internal(name, "bad field names");
    };
    let fields: Vec<&str> = fields.split(", ").filter(|f| !f.is_empty()).collect();
    let start = match arguments(name, &fields, args) {
        Ok(start) => start,
        Err(end) => return end,
    };
    let typed = |t: &Typed| (t.record.expose_provenance() as u64, t.size);
    let state = (app.stateful != 0).then(|| {
        let (record, size) = typed(&app.state);
        (STATE as usize, record, size)
    });
    let looped = app::App {
        name,
        grants: &grants,
        update: HANDLER as usize,
        state,
        start: Some(app.start),
        message: typed(&app.message),
        effects: Some(app.effects.expose_provenance() as u64),
        view: None,
    };
    on_runner(|| {
        let mut ctx = Context::new(u64::MAX);
        let mut broken = false;
        let mut call = |ctx: &mut Context, id: usize, args: &[u64]| {
            let at =
                |i: usize| ptr::with_exposed_provenance_mut(args.get(i).map_or(0, |&a| a as usize));
            let (status, failed) = lend(ctx, || call(id as u64, at(0), at(1), at(2)));
            broken |= failed;
            status
        };
        let served = serve(
            &mut ctx,
            &looped,
            &start,
            &mut call,
            &mut Host(Fd1::default()),
        );
        let clean = ctx.heap.leaks().is_empty() && ctx.heap.faults().is_empty();
        match served {
            _ if broken => internal(name, "runtime panicked"),
            Ok(_) if !clean => internal(name, "memory ledger not clean"),
            Ok(end) => end,
            Err(Stopped::Trap) => trapped(name, take_trap(), ctx.trap.kind),
            Err(Stopped::Time) => internal(name, "a call returned a non-zero status"),
        }
    })
}

/// The grants of `word`'s bits, or `None` when an unknown bit is set.
fn grants(word: u64) -> Option<BTreeSet<Grant>> {
    let all = [Grant::Console, Grant::ReadFiles, Grant::WriteFiles];
    if word >> all.len() != 0 {
        return None;
    }
    let set = all
        .into_iter()
        .enumerate()
        .filter(|(bit, _)| word >> bit & 1 == 1);
    Some(set.map(|(_, g)| g).collect())
}

/// The trap line: compiled code's or a row's report, or a `memory` trap the loop met itself.
fn trapped(app: &str, trap: Option<Trapped>, kind: u64) -> End {
    let text = match trap {
        Some((kind, text)) => [kind, text].map(String::from_utf8_lossy).concat(),
        None if kind == MEMORY => "memory".into(),
        None => return internal(app, "a call returned a non-zero status"),
    };
    (STOPPED as u8, format!("{app}: trap {text}\n"))
}

fn internal(app: &str, why: &str) -> End {
    (
        INTERNAL_ERROR as u8,
        format!("{app}: internal error: {why}\n"),
    )
}
