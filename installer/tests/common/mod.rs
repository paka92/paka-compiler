//! Compiles a Paka program to dev machine code, installs every definition and runs one.

use paka::core_form::Program;
use paka::emit::Emitter;
use paka::source::Source;
use paka_installer::{Slots, install};
use paka_rt::abi::dev::Context;

/// The checked program and a slot table holding all its definitions, functions then examples.
pub fn load(source: &str) -> (Program, Slots) {
    let source = Source::file("main.paka", source.as_bytes().to_vec());
    let (p, ..) = paka::check("main.paka", &source).unwrap_or_else(|d| panic!("{d}"));
    let mut emitter = Emitter::new().unwrap();
    let defs = p.fns.len() + p.examples.len();
    let code: Vec<Vec<u8>> = (0..defs).map(|d| emitter.code(&p, d).unwrap()).collect();
    let installed = install(&code.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
    let slots = Slots::new(defs);
    let group: Vec<_> = installed
        .iter()
        .enumerate()
        .map(|(id, c)| (id, c.address))
        .collect();
    slots.swap(&group).unwrap();
    (p, slots)
}

/// Runs definition `id`, which takes no arguments: its status and the word it wrote.
pub fn run(slots: &Slots, id: usize, ctx: &mut Context) -> (i32, u64) {
    let mut out = 0u64;
    // SAFETY: `load` filled every slot with its definition's code, compiled as M4-05 section 5
    // says; definition `id` takes no arguments and writes at most a word, and `ctx` and `out`
    // live through the run.
    let status = unsafe { slots.run(id, (&raw mut *ctx).cast(), (&raw mut out).cast()) };
    (status.unwrap(), out)
}
