//! A migrate checked against two versions of a program (docs/decisions/M5-07.md sections 1.3, 1.4
//! and 2). It is checked as one more function of the new program, which also holds the running
//! version of each changed stored type under the name `old Name`, one no source can write. The
//! parameter's type and each variant a pattern names take that version; every other name in the
//! body, type names too, takes the new one. To the checker an old value is then of a type of its
//! own; what the body may do with one is checked here.

use crate::ast::{ArmBody, BinOp, Block, Def, Entry, Expr, ExprKind as K, Pat, PatKind, Pos};
use crate::ast::{StmtKind, StrPart, Type};
use crate::core_form::{FnId, Program};
use crate::diagnostic::Diagnostic;
use crate::parser::{parse_def, parse_files, parse_migrate, split};
use crate::sig::{Node, Ty, Types};
use crate::source::Source;
use crate::{body, lower, resolve, sig};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const PATH: &str = "main.paka";

/// A checked migrate: the new program with the old types and then the migrate added after its own
/// definitions, so the new program's ids hold in it. `function` is the migrate.
pub struct Migrate {
    pub program: Program,
    pub function: FnId,
}

/// Checks `text`, a migrate from the running version `old` to `new`, a program that checks.
/// Positions in an error are in `text`.
pub fn check_migrate(old: &Source, new: &Source, text: &str) -> Result<Migrate, Diagnostic> {
    let (old, new_version) = (Version::of(old)?, Version::of(new)?);
    let stored = old.stored();
    let changed: Vec<_> = (old.changed(&new_version).into_iter())
        .filter(|t| stored.contains(t))
        .collect();
    let mut aliases = BTreeMap::new();
    let mut old_text = vec![];
    for &name in &changed {
        let (text, def) = &old.types[name];
        aliases.insert(name, format!("old {name}"));
        if let Def::Enum { variants, .. } = def {
            aliases.extend(
                variants
                    .iter()
                    .map(|v| (v.name.text, format!("old {}", v.name.text))),
            );
        }
        old_text.extend_from_slice(text);
        old_text.extend_from_slice(b"\n\n");
    }
    let mut source = Source::default();
    for f in &new.files {
        source.add(f.path.clone(), new.text[f.bytes.clone()].to_vec());
    }
    source.add("old types".into(), old_text);
    source.add("migrate".into(), text.as_bytes().to_vec());
    let aliases: BTreeMap<&str, &str> = aliases.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let states = (old.state, new_version.state);
    let checked = check(&source, &aliases, &changed, states, &old);
    checked.map_err(|d| source.locate(d).0.in_definition("migrate"))
}

/// The report of `app`'s migrate stopped past `limit`, which counts as a trap (M5-07 section 2).
pub fn late(app: &str, limit: Duration) -> String {
    format!(
        "{app}: stopped time in migrate, past {} ms\n",
        limit.as_millis()
    )
}

/// The migrate in the last file of `source`, after the new program and the old types.
fn check(
    source: &Source,
    aliases: &BTreeMap<&str, &str>,
    changed: &[&str],
    (old_state, new_state): (Option<&str>, Option<&str>),
    old: &Version,
) -> Result<Migrate, Diagnostic> {
    let [new @ .., olds, last] = &source.files[..] else {
        unreachable!("a migrate's source holds the program, the old types and the migrate");
    };
    let range = |f: &crate::source::File| (f.bytes.clone(), f.line);
    let mut program = parse_files(PATH, &source.text, new.iter().map(range))?;
    let first_old = program.defs.len();
    let old_defs = parse_files(PATH, &source.text, [range(olds)])?;
    program.defs.extend(old_defs.defs);
    program.lines.extend(old_defs.lines);
    for def in &mut program.defs[first_old..] {
        rename_def(def, aliases);
    }
    let mut def = parse_migrate(
        PATH,
        &source.text[..last.bytes.end],
        (last.bytes.start, last.line),
    )?;
    signature(&def, old_state, new_state)?;
    if let Some(&t) = changed.iter().find(|t| old.reached(t).contains(*t)) {
        let message =
            format!("{t} holds itself and changed, so no migrate can convert it; send reset");
        return Err(at_param(&def, "migrate-needs-reset", message));
    }
    let mut walk = Walk {
        aliases,
        uses: vec![],
    };
    if let Def::Fn {
        name, params, body, ..
    } = &mut def
    {
        name.text = "(migrate)";
        rename(&mut params[0].ty, aliases);
        walk.block(body);
    }
    program.defs.push(def);
    program.lines.push(last.line);
    let resolved = resolve::resolve(PATH, &program)?;
    let mut table = sig::Table::build(PATH, &program, &resolved);
    let bodies = body::check(PATH, &program, &resolved, &mut table);
    if let Some(d) = bodies.defs.iter().flatten().next() {
        return Err(d.clone());
    }
    let olds: BTreeSet<usize> = (first_old..program.defs.len() - 1).collect();
    let types = bodies.types.last().expect("the migrate is checked");
    if let Some(d) = misuse(&table.types, types, &walk.uses, &olds) {
        return Err(d);
    }
    let files = (source.files.iter())
        .map(|f| (f.name.clone(), f.line))
        .collect();
    let (program, _) = lower::lower(files, &source.text, &program, &resolved, &table, &bodies);
    let function = program.fns.len() - 1;
    Ok(Migrate { program, function })
}

