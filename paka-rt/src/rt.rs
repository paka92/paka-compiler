//! The run loop of ABI section 5 and the safe decoding around it.

use crate::abi::dev::{MARGIN, RUNNER_STACK};
use crate::abi::{List, Str, Update, paka_rt1_stack_limit, take_trap};
use std::fs::File;
use std::io::{self, ErrorKind, Write};
use std::os::fd::AsFd;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

/// The status of a stop that is the program's doing (ABI section 7).
const STOPPED: i32 = 3;

/// The status of an internal error (ABI section 7).
pub const INTERNAL_ERROR: i32 = 4;

/// An effect the runtime can perform.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Decoded {
    Print(&'static [u8]),
    Exit,
}

/// An input the runtime refuses before reading through it (ABI section 7).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Refused {
    AppName,
    Grants,
    UpdateNull,
    EffectList,
    EffectTag,
    Effect,
}

impl Refused {
    /// The `<reason>` of the internal-error line.
    pub fn reason(self) -> &'static str {
        match self {
            Refused::AppName => "bad app name",
            Refused::Grants => "bad grants",
            Refused::UpdateNull => "update is null",
            Refused::EffectList => "bad effect list",
            Refused::EffectTag => "unknown effect tag",
            Refused::Effect => "bad effect",
        }
    }
}

/// The decoded `grants` word.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Grants {
    pub console: bool,
}

pub fn decode_grants(word: u64) -> Result<Grants, Refused> {
    match word {
        0 => Ok(Grants { console: false }),
        1 => Ok(Grants { console: true }),
        _ => Err(Refused::Grants),
    }
}

/// The status of a wrong number of arguments (ABI section 7).
const USAGE: i32 = 2;

