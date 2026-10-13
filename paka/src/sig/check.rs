//! The check over the converted definitions together: a type must have a finite value (M2-05
//! section 1.9).

use super::{Checked, Types};
use crate::ast::Program;
use crate::diagnostic::Diagnostic;
use std::mem;

/// Whether each definition has a finite value to write down; a definition that is no type has.
/// A record has when every field has, an enum when some variant has only such fields, a Result
/// when one side has; any other built-in type has, down to an empty List, an empty Map and a
/// `None`. The least fixed point, by a worklist: one step for each field.
fn grounded(types: &Types, defs: &[Checked]) -> Vec<bool> {
    let mut ground: Vec<_> = defs.iter().map(|d| !d.item.is_type()).collect();
    // The fields still without a value of each alternative, with its type.
    let mut alts: Vec<(usize, usize)> = Vec::new();
    // The alternative of each field that still waits, and which fields wait for each type.
    let mut slots: Vec<Option<usize>> = Vec::new();
    let mut waiting = vec![Vec::new(); defs.len()];
    let mut needs = Vec::new();
    for (t, def) in defs.iter().enumerate() {
        for fields in def.item.alternatives() {
            let mut open = 0;
            for f in fields {
                needs.clear();
                if !types.needs(f.ty, &mut needs) {
                    needs.iter().for_each(|&j| waiting[j].push(slots.len()));
                    slots.push(Some(alts.len()));
                    open += 1;
                }
            }
            alts.push((t, open));
        }
    }
    let mut ready: Vec<_> = (0..alts.len()).filter(|&a| alts[a].1 == 0).collect();
    while let Some(a) = ready.pop() {
        let owner = alts[a].0;
        if mem::replace(&mut ground[owner], true) {
            continue;
        }
        for slot in mem::take(&mut waiting[owner]) {
            if let Some(b) = slots[slot].take() {
                alts[b].1 -= 1;
                if alts[b].1 == 0 {
                    ready.push(b);
                }
            }
        }
    }
    ground
}

/// A type with no finite value that holds itself, directly or through other such types, is
/// rejected at its name. A type that merely holds one is not: it has its own error only if it is
/// on the cycle.
pub(super) fn finite_values(
    path: &str,
    program: &Program,
    types: &Types,
    defs: &[Checked],
) -> Vec<(usize, Diagnostic)> {
    let ground = grounded(types, defs);
    let mut refs = Vec::new();
    let mut held = |i: usize| {
        if ground[i] {
            return Vec::new();
        }
        refs.clear();
        (defs[i].item.each_field()).for_each(|f| _ = types.needs(f.ty, &mut refs));
        refs.retain(|&j| !ground[j]);
        mem::take(&mut refs)
    };
    let held: Vec<Vec<usize>> = (0..defs.len()).map(&mut held).collect();
    let cycles = super::graph::components(&held);
    let mut group_of = vec![0; defs.len()];
    for (g, comp) in cycles.iter().enumerate() {
        comp.iter().for_each(|&m| group_of[m] = g);
    }
    let mut out = Vec::new();
    for comp in cycles
        .iter()
        .filter(|c| c.len() > 1 || held[c[0]].contains(&c[0]))
    {
        for &i in comp {
            let blamed = defs[i].item.each_field().find_map(|f| {
                refs.clear();
                types.needs(f.ty, &mut refs);
                let back = refs
                    .iter()
                    .find(|&&j| !ground[j] && group_of[j] == group_of[i]);
                back.map(|&j| (f.name, j))
            });
            let Some((field, other)) = blamed else {
                continue;
            };
            let (name, other) = (program.defs[i].name(), program.defs[other].name().text);
            let message = format!(
                "{0} holds itself through {field} with nothing between; write {field}: Option<{other}> or put it in a List or an enum variant",
                name.text
            );
            let d = Diagnostic::new(path, name.at, name.text.len(), "no-finite-value", message);
            out.push((i, d.in_definition(name.text)));
        }
    }
    out
}