/// One parameter, read only, of the running version's state type, and the new state as result.
fn signature(
    def: &Def,
    old_state: Option<&str>,
    new_state: Option<&str>,
) -> Result<(), Diagnostic> {
    let Def::Fn {
        name,
        params,
        result,
        ..
    } = def
    else {
        unreachable!("a migrate parses as a function");
    };
    let fail = |at: Pos, len: usize, message: String| {
        Err(Diagnostic::new(
            PATH,
            at,
            len,
            "bad-migrate-signature",
            message,
        ))
    };
    let (old_state, new_state) = (old_state.unwrap_or("none"), new_state.unwrap_or("none"));
    let [param] = &params[..] else {
        let message = format!("a migrate takes one parameter: migrate(old: {old_state})");
        return fail(name.at, name.text.len(), message);
    };
    if param.inout || param.labelled {
        let message = format!(
            "a migrate reads its parameter: write {}: {old_state}",
            param.name.text
        );
        return fail(param.name.at, param.name.text.len(), message);
    }
    if !is_named(&param.ty, old_state) {
        let message = format!("the parameter is the running version's state: write {old_state}");
        return Err(at_param(def, "bad-migrate-signature", message));
    }
    match result {
        Some(t) if is_named(t, new_state) => Ok(()),
        Some(t) => {
            let message = format!("a migrate gives the new version's state: write {new_state}");
            fail(type_at(t), t.to_string().len(), message)
        }
        None => {
            let message = format!("a migrate gives the new state: write -> {new_state}");
            fail(name.at, name.text.len(), message)
        }
    }
}

fn is_named(t: &Type, state: &str) -> bool {
    matches!(t, Type::Name { name, args } if name.text == state && args.is_empty())
}

fn type_at(t: &Type) -> Pos {
    match t {
        Type::Name { name, .. } => name.at,
        Type::Fn { at, .. } => *at,
    }
}

/// An error at the migrate's parameter type.
fn at_param(def: &Def, code: &'static str, message: String) -> Diagnostic {
    let Def::Fn { params, .. } = def else {
        unreachable!("a migrate parses as a function");
    };
    let t = &params[0].ty;
    Diagnostic::new(PATH, type_at(t), t.to_string().len(), code, message)
}

/// The first use of a value the body may not make, in source order: an effect built, or an old
/// value passed to a function or compared.
fn misuse(
    types: &Types,
    met: &BTreeMap<(Pos, usize), Ty>,
    uses: &[((Pos, usize), Use)],
    olds: &BTreeSet<usize>,
) -> Option<Diagnostic> {
    let effect = |n: &Node| matches!(n, Node::Name("Effect", _));
    let old = |n: &Node| matches!(n, Node::User(i) if olds.contains(i));
    let effects = (met.iter())
        .filter(|(_, ty)| holds(types, **ty, &effect))
        .map(|(&(at, len), &ty)| {
            let message = format!("a migrate is pure: it builds no {}", types.show(ty));
            Diagnostic::new(PATH, at, len, "effect-in-migrate", message)
        });
    let misused = (uses.iter())
        .filter_map(|&(span, how)| Some((span, how, *met.get(&span)?)))
        .filter(|&(_, _, ty)| holds(types, ty, &old))
        .map(|((at, len), how, ty)| {
            let what = match how {
                Use::Passed => "passed to a function",
                Use::Compared => "compared",
            };
            let message = format!(
                "this {} is of the running version: it cannot be {what}, only taken apart: \
                 read its fields, match it, loop over it or index it",
                types.show(ty)
            );
            Diagnostic::new(PATH, at, len, "old-value-use", message)
        });
    effects.chain(misused).min_by_key(|d| d.position)
}

