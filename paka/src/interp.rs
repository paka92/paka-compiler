//! The reference interpreter: runs the core form (docs/core.md) and performs the effects the
//! handler returns (spec/m1.md section 9, M2-15).

use crate::core_form::{App, Field, FnId, Program, Type};
use crate::core_form::{Fn as Func, Site};
use crate::source::in_file;
use effects::{End, Host, Queued, World, perform};
pub use eval::Left;
use eval::{Done, Kind, Machine, Stop, Trap};
use paka_rt::route::Event;
use paka_rt::tree::Node;
use show::show;
use std::any::Any;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::Write;
use std::sync::Once;
use std::{panic, thread};
use value::{Items, Message, Value};
pub use windowless::Windowless;

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
mod view_tests;
mod windowless;
#[cfg(test)]
mod windowless_tests;
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
    let (status, line) = match start_message(core, app, headless(app), args) {
        Ok(start) => serve(core, app, start, world),
        Err(stop) => stop,
    };
    let _ = err.write_all(line.as_bytes());
    status
}

/// Variant `start` with its fields filled from the arguments, or the usage line.
fn start_message(core: &Program, app: &App, start: usize, args: &[OsString]) -> Result<Value, End> {
    let fields: Vec<_> = (start_fields(core, app, start).iter())
        .map(|f| f.name.as_str())
        .collect();
    let texts = paka_rt::effects::arguments(&app.name, &fields, args)?;
    let items = texts.into_iter().map(|t| Value::Str(t.into())).collect();
    Ok(Value::Variant(start, Items(items)))
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
                Queued::Event(_) => unreachable!("a headless app has no events"),
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
    match on_big_stack(|| f(&mut Machine::new(core))) {
        Ok(Ok(done)) => Ok(done),
        Ok(Err(stop)) => Err(ended(core, app, stop)),
        Err(reason) => Err(ended(core, app, Stop::Internal(reason))),
    }
}

/// The end a trap or a fault brings.
fn ended(core: &Program, app: &App, stop: Stop) -> End {
    match stop {
        Stop::Trap(trap) => (3, report(core, &app.name, &trap)),
        Stop::Internal(reason) => (4, format!("{}: internal error: {reason}\n", app.name)),
        _ => unreachable!("a loop holds its break"),
    }
}

/// The fields of the start message's variant.
fn start_fields<'a>(core: &'a Program, app: &App, start: usize) -> &'a [Field] {
    let message = core.fns[app.update]
        .params
        .last()
        .expect("a message parameter");
    let Type::Enum(msg) = core.types[message.ty] else {
        unreachable!("the message is a user enum");
    };
    &core.enums[msg].variants[start].fields
}

/// The start variant of an app with no view, which always has one.
fn headless(app: &App) -> usize {
    app.start.expect("a headless app has a start")
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
    let ((file, _), line) = in_file(&core.files, |f| f.1, at.line);
    format!("{file}:{line}:{}", at.col)
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

/// The tree view `id` draws from `args` (M5-02); a trap or fault gives its message.
pub fn draw(core: &Program, id: FnId, args: Vec<Value>) -> Result<Node<Message>, String> {
    match on_big_stack(|| Machine::new(core).draw(id, args)) {
        Ok(Ok(node)) => Ok(node),
        Ok(Err(Stop::Trap(trap))) => {
            let at = place(core, trap.site);
            Err(format!("trap {} at {at}", trap.kind.name()))
        }
        Ok(Err(Stop::Internal(reason))) | Err(reason) => Err(format!("internal error: {reason}")),
        Ok(Err(_)) => unreachable!("a loop holds its break"),
    }
}

/// Runs UI example `f`: each action's events from its `ui_start` and what it left (M5-26).
pub fn ui_log(core: &Program, f: &Func) -> Result<Vec<(Vec<Event>, Left)>, String> {
    on_big_stack(|| {
        let mut machine = Machine::new(core);
        machine.ui_log = Some(vec![]);
        match machine.example(f) {
            Err(Stop::Internal(reason)) => Err(reason),
            _ => Ok(machine.ui_log.unwrap_or_default()),
        }
    })?
}

/// Runs every example in name order and prints its status (M2-06 section 1.6); the status is 0
/// when all pass, else 3.
pub fn examples(core: &Program, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let mut order: Vec<_> = core.examples.iter().chain(&core.ui_examples).collect();
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
    let mut machine = Machine::new(core);
    let done = machine.example(f);
    Ok(match done {
        Ok(_) if let Some((site, node)) = machine.broken => (
            false,
            format!("{name} fail at {}\n  {node}\n", place(core, site)),
        ),
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
        Err(Stop::Ended(line)) => (false, format!("{name} trapped\n  {line}")),
        Err(Stop::Internal(reason)) => return Err(reason),
        Err(_) => unreachable!("a loop holds its break"),
    })
}
