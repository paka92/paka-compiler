//! Compiles a Paka program to dev machine code, installs every definition and runs one; a `Dev`
//! session rebuilds it after each edit.
// Each test crate uses a part.
#![allow(dead_code)]

use paka::core_form::Program;
use paka::emit::{Cache, Emitter};
use paka::hash::Hash;
use paka::layout::Layouts;
use paka::source::Source;
use paka_installer::{Loader, Slots};
use paka_rt::abi::dev::Context;
use paka_rt::abi::dev::records::Records;

pub const UNWRITTEN: u64 = u64::MAX;

/// The checked program and a slot table holding all its definitions, functions then examples,
/// then its type records.
pub fn load(source: &str) -> (Program, Slots) {
    try_load(source).unwrap_or_else(|e| panic!("{e}"))
}

/// As `load`; the error is the program's first diagnostic or the emitter's first refusal.
pub fn try_load(source: &str) -> Result<(Program, Slots), String> {
    let mut dev = Dev::default();
    dev.edit(source)?;
    Ok((dev.program, dev.slots))
}

/// A dev session (M4-37): the last installed program, its keys and its slots.
pub struct Dev {
    emitter: Emitter,
    pub cache: Cache,
    pub loader: Loader<Hash>,
    pub slots: Slots,
    pub program: Program,
    pub keys: Vec<Hash>,
}

impl Default for Dev {
    fn default() -> Dev {
        Dev {
            emitter: Emitter::new().unwrap(),
            cache: Cache::default(),
            loader: Loader::default(),
            slots: Slots::new(0),
            program: Program::default(),
            keys: vec![],
        }
    }
}

impl Dev {
    /// Checks, compiles and installs `source` in one swap; on an error the last program stays.
    pub fn edit(&mut self, source: &str) -> Result<(), String> {
        let source = Source::file("main.paka", source.as_bytes().to_vec());
        let (mut p, ..) = paka::check("main.paka", &source).map_err(|d| d.to_string())?;
        let layouts = Layouts::new(&mut p);
        let keys = self.cache.build(&mut self.emitter, &p, &layouts)?;
        let mut group = self.loader.group(&keys, |k| self.cache.code(k)).unwrap();
        let records = Box::leak(Box::new(Records::new(&layouts.specs())));
        let types = (0..p.types.len()).map(|t| (keys.len() + t, records.address(t) as usize));
        group.extend(types);
        if group.len() != self.keys.len() + self.program.types.len() {
            self.slots = Slots::new(group.len());
        }
        self.slots.swap(&group).unwrap();
        (self.program, self.keys) = (p, keys);
        Ok(())
    }
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
