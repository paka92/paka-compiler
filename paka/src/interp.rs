//! The reference interpreter: runs the core form (docs/core.md) and performs the effects the
//! handler returns (spec/m1.md section 9, M2-15).

use crate::core_form::{App, Field, Program, Type};
use crate::core_form::{Fn as Func, Site};
use effects::{End, Host, Queued, World, perform};
use eval::{Done, Kind, Machine, Stop, Trap};
use show::show;
use std::any::Any;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::Write;
use std::sync::Once;
use std::{panic, thread};
use value::{Items, Value};

mod builtin;
mod effects;
mod eval;
mod generic;
mod number;
mod show;
#[cfg(test)]
mod tests;
mod text;
mod value;
#[cfg(test)]
mod world_tests;

/// Host stack of the run thread and of every segment the evaluation adds to it (`eval::BUDGET`
/// levels each); the pages are only committed when used.
const STACK: usize = 32 << 20;
/// The longest detail line of a trap report, in bytes.
const DETAIL: usize = 256;
/// The name of the thread that runs the program; its panics are reported as an internal error.
const RUN_THREAD: &str = "paka-run";
/// The most frames a trap report names.
const NAMED: usize = 8;

/// Runs the program with the command-line arguments after its file, `out` being descriptor 1 and
/// file paths relative to the working directory; returns its status.
pub fn run(
    core: &Program,
    args: &[OsString],
    out: &mut (impl Write + Send),
    err: &mut impl Write,
) -> u8 {
    run_in(core, args, &mut Host(out), err)
}

/// The same in any world.
pub fn run_in(
    core: &Program,
    args: &[OsString],
    world: &mut (impl World + Send),
    err: &mut impl Write,
) -> u8 {
    let app = core.app.as_ref().expect("a program with an app");
    let (status, line) = match start_message(core, app, args) {
        Ok(start) => serve(core, app, start, world),
        Err(stop) => stop,
    };
    let _ = err.write_all(line.as_bytes());
    status
}

/// The start variant with its fields filled from the arguments, or the usage line.
fn start_message(core: &Program, app: &App, args: &[OsString]) -> Result<Value, End> {
    let fields = start_fields(core, app);
    let usage = |line: String| (2, format!("{}: {line}\n", app.name));
    if args.len() != fields.len() {
        let plural = if fields.len() == 1 { "" } else { "s" };
        let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        let names = if names.is_empty() {
            String::new()
        } else {
            format!(" ({})", names.join(", "))
        };
        let (want, got) = (fields.len(), args.len());
        return Err(usage(format!(
            "expected {want} argument{plural}{names}, got {got}"
        )));
    }
    let text = |(i, arg): (usize, &OsString)| match arg.to_str() {
        Some(text) => Ok(Value::Str(text.into())),
        None => Err(usage(format!("argument {} is not text", i + 1))),
    };
    let items = args
        .iter()
        .enumerate()
        .map(text)
        .collect::<Result<_, _>>()?;
    Ok(Value::Variant(app.start, Items(items)))
}

/// The message loop: the handler answers each message in turn and its effects are performed.
/// The whole loop runs on one big-stack thread.
fn serve(core: &Program, app: &App, start: Value, world: &mut (impl World + Send)) -> End {
    let run = guest(core, app, |machine| {
        let mut state = machine.state(app)?;
        let mut queue = VecDeque::from([Queued::Message(start)]);
        while let Some(next) = queue.pop_front() {
            let message = match next {
                Queued::Message(message) => message,
                Queued::Answer(f, arg) => machine.answer(f, arg)?,
            };
            let (mut effects, next) = machine.update(app, state, message)?;
            state = next;
            if let Some(end) = perform(app, &mut effects, world, &mut queue) {
                return Ok(end);
            }
        }
        Ok((3, format!("{}: ended without Exit\n", app.name)))
    });
    run.unwrap_or_else(|end| end)
}

/// Runs `f` on a machine with room for the deepest call chain; a trap or fault ends the run.
fn guest<T: Send>(
    core: &Program,
    app: &App,
    f: impl FnOnce(&mut Machine) -> Done<T> + Send,
) -> Result<T, End> {
    let internal = |reason: &str| (4, format!("{}: internal error: {reason}\n", app.name));
    match on_big_stack(|| f(&mut Machine::new(core))) {
        Ok(Ok(done)) => Ok(done),
        Ok(Err(Stop::Trap(trap))) => Err((3, report(core, &app.name, &trap))),
        Ok(Err(Stop::Internal(reason))) | Err(reason) => Err(internal(&reason)),
        Ok(Err(_)) => unreachable!("a loop holds its break"),
    }
}

