//! The dev-mode runner (docs/decisions/M4-04.md). `paka serve` starts it and keeps it; each line on
//! standard input is one JSON request, answered by one line. `files` installs a program, which
//! checks, and `change` the same program with some files replaced; a run request runs its app as M4-03 section 3.4 says, or answers null when the program
//! has no app by that name. A program the dev back end does not compile yet runs on the
//! interpreter, in this process all the same. A visual app runs on the interpreter in a window,
//! answered at once with its first tree; the next run ends it (docs/decisions/M5-09.md). A Look
//! call, sent on as the server got it, is answered by that app with the server's reply line. An
//! `install` swaps, resets or migrates a version into that app as the server decided (M5-42).

use paka::core_form::Program;
use paka::interp;
use paka::serve::{serve, trap};
use paka::session::visual::{Carry, Shown, Visual, not_running};
use paka::source::Source;
use paka_installer::dev::Dev;
use paka_json::{Value, parse};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Host;
use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::time::Duration;

/// The heap a run may fill.
const HEAP: u64 = 1 << 30;

fn main() -> io::Result<()> {
    let mut dev = Dev::default();
    // The installed program's files, and its checked form when the dev back end refused it.
    let (mut files, mut interpreted) = (vec![], None);
    // The app shown in a window, and the files of the version it runs.
    let mut shown: Option<(Visual, Vec<(String, String)>)> = None;
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let request = parse(&line).expect("the server sends JSON");
        if request.get("call").is_some() {
            let look = |call: &_| shown.as_ref().ok_or_else(not_running)?.0.look(call);
            serve(line.as_bytes(), &mut out, look)?;
            continue;
        }
        let ask = |key| request.get(key);
        let reply = match (ask("files"), ask("change"), ask("install")) {
            (_, _, Some(version)) => swapped(&mut shown, version),
            (Some(all), _, _) => {
                files = items(Some(all)).iter().map(file).collect();
                interpreted = install(&mut dev, &files);
                Value::object([])
            }
            (_, Some(changed), _) => {
                for (path, text) in items(Some(changed)).iter().map(file) {
                    let held = files.iter_mut().find(|f| f.0 == path);
                    held.expect("a change replaces a held file").1 = text;
                }
                interpreted = install(&mut dev, &files);
                Value::object([])
            }
            _ => run(&dev, interpreted.as_ref(), &files, &request, &mut shown),
        };
        writeln!(out, "{reply}")?;
        out.flush()?;
    }
    Ok(())
}

fn source(files: &[(String, String)]) -> Source {
    let mut source = Source::default();
    for (path, text) in files {
        source.add(path.into(), text.as_str().into());
    }
    source
}

fn file(v: &Value) -> (String, String) {
    (text(v, "path").into(), text(v, "text").into())
}

/// Installs the program in `files`; its checked form when the dev back end refuses it.
fn install(dev: &mut Dev, files: &[(String, String)]) -> Option<Program> {
    let refused = dev.edit(&source(files)).is_err();
    refused.then(|| checked(files))
}

/// The checked form of the program in `files`.
fn checked(files: &[(String, String)]) -> Program {
    let checked = paka::check("main.paka", &source(files));
    checked.expect("the server sends a program that checks").0
}

fn run(
    dev: &Dev,
    interpreted: Option<&Program>,
    files: &[(String, String)],
    request: &Value,
    shown: &mut Option<(Visual, Vec<(String, String)>)>,
) -> Value {
    *shown = None;
    let program = interpreted.unwrap_or(&dev.program);
    let app = text(request, "app");
    let Some(found) = program.app.as_ref().filter(|a| a.name == app) else {
        return Value::Null;
    };
    let args: Vec<OsString> = (items(request.get("args")).iter())
        .map(|a| a.as_str().expect("an argument is a string").into())
        .collect();
    if found.view.is_some() {
        let (visual, first) = Visual::start(checked(files), args, true);
        let reply = first.run(app, visual.window.is_some());
        *shown = first.end.is_none().then(|| (visual, files.to_vec()));
        return reply;
    }
    let number = |key| request.get(key).and_then(Value::as_u64).expect("a limit");
    let max = number("max_bytes") as usize;
    // Standard error holds only the run's last line or report, so it is cut once the run ends.
    let (mut world, mut err) = (Host(Cut::new(max)), vec![]);
    let status = match interpreted {
        Some(p) => Some(interp::run_in(p, &args, &mut world, &mut err)),
        None => {
            let limit = Duration::from_millis(number("limit_ms"));
            let ran = dev.run(&args, &mut world, HEAP, limit);
            let (status, line) = match ran.end {
                Ok(end) => (Some(end.0), end.1),
                Err(Stopped::Trap) => (Some(3), dev.report(&ran.trap)),
                Err(Stopped::Time) => (None, String::new()),
            };
            err = line.into_bytes();
            status
        }
    };
    let trap = trap(app, &String::from_utf8_lossy(&err));
    let mut stderr = Cut::new(max);
    let _ = stderr.write_all(&err);
    let stopped = match (status, world.0.cut || stderr.cut) {
        (None, _) => Some("time"),
        (_, true) => Some("output"),
        _ => None,
    };
    Value::object([
        ("stdout", world.0.text().into()),
        ("stderr", stderr.text().into()),
        ("status", status.map(usize::from).into()),
        ("trap", trap),
        ("stopped", stopped.into()),
    ])
}

/// Installs the version in `request` into the app shown, its state carried as the edit's state
/// choice names; what the app shows after, or the migrate's check error.
fn swapped(shown: &mut Option<(Visual, Vec<(String, String)>)>, request: &Value) -> Value {
    let Some((visual, old)) = shown else {
        return Shown::gone().value();
    };
    let new: Vec<_> = items(request.get("files")).iter().map(file).collect();
    let carry = Carry::of(request.get("state").and_then(Value::as_str));
    let limit = request
        .get("limit_ms")
        .and_then(Value::as_u64)
        .expect("a limit");
    match visual.install(old, &new, &carry, Duration::from_millis(limit)) {
        Err(error) => Value::object([("error", error)]),
        Ok(after) if after.end.is_some() => {
            *shown = None;
            after.value()
        }
        Ok(after) => {
            if after.trap.is_none() {
                *old = new;
            }
            after.value()
        }
    }
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).expect("a string field")
}

fn items(v: Option<&Value>) -> &[Value] {
    match v {
        Some(Value::Array(items)) => items,
        _ => &[],
    }
}

/// An output stream cut at `max` bytes, at the start of a character.
struct Cut {
    bytes: Vec<u8>,
    max: usize,
    cut: bool,
}

impl Cut {
    fn new(max: usize) -> Cut {
        Cut {
            bytes: vec![],
            max,
            cut: false,
        }
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

impl Write for Cut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut room = match self.cut {
            true => 0,
            false => buf.len().min(self.max - self.bytes.len()),
        };
        while room > 0 && room < buf.len() && buf[room] & 0xC0 == 0x80 {
            room -= 1;
        }
        self.cut |= room < buf.len();
        self.bytes.extend_from_slice(&buf[..room]);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
