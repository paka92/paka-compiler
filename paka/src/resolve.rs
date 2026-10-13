//! Name resolution (docs/decisions/M2-02.md section 1.3, M2-14.md section 1.2). Binds every use of
//! a name to its definition and reports the first duplicate or unknown name, with the nearest known
//! name as the fix. No types are looked at, so a field read `x.f` and the labels of a call are left
//! to the checker.

use crate::ast::{
    Arg, Arm, ArmBody, Block, Def, Entry, Expr, ExprKind, Field, Pat, PatKind, Pos, Program, Stmt,
    StmtKind, StrPart, Type,
};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

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
    /// A field label of a record literal: the type's index in `Program::defs`, then the field's.
    Field(usize, usize),
    /// A name of `BUILTINS`.
    Builtin(&'static str),
}

/// The binding of every name use, by the position of the name.
#[derive(Debug)]
pub struct Resolved {
    uses: BTreeMap<Pos, Binding>,
}

impl Resolved {
    pub fn get(&self, at: Pos) -> Option<Binding> {
        self.uses.get(&at).copied()
    }
}

/// What a name in the pool is: where a use may name it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Type,
    Variant,
    Fn,
    Grant,
    /// An `example` or an `app`: it holds a name but is never used by name.
    Other,
}

/// The built-in names: M2-14 section 1.1 (types, constructors), section 1.2 (63 functions) and
/// M2-15 section 1.3 (effects, grants).
pub const BUILTINS: [(Kind, &str); 4] = [
    (
        Kind::Type,
        "Int Float Bool String List Map Option Result Effect",
    ),
    (
        Kind::Variant,
        "Some None Ok Err Print ReadFile WriteFile Exit ExitWith",
    ),
    (Kind::Grant, "Console ReadFiles WriteFiles"),
    (
        Kind::Fn,
        "min max abs to_float to_int sqrt floor ceil trunc round fixed
         wrap_add wrap_sub wrap_mul bit_and bit_or bit_xor bit_not shift_left shift_right
         len is_empty contains find parse_int parse_float code_at from_code
         split chars lines words join repeat pad_left pad_right
         trim trim_start trim_end starts_with ends_with replace upper lower
         append extend pop insert remove_at swap reverse sort sort_by list_of
         get has remove keys values unwrap_or ok_or assert unreachable",
    ),
];

const VALUES: &[Kind] = &[Kind::Fn, Kind::Variant];
const TYPES: &[Kind] = &[Kind::Type];
const VARIANTS: &[Kind] = &[Kind::Variant];
const GRANTS: &[Kind] = &[Kind::Grant];

pub fn resolve<'a>(path: &'a str, program: &'a Program<'a>) -> Res<Resolved> {
    let mut resolver = Resolver {
        path,
        program,
        pool: builtins(),
        locals: Vec::new(),
        uses: BTreeMap::new(),
    };
    for (i, def) in program.defs.iter().enumerate() {
        resolver
            .declare_def(i, def)
            .map_err(|d| d.in_definition(def.name().text))?;
    }
    for def in &program.defs {
        let result = resolver.def(def);
        result.map_err(|d| d.in_definition(def.name().text))?;
    }
    Ok(Resolved {
        uses: resolver.uses,
    })
}

fn builtins<'a>() -> BTreeMap<&'a str, (Kind, Binding)> {
    let rows = BUILTINS.iter().flat_map(|&(kind, words)| {
        words
            .split_whitespace()
            .map(move |word| (word, (kind, Binding::Builtin(word))))
    });
    rows.collect()
}

/// The candidate closest to `name` by edit distance, if close enough. Ties go to the first in byte
/// order; only names of the same letter case count (case carries meaning, M2-02.3).
fn nearest<'n>(name: &str, candidates: impl Iterator<Item = &'n str>) -> Option<&'n str> {
    let upper = |s: &str| s.starts_with(char::is_uppercase);
    let near = candidates.filter(|c| upper(c) == upper(name));
    let scored = near.map(|c| (distance(name, c), c));
    let (d, best) = scored.min()?;
    (d <= name.len().div_ceil(3)).then_some(best)
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

struct Resolver<'a> {
    path: &'a str,
    program: &'a Program<'a>,
    /// Definitions, variants and built-ins: one pool (M2-02.2).
    pool: BTreeMap<&'a str, (Kind, Binding)>,
    /// Every local visible at this point, outermost first, with the position of its name.
    locals: Vec<(&'a str, Pos)>,
    uses: BTreeMap<Pos, Binding>,
}

