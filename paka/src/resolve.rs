//! Name resolution (docs/decisions/M2-02.md section 1.3, M2-14.md section 1.2). Binds every use of
//! a name to its definition and reports duplicate and unknown names, with the nearest known name as
//! the fix. No types are looked at, so a field read `x.f` and the labels of calls and records are
//! left to the checker. A local no use names is the `unused-name` warning (M2-02 section 5).

use crate::ast::{
    Arm, ArmBody, Block, Def, Entry, Expr, ExprKind, Field, Name, Pat, PatKind, Pos, Program, Stmt,
    StmtKind, StrPart, Type,
};
use crate::builtins::{self, Kind};
use crate::diagnostic::Diagnostic;
use crate::source::in_file;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

type Res<T = ()> = Result<T, Diagnostic>;

/// What a use of a name stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {
    /// A `let`, `var`, parameter, `for` name or pattern name, by the position of its name.
    Local(Pos),
    /// A `type`, `enum`, `fn`, `example` or `app`, by its index in `Program::defs`.
    Def(usize),
    /// A variant: its enum's index in `Program::defs`, then its index in the enum.
    Variant(usize, usize),
    /// A name of the built-in table.
    Builtin(&'static str),
}

/// The outcome for a program whose declarations are sound. A name error in one definition never
/// hides or changes the result of another.
#[derive(Debug)]
pub struct Resolved {
    uses: BTreeMap<Pos, Binding>,
    /// The first name error of each definition, by index. An app's name errors are not here but in
    /// `entry_error`, so its result is always `Ok`.
    pub defs: Vec<Res>,
    entries: BTreeMap<Pos, Diagnostic>,
    /// An `unused-name` warning per local that no use names, in source order.
    pub unused: Vec<Diagnostic>,
    /// The names each definition looked up: the definitions they name, as for a variant its
    /// enum, and as written those that name nothing (M4-09). Built-ins and locals are left out.
    pub reads: Vec<BTreeSet<String>>,
}

impl Resolved {
    pub fn get(&self, at: Pos) -> Option<Binding> {
        self.uses.get(&at).copied()
    }

    /// The name error of an app entry's value, by the position where the value starts: the name
    /// after `state:` or `update:`, the first token after `start:`, each name of `grants:`. The
    /// checker reports it at its turn in the order of M2-15 section 1.1, after its own key tests.
    pub fn entry_error(&self, at: Pos) -> Option<&Diagnostic> {
        self.entries.get(&at)
    }
}

const VALUES: &[Kind] = &[Kind::Fn, Kind::Variant];
const TYPES: &[Kind] = &[Kind::Type];
const VARIANTS: &[Kind] = &[Kind::Variant];
const GRANTS: &[Kind] = &[Kind::Grant];

/// Selects the pool entries of the given kinds.
fn of(kinds: &'static [Kind]) -> impl Fn(Kind, Binding) -> bool {
    move |kind, _| kinds.contains(&kind)
}

/// Declaration errors come first, as `Err`; then each definition is resolved on its own.
pub fn resolve<'a>(path: &'a str, program: &'a Program<'a>) -> Res<Resolved> {
    let mut resolver = Resolver {
        path,
        program,
        pool: builtins(),
        locals: Vec::new(),
        bound: Vec::new(),
        uses: BTreeMap::new(),
        entries: BTreeMap::new(),
        reads: RefCell::default(),
    };
    for (i, def) in program.defs.iter().enumerate() {
        resolver
            .declare_def(i, def)
            .map_err(|d| d.in_definition(def.name().text))?;
    }
    let (mut defs, mut bound, mut reads) = (Vec::new(), Vec::new(), Vec::new());
    for def in &program.defs {
        let name = def.name().text;
        defs.push(resolver.def(def).map_err(|d| d.in_definition(name)));
        reads.push(resolver.reads.take());
        bound.extend(resolver.bound.drain(..).map(|local| (local, name)));
    }
    let used: BTreeSet<Pos> = (resolver.uses.values())
        .filter_map(|b| match b {
            Binding::Local(at) => Some(*at),
            _ => None,
        })
        .collect();
    let unused = bound.into_iter().filter(|((_, at), _)| !used.contains(at));
    let warn = |((local, at), def): ((&str, Pos), &str)| {
        let message = format!("{local} is never used");
        Diagnostic::new(path, at, local.len(), "unused-name", message).in_definition(def)
    };
    Ok(Resolved {
        uses: resolver.uses,
        defs,
        entries: resolver.entries,
        unused: unused.map(warn).collect(),
        reads,
    })
}

