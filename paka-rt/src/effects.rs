//! The effect performer both back ends share (docs/decisions/M2-15.md sections 1.3 to 1.8,
//! M4-05 section 3): the start arguments, the grant check, what each effect does to the world,
//! the answer it queues and the end it may bring. Each back end decodes its own values.

use crate::ops::{CAP, Int};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::OpenOptionsExt;

/// `O_NONBLOCK`, which std does not name: a FIFO swapped in after the first look cannot block
/// the open. Elsewhere the open is plain.
#[cfg(target_os = "linux")]
const NONBLOCK: i32 = 0o4000;
#[cfg(not(target_os = "linux"))]
const NONBLOCK: i32 = 0;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grant {
    Console,
    ReadFiles,
    WriteFiles,
}

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
    /// Whether granted foreign effects are dropped rather than performed.
    fn drops(&self) -> bool {
        false
    }
}

/// An effect with its fields: `F` is an answer function, `M` a message.
pub enum Effect<'a, F, M> {
    Print(&'a str),
    ReadFile(&'a str, F, F),
    WriteFile(&'a str, &'a str, M, F),
    Exit,
    ExitWith(Int),
}

/// What an effect queues: a message, or an answer function and the text it is applied to.
pub enum Next<F, M> {
    Message(M),
    Answer(F, String),
}

/// The start message's fields from the command-line arguments, or the usage line.
pub fn arguments(app: &str, fields: &[&str], args: &[OsString]) -> Result<Vec<String>, End> {
    let usage = |line: String| (2, format!("{app}: {line}\n"));
    if args.len() != fields.len() {
        let plural = if fields.len() == 1 { "" } else { "s" };
        let names = match fields {
            [] => String::new(),
            _ => format!(" ({})", fields.join(", ")),
        };
        let (want, got) = (fields.len(), args.len());
        return Err(usage(format!(
            "expected {want} argument{plural}{names}, got {got}"
        )));
    }
    let text = |(i, arg): (usize, &OsString)| match arg.to_str() {
        Some(text) => Ok(text.to_string()),
        None => Err(usage(format!("argument {} is not text", i + 1))),
    };
    args.iter().enumerate().map(text).collect()
}

/// Performs one effect of app `app`: the answer it queues, if any, or the end it brings.
pub fn perform<F, M>(
    app: &str,
    grants: &BTreeSet<Grant>,
    effect: Effect<F, M>,
    world: &mut impl World,
) -> Result<Option<Next<F, M>>, End> {
    let stop = |why: String| (3, format!("{app}: {why}\n"));
    if let Some((grant, name, needed)) = grant_for(&effect)
        && !grants.contains(&grant)
    {
        return Err(stop(format!(
            "effect {name} refused: grant {needed} is missing"
        )));
    }
    let next = match effect {
        Effect::Exit => return Err((0, String::new())),
        Effect::ExitWith(status) => return Err(ending(status).unwrap_or_else(stop)),
        Effect::Print(text) => {
            let printed = world.print(text);
            printed.map_err(|why| stop(format!("effect Print failed: {why}")))?;
            return Ok(None);
        }
        Effect::ReadFile(path, then, or) => match usable(path).and_then(|p| world.read(p)) {
            Ok(text) => Next::Answer(then, text),
            Err(why) => Next::Answer(or, why.into()),
        },
        Effect::WriteFile(path, text, then, or) => {
            match usable(path).and_then(|p| world.write(p, text)) {
                Ok(()) => Next::Message(then),
                Err(why) => Next::Answer(or, why.into()),
            }
        }
    };
    Ok(Some(next))
}

/// The grant an effect needs, the effect's name and the grant's.
fn grant_for<F, M>(effect: &Effect<F, M>) -> Option<(Grant, &'static str, &'static str)> {
    match effect {
        Effect::Print(_) => Some((Grant::Console, "Print", "Console")),
        Effect::ReadFile(..) => Some((Grant::ReadFiles, "ReadFile", "ReadFiles")),
        Effect::WriteFile(..) => Some((Grant::WriteFiles, "WriteFile", "WriteFiles")),
        Effect::Exit | Effect::ExitWith(_) => None,
    }
}

/// The end a status brings: 0 or 5 to 255 is the program's choice, any other is a stop.
#[allow(clippy::disallowed_methods)] // the value leaves Paka as the process status
fn ending(status: Int) -> Result<End, String> {
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

    /// The path's kind first, then the open, then the opened file's kind and size (M2-15 section
    /// 1.7).
    fn read(&mut self, path: &str) -> Result<String, Reason> {
        if !fs::metadata(path).map_err(|e| file_reason(&e))?.is_file() {
            return Err("not a file");
        }
        let open = File::options().read(true).custom_flags(NONBLOCK).open(path);
        let file = open.map_err(|e| file_reason(&e))?;
        let meta = file.metadata().map_err(|e| file_reason(&e))?;
        if !meta.is_file() {
            return Err("not a file");
        }
        if meta.len() > CAP as u64 {
            return Err("failed");
        }
        let mut bytes = Vec::with_capacity(meta.len() as usize);
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

/// The world of a UI example (M5-06), which drops every granted effect.
pub struct Dropped;

impl World for Dropped {
    fn print(&mut self, _: &str) -> Result<(), Reason> {
        Ok(())
    }

    fn read(&mut self, _: &str) -> Result<String, Reason> {
        Err("dropped")
    }

    fn write(&mut self, _: &str, _: &str) -> Result<(), Reason> {
        Ok(())
    }

    fn drops(&self) -> bool {
        true
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

    #[cfg(target_os = "linux")]
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
        if ignored {
            eprintln!("skipped: this process ignores file permissions");
        } else {
            assert_eq!(read, Err("no permission"));
        }
    }
}