impl<'a> Resolver<'a> {
    fn fail(&self, at: Pos, len: usize, code: &'static str, message: String) -> Diagnostic {
        Diagnostic::new(self.path, at, len, code, message)
    }

    /// `name` is `what`; when a candidate is close, it is the message's hint and the fix.
    fn suggest<'n>(
        &self,
        code: &'static str,
        (at, name): (Pos, &str),
        what: &str,
        candidates: impl Iterator<Item = &'n str>,
    ) -> Diagnostic {
        let fail = |message| self.fail(at, name.len(), code, message);
        match nearest(name, candidates) {
            Some(near) => {
                fail(format!("`{name}` is {what}; did you mean `{near}`?")).with_fix(near)
            }
            None => fail(format!("`{name}` is {what}")),
        }
    }

    fn pool_get(&self, name: &str, kinds: &[Kind]) -> Option<Binding> {
        let (kind, binding) = self.pool.get(name)?;
        kinds.contains(kind).then_some(*binding)
    }

    fn pool_names<'s>(&'s self, kinds: &'s [Kind]) -> impl Iterator<Item = &'a str> + 's {
        let wanted = self
            .pool
            .iter()
            .filter(|(_, (kind, _))| kinds.contains(kind));
        wanted.map(|(name, _)| *name)
    }

    fn record(&mut self, at: Pos, found: Binding) -> Res {
        self.uses.insert(at, found);
        Ok(())
    }

    /// Looks `name` up in the pool and records the use.
    fn pool_use(&mut self, name: &str, at: Pos, kinds: &[Kind]) -> Option<Binding> {
        let found = self.pool_get(name, kinds)?;
        self.uses.insert(at, found);
        Some(found)
    }

    fn declare(&mut self, name: &'a str, at: Pos, kind: Kind, binding: Binding) -> Res {
        let Some((_, old)) = self.pool.insert(name, (kind, binding)) else {
            return Ok(());
        };
        let message = match old {
            Binding::Builtin(_) => format!("{name} is a built-in; rename it"),
            _ => format!("`{name}` is already declared"),
        };
        Err(self.fail(at, name.len(), "duplicate-name", message))
    }

    fn declare_def(&mut self, i: usize, def: &'a Def<'a>) -> Res {
        let name = def.name();
        let kind = match def {
            Def::Type { .. } | Def::Enum { .. } => Kind::Type,
            Def::Fn { .. } => Kind::Fn,
            Def::Example { .. } | Def::App { .. } => Kind::Other,
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
            Def::App { entries, .. } => self.app(entries),
        }
    }

    /// A field name is held once per type or variant; its type and default are uses.
    fn fields(&mut self, fields: &'a [Field<'a>]) -> Res {
        for (i, f) in fields.iter().enumerate() {
            if fields[..i].iter().any(|g| g.name.text == f.name.text) {
                let message = format!("`{}` is already a field here", f.name.text);
                return Err(self.fail(f.name.at, f.name.text.len(), "duplicate-name", message));
            }
            self.ty(&f.ty)?;
            f.default.iter().try_for_each(|d| self.expr(d))?;
        }
        Ok(())
    }

    /// Entries in the order of spec/m1.md section 8: `update`, `start`, `grants`; `state` first.
    fn app(&mut self, entries: &'a [Entry<'a>]) -> Res {
        let rank = |e: &Entry| match e {
            Entry::State(..) => 0,
            Entry::Update(..) => 1,
            Entry::Start(..) => 2,
            Entry::Grants(..) => 3,
        };
        let mut ordered: Vec<_> = entries.iter().collect();
        ordered.sort_by_key(|e| rank(e));
        for entry in ordered {
            match entry {
                Entry::State(_, n) => self.ty_name(n.text, n.at)?,
                Entry::Update(_, n) => self.update(n.text, n.at)?,
                Entry::Start(_, e) => self.expr(e)?,
                Entry::Grants(_, names) => {
                    names.iter().try_for_each(|n| self.grant(n.text, n.at))?
                }
            }
        }
        Ok(())
    }

    fn update(&mut self, name: &str, at: Pos) -> Res {
        let own = |b: &Binding| matches!(b, Binding::Def(_));
        match self.pool_get(name, &[Kind::Fn]).filter(own) {
            Some(found) => self.record(at, found),
            None => {
                let fns = self
                    .pool
                    .iter()
                    .filter(|(_, (k, b))| *k == Kind::Fn && own(b));
                let what = "not a function of the program";
                Err(self.suggest("unknown-name", (at, name), what, fns.map(|(n, _)| *n)))
            }
        }
    }

    fn grant(&mut self, name: &str, at: Pos) -> Res {
        match self.pool_use(name, at, GRANTS) {
            Some(_) => Ok(()),
            None => {
                let near = self.pool_names(GRANTS);
                Err(self.suggest("unknown-grant", (at, name), "not a grant", near))
            }
        }
    }

    fn ty_name(&mut self, name: &str, at: Pos) -> Res {
        match self.pool_use(name, at, TYPES) {
            Some(_) => Ok(()),
            None => {
                let message = format!("`{name}` is not a type");
                Err(self.fail(at, name.len(), "unknown-type", message))
            }
        }
    }

    fn ty(&mut self, t: &'a Type<'a>) -> Res {
        match t {
            Type::Name { name, args } => {
                self.ty_name(name.text, name.at)?;
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
            let message = format!("`{name}` is already bound at line {}", first.0);
            return Err(self.fail(at, name.len(), "duplicate-local", message));
        }
        self.locals.push((name, at));
        Ok(())
    }

    fn block(&mut self, block: &'a Block<'a>) -> Res {
        self.scoped(|r| block.stmts.iter().try_for_each(|s| r.stmt(s)))
    }

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

    fn expr(&mut self, e: &'a Expr<'a>) -> Res {
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Hole(_) => Ok(()),
            ExprKind::Str(parts) => parts.iter().try_for_each(|part| match part {
                StrPart::Expr(x) => self.expr(x),
                StrPart::Text(_) => Ok(()),
            }),
            ExprKind::Name(name) => self.value(name, e.at),
            ExprKind::Record(name, args) => {
                let record = self.record_type(name, e.at)?;
                args.iter().try_for_each(|a| self.record_arg(record, a))
            }
            ExprKind::List(items) => items.iter().try_for_each(|x| self.expr(x)),
            ExprKind::Map(entries) => entries
                .iter()
                .try_for_each(|(k, v)| self.expr(k).and_then(|()| self.expr(v))),
            ExprKind::Unary(_, x)
            | ExprKind::Try(x)
            | ExprKind::Inout(x)
            | ExprKind::Field(x, _) => self.expr(x),
            ExprKind::Binary(_, a, b) | ExprKind::Range(a, b) | ExprKind::Index(a, b) => {
                self.expr(a)?;
                self.expr(b)
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee)?;
                args.iter().try_for_each(|a| self.expr(&a.value))
            }
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
        }
    }

    /// A lower-case or Upper-case name in an expression: the innermost local, else a function or
    /// a variant. A type, an example and an app are never values.
    fn value(&mut self, name: &'a str, at: Pos) -> Res {
        let local = self.locals.iter().rfind(|l| l.0 == name);
        let local = local.map(|&(_, declared)| Binding::Local(declared));
        match local.or_else(|| self.pool_get(name, VALUES)) {
            Some(found) => self.record(at, found),
            None => {
                let near = self
                    .locals
                    .iter()
                    .map(|l| l.0)
                    .chain(self.pool_names(VALUES));
                Err(self.suggest("unknown-name", (at, name), "not defined here", near))
            }
        }
    }

    /// The index of the `type` a record literal builds.
    fn record_type(&mut self, name: &str, at: Pos) -> Res<usize> {
        match self.pool_use(name, at, TYPES) {
            Some(Binding::Def(i)) if matches!(self.program.defs[i], Def::Type { .. }) => Ok(i),
            _ => {
                let message = format!("`{name}` is not a record type");
                Err(self.fail(at, name.len(), "unknown-type", message))
            }
        }
    }

    fn record_arg(&mut self, record: usize, arg: &'a Arg<'a>) -> Res {
        if let Some(label) = &arg.label {
            let Def::Type { fields, .. } = &self.program.defs[record] else {
                return Ok(());
            };
            let Some(i) = fields.iter().position(|f| f.name.text == label.text) else {
                let near = fields.iter().map(|f| f.name.text);
                let what = "not a field of this record";
                return Err(self.suggest("unknown-name", (label.at, label.text), what, near));
            };
            self.record(label.at, Binding::Field(record, i))?;
        }
        self.expr(&arg.value)
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
                    let near = self.pool_names(VARIANTS);
                    let d = self.suggest("unknown-name", (pat.at, *name), "not a variant", near);
                    return Err(d);
                }
                parts.iter().try_for_each(|p| self.pat(p))
            }
            PatKind::Wild | PatKind::Int(_) | PatKind::Str(_) | PatKind::Bool(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