fn builtins<'a>() -> BTreeMap<&'a str, (Kind, Binding)> {
    let rows = builtins::ROWS.iter().map(|&(kind, row)| {
        let name = builtins::name(row);
        (name, (kind, Binding::Builtin(name)))
    });
    rows.collect()
}

/// The candidate closest to `name` by edit distance, if within a third of its length. Ties go to
/// the first in byte order; only names of the same letter case count (case carries meaning,
/// M2-02.3).
fn nearest<'n>(name: &str, candidates: impl Iterator<Item = &'n str>) -> Option<&'n str> {
    let near = candidates.filter_map(|c| near(name, c).map(|d| (d, c)));
    near.min().map(|(_, best)| best)
}

/// The distance from `name` to `candidate` when it is near enough to be suggested.
pub(crate) fn near(name: &str, candidate: &str) -> Option<usize> {
    let upper = |s: &str| s.starts_with(char::is_uppercase);
    let d = (upper(name) == upper(candidate)).then(|| distance(name, candidate))?;
    (d <= name.len() / 3).then_some(d)
}

/// An error at `name` with the message `head`; when a candidate is near, the message ends with
/// the hint and the fix is the candidate.
pub(crate) fn suggestion<'n>(
    path: &str,
    code: &'static str,
    (at, name): (Pos, &str),
    head: &str,
    candidates: impl Iterator<Item = &'n str>,
) -> Diagnostic {
    let fail = |message: &str| Diagnostic::new(path, at, name.len(), code, message);
    match nearest(name, candidates) {
        Some(near) => fail(&format!("{head}; did you mean {near}?")).with_fix(near),
        None => fail(head),
    }
}

/// The message for a name from another language's error handling, with what to write instead
/// (M2-12 section 1.9).
pub(crate) fn habit(name: &str) -> Option<String> {
    let hint = match name {
        "null" | "nil" => return Some(format!("Paka has no {name}; write None and a match")),
        "unwrap" | "expect" => "write a match, `?` or unwrap_or",
        "is_some" | "is_none" | "is_ok" | "is_err" => "match it",
        "panic" => "write unreachable() or assert(false)",
        _ => return None,
    };
    Some(format!("{name} is not defined here; {hint}"))
}

fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.bytes().enumerate() {
        let mut diagonal = std::mem::replace(&mut row[0], i + 1);
        for (j, y) in b.bytes().enumerate() {
            let next = (row[j] + 1)
                .min(row[j + 1] + 1)
                .min(diagonal + usize::from(x != y));
            diagonal = std::mem::replace(&mut row[j + 1], next);
        }
    }
    row[b.len()]
}

/// The operand a chain continues from: the first operand of an operator, the callee, the receiver.
fn first_operand<'a>(e: &'a Expr<'a>) -> Option<&'a Expr<'a>> {
    match &e.kind {
        ExprKind::Unary(_, x)
        | ExprKind::Try(x, _)
        | ExprKind::Inout(x)
        | ExprKind::Field(x, _)
        | ExprKind::Binary(_, _, x, _)
        | ExprKind::Range(x, _)
        | ExprKind::Index(x, _)
        | ExprKind::Call(x, _) => Some(x.as_ref()),
        _ => None,
    }
}

