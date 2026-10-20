//! Checking an edited function alone (M4-45). A body is checked against signatures only
//! (docs/decisions/M2-02.md), so a function checks in a small program of itself, a stub of each
//! function it names and the text of each type, the same as in the whole program. Its core form
//! then replaces the old one in the installed program, its ids renumbered by name.
//!
//! A caller takes this path only for a function that checks clean with the interface it had;
//! anything else, such as an error, whose suggestions would come from the small program, goes
//! through the whole program.

use crate::core_form::*;
use crate::parser::parse_def;
use crate::source::Source;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

const PATH: &str = "main.paka";

/// A program's definitions by each name they define.
#[derive(Default)]
pub struct Names(BTreeMap<String, Vec<String>>);

impl Names {
    pub fn add(&mut self, text: &str) {
        for name in defined(text) {
            self.0.entry(name).or_default().push(text.into());
        }
    }

    pub fn remove(&mut self, text: &str) {
        for name in defined(text) {
            let texts = self
                .0
                .get_mut(&name)
                .expect("a removed definition was added");
            texts.retain(|t| t != text);
            if texts.is_empty() {
                self.0.remove(&name);
            }
        }
    }

    /// The program the function `text` checks in alone: `text` first, then each function it names
    /// as a stub with a hole for its body, and each record and enum, through what those name.
    /// `None` when it names an app or an example.
    pub fn alone(&self, text: &str) -> Option<String> {
        let own = first_word(text.strip_prefix("fn ")?);
        let mut out = vec![text.to_string()];
        let (mut seen, mut taken) = (BTreeSet::from([own]), BTreeSet::new());
        let mut todo: Vec<&str> = words(text).collect();
        while let Some(word) = todo.pop() {
            if !seen.insert(word) {
                continue;
            }
            for def in self.0.get(word).into_iter().flatten() {
                if !taken.insert(def.as_str()) {
                    continue;
                }
                let (kept, named) = match def.split_once(' ')?.0 {
                    "fn" => stub(def)?,
                    "type" | "enum" => (def.clone(), def.as_str()),
                    _ => return None,
                };
                todo.extend(words(named));
                out.push(kept);
            }
        }
        Some(out.join("\n\n") + "\n")
    }
}

/// The names `text` defines: its own, and an enum's variants.
fn defined(text: &str) -> Vec<String> {
    if !text.starts_with("enum ") {
        let rest = text.split_once(' ').map_or("", |(_, r)| r);
        return vec![first_word(rest).into()];
    }
    let def = parse_def(PATH, text.as_bytes()).expect("a stored definition parses");
    def.names().into_iter().map(String::from).collect()
}

/// The function `text` with a hole for its body, or an empty body when it has no result; and its
/// header.
fn stub(text: &str) -> Option<(String, &str)> {
    let header = &text[..text.find('{')?];
    let body = if header.contains("->") {
        "{\n  ??stub\n}"
    } else {
        "{\n}"
    };
    (!header.contains('"')).then(|| (format!("{header}{body}"), header))
}

fn first_word(text: &str) -> &str {
    words(text).next().unwrap_or_default()
}

