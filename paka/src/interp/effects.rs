//! The effects a handler returns, performed against a world (docs/decisions/M2-15.md sections
//! 1.3 to 1.8): the grant check, the answers each effect queues and the end it may bring.

use super::eval::CAP;
use super::value::{FnKey, Value};
use crate::core_form::{App, Effect, Grant};
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::mem;

const PRINT: usize = Effect::Print as usize;
const READ_FILE: usize = Effect::ReadFile as usize;
const WRITE_FILE: usize = Effect::WriteFile as usize;
const EXIT: usize = Effect::Exit as usize;
const EXIT_WITH: usize = Effect::ExitWith as usize;

/// The effects' names, by variant number.
const NAMES: [&str; 5] = ["Print", "ReadFile", "WriteFile", "Exit", "ExitWith"];

/// A status and the line that explains it; the line is empty when the program chose its end.
pub type End = (u8, String);

/// Why an effect failed, in the words a program is given.
pub type Reason = &'static str;

/// What an effect touches. The host is `Host`; tests bring a world in memory.
pub trait World {
    /// The text and a line feed, written whole.
    fn print(&mut self, text: &str) -> Result<(), Reason>;
    fn read(&mut self, path: &str) -> Result<String, Reason>;
    /// Creates or replaces the file.
    fn write(&mut self, path: &str, text: &str) -> Result<(), Reason>;
}

/// A message waiting its turn: given, or an answer whose function is applied at the front.
pub enum Queued {
    Message(Value),
    Answer(FnKey, Value),
}

/// Performs the effects in order, queueing the answers; `Some` when one ends the run, `None`
/// after the last.
pub fn perform(
    app: &App,
    answer: &mut Value,
    world: &mut impl World,
    queue: &mut VecDeque<Queued>,
) -> Option<End> {
    let Value::List(effects) = answer else {
        unreachable!("a checked List");
    };
    for effect in &mut effects.0 {
        let Value::Variant(number, fields) = effect else {
            unreachable!("a checked Effect");
        };
        match one(app, *number, &mut fields.0, world) {
            Ok(Some(queued)) => queue.push_back(queued),
            Ok(None) => {}
            Err(end) => return Some(end),
        }
    }
    None
}

/// One effect: the answer it queues, if any, or the end it brings.
fn one(
    app: &App,
    number: usize,
    fields: &mut [Value],
    world: &mut impl World,
) -> Result<Option<Queued>, End> {
    let stop = |why: String| (3, format!("{}: {why}\n", app.name));
    let name = NAMES[number];
    if let Some((grant, needed)) = grant_for(number)
        && !app.grants.contains(&grant)
    {
        return Err(stop(format!(
            "effect {name} refused: grant {needed} is missing"
        )));
    }
    let queued = match (number, fields) {
        (EXIT, _) => return Err((0, String::new())),
        (EXIT_WITH, [Value::Int(status)]) => return Err(ending(*status).unwrap_or_else(stop)),
        (PRINT, [Value::Str(text)]) => {
            let printed = world.print(text.as_str());
            printed.map_err(|why| stop(format!("effect Print failed: {why}")))?;
            return Ok(None);
        }
        (READ_FILE, [Value::Str(path), Value::Fn(then), Value::Fn(or)]) => {
            match usable(path.as_str()).and_then(|path| world.read(path)) {
                Ok(text) => Queued::Answer(*then, Value::Str(text.into())),
                Err(why) => Queued::Answer(*or, Value::Str(why.into())),
            }
        }
        (WRITE_FILE, [Value::Str(path), Value::Str(text), then, Value::Fn(or)]) => {
            match usable(path.as_str()).and_then(|path| world.write(path, text.as_str())) {
                Ok(()) => Queued::Message(mem::take(then)),
                Err(why) => Queued::Answer(*or, Value::Str(why.into())),
            }
        }
        _ => unreachable!("a checked Effect"),
    };
    Ok(Some(queued))
}

/// The grant an effect needs and its name.
fn grant_for(number: usize) -> Option<(Grant, &'static str)> {
    match number {
        PRINT => Some((Grant::Console, "Console")),
        READ_FILE => Some((Grant::ReadFiles, "ReadFiles")),
        WRITE_FILE => Some((Grant::WriteFiles, "WriteFiles")),
        _ => None,
    }
}