/// Whether `ty` or a type in it is one `hit` picks.
fn holds(types: &Types, ty: Ty, hit: &impl Fn(&Node) -> bool) -> bool {
    let node = types.node(ty);
    hit(node)
        || match node {
            Node::Name(_, args) => args.iter().any(|&t| holds(types, t, hit)),
            Node::Fn(params, result) => {
                (params.iter().chain(result)).any(|&t| holds(types, t, hit))
            }
            Node::User(_) | Node::Var(_) => false,
        }
}

/// An old type under its alias, holding old types where it holds changed ones; defaults dropped.
fn rename_def<'a>(def: &mut Def<'a>, aliases: &BTreeMap<&str, &'a str>) {
    let rename_name = |name: &mut crate::ast::Name<'a>| name.text = aliases[name.text];
    match def {
        Def::Type { name, fields } => {
            rename_name(name);
            for f in fields {
                rename(&mut f.ty, aliases);
                f.default = None;
            }
        }
        Def::Enum { name, variants } => {
            rename_name(name);
            for v in variants {
                rename_name(&mut v.name);
                v.fields.iter_mut().for_each(|f| rename(&mut f.ty, aliases));
            }
        }
        _ => unreachable!("only types are carried over"),
    }
}

fn rename<'a>(t: &mut Type<'a>, aliases: &BTreeMap<&str, &'a str>) {
    match t {
        Type::Name { name, args } => {
            if let Some(alias) = aliases.get(name.text) {
                name.text = alias;
            }
            args.iter_mut().for_each(|t| rename(t, aliases));
        }
        Type::Fn { params, result, .. } => {
            params.iter_mut().for_each(|t| rename(t, aliases));
            result.iter_mut().for_each(|t| rename(t, aliases));
        }
    }
}

#[derive(Clone, Copy)]
enum Use {
    Passed,
    Compared,
}

/// A walk of the migrate's body before it is checked: patterns take the old variants, and each
/// argument and compared operand is noted by its span.
struct Walk<'m, 'a> {
    aliases: &'m BTreeMap<&'m str, &'a str>,
    uses: Vec<((Pos, usize), Use)>,
}

