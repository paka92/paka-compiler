//! The run loop of an app in release (ABI sections 13 and 14): the dev run loop, its calls going
//! to compiled code through the app's `call`, on the release heap.

use crate::abi::dev::app::{self, Call, Stopped, serve};
use crate::abi::dev::windowless::{Code, Windowless};
use crate::abi::dev::{Context, MEMORY, Trap};
use crate::abi::heap::lend;
use crate::abi::shown::Shapes;
use crate::abi::{
    ANSWER, App, HANDLER, NO_START, STATE, Trapped, Typed, VIEW, peek_trap, take_trap,
};
use crate::effects::{Dropped, End, Grant, Host, World, arguments};
use crate::foreign::{self, Function};
use crate::rt::{Fd1, INTERNAL_ERROR, STOPPED, on_runner};
use paka_gui::window::Window;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::ptr;
use std::sync::mpsc;

/// How a visual app is run; only the visual entry passes it, so a headless binary links no window.
pub type Draw =
    fn(&mut Context, &app::App, &[String], &mut Call, &str, bool) -> Result<End, Stopped>;

/// Runs `app` with the process's arguments, writing its stop line, if any, to standard error; the
/// process status. `draw` is `Some` exactly for a visual app.
pub fn run_app(app: &App, draw: Option<Draw>) -> i32 {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (status, line) = match app.name.bytes().map(str::from_utf8) {
        Some(Ok(name)) => catch_unwind(AssertUnwindSafe(|| served(name, app, &args, draw)))
            .unwrap_or_else(|_| internal(name, "runtime panicked")),
        _ => internal("?", "bad app name"),
    };
    let _ = io::stderr().write_all(line.as_bytes());
    status.into()
}

fn served(name: &str, app: &App, args: &[OsString], draw: Option<Draw>) -> End {
    if (app.visual != 0) != draw.is_some() {
        return internal(name, "wrong entry for the app");
    }
    let Some(grants) = grants(app.grants) else {
        return internal(name, "bad grants");
    };
    let Some(call) = app.call else {
        return internal(name, "call is null");
    };
    let Some(Ok(fields)) = app.fields.bytes().map(str::from_utf8) else {
        return internal(name, "bad field names");
    };
    let Some(Ok(types)) = app.types.bytes().map(str::from_utf8) else {
        return internal(name, "bad types");
    };
    let Some(table) = app
        .foreign
        .bytes()
        .and_then(|t| foreign::functions(str::from_utf8(t).ok()?))
    else {
        return internal(name, "bad foreign table");
    };
    let fields: Vec<&str> = fields.split(", ").filter(|f| !f.is_empty()).collect();
    let replay = app.visual != 0 && args == ["--paka-replay"];
    let start = match arguments(name, &fields, if replay { &[] } else { args }) {
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
        start: (app.start != NO_START).then_some(app.start),
        message: typed(&app.message),
        effects: (!app.effects.is_null()).then(|| app.effects.expose_provenance() as u64),
        view: (app.visual != 0).then_some(VIEW as usize),
        foreign: &table,
        host: &beside,
        answers: [0, 1, 2].map(|kind| Some((ANSWER - kind) as usize)),
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
        let served = match draw {
            Some(draw) => draw(&mut ctx, &looped, &start, &mut call, types, replay),
            None => serve(
                &mut ctx,
                &looped,
                &start,
                &mut call,
                &mut Host(Fd1::default()),
            ),
        };
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

/// The host of `f`'s declaration, which a released program carries beside its binary: the
/// binary's file name, a dot and the declaration's name.
fn beside(f: &Function) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut name = exe.file_name()?.to_os_string();
    name.push(format!(".{}", f.grant));
    Some(exe.with_file_name(name))
}

/// Runs a visual app (ABI section 14): with `--paka-replay`, on the events of standard input;
/// else in a window; with no display, launches it and writes its first tree to standard output,
/// a trap at launch or in the first view stopping it.
pub fn drawn(
    ctx: &mut Context,
    app: &app::App,
    start: &[String],
    call: &mut Call,
    types: &str,
    replay: bool,
) -> Result<End, Stopped> {
    let shapes = Shapes::new(types);
    let mut show = |t, at| shapes.show(t, at);
    let at = |_: &Trap| peek_trap().map_or(String::new(), |t| format!("trap {}", text(t)));
    let code = Code {
        call,
        show: &mut show,
        at: &at,
    };
    if replay {
        let mut dropped = Dropped;
        let mut ui = Windowless::start(ctx, code, app, start, &mut dropped);
        let lines = io::stdin().lines().map(Result::unwrap_or_default);
        return ui.replay(lines, &mut io::stdout().lock(), |t| report(app.name, t));
    }
    let mut world = Host(Fd1::default());
    let mut ui = Windowless::start(ctx, code, app, start, &mut world);
    if let Some(end) = windowed(app.name, &mut ui) {
        return end;
    }
    let (end, trap) = (ui.end.clone(), ui.traps.last().map(|t| t.kind));
    let tree = ui.drawing.root.to_string();
    drop(ui);
    match (end, trap) {
        (Some(end), _) => end,
        (None, Some(kind)) => {
            ctx.trap.kind = kind;
            Err(Stopped::Trap)
        }
        (None, None) => {
            let _ = io::stdout().write_all(tree.as_bytes());
            Ok((0, String::new()))
        }
    }
}

/// Runs `ui` in a window until the app ends or the window closes, which ends it with status 0;
/// `None` with no display.
fn windowed<W: World>(name: &str, ui: &mut Windowless<W>) -> Option<Result<End, Stopped>> {
    let (input, events) = mpsc::channel();
    let closer = input.clone();
    let on_close = move || drop(closer.send(None));
    let on_input = move |event| drop(input.send(Some(event)));
    let window = Window::open(name, &ui.drawing.root, on_close, on_input)?;
    let mut seen = reported(name, ui, 0);
    while ui.end.is_none() {
        let Ok(Some(event)) = events.recv() else {
            return Some(Ok((0, String::new())));
        };
        ui.send([event]);
        seen = reported(name, ui, seen);
        window.show(&ui.drawing.root);
    }
    ui.end.clone()
}

/// Writes the line of the last trap `ui` kept past its first `seen` to standard error; the count
/// it keeps now.
fn reported<W>(name: &str, ui: &Windowless<W>, seen: usize) -> usize {
    if let Some(trap) = ui.traps.get(seen..).and_then(<[Trap]>::last) {
        report(name, trap);
    }
    ui.traps.len()
}

/// Writes the line of `trap` to standard error.
fn report(name: &str, trap: &Trap) {
    let (_, line) = trapped(name, take_trap(), trap.kind);
    let _ = io::stderr().write_all(line.as_bytes());
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
        Some(trap) => text(trap),
        None if kind == MEMORY => "memory".into(),
        None => return internal(app, "a call returned a non-zero status"),
    };
    (STOPPED as u8, format!("{app}: trap {text}\n"))
}

/// A trap's report after `trap `: its kind's words, then its text.
fn text((kind, text): Trapped) -> String {
    [kind, text].map(String::from_utf8_lossy).concat()
}

fn internal(app: &str, why: &str) -> End {
    (
        INTERNAL_ERROR as u8,
        format!("{app}: internal error: {why}\n"),
    )
}
