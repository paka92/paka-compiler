//! The reference interpreter: runs the core form (docs/core.md) and performs the effects the
//! handler returns (spec/m1.md section 9, M2-15). Effects other than `Print` and `Exit` are
//! M2-36's.

use crate::core_form::{App, Effect, Field, Grant, Program, Type};
use crate::core_form::{Fn as Func, Site};
use eval::{Kind, Machine, Stop, Trap};
use show::show;
use std::any::Any;
use std::io::{self, ErrorKind, Write};
use std::sync::Once;
use std::{panic, thread};
use value::Value;

mod builtin;
mod eval;
mod generic;
mod show;
#[cfg(test)]
mod tests;
mod value;

/// Host stack of the run thread and of every segment the evaluation adds to it (`eval::BUDGET`
/// levels each); the pages are only committed when used.
const STACK: usize = 32 << 20;
/// The longest detail line of a trap report, in bytes.
const DETAIL: usize = 256;
/// The name of the thread that runs the program; its panics are reported as an internal error.
const RUN_THREAD: &str = "paka-run";
/// The most frames a trap report names.
const NAMED: usize = 8;

const PRINT: usize = Effect::Print as usize;
const EXIT: usize = Effect::Exit as usize;

/// A status and the line that explains it.
type End = (u8, String);

/// Runs the program; returns its status.
pub fn run(core: &Program, out: &mut impl Write, err: &mut impl Write) -> u8 {
    let app = core.app.as_ref().expect("a program with an app");
    let (status, line) = match handle(core, app) {
        Ok(effects) => match perform(app, &effects, out) {
            None => return 0,
            Some(stop) => stop,
        },
        Err(stop) => stop,
    };
    let _ = err.write_all(line.as_bytes());
    status
}

/// The effects the handler returns for the start message.
fn handle(core: &Program, app: &App) -> Result<Value, End> {
    let fields = start_fields(core, app);
    if !fields.is_empty() {
        let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        let plural = if names.len() == 1 { "" } else { "s" };
        let (count, names) = (names.len(), names.join(", "));
        let line = format!("expected {count} argument{plural} ({names}), got 0");
        return Err((2, format!("{}: {line}\n", app.name)));
    }
    let internal = |reason: &str| (4, format!("{}: internal error: {reason}\n", app.name));
    match on_big_stack(|| Machine::new(core).handle(app)) {
        Ok(Ok(answer)) => Ok(answer),
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

/// `file:line:col` of a site; the file without its directory.
fn place(core: &Program, site: Site) -> String {
    let at = &core.sites[site];
    let file = core.file.rsplit('/').next().unwrap_or_default();
    format!("{file}:{}:{}", at.line, at.col)
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

/// Performs the effects in order; `None` after `Exit`, or the status and line that end the run.
fn perform(app: &App, answer: &Value, out: &mut impl Write) -> Option<End> {
    let Value::List(effects) = answer else {
        unreachable!("a checked List");
    };
    let stop = |status, reason: &str| Some((status, format!("{}: {reason}\n", app.name)));
    for effect in &effects.0 {
        let Value::Variant(number, fields) = effect else {
            unreachable!("a checked Effect");
        };
        match (*number, &fields.0[..]) {
            (EXIT, _) => return None,
            (PRINT, _) if !app.grants.contains(&Grant::Console) => {
                return stop(3, "effect Print refused: grant Console is missing");
            }
            (PRINT, [Value::Str(text)]) => {
                if let Err(e) = print(out, text.as_str()) {
                    return stop(3, &format!("effect Print failed: {}", reason(&e)));
                }
            }
            _ => return stop(4, "internal error: effect not implemented yet"),
        }
    }
    stop(3, "ended without Exit")
}

/// The text and its line feed in one buffer, then a flush.
fn print(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(format!("{text}\n").as_bytes())?;
    out.flush()
}

fn reason(e: &io::Error) -> &'static str {
    match e.kind() {
        ErrorKind::StorageFull => "no space left on device",
        ErrorKind::BrokenPipe => "broken pipe",
        _ => "write error",
    }
}