/// The end a status brings: 0 or 5 to 255 is the program's choice, any other is a stop.
#[allow(clippy::disallowed_methods)] // the value leaves Paka as the process status
fn ending(status: paka_rt::ops::Int) -> Result<End, String> {
    match u8::try_from(status.get()) {
        Ok(code @ (0 | 5..)) => Ok((code, String::new())),
        _ => Err(format!(
            "effect ExitWith failed: status {status} is not 0 or 5 to 255"
        )),
    }
}

/// The path, unless no file can have it: empty, or holding a zero byte.
fn usable(path: &str) -> Result<&str, Reason> {
    if path.is_empty() || path.contains('\0') {
        return Err("failed");
    }
    Ok(path)
}

/// The host: descriptor 1 through `out`, and files by path, relative to the working directory.
pub struct Host<W>(pub W);

impl<W: Write> World for Host<W> {
    fn print(&mut self, text: &str) -> Result<(), Reason> {
        let line = format!("{text}\n");
        self.0
            .write_all(line.as_bytes())
            .map_err(|e| match e.kind() {
                ErrorKind::StorageFull | ErrorKind::QuotaExceeded => "no space left on device",
                ErrorKind::BrokenPipe => "broken pipe",
                _ => "write error",
            })
    }

    fn read(&mut self, path: &str) -> Result<String, Reason> {
        let meta = fs::metadata(path).map_err(|e| file_reason(&e))?;
        if !meta.is_file() {
            return Err("not a file");
        }
        if meta.len() > CAP as u64 {
            return Err("failed");
        }
        let mut bytes = Vec::with_capacity(meta.len() as usize);
        let file = File::open(path).map_err(|e| file_reason(&e))?;
        file.take(CAP as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| file_reason(&e))?;
        if bytes.len() > CAP {
            return Err("failed");
        }
        String::from_utf8(bytes).map_err(|_| "not text")
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), Reason> {
        fs::write(path, text).map_err(|e| file_reason(&e))
    }
}

/// The reason a file operation's host error is given as.
fn file_reason(e: &io::Error) -> Reason {
    match e.kind() {
        ErrorKind::NotFound | ErrorKind::NotADirectory => "not found",
        ErrorKind::PermissionDenied => "no permission",
        ErrorKind::IsADirectory => "not a file",
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => "no space",
        _ => "failed",
    }
}

/// The host against real descriptors and files (docs/decisions/M2-15.md sections 1.6 and 1.7).
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    #[test]
    fn a_print_to_a_full_device_is_no_space_left_on_device() {
        let full = File::options().write(true).open("/dev/full").unwrap();
        assert_eq!(Host(full).print("x"), Err("no space left on device"));
    }

    #[test]
    fn a_print_to_a_closed_pipe_is_broken_pipe() {
        let (reader, writer) = io::pipe().unwrap();
        drop(reader);
        assert_eq!(Host(writer).print("x"), Err("broken pipe"));
    }

    /// What a built program's write to a closed descriptor 1 meets: `EBADF`.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(9))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_print_to_a_closed_descriptor_is_write_error() {
        assert_eq!(Host(Closed).print("x"), Err("write error"));
    }

    /// A file one byte over the String limit, sparse, whose first byte is not UTF-8.
    fn too_big(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("paka-host-{}-{name}", std::process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(&[0xff]).unwrap();
        file.set_len(CAP as u64 + 1).unwrap();
        path
    }

    #[test]
    fn a_file_over_the_limit_is_failed_before_it_is_decoded() {
        let path = too_big("over");
        let read = Host(io::sink()).read(path.to_str().unwrap());
        fs::remove_file(&path).unwrap();
        assert_eq!(read, Err("failed"));
    }

    #[test]
    fn no_permission_comes_before_the_limit() {
        let path = too_big("locked");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        let ignored = File::open(&path).is_ok(); // a process that ignores permissions
        let read = Host(io::sink()).read(path.to_str().unwrap());
        fs::remove_file(&path).unwrap();
        if !ignored {
            assert_eq!(read, Err("no permission"));
        }
    }
}