/// Why a run stopped, short of `Exit`.
#[derive(Debug, PartialEq, Eq)]
enum Stop {
    Usage(usize),
    Ended,
    Missing,
    Failed(&'static str),
    /// A trap's kind and position.
    Trapped(&'static [u8]),
    Internal(&'static str),
}

impl From<Refused> for Stop {
    fn from(refused: Refused) -> Stop {
        Stop::Internal(refused.reason())
    }
}

impl Stop {
    fn status(&self) -> i32 {
        match self {
            Stop::Internal(_) => INTERNAL_ERROR,
            Stop::Usage(_) => USAGE,
            _ => STOPPED,
        }
    }

    /// The line's text after `<app>: `, in two pieces.
    fn pieces(&self) -> [&'static [u8]; 2] {
        match self {
            Stop::Usage(_) => [b"expected 0 arguments, got ", b""],
            Stop::Ended => [b"ended without Exit", b""],
            Stop::Missing => [b"effect Print refused: grant Console is missing", b""],
            Stop::Failed(reason) => [b"effect Print failed: ", reason.as_bytes()],
            Stop::Trapped(text) => [b"trap ", text],
            Stop::Internal(reason) => [b"internal error: ", reason.as_bytes()],
        }
    }
}

/// File descriptor 1 itself, which `Print` writes to. Rust's `Stdout` is buffered and calls a
/// closed descriptor a success. Closed at start, this holds the error and every write fails.
pub struct Fd1(io::Result<File>);

impl Default for Fd1 {
    fn default() -> Fd1 {
        Fd1(io::stdout().as_fd().try_clone_to_owned().map(File::from))
    }
}

impl Write for Fd1 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.0 {
            Ok(file) => file.write(buf),
            Err(e) => Err(e.kind().into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Runs on descriptor 1 and standard error, with the arguments the process was started with.
pub fn run(name: Str, update: Option<Update>, start: u64, grants: u64) -> i32 {
    let args = std::env::args_os().len().saturating_sub(1);
    run_with(
        &mut Fd1::default(),
        &mut io::stderr(),
        name.bytes(),
        update,
        start,
        grants,
        args,
    )
}

/// The run loop of ABI section 5, on any sinks. `name` is the decoded app name, `None` when it
/// was refused; `args` counts the command-line arguments after the program's name.
pub(crate) fn run_with(
    out: &mut dyn Write,
    err: &mut dyn Write,
    name: Option<&[u8]>,
    update: Option<Update>,
    start: u64,
    grants: u64,
    args: usize,
) -> i32 {
    let mut app: &[u8] = b"?";
    let caught = catch_unwind(AssertUnwindSafe(|| {
        app = name.ok_or(Refused::AppName)?;
        perform(out, update, start, grants, args)
    }));
    match caught {
        Ok(Ok(())) => 0,
        Ok(Err(why)) => stop(err, app, why),
        Err(_) => stop(err, app, Stop::Internal("runtime panicked")),
    }
}

/// Calls `update` once and performs its effects; `Ok` after `Exit`. The start message has no
/// fields, so any argument is a usage error.
fn perform(
    out: &mut dyn Write,
    update: Option<Update>,
    start: u64,
    grants: u64,
    args: usize,
) -> Result<(), Stop> {
    let grants = decode_grants(grants)?;
    let update = update.ok_or(Refused::UpdateNull)?;
    if args != 0 {
        return Err(Stop::Usage(args));
    }
    let mut slot = List::UNWRITTEN;
    match on_runner(|| (update(start, &mut slot), take_trap())) {
        (0, _) => {}
        (1, Some(text)) => return Err(Stop::Trapped(text)),
        _ => return Err(Stop::Internal("update returned a non-zero status")),
    }
    for effect in slot.effects()? {
        match effect.decode()? {
            Decoded::Exit => return Ok(()),
            Decoded::Print(_) if !grants.console => return Err(Stop::Missing),
            Decoded::Print(text) => print(out, text)?,
        }
    }
    Err(Stop::Ended)
}

/// Runs `f` on a new thread with a `RUNNER_STACK` stack, the stack limit `MARGIN` above its
/// bottom; a local's address stands for the top.
fn on_runner<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    std::thread::scope(|s| {
        let runner = std::thread::Builder::new().stack_size(RUNNER_STACK);
        let run = runner.spawn_scoped(s, || {
            let mark = 0u8;
            let top = (&raw const mark).addr() as u64;
            paka_rt1_stack_limit.store(top - (RUNNER_STACK - MARGIN) as u64, Ordering::Relaxed);
            f()
        });
        let run = run.expect("a runner thread");
        run.join().unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

/// The text and a line feed in one buffer, one write.
fn print(out: &mut dyn Write, text: &[u8]) -> Result<(), Stop> {
    // The reservation covers the line feed, so the push cannot reallocate.
    let mut line = buffer(text.len() + 1)?;
    line.extend_from_slice(text);
    line.push(b'\n');
    out.write_all(&line).map_err(|e| Stop::Failed(reason(&e)))
}

fn buffer(len: usize) -> Result<Vec<u8>, Stop> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(len)
        .map_err(|_| Stop::Internal("out of memory"))?;
    Ok(buffer)
}

fn reason(e: &io::Error) -> &'static str {
    match e.kind() {
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => "no space left on device",
        ErrorKind::BrokenPipe => "broken pipe",
        _ => "write error",
    }
}

/// Writes the stop's line piece by piece, without allocating; a failure to write is ignored.
fn stop(err: &mut dyn Write, app: &[u8], why: Stop) -> i32 {
    let [text, reason] = why.pieces();
    let line = [app, b": ", text, reason];
    let _ = line.iter().try_for_each(|piece| err.write_all(piece));
    if let Stop::Usage(count) = why {
        let _ = write!(err, "{count}");
    }
    let _ = err.write_all(b"\n");
    why.status()
}

#[cfg(test)]
mod run_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{Effect, List};

    #[test]
    fn grants_bit_0_is_console() {
        assert_eq!(decode_grants(0), Ok(Grants { console: false }));
        assert_eq!(decode_grants(1), Ok(Grants { console: true }));
    }

    #[test]
    fn any_other_grant_bit_is_refused() {
        for bit in 1..64 {
            assert_eq!(decode_grants(1 << bit), Err(Refused::Grants), "bit {bit}");
            assert_eq!(
                decode_grants(1 | 1 << bit),
                Err(Refused::Grants),
                "bit {bit}"
            );
        }
        assert_eq!(decode_grants(u64::MAX), Err(Refused::Grants));
    }

    #[test]
    fn reasons_are_the_documents_texts() {
        assert_eq!(Refused::AppName.reason(), "bad app name");
        assert_eq!(Refused::Grants.reason(), "bad grants");
        assert_eq!(Refused::UpdateNull.reason(), "update is null");
        assert_eq!(Refused::EffectList.reason(), "bad effect list");
        assert_eq!(Refused::EffectTag.reason(), "unknown effect tag");
        assert_eq!(Refused::Effect.reason(), "bad effect");
    }

    extern "C" fn light(_msg: u64, out: &mut List) -> i32 {
        const FX: &[Effect] = &[Effect::print(b"let there be light"), Effect::EXIT];
        *out = List::new(FX);
        0
    }

    extern "C" fn stores_nothing(_msg: u64, _out: &mut List) -> i32 {
        0
    }

    // This module may not use unsafe: the loop's side of an update call needs none.
    #[test]
    fn a_fake_update_is_called_and_decoded_without_unsafe() {
        let update: Update = light;
        let mut slot = List::UNWRITTEN;
        assert_eq!(update(0, &mut slot), 0);
        let effects = slot.effects().unwrap();
        assert_eq!(effects.len(), 2);
        assert_eq!(
            effects[0].decode(),
            Ok(Decoded::Print(b"let there be light"))
        );
        assert_eq!(effects[1].decode(), Ok(Decoded::Exit));
    }

    #[test]
    fn an_update_that_stores_nothing_leaves_a_refused_list() {
        let mut slot = List::UNWRITTEN;
        assert_eq!(stores_nothing(0, &mut slot), 0);
        assert_eq!(slot.effects().err(), Some(Refused::EffectList));
    }
}
