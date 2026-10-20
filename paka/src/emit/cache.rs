//! Compiles only the definitions whose key is new (M4-37). A key is the SHA-256 of the compiler
//! version, the target and mode, and the definition's Cranelift IR. The IR holds everything the
//! code depends on: the body, the signature and slot of each callee and the layouts it reads. So an
//! edit recompiles the edited definition and only the users whose code it changes.

use super::{Emitter, Refusal};
use crate::core_form::Program;
use crate::hash::Hash;
use crate::layout::Layouts;
use crate::sha256::sha256;
use std::borrow::Cow;
use std::collections::BTreeMap;

const STAMP: &str = concat!(
    "paka ",
    env!("CARGO_PKG_VERSION"),
    " x86_64-unknown-linux-gnu+sse4.1 dev\n"
);

/// Machine code by key, and how many definitions it compiled.
#[derive(Default)]
pub struct Cache {
    code: BTreeMap<Hash, Vec<u8>>,
    pub compiles: usize,
}

impl Cache {
    /// The key of each definition of `p`, functions, examples then constructors, compiling the
    /// ones not held.
    /// A refusal keeps nothing of this build.
    pub fn build(
        &mut self,
        e: &mut Emitter,
        p: &Program,
        l: &Layouts,
    ) -> Result<Vec<Hash>, Refusal> {
        let (mut keys, mut fresh) = (vec![], BTreeMap::new());
        for def in 0..super::definitions(p) {
            let ir = e.ir(p, l, def)?;
            let key = sha256(format!("{STAMP}{}", keyed(e.plant, &ir.to_string())).as_bytes());
            if !self.code.contains_key(&key) {
                fresh.insert(key, e.compile(ir)?);
            }
            keys.push(key);
        }
        self.compiles += fresh.len();
        self.code.extend(fresh);
        Ok(keys)
    }

    /// The machine code of a key `build` returned.
    pub fn code(&self, key: &Hash) -> &[u8] {
        &self.code[key]
    }
}

/// The IR a key is made of. Plant 6 leaves out its constants and plant 7 its loads, so code goes
/// stale after an edit (M4-43's self-test).
fn keyed(plant: u8, ir: &str) -> Cow<'_, str> {
    let left_out: &[&str] = match plant {
        6 => &["const", "_imm"],
        7 => &["load"],
        _ => return ir.into(),
    };
    // A use of a constant shows its value after a `;`.
    let lines = ir.lines().map(|l| l.split(';').next().unwrap_or(l));
    lines
        .filter(|l| !left_out.iter().any(|w| l.contains(w)))
        .collect()
}
