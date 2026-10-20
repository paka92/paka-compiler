//! The dev-mode runner (docs/decisions/M4-04.md). `paka serve` starts it and keeps it; each line on
//! standard input is one JSON request, answered by one line. `files` installs a program, which
//! checks, and `change` the same program with some files replaced; a run request runs its app as M4-03 section 3.4 says, or answers null when the program
//! has no app by that name. A program the dev back end does not compile yet runs on the
//! interpreter, in this process all the same.

use paka::core_form::Program;
use paka::interp;
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
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let request = parse(&line?).expect("the server sends JSON");
        let reply = match (request.get("files"), request.get("change")) {
            (Some(all), _) => {
                files = items(Some(all)).iter().map(file).collect();
                interpreted = install(&mut dev, &files);
                Value::object([])
            }
            (_, Some(changed)) => {
                for (path, text) in items(Some(changed)).iter().map(file) {
                    let held = files.iter_mut().find(|f| f.0 == path);
                    held.expect("a change replaces a held file").1 = text;
                }
                interpreted = install(&mut dev, &files);
                Value::object([])
            }
            _ => run(&dev, interpreted.as_ref(), &request),
        };
        writeln!(out, "{reply}")?;
        out.flush()?;
    }
    Ok(())
}

fn file(v: &Value) -> (String, String) {
    (text(v, "path").into(), text(v, "text").into())
}

/// Installs the program in `files`; its checked form when the dev back end refuses it.
fn install(dev: &mut Dev, files: &[(String, String)]) -> Option<Program> {
    let mut source = Source::default();
    for (path, text) in files {
        source.add(path.into(), text.as_str().into());
    }
    let refused = dev.edit(&source).is_err();
    refused.then(|| {
        let checked = paka::check("main.paka", &source);
        checked.expect("the server sends a program that checks").0
    })
}

fn run(dev: &Dev, interpreted: Option<&Program>, request: &Value) -> Value {
    let program = interpreted.unwrap_or(&dev.program);
    let app = text(request, "app");
    if program
        .app
        .as_ref()
        .is_none_or(|a| a.name != app || a.view.is_some())
    {
        return Value::Null;
    }
    let args: Vec<OsString> = (items(request.get("args")).iter())
        .map(|a| a.as_str().expect("an argument is a string").into())
        .collect();
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

/// M4-03's trap object, read from a report's first lines; null when the run did not trap.
fn trap(app: &str, stderr: &str) -> Value {
    let mut lines = stderr.lines();
    let head = lines
        .next()
        .and_then(|l| l.strip_prefix(app)?.strip_prefix(": trap "));
    let Some((kind, at)) = head.and_then(|h| h.split_once(" at ")) else {
        return Value::Null;
    };
    let mut place = at.rsplitn(3, ':').map(|n| n.parse::<usize>().ok());
    let (column, line) = (place.next().flatten(), place.next().flatten());
    let hole = lines.next().and_then(|l| l.trim_start().strip_prefix("??"));
    Value::object([
        ("kind", kind.into()),
        ("line", line.into()),
        ("column", column.into()),
        ("hole", hole.filter(|_| kind == "hole").into()),
    ])
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