struct Resolver<'a> {
    path: &'a str,
    program: &'a Program<'a>,
    /// Definitions, variants and built-ins: one pool (M2-02.2).
    pool: BTreeMap<&'a str, (Kind, Binding)>,
    /// Every local visible at this point, outermost first, with the position of its name.
    locals: Vec<(&'a str, Pos)>,
    /// Every local the definition being resolved has bound so far.
    bound: Vec<(&'a str, Pos)>,
    uses: BTreeMap<Pos, Binding>,
    entries: BTreeMap<Pos, Diagnostic>,
    reads: RefCell<BTreeSet<String>>,
}

impl<'a> Resolver<'a> {
    fn fail(&self, at: Pos, len: usize, code: &'static str, message: String) -> Diagnostic {
        Diagnostic::new(self.path, at, len, code, message)
    }

    /// `name` is `what`; when a candidate is near, it is the message's hint and the fix.
    fn suggest<'n>(
        &self,
        code: &'static str,
        (at, name): (Pos, &str),
        what: &str,
        candidates: impl Iterator<Item = &'n str>,
    ) -> Diagnostic {
        let head = format!("{name} is {what}");
        let near = nearest(name, candidates);
        if let Some(near) = near.filter(|n| self.pool.contains_key(n)) {
            self.pool_get(near, |_, _| true);
        }
        suggestion(self.path, code, (at, name), &head, near.into_iter())
    }

    /// Logs the lookup in `reads`: the definition found, or `name` when nothing is.
    fn pool_get(&self, name: &str, keep: impl Fn(Kind, Binding) -> bool) -> Option<Binding> {
        let found = self.pool.get(name);
        let read = match found {
            Some((_, Binding::Def(i) | Binding::Variant(i, _))) => {
                Some(self.program.defs[*i].name().text)
            }
            Some(_) => None,
            None => Some(name),
        };
        if let Some(read) = read.filter(|r| !self.reads.borrow().contains(*r)) {
            self.reads.borrow_mut().insert(read.into());
        }
        let &(kind, binding) = found?;
        keep(kind, binding).then_some(binding)
    }

    fn names<'s>(
        &'s self,
        keep: impl Fn(Kind, Binding) -> bool + 's,
    ) -> impl Iterator<Item = &'a str> + 's {
        let wanted = self.pool.iter().filter(move |&(_, &(k, b))| keep(k, b));
        wanted.map(|(name, _)| *name)
    }

    fn record(&mut self, at: Pos, found: Binding) -> Res {
        self.uses.insert(at, found);
        Ok(())
    }

    /// Looks `name` up in the pool and records the use.
    fn pool_use(&mut self, name: &str, at: Pos, kinds: &'static [Kind]) -> Option<Binding> {
        let found = self.pool_get(name, of(kinds))?;
        self.uses.insert(at, found);
        Some(found)
    }

    fn declare(&mut self, name: &'a str, at: Pos, kind: Kind, binding: Binding) -> Res {
        let Some((_, old)) = self.pool.insert(name, (kind, binding)) else {
            return Ok(());
        };
        let message = match old {
            Binding::Builtin(_) => format!("{name} is a built-in; rename it"),
            _ => format!("{name} is already declared"),
        };
        Err(self.fail(at, name.len(), "duplicate-name", message))
    }

    fn declare_def(&mut self, i: usize, def: &'a Def<'a>) -> Res {
        let name = def.name();
        let kind = match def {
            Def::Type { .. } | Def::Enum { .. } => Kind::Type,
            Def::Fn { .. } => Kind::Fn,
            Def::Example { .. } | Def::App { .. } => Kind::Other,
            Def::View { .. } => unreachable!("check refuses views"),
        };
        self.declare(name.text, name.at, kind, Binding::Def(i))?;
        let Def::Enum { variants, .. } = def else {
            return Ok(());
        };
        for (j, v) in variants.iter().enumerate() {
            self.declare(
                v.name.text,
                v.name.at,
                Kind::Variant,
                Binding::Variant(i, j),
            )?;
        }
        Ok(())
    }

    fn def(&mut self, def: &'a Def<'a>) -> Res {
        match def {
            Def::Type { fields, .. } => self.fields(fields),
            Def::Enum { variants, .. } => variants.iter().try_for_each(|v| self.fields(&v.fields)),
            Def::Fn {
                params,
                result,
                body,
                ..
            } => self.scoped(|r| {
                for p in params {
                    r.ty(&p.ty)?;
                    r.bind(p.name.text, p.name.at)?;
                }
                result.iter().try_for_each(|t| r.ty(t))?;
                r.block(body)
            }),
            Def::Example { body, .. } => self.block(body),
            Def::App { name, entries } => {
                self.app(name.text, entries);
                Ok(())
            }
            Def::View { .. } => unreachable!("check refuses views"),
        }
    }

    /// A field name is held once per type or variant; its type and default are uses.
    fn fields(&mut self, fields: &'a [Field<'a>]) -> Res {
        for (i, f) in fields.iter().enumerate() {
            if fields[..i].iter().any(|g| g.name.text == f.name.text) {
                let message = format!("{} is already a field here", f.name.text);
                return Err(self.fail(f.name.at, f.name.text.len(), "duplicate-name", message));
            }
            self.ty(&f.ty)?;
            f.default.iter().try_for_each(|d| self.expr(d))?;
        }
        Ok(())
    }

    /// Each entry value is resolved alone and its error kept in `entries`, so the checker can order
    /// them after its tests of the keys (`Resolved::entry_error`).
    fn app(&mut self, app: &str, entries: &'a [Entry<'a>]) {
        for entry in entries {
            let results = match entry {
                Entry::State(_, n) => vec![(n.at, self.ty_name(n.text, n.at))],
                Entry::Update(_, n) => vec![(n.at, self.update(n.text, n.at))],
                Entry::Start(_, e) => vec![(e.at, self.expr(e))],
                Entry::Grants(_, names) => {
                    let grant = |n: &Name| (n.at, self.grant(n.text, n.at));
                    names.iter().map(grant).collect()
                }
                Entry::View(..) => unreachable!("check refuses views"),
            };
            for (at, result) in results {
                if let Err(d) = result {
                    self.entries.insert(at, d.in_definition(app));
                }
            }
        }
    }

    fn update(&mut self, name: &str, at: Pos) -> Res {
        let own = |k, b| k == Kind::Fn && matches!(b, Binding::Def(_));
        match self.pool_get(name, own) {
            Some(found) => self.record(at, found),
            None => {
                let what = "not a function of the program";
                Err(self.suggest("unknown-name", (at, name), what, self.names(own)))
            }
        }
    }

    fn grant(&mut self, name: &str, at: Pos) -> Res {
        match self.pool_use(name, at, GRANTS) {
            Some(_) => Ok(()),
            None => {
                let near = self.names(of(GRANTS));
                Err(self.suggest("unknown-grant", (at, name), "not a grant", near))
            }
        }
    }

    fn ty_name(&mut self, name: &str, at: Pos) -> Res {
        match self.pool_use(name, at, TYPES) {
            Some(_) => Ok(()),
            None => {
                let near = self.names(of(TYPES));
                Err(self.suggest("unknown-type", (at, name), "not a type", near))
            }
        }
    }

    /// Outside in: the name, then the number of arguments, then the arguments.
    fn ty(&mut self, t: &'a Type<'a>) -> Res {
        match t {
            Type::Name { name, args } => {
                self.ty_name(name.text, name.at)?;
                let want = builtins::arity(name.text);
                if args.len() != want {
                    let s = if want == 1 { "" } else { "s" };
                    let message = format!("{} takes {want} type argument{s}", name.text);
                    return Err(self.fail(
                        name.at,
                        name.text.len(),
                        "wrong-type-arguments",
                        message,
                    ));
                }
                args.iter().try_for_each(|a| self.ty(a))
            }
            Type::Fn { params, result, .. } => {
                params.iter().try_for_each(|p| self.ty(p))?;
                result.iter().try_for_each(|r| self.ty(r))
            }
        }
    }

    /// Runs `f`, then forgets the locals it bound.
    fn scoped(&mut self, f: impl FnOnce(&mut Self) -> Res) -> Res {
        let mark = self.locals.len();
        let result = f(self);
        self.locals.truncate(mark);
        result
    }

    /// A local may not reuse a name visible at that point (M2-02.3).
    fn bind(&mut self, name: &'a str, at: Pos) -> Res {
        if let Some(&(_, first)) = self.locals.iter().find(|l| l.0 == name) {
            let (_, line) = in_file(&self.program.lines, |&l| l, first.0);
            let message = format!("{name} is already bound at line {line}");
            return Err(self.fail(at, name.len(), "duplicate-local", message));
        }
        self.locals.push((name, at));
        self.bound.push((name, at));
        Ok(())
    }

    fn block(&mut self, block: &'a Block<'a>) -> Res {
        self.scoped(|r| block.stmts.iter().try_for_each(|s| r.stmt(s)))
    }

    /// A `let` binds its name after its type and value: the value cannot see it.
    fn stmt(&mut self, stmt: &'a Stmt<'a>) -> Res {
        match &stmt.kind {
            StmtKind::Let {
                name, ty, value, ..
            } => {
                ty.iter().try_for_each(|t| self.ty(t))?;
                self.expr(value)?;
                self.bind(name.text, name.at)
            }
            StmtKind::Discard(e) | StmtKind::Expr(e) => self.expr(e),
            StmtKind::Assign { target, value, .. } => {
                self.expr(target)?;
                self.expr(value)
            }
            StmtKind::Return(value) => value.iter().try_for_each(|e| self.expr(e)),
            StmtKind::Break | StmtKind::Continue => Ok(()),
            StmtKind::For { name, list, body } => {
                self.expr(list)?;
                self.scoped(|r| {
                    r.bind(name.text, name.at)?;
                    r.block(body)
                })
            }
            StmtKind::While { cond, body } => {
                self.expr(cond)?;
                self.block(body)
            }
        }
    }

    /// A chain (operators, calls, fields, items, `?`) is walked by a loop down its first operands,
    /// so the recursion is as deep as the nesting of brackets, not as long as the chain
    /// (M2-02.1.7). The order is the evaluation order: first operand, then the rest, outwards.
    fn expr(&mut self, e: &'a Expr<'a>) -> Res {
        let mut chain = Vec::new();
        let mut head = e;
        while let Some(first) = first_operand(head) {
            chain.push(head);
            head = first;
        }
        self.leaf(head)?;
        chain.iter().rev().try_for_each(|link| self.rest(link))
    }

    /// The operands of a chain link other than its first.
    fn rest(&mut self, link: &'a Expr<'a>) -> Res {
        match &link.kind {
            ExprKind::Binary(_, _, _, x) | ExprKind::Range(_, x) | ExprKind::Index(_, x) => {
                self.expr(x)
            }
            ExprKind::Call(_, args) => args.iter().try_for_each(|a| self.expr(&a.value)),
            _ => Ok(()),
        }
    }

    /// An expression that is not a link of a chain.
    fn leaf(&mut self, e: &'a Expr<'a>) -> Res {
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Hole(_) => Ok(()),
            ExprKind::Str(parts) => parts.iter().try_for_each(|part| match part {
                StrPart::Expr(x) => self.expr(x),
                StrPart::Text(_) => Ok(()),
            }),
            ExprKind::Name(name) => self.value(name, e.at),
            ExprKind::Record(name, args) => {
                self.record_type(name, e.at)?;
                args.iter().try_for_each(|a| self.expr(&a.value))
            }
            ExprKind::List(items) => items.iter().try_for_each(|x| self.expr(x)),
            ExprKind::Map(entries) => entries
                .iter()
                .try_for_each(|(k, v)| self.expr(k).and_then(|()| self.expr(v))),
            ExprKind::If {
                branches,
                otherwise,
            } => {
                for (cond, block) in branches {
                    self.expr(cond)?;
                    self.block(block)?;
                }
                otherwise.iter().try_for_each(|b| self.block(b))
            }
            ExprKind::Match { subject, arms } => {
                self.expr(subject)?;
                arms.iter().try_for_each(|a| self.arm(a))
            }
            ExprKind::Children(..) => unreachable!("check refuses views"),
            // Links of a chain: `expr` walks them.
            ExprKind::Unary(..)
            | ExprKind::Try(..)
            | ExprKind::Inout(_)
            | ExprKind::Field(..)
            | ExprKind::Binary(..)
            | ExprKind::Range(..)
            | ExprKind::Index(..)
            | ExprKind::Call(..) => Ok(()),
        }
    }

    /// A lower-case or Upper-case name in an expression: the innermost local, else a function or
    /// a variant. A type, an example and an app are never values.
    fn value(&mut self, name: &'a str, at: Pos) -> Res {
        let local = self.locals.iter().rfind(|l| l.0 == name);
        let local = local.map(|&(_, declared)| Binding::Local(declared));
        match local.or_else(|| self.pool_get(name, of(VALUES))) {
            Some(found) => self.record(at, found),
            None => {
                if let Some(message) = habit(name) {
                    return Err(Diagnostic::new(
                        self.path,
                        at,
                        name.len(),
                        "unknown-name",
                        message,
                    ));
                }
                let locals = self.locals.iter().map(|l| l.0);
                let near = locals.chain(self.names(of(VALUES)));
                Err(self.suggest("unknown-name", (at, name), "not defined here", near))
            }
        }
    }

    /// A record literal names a `type`; its labels are the checker's.
    fn record_type(&mut self, name: &str, at: Pos) -> Res {
        let defs = &self.program.defs;
        let record = |k, b| {
            k == Kind::Type && matches!(b, Binding::Def(i) if matches!(defs[i], Def::Type { .. }))
        };
        match self.pool_get(name, record) {
            Some(found) => self.record(at, found),
            None => {
                let near = self.names(record);
                Err(self.suggest("unknown-type", (at, name), "not a record type", near))
            }
        }
    }

    /// The names a pattern binds are visible in the arm alone.
    fn arm(&mut self, arm: &'a Arm<'a>) -> Res {
        self.scoped(|r| {
            arm.patterns.iter().try_for_each(|p| r.pat(p))?;
            match &arm.body {
                ArmBody::Block(b) => r.block(b),
                ArmBody::Line(s) => r.stmt(s),
            }
        })
    }

    fn pat(&mut self, pat: &'a Pat<'a>) -> Res {
        match &pat.kind {
            PatKind::Bind(name) => self.bind(name, pat.at),
            PatKind::Variant(name, parts) => {
                if self.pool_use(name, pat.at, VARIANTS).is_none() {
                    let near = self.names(of(VARIANTS));
                    return Err(self.suggest(
                        "unknown-name",
                        (pat.at, name),
                        "not a variant",
                        near,
                    ));
                }
                parts.iter().try_for_each(|p| self.pat(p))
            }
            PatKind::Wild | PatKind::Int(_) | PatKind::Str(_) | PatKind::Bool(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
