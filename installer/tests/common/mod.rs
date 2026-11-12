//! Compiles a Paka program to dev machine code, installs every definition and runs one.
// Each test crate uses a part.
#![allow(dead_code)]

use paka::core_form::Program;
use paka::source::Source;
use paka_installer::Slots;
pub use paka_installer::dev::Dev;
use paka_rt::abi::dev::Context;

pub const UNWRITTEN: u64 = u64::MAX;

/// The checked program and a slot table holding all its definitions, functions then examples,
/// then its type records.
pub fn load(source: &str) -> (Program, Slots) {
    try_load(source).unwrap_or_else(|e| panic!("{e}"))
}

/// As `load`; the error is the program's first diagnostic or the emitter's first refusal.
pub fn try_load(source: &str) -> Result<(Program, Slots), String> {
    let mut dev = Dev::default();
    dev.edit(&self::source(source))?;
    Ok((dev.program, dev.slots))
}

/// `text` as the program's one file.
pub fn source(text: &str) -> Source {
    Source::file("main.paka", text.as_bytes().to_vec())
}

/// Runs definition `id`, which takes no arguments: its status and the word it wrote, `UNWRITTEN`
/// when it wrote none.
pub fn run(slots: &Slots, id: usize, ctx: &mut Context) -> (i32, u64) {
    let mut out = UNWRITTEN;
    // SAFETY: `load` filled every slot with its definition's code, compiled as M4-05 section 5
    // says; definition `id` takes no arguments and writes at most a word, and `ctx` and `out`
    // live through the run.
    let status = unsafe { slots.run(id, (&raw mut *ctx).cast(), (&raw mut out).cast()) };
    (status.unwrap(), out)
}