impl<'a> Walk<'_, 'a> {
    fn block(&mut self, b: &mut Block<'a>) {
        for s in &mut b.stmts {
            self.stmt(&mut s.kind);
        }
    }

    fn stmt(&mut self, s: &mut StmtKind<'a>) {
        match s {
            StmtKind::Let { value: e, .. } | StmtKind::Discard(e) | StmtKind::Expr(e) => {
                self.expr(e)
            }
            StmtKind::Return(e) => e.iter_mut().for_each(|e| self.expr(e)),
            StmtKind::Assign { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            StmtKind::For { list: e, body, .. } | StmtKind::While { cond: e, body } => {
                self.expr(e);
                self.block(body);
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn expr(&mut self, e: &mut Expr<'a>) {
        match &mut e.kind {
            K::Int(_) | K::Float(_) | K::Bool(_) | K::Hole(_) | K::Name(_) => {}
            K::Str(parts) => (parts.iter_mut()).for_each(|p| match p {
                StrPart::Expr(x) => self.expr(x),
                StrPart::Text(_) => {}
            }),
            K::Record(_, args) => args.iter_mut().for_each(|a| self.expr(&mut a.value)),
            K::List(items) => items.iter_mut().for_each(|x| self.expr(x)),
            K::Map(pairs) => (pairs.iter_mut()).for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            K::Unary(_, x) | K::Field(x, _) | K::Try(x, _) | K::Inout(x) => self.expr(x),
            K::Binary(op, _, l, r) => {
                if matches!(op, BinOp::Eq | BinOp::Ne) {
                    self.uses.push(((l.at, l.len), Use::Compared));
                    self.uses.push(((r.at, r.len), Use::Compared));
                }
                self.expr(l);
                self.expr(r);
            }
            K::Range(a, b) | K::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            K::Call(f, args) => {
                self.expr(f);
                for a in args {
                    self.uses.push(((a.value.at, a.value.len), Use::Passed));
                    self.expr(&mut a.value);
                }
            }
            K::If {
                branches,
                otherwise,
            } => {
                for (cond, b) in branches {
                    self.expr(cond);
                    self.block(b);
                }
                otherwise.iter_mut().for_each(|b| self.block(b));
            }
            K::Match { subject, arms } => {
                self.expr(subject);
                for arm in arms {
                    arm.patterns.iter_mut().for_each(|p| self.pat(p));
                    match &mut arm.body {
                        ArmBody::Block(b) => self.block(b),
                        ArmBody::Line(s) => self.stmt(&mut s.kind),
                    }
                }
            }
            K::Children(call, b) => {
                self.expr(call);
                self.block(b);
            }
        }
    }

    fn pat(&mut self, p: &mut Pat<'a>) {
        if let PatKind::Variant(name, parts) = &mut p.kind {
            if let Some(alias) = self.aliases.get(name) {
                *name = alias;
            }
            parts.iter_mut().for_each(|p| self.pat(p));
        }
    }
}

/// A version's state type and its records and enums by name, each with its text.
struct Version<'s> {
    state: Option<&'s str>,
    types: BTreeMap<&'s str, (&'s [u8], Def<'s>)>,
}

impl<'s> Version<'s> {
    fn of(source: &'s Source) -> Result<Version<'s>, Diagnostic> {
        let mut version = Version {
            state: None,
            types: BTreeMap::new(),
        };
        for file in &source.files {
            for (_, text) in split(PATH, &source.text[file.bytes.clone()])? {
                match parse_def(PATH, text)? {
                    def @ (Def::Type { .. } | Def::Enum { .. }) => {
                        version.types.insert(def.name().text, (text, def));
                    }
                    Def::App { entries, .. } => {
                        version.state = entries.iter().find_map(|e| match e {
                            Entry::State(_, n) => Some(n.text),
                            _ => None,
                        })
                    }
                    _ => {}
                }
            }
        }
        Ok(version)
    }

    /// The state type and every type it reaches.
    fn stored(&self) -> BTreeSet<&'s str> {
        let mut stored = self.state.map_or_else(BTreeSet::new, |s| self.reached(s));
        stored.extend(self.state.filter(|s| self.types.contains_key(s)));
        stored
    }

    /// The types reached from `name` through what each holds; `name` only when it holds itself.
    fn reached(&self, name: &str) -> BTreeSet<&'s str> {
        let mut reached = BTreeSet::new();
        let mut next = self.held(name);
        while let Some(t) = next.pop() {
            if reached.insert(t) {
                next.extend(self.held(t));
            }
        }
        reached
    }

    /// The types of this version that the fields and payloads of type `name` name.
    fn held(&self, name: &str) -> Vec<&'s str> {
        let mut held = vec![];
        if let Some((_, def)) = self.types.get(name) {
            shape(def)
                .1
                .iter()
                .for_each(|(_, types)| types.iter().for_each(|t| names(t, &mut held)));
        }
        held.retain(|t| self.types.contains_key(t));
        held
    }

    /// The types that differ from `new` in anything but defaults, or hold one that does.
    fn changed(&self, new: &Version) -> BTreeSet<&'s str> {
        let differs = |name: &str, def: &Def| {
            new.types
                .get(name)
                .is_none_or(|(_, n)| spelled(n) != spelled(def))
        };
        let mut changed: BTreeSet<_> = (self.types.iter())
            .filter(|(name, (_, def))| differs(name, def))
            .map(|(name, _)| *name)
            .collect();
        loop {
            let more: Vec<_> = (self.types.keys())
                .filter(|t| !changed.contains(*t))
                .filter(|t| self.held(t).iter().any(|h| changed.contains(h)))
                .copied()
                .collect();
            if more.is_empty() {
                return changed;
            }
            changed.extend(more);
        }
    }
}

/// Whether a type is a record, and its fields or its variants, each with the types it holds.
fn shape<'d, 's>(def: &'d Def<'s>) -> (bool, Vec<(&'s str, Vec<&'d Type<'s>>)>) {
    match def {
        Def::Type { fields, .. } => {
            let parts = fields.iter().map(|f| (f.name.text, vec![&f.ty]));
            (true, parts.collect())
        }
        Def::Enum { variants, .. } => {
            let held = |v: &'d crate::ast::Variant<'s>| v.fields.iter().map(|f| &f.ty).collect();
            (
                false,
                variants.iter().map(|v| (v.name.text, held(v))).collect(),
            )
        }
        _ => (false, vec![]),
    }
}

/// A type's shape as written, defaults aside.
fn spelled(def: &Def) -> (bool, Vec<(String, Vec<String>)>) {
    let (record, parts) = shape(def);
    let spell =
        |(n, ts): (&str, Vec<&Type>)| (n.to_string(), ts.iter().map(|t| t.to_string()).collect());
    (record, parts.into_iter().map(spell).collect())
}

/// Pushes every type name `t` writes.
fn names<'s>(t: &Type<'s>, out: &mut Vec<&'s str>) {
    match t {
        Type::Name { name, args } => {
            out.push(name.text);
            args.iter().for_each(|t| names(t, out));
        }
        Type::Fn { params, result, .. } => {
            params.iter().for_each(|t| names(t, out));
            result.iter().for_each(|t| names(t, out));
        }
    }
}
