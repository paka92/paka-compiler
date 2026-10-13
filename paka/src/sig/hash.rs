//! The interface hash (docs/decisions/M2-09.md section 3): a 64-bit FNV-1a over a text that has
//! what an importer can see of a definition and nothing else. A user type stands in a text as its
//! own hash, so a change deep in a chain of types reaches every signature above it. Types that
//! hold each other are one group: the group is hashed as one text with each member written by
//! name, and a member's hash is the group's with the member's name.

use super::{Item, Res, Ty, Types, join};
use crate::ast::{self, Def, Entry, Expr, ExprKind, Program, StrPart, UnOp};
use std::fmt::Write;

type User<'u> = &'u dyn Fn(usize, &mut String);

pub(super) fn fnv(text: &str) -> u64 {
    let step = |hash: u64, byte: u8| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    text.bytes().fold(0xcbf2_9ce4_8422_2325, step)
}

/// A constant as written, with `_` for what a constant cannot be.
fn constant(e: &Expr, out: &mut String) {
    let args = |args: &[ast::Arg], out: &mut String| {
        join(out, args, |out, a| {
            if let Some(label) = &a.label {
                let _ = write!(out, "{}:", label.text);
            }
            constant(&a.value, out);
        });
    };
    match &e.kind {
        ExprKind::Int(s) | ExprKind::Float(s) | ExprKind::Name(s) => out.push_str(s),
        ExprKind::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        ExprKind::Str(parts) => {
            let text: String = (parts.iter())
                .map(|p| {
                    if let StrPart::Text(t) = p {
                        t.as_str()
                    } else {
                        "_"
                    }
                })
                .collect();
            let _ = write!(out, "{text:?}");
        }
        ExprKind::Unary(UnOp::Neg, x) => {
            out.push('-');
            constant(x, out);
        }
        ExprKind::List(items) => {
            out.push('[');
            join(out, items, |out, x| constant(x, out));
            out.push(']');
        }
        ExprKind::Map(pairs) => {
            out.push_str(if pairs.is_empty() { "[:" } else { "[" });
            join(out, pairs, |out, (k, v)| {
                constant(k, out);
                out.push(':');
                constant(v, out);
            });
            out.push(']');
        }
        ExprKind::Record(name, fields) => {
            let _ = write!(out, "{name}{{");
            args(fields, out);
            out.push('}');
        }
        ExprKind::Call(callee, fields) => {
            constant(callee, out);
            out.push('(');
            args(fields, out);
            out.push(')');
        }
        _ => out.push('_'),
    }
}

fn field(types: &Types, user: User, name: &str, ty: Ty, out: &mut String) {
    let _ = write!(out, "{name}:");
    types.render(ty, out, user);
}

fn type_text(def: &Def, item: &Item, types: &Types, user: User, out: &mut String) {
    match (def, item) {
        (Def::Type { name, fields }, Item::Record(have)) => {
            let _ = write!(out, "type {}{{", name.text);
            join(out, have.iter().zip(fields), |out, (f, written)| {
                field(types, user, f.name, f.ty, out);
                if let Some(default) = &written.default {
                    out.push('=');
                    constant(default, out);
                }
            });
            out.push('}');
        }
        (Def::Enum { name, .. }, Item::Enum(variants)) => {
            let _ = write!(out, "enum {}{{", name.text);
            join(out, variants, |out, v| {
                out.push_str(v.name);
                if v.fields.is_empty() {
                    return;
                }
                out.push('(');
                join(out, &v.fields, |out, f| {
                    field(types, user, f.name, f.ty, out)
                });
                out.push(')');
            });
            out.push('}');
        }
        _ => {}
    }
}

fn fn_text(item: &Item, types: &Types, user: User) -> String {
    let Item::Fn(sig) = item else {
        return String::new();
    };
    let mut out = String::from("fn(");
    join(&mut out, &sig.params, |out, p| {
        if p.labelled {
            let _ = write!(out, "{}:", p.name);
        }
        if p.inout {
            out.push_str("inout ");
        }
        types.render(p.ty, out, user);
    });
    out.push(')');
    if let Some(result) = sig.result {
        out.push_str("->");
        types.render(result, &mut out, user);
    }
    out
}

/// The keys of an app in a fixed order, so that a reordering changes nothing.
fn app_text(entries: &[Entry], state: Option<Ty>, types: &Types, user: User) -> String {
    let piece = |entry: &Entry| {
        let mut out = String::new();
        match entry {
            Entry::State(_, n) => {
                out.push_str("state:");
                match state {
                    Some(t) => types.render(t, &mut out, user),
                    None => out.push_str(n.text),
                }
            }
            Entry::Update(_, n) => {
                let _ = write!(out, "update:{}", n.text);
            }
            Entry::Start(_, e) => {
                out.push_str("start:");
                constant(e, &mut out);
            }
            Entry::Grants(_, names) => {
                out.push_str("grants:[");
                join(&mut out, names, |out, n| out.push_str(n.text));
                out.push(']');
            }
        }
        out
    };
    let mut pieces: Vec<_> = entries.iter().map(piece).collect();
    pieces.sort();
    format!("app{{{}}}", pieces.join(", "))
}

/// The hash of every sound type, fn and app. `comps` are the groups of types that hold each
/// other, each after the groups it holds.
pub(super) fn all(
    program: &Program,
    types: &Types,
    defs: &[Res<Item>],
    comps: &[Vec<usize>],
) -> Vec<Option<u64>> {
    let mut hashes = vec![None; defs.len()];
    let mut group_of = vec![0; defs.len()];
    for (g, comp) in comps.iter().enumerate() {
        comp.iter().for_each(|&m| group_of[m] = g);
    }
    // A type of another group, if it has a hash, stands as that hash; any other by its name.
    let by =
        |hashes: &[Option<u64>], inside: Option<usize>, j: usize, out: &mut String| match hashes[j]
        {
            Some(h) if inside != Some(group_of[j]) => drop(write!(out, "#{h:016x}")),
            _ => out.push_str(types.names[j]),
        };
    for (g, comp) in comps.iter().enumerate() {
        let mut members: Vec<_> = (comp.iter().copied())
            .filter(|&m| matches!(&defs[m], Ok(item) if item.is_type()))
            .collect();
        members.sort_by_key(|&m| types.names[m]);
        let mut text = String::new();
        let user = |j, out: &mut String| by(&hashes, Some(g), j, out);
        join(&mut text, &members, |out, &m| {
            let Ok(item) = &defs[m] else { return };
            type_text(&program.defs[m], item, types, &user, out);
        });
        let group = fnv(&text);
        for m in members {
            hashes[m] = Some(fnv(&format!("{group:016x}{}", types.names[m])));
        }
    }
    for (i, def) in program.defs.iter().enumerate() {
        let user = |j, out: &mut String| by(&hashes, None, j, out);
        let text = match (def, &defs[i]) {
            (Def::Fn { .. }, Ok(item)) => fn_text(item, types, &user),
            (Def::App { entries, .. }, Ok(Item::App { state, .. })) => {
                app_text(entries, *state, types, &user)
            }
            _ => continue,
        };
        hashes[i] = Some(fnv(&text));
    }
    hashes
}
