//! The Paka runtime of milestone 1, written to `docs/abi/m1.md`.

#![deny(unsafe_code)]
#![deny(clippy::undocumented_unsafe_blocks)]

#[allow(unsafe_code)]
pub mod abi;

use abi::{Str, Update};
use std::io::{self, Write};

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

/// Why a run stopped.
#[derive(Debug, PartialEq, Eq)]
enum Stop {
    Internal(&'static str),
}

/// Runs on the real standard streams.
pub fn run(name: Str, update: Option<Update>, start: u64, grants: u64) -> i32 {
    let (mut out, mut err) = (io::stdout(), io::stderr());
    run_with(&mut out, &mut err, name.bytes(), update, start, grants)
}

/// The run loop, on any sinks. `name` is the decoded app name, `None` when it was refused.
pub fn run_with(
    _out: &mut dyn Write,
    _err: &mut dyn Write,
    _name: Option<&[u8]>,
    _update: Option<Update>,
    _start: u64,
    _grants: u64,
) -> i32 {
    INTERNAL_ERROR
}

fn buffer(_len: usize) -> Result<Vec<u8>, Stop> {
    Err(Stop::Internal("stub"))
}

fn stop(_err: &mut dyn Write, _app: &[u8], _why: Stop) -> i32 {
    0
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