/// Every word that could be a name, in strings and comments too.
fn words(text: &str) -> impl Iterator<Item = &str> {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.split(move |c| !word(c))
        .filter(|w| w.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_'))
}

/// Lowers the function `text`, alone, into `p` in place of the function of its name, which stood
/// on the global lines `old`. Its sites go last; the lines after it move by its change in length.
/// `None`, leaving `p` as it was, when it does not check clean alone, or its signature or a type it
/// uses is not one `p` has.
pub fn relower(p: &mut Program, names: &Names, text: &str, old: Range<usize>) -> Option<FnId> {
    let source = Source::file(PATH, names.alone(text)?.into_bytes());
    let (mut alone, ..) = crate::check(PATH, &source).ok()?;
    let fns = alone.fns.iter().map(|f| f.name.clone()).collect();
    let f = alone.fns.swap_remove(0);
    let id = p.fns.iter().position(|g| g.name == f.name)?;
    let mut map = Map {
        from: &alone,
        fns,
        to: p,
        first: f.site,
        last: f.site,
        base: p.sites.len(),
    };
    let f = map.function(f)?;
    let used = map.first..=map.last;
    if (&f.params, f.result) != (&p.fns[id].params, p.fns[id].result) {
        return None;
    }
    let lines = text.lines().count();
    (p.sites.iter_mut().map(|s| &mut s.line))
        .chain(p.files.iter_mut().map(|f| &mut f.1))
        .filter(|line| **line >= old.end)
        .for_each(|line| *line = *line + lines - old.len());
    p.sites.extend(alone.sites.drain(used).map(|s| TrapSite {
        line: s.line + old.start - 1,
        ..s
    }));
    p.fns[id] = f;
    Some(id)
}

/// Renumbers a function's ids from the program it was lowered in to `to`.
struct Map<'p> {
    from: &'p Program,
    /// The names of `from`'s functions.
    fns: Vec<String>,
    to: &'p Program,
    /// The function's first site and the last it uses, in `from`.
    first: Site,
    last: Site,
    /// Where its sites start in `to`.
    base: Site,
}

