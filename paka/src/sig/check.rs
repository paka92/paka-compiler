//! Checks over the converted definitions together: a type must have a finite value (M2-05
//! section 1.9) and an app's state holds no function type (M2-09 section 1.11).

use super::{Item, Node, Res, Types};
use crate::ast::{Def, Entry, Program};
use crate::diagnostic::Diagnostic;

fn is_type(def: &Res<Item>) -> bool {
    def.as_ref().is_ok_and(Item::is_type)
}

/// For each definition that is a sound type, the sound types its fields mention.
pub(super) fn edges(types: &Types, defs: &[Res<Item>]) -> Vec<Vec<usize>> {
    let mut refs = Vec::new();
    let edge = |def: &Res<Item>| {
        refs.clear();
        let Ok(item) = def else { return Vec::new() };
        item.each_field()
            .for_each(|f| types.refs(f.ty, true, &mut refs));
        refs.iter()
            .map(|r| r.0)
            .filter(|&j| is_type(&defs[j]))
            .collect()
    };
    defs.iter().map(edge).collect()
}

/// Whether a value of the type can be written down: a record when every field can, an enum when
/// some variant has only such fields, a Result when one side can; any other built-in type can,
/// down to an empty List, an empty Map and a `None`. Least fixed point, a group at a time.
fn grounded(types: &Types, defs: &[Res<Item>], comps: &[Vec<usize>]) -> Vec<bool> {
    let mut ground = vec![true; defs.len()];
    let holds = |ground: &[bool], ty| holds(types, ground, ty);
    let eval = |ground: &[bool], item: &Item| match item {
        Item::Record(fields) => fields.iter().all(|f| holds(ground, f.ty)),
        Item::Enum(variants) => variants
            .iter()
            .any(|v| v.fields.iter().all(|f| holds(ground, f.ty))),
        _ => true,
    };
    for comp in comps {
        let members: Vec<_> = comp.iter().filter(|&&m| is_type(&defs[m])).collect();
        members.iter().for_each(|&&m| ground[m] = false);
        let mut changed = !members.is_empty();
        while std::mem::take(&mut changed) {
            for &&m in &members {
                let now = defs[m].as_ref().is_ok_and(|item| eval(&ground, item));
                changed |= now && !ground[m];
                ground[m] |= now;
            }
        }
    }
    ground
}

fn holds(types: &Types, ground: &[bool], ty: super::Ty) -> bool {
    match types.node(ty) {
        Node::User(j) => ground[*j],
        Node::Name("Result", args) => args.iter().any(|&a| holds(types, ground, a)),
        _ => true,
    }
}

/// A type with no finite value that holds itself, directly or through other such types, is
/// rejected at its name. A type that merely holds one is not: it has its own error only if it is
/// on the cycle.
pub(super) fn finite_values(
    path: &str,
    program: &Program,
    types: &Types,
    defs: &[Res<Item>],
    comps: &[Vec<usize>],
) -> Vec<(usize, Diagnostic)> {
    let ground = grounded(types, defs, comps);
    let ungrounded = |j: usize| is_type(&defs[j]) && !ground[j];
    let mut refs = Vec::new();
    let mut held = |i: usize| {
        let Ok(item) = &defs[i] else {
            return Vec::new();
        };
        if !ungrounded(i) {
            return Vec::new();
        }
        refs.clear();
        item.each_field()
            .for_each(|f| types.refs(f.ty, true, &mut refs));
        refs.iter()
            .filter(|r| r.1 && ungrounded(r.0))
            .map(|r| r.0)
            .collect()
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
            let Ok(item) = &defs[i] else { continue };
            let blamed = item.each_field().find_map(|f| {
                refs.clear();
                types.refs(f.ty, true, &mut refs);
                let back = refs
                    .iter()
                    .find(|r| r.1 && ungrounded(r.0) && group_of[r.0] == group_of[i]);
                back.map(|r| (f.name, r.0))
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

/// The first field, looking from the type `root` through the types its fields mention, whose type
/// holds a function type.
fn function_field<'d>(types: &Types, defs: &'d [Res<Item>], root: usize) -> Option<&'d str> {
    let mut seen = vec![false; defs.len()];
    seen[root] = true;
    let (mut queue, mut refs) = (vec![root], Vec::new());
    let mut next = 0;
    while let Some(&t) = queue.get(next) {
        next += 1;
        let Ok(item) = &defs[t] else { continue };
        for f in item.each_field() {
            if types.has_fn(f.ty) {
                return Some(f.name);
            }
            refs.clear();
            types.refs(f.ty, true, &mut refs);
            for &(j, _) in &refs {
                if !std::mem::replace(&mut seen[j], true) {
                    queue.push(j);
                }
            }
        }
    }
    None
}

/// An app whose state holds a function type is rejected at the type named after `state:`.
pub(super) fn state_functions(
    path: &str,
    program: &Program,
    types: &Types,
    defs: &[Res<Item>],
) -> Vec<(usize, Diagnostic)> {
    let mut out = Vec::new();
    for (i, def) in program.defs.iter().enumerate() {
        let (
            Def::App { name, entries },
            Ok(Item::App {
                state: Some(state), ..
            }),
        ) = (def, &defs[i])
        else {
            continue;
        };
        let (Node::User(root), Some(Entry::State(_, written))) = (
            types.node(*state),
            entries.iter().find(|e| matches!(e, Entry::State(..))),
        ) else {
            continue;
        };
        let Some(field) = function_field(types, defs, *root) else {
            continue;
        };
        let message = format!(
            "state {} holds a function type in field {field}; keep a message value instead and match on it in update",
            written.text
        );
        let d = Diagnostic::new(
            path,
            written.at,
            written.text.len(),
            "type-mismatch",
            message,
        );
        out.push((i, d.in_definition(name.text)));
    }
    out
}
