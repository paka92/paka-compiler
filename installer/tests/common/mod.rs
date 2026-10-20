//! Compiles a Paka program to dev machine code, installs every definition and runs one.

use paka::core_form::Program;
use paka::emit::Emitter;
use paka::layout::Layouts;
use paka::source::Source;
use paka_installer::{Slots, install};
use paka_rt::abi::dev::Context;

pub const UNWRITTEN: u64 = u64::MAX;

/// The checked program and a slot table holding all its definitions, functions then examples.
pub fn load(source: &str) -> (Program, Slots) {
    try_load(source).unwrap_or_else(|e| panic!("{e}"))
}

/// As `load`; the error is the program's first diagnostic or the emitter's first refusal.
pub fn try_load(source: &str) -> Result<(Program, Slots), String> {
    let source = Source::file("main.paka", source.as_bytes().to_vec());
    let (mut p, ..) = paka::check("main.paka", &source).map_err(|d| d.to_string())?;
    let layouts = Layouts::new(&mut p);
    let mut emitter = Emitter::new()?;
    let defs = p.fns.len() + p.examples.len();
    let code = (0..defs).map(|d| emitter.code(&p, &layouts, d));
    let code = code.collect::<Result<Vec<_>, _>>()?;
    let installed = install(&code.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
    let slots = Slots::new(defs);
    let group: Vec<_> = installed
        .iter()
        .enumerate()
        .map(|(id, c)| (id, c.address))
        .collect();
    slots.swap(&group).unwrap();
    Ok((p, slots))
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