impl Map<'_> {
    fn function(&mut self, f: Fn) -> Option<Fn> {
        let mut f = f;
        f.site = self.site(f.site)?;
        for p in &mut f.params {
            p.ty = self.ty(p.ty)?;
        }
        for t in &mut f.locals {
            *t = self.ty(*t)?;
        }
        f.result = self.maybe(f.result)?;
        self.block(&mut f.body)?;
        for s in &mut f.claims {
            *s = self.site(*s)?;
        }
        Some(f)
    }

    fn ty(&self, t: Ty) -> Option<Ty> {
        let tys = |ts: &[Ty]| ts.iter().map(|&t| self.ty(t)).collect::<Option<Vec<_>>>();
        let to = match &self.from.types[t] {
            Type::Int => Type::Int,
            Type::Float => Type::Float,
            Type::Bool => Type::Bool,
            Type::String => Type::String,
            Type::List(a) => Type::List(self.ty(*a)?),
            Type::Map(k, v) => Type::Map(self.ty(*k)?, self.ty(*v)?),
            Type::Option(a) => Type::Option(self.ty(*a)?),
            Type::Result(a, b) => Type::Result(self.ty(*a)?, self.ty(*b)?),
            Type::Effect(a) => Type::Effect(self.ty(*a)?),
            Type::Fn(ps, r) => Type::Fn(tys(ps)?, self.maybe(*r)?),
            Type::Record(r) => Type::Record(self.record(*r)?),
            Type::Enum(e) => Type::Enum(self.enum_id(*e)?),
        };
        self.to.types.iter().position(|known| *known == to)
    }

    /// `t` renumbered, kept `None` when there is none; `None` when `to` lacks it.
    fn maybe(&self, t: Option<Ty>) -> Option<Option<Ty>> {
        t.map_or(Some(None), |t| self.ty(t).map(Some))
    }

    fn record(&self, r: RecordId) -> Option<RecordId> {
        let name = &self.from.records[r].name;
        self.to.records.iter().position(|x| x.name == *name)
    }

    fn enum_id(&self, e: EnumId) -> Option<EnumId> {
        let name = &self.from.enums[e].name;
        self.to.enums.iter().position(|x| x.name == *name)
    }

    fn fn_id(&self, f: FnId) -> Option<FnId> {
        let name = &self.fns[f];
        self.to.fns.iter().position(|x| x.name == *name)
    }

    fn site(&mut self, s: Site) -> Option<Site> {
        self.last = self.last.max(s);
        Some(self.base + s.checked_sub(self.first)?)
    }

    fn tag(&self, tag: &mut Tag) -> Option<()> {
        if let Tag::User(e, _) = tag {
            *e = self.enum_id(*e)?;
        }
        Some(())
    }

    fn block(&mut self, b: &mut Block) -> Option<()> {
        for s in &mut b.stmts {
            self.stmt(s)?;
        }
        b.value.as_deref_mut().map_or(Some(()), |e| self.expr(e))
    }

    fn stmt(&mut self, s: &mut Stmt) -> Option<()> {
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => self.expr(e),
            Stmt::Assign(place, op, e) => {
                if let Some(Compound::Int(_, s) | Compound::Join(s)) = op {
                    *s = self.site(*s)?;
                }
                self.place(place)?;
                self.expr(e)
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => Some(()),
            Stmt::For(_, walk, b) => {
                match walk {
                    Walk::List(e) => self.expr(e)?,
                    Walk::Range(a, z) => {
                        self.expr(a)?;
                        self.expr(z)?;
                    }
                }
                self.block(b)
            }
            Stmt::While(e, b) => {
                self.expr(e)?;
                self.block(b)
            }
        }
    }

    fn place(&mut self, p: &mut Place) -> Option<()> {
        if let Root::Value(e) = &mut p.root {
            self.expr(e)?;
        }
        for step in &mut p.steps {
            match step {
                Step::Field(_) => {}
                Step::Item(e, s) | Step::Key(e, s) => {
                    self.expr(e)?;
                    *s = self.site(*s)?;
                }
                Step::Slice(a, z, s) => {
                    self.expr(a)?;
                    self.expr(z)?;
                    *s = self.site(*s)?;
                }
            }
        }
        Some(())
    }

    fn exprs<'e>(&mut self, es: impl IntoIterator<Item = &'e mut Expr>) -> Option<()> {
        es.into_iter().try_for_each(|e| self.expr(e))
    }

    fn expr(&mut self, e: &mut Expr) -> Option<()> {
        e.ty = self.maybe(e.ty)?;
        match &mut e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Str(_) => {}
            ExprKind::Interp(parts, s) => {
                *s = self.site(*s)?;
                self.exprs(parts)?;
            }
            ExprKind::Hole(s) => *s = self.site(*s)?,
            ExprKind::Place(p) => self.place(p)?,
            ExprKind::Record(r, es) => {
                *r = self.record(*r)?;
                self.exprs(es)?;
            }
            ExprKind::List(es) => self.exprs(es)?,
            ExprKind::Map(pairs) => self.exprs(pairs.iter_mut().flat_map(|(k, v)| [k, v]))?,
            ExprKind::Variant(tag, es) => {
                self.tag(tag)?;
                self.exprs(es)?;
            }
            ExprKind::FnValue(FnRef::Fn(f)) => *f = self.fn_id(*f)?,
            ExprKind::FnValue(FnRef::Variant(e, _)) => *e = self.enum_id(*e)?,
            ExprKind::Unary(op, a) => {
                if let Un::Neg(s) = op {
                    *s = self.site(*s)?;
                }
                self.expr(a)?;
            }
            ExprKind::Binary(op, a, z) => {
                if let Bin::Int(_, s) | Bin::Join(s) | Bin::Eq(s) | Bin::Ne(s) = op {
                    *s = self.site(*s)?;
                }
                self.expr(a)?;
                self.expr(z)?;
            }
            ExprKind::Call(callee, args, s) => {
                *s = self.site(*s)?;
                match callee {
                    Callee::Fn(f) => *f = self.fn_id(*f)?,
                    Callee::Builtin(_) => {}
                    Callee::Value(e) => self.expr(e)?,
                }
                for arg in args {
                    match arg {
                        Arg::Value(e) => self.expr(e)?,
                        Arg::Inout(p) => self.place(p)?,
                    }
                }
            }
            ExprKind::If(c, then, other) => {
                self.expr(c)?;
                self.block(then)?;
                if let Some(b) = other {
                    self.block(b)?;
                }
            }
            ExprKind::Match(e, arms) => {
                self.expr(e)?;
                for arm in arms {
                    arm.tags.iter_mut().try_for_each(|t| self.tag(t))?;
                    self.block(&mut arm.body)?;
                }
            }
        }
        Some(())
    }
}