/// The fields of the start message's variant.
fn start_fields<'a>(core: &'a Program, app: &App) -> &'a [Field] {
    let message = core.fns[app.update]
        .params
        .last()
        .expect("a message parameter");
    let Type::Enum(msg) = core.types[message.ty] else {
        unreachable!("the message is a user enum");
    };
    &core.enums[msg].variants[app.start].fields
}

/// Runs `f` on a thread with room for the deepest call chain; the reason if the thread cannot
/// start or panics. A panic there says nothing on its own: the caller reports its reason.
fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> Result<T, String> {
    static QUIET: Once = Once::new();
    QUIET.call_once(|| {
        let loud = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if thread::current().name() != Some(RUN_THREAD) {
                loud(info);
            }
        }));
    });
    thread::scope(|scope| {
        let host = thread::Builder::new()
            .name(RUN_THREAD.into())
            .stack_size(STACK);
        let run = host.spawn_scoped(scope, f).map_err(|e| e.to_string())?;
        run.join().map_err(|p| panic_reason(&*p))
    })
}

fn panic_reason(payload: &(dyn Any + Send)) -> String {
    let text = payload.downcast_ref::<&str>().copied();
    text.or(payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("the run failed")
        .into()
}

/// The trap report of M2-06 section 1.2.
fn report(core: &Program, app: &str, trap: &Trap) -> String {
    let kind = trap.kind.name();
    format!(
        "{app}: trap {kind} at {}\n{}",
        place(core, trap.site),
        help(trap)
    )
}

/// `file:line:col` of a site, in the file that holds it.
fn place(core: &Program, site: Site) -> String {
    let at = &core.sites[site];
    let (file, first) = (core.files.iter().rfind(|f| f.1 <= at.line)).expect("a site is in a file");
    format!("{file}:{}:{}", at.line + 1 - first, at.col)
}

/// A trap's detail line and call chain, which `assert`, `stack` and `memory` do not have.
fn help(trap: &Trap) -> String {
    if matches!(trap.kind, Kind::Assert | Kind::Stack | Kind::Memory) {
        return String::new();
    }
    let cut = trap.detail.floor_char_boundary(DETAIL);
    let named = trap.frames.iter().take(NAMED);
    let chain = named.map(String::as_str).collect::<Vec<_>>();
    let more = if trap.frames.len() > NAMED {
        ", ..."
    } else {
        ""
    };
    let chain = chain.join(", called from ");
    format!("  {}\n  in {chain}{more}\n", &trap.detail[..cut])
}

/// Runs every example in name order and prints its status (M2-06 section 1.6); the status is 0
/// when all pass, else 3.
pub fn examples(core: &Program, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let mut order: Vec<_> = core.examples.iter().collect();
    order.sort_by(|a, b| a.name.cmp(&b.name));
    let entries = on_big_stack(|| {
        order
            .iter()
            .map(|f| entry(core, f))
            .collect::<Result<Vec<_>, _>>()
    });
    match entries {
        Ok(Ok(entries)) => {
            let text: String = entries.iter().map(|(_, text)| text.as_str()).collect();
            let _ = out.write_all(text.as_bytes());
            if entries.iter().all(|(passed, _)| *passed) {
                0
            } else {
                3
            }
        }
        Ok(Err(reason)) | Err(reason) => {
            let _ = writeln!(err, "internal error: {reason}");
            4
        }
    }
}

/// Whether the example passed, and its entry; the reason if the interpreter faults.
fn entry(core: &Program, f: &Func) -> Result<(bool, String), String> {
    let name = &f.name;
    Ok(match Machine::new(core).example(f) {
        Ok(None) => (true, format!("{name} pass\n")),
        Ok(Some(fail)) => {
            let mut text = format!("{name} fail at {}\n", place(core, f.claims[fail.claim]));
            if let Some((left, right, ty)) = fail.sides {
                let (left, right) = (show(core, ty, &left), show(core, ty, &right));
                text += &format!("  left: {left}\n  right: {right}\n");
            }
            (false, text)
        }
        Err(Stop::Trap(trap)) => {
            let at = place(core, trap.site);
            let text = match trap.kind {
                Kind::Hole => format!("{name} unfinished at {at}\n{}", help(&trap)),
                kind => format!(
                    "{name} trapped at {at}\n  trap {}\n{}",
                    kind.name(),
                    help(&trap)
                ),
            };
            (false, text)
        }
        Err(Stop::Internal(reason)) => return Err(reason),
        Err(_) => unreachable!("a loop holds its break"),
    })
}
