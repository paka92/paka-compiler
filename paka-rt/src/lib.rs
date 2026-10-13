//! The Paka runtime of milestone 1, written to `docs/abi/m1.md`.

#![deny(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]

#[allow(unsafe_code)]
pub mod abi;

use abi::{List, Str, Update};
use std::io::{self, ErrorKind, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

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

/// Why a run stopped, short of `Exit`.
#[derive(Debug, PartialEq, Eq)]
enum Stop {
    Ended,
    Missing,
    Failed(&'static str),
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
            _ => STOPPED,
        }
    }

    /// The line's text after `<app>: `, in two pieces.
    fn pieces(&self) -> [&'static str; 2] {
        match self {
            Stop::Ended => ["ended without Exit", ""],
            Stop::Missing => ["effect Print refused: grant Console is missing", ""],
            Stop::Failed(reason) => ["effect Print failed: ", reason],
            Stop::Internal(reason) => ["internal error: ", reason],
        }
    }
}

/// Runs on the real standard streams.
pub fn run(name: Str, update: Option<Update>, start: u64, grants: u64) -> i32 {
    let (mut out, mut err) = (io::stdout(), io::stderr());
    run_with(&mut out, &mut err, name.bytes(), update, start, grants)
}

/// The run loop of ABI section 5, on any sinks. `name` is the decoded app name, `None` when it
/// was refused.
pub(crate) fn run_with(
    out: &mut dyn Write,
    err: &mut dyn Write,
    name: Option<&[u8]>,
    update: Option<Update>,
    start: u64,
    grants: u64,
) -> i32 {
    let mut app: &[u8] = b"?";
    let caught = catch_unwind(AssertUnwindSafe(|| {
        app = name.ok_or(Refused::AppName)?;
        perform(out, update, start, grants)
    }));
    match caught {
        Ok(Ok(())) => 0,
        Ok(Err(why)) => stop(err, app, why),
        Err(_) => stop(err, app, Stop::Internal("runtime panicked")),
    }
}

/// Calls `update` once and performs its effects; `Ok` after `Exit`.
fn perform(
    out: &mut dyn Write,
    update: Option<Update>,
    start: u64,
    grants: u64,
) -> Result<(), Stop> {
    let grants = decode_grants(grants)?;
    let update = update.ok_or(Refused::UpdateNull)?;
    let mut slot = List::UNWRITTEN;
    if update(start, &mut slot) != 0 {
        return Err(Stop::Internal("update returned a non-zero status"));
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

/// The text and a line feed in one buffer, one write, then a flush.
fn print(out: &mut dyn Write, text: &[u8]) -> Result<(), Stop> {
    // The reservation covers the line feed, so the push cannot reallocate.
    let mut line = buffer(text.len() + 1)?;
    line.extend_from_slice(text);
    line.push(b'\n');
    out.write_all(&line)
        .and_then(|()| out.flush())
        .map_err(|e| Stop::Failed(reason(&e)))
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
        ErrorKind::StorageFull => "no space left on device",
        ErrorKind::BrokenPipe => "broken pipe",
        _ => "write error",
    }
}

/// Writes the stop's line piece by piece, without allocating; a failure to write is ignored.
fn stop(err: &mut dyn Write, app: &[u8], why: Stop) -> i32 {
    let [text, reason] = why.pieces();
    let line = [app, b": ", text.as_bytes(), reason.as_bytes(), b"\n"];
    let _ = line.iter().try_for_each(|piece| err.write_all(piece));
    why.status()
}

#[cfg(test)]
mod run_tests;

#[cfg(test)]
mod tests {
    use super::abi::{Effect, List};
    use super::*;

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
