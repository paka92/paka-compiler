//! The body and interface hash of every definition (M4-11, docs/decisions/M2-09.md section 3).
//! Both come from the parsed definition with positions left out, so layout cannot change them.

use crate::ast::*;
use crate::sha256::sha256;
use std::collections::{BTreeMap, BTreeSet};

pub type Hash = [u8; 32];

/// Each definition's body and interface hash. The body hash covers the whole definition and
/// what each name it read resolves to; `reads` are those names, per definition.
pub(crate) fn hashes(program: &Program, reads: &[BTreeSet<String>]) -> Vec<(Hash, Hash)> {
    let defs: BTreeMap<_, _> = program.defs.iter().map(|d| (d.name().text, d)).collect();
    let types = type_hashes(&defs);
    let iter = program.defs.iter().zip(reads);
    iter.map(|(def, reads)| (body(def, reads, &defs), interface(def, &types)))
        .collect()
}

fn body(def: &Def, reads: &BTreeSet<String>, defs: &BTreeMap<&str, &Def>) -> Hash {
    let none = BTreeMap::new();
    let mut key = Key::new(&none);
    key.def(def);
    for read in reads {
        key.word(read);
        key.word(defs.get(read.as_str()).map_or("none", |d| kind(d)));
    }
    sha256(key.out.as_bytes())
}

fn interface(def: &Def, types: &BTreeMap<&str, Hash>) -> Hash {
    if let Some(hash) = types.get(def.name().text) {
        return *hash;
    }
    let mut key = Key::new(types);
    key.word(kind(def));
    key.word(def.name().text);
    match def {
        Def::Fn { params, result, .. } => key.signature(params, result.as_ref()),
        Def::View { params, result, .. } => key.signature(params, Some(result)),
        Def::App { entries, .. } => entries.iter().for_each(|e| key.entry(e)),
        Def::Example { .. } | Def::Type { .. } | Def::Enum { .. } => {}
    }
    sha256(key.out.as_bytes())
}

fn kind(def: &Def) -> &'static str {
    match def {
        Def::Type { .. } => "type",
        Def::Enum { .. } => "enum",
        Def::Fn { .. } => "fn",
        Def::Example { .. } => "example",
        Def::App { .. } => "app",
        Def::View { .. } => "view",
    }
}

/// Each record and enum's hash. A type's hash covers its layout with every user type it names
/// replaced by that type's hash; types that reach each other are hashed as one group, where
/// they name each other by name, and each member's hash is the group's plus its name.
fn type_hashes<'a>(defs: &BTreeMap<&'a str, &Def<'a>>) -> BTreeMap<&'a str, Hash> {
    let named = |def: &Def<'a>| {
        let mut names = BTreeSet::new();
        for field in fields(def) {
            type_names(&field.ty, &mut names);
        }
        names.retain(|n| defs.get(n).is_some_and(|d| is_type(d)));
        names
    };
    let edges: BTreeMap<_, _> = (defs.iter())
        .filter(|(_, d)| is_type(d))
        .map(|(n, d)| (*n, named(d)))
        .collect();
    let reach: BTreeMap<_, _> = edges.keys().map(|n| (*n, reach(n, &edges))).collect();
    let mut hashes = BTreeMap::new();
    for name in edges.keys() {
        hash_group(name, defs, &reach, &mut hashes);
    }
    hashes
}

fn is_type(def: &Def) -> bool {
    matches!(def, Def::Type { .. } | Def::Enum { .. })
}

fn fields<'d, 'a>(def: &'d Def<'a>) -> Vec<&'d Field<'a>> {
    match def {
        Def::Type { fields, .. } => fields.iter().collect(),
        Def::Enum { variants, .. } => variants.iter().flat_map(|v| &v.fields).collect(),
        _ => vec![],
    }
}

fn type_names<'a>(ty: &Type<'a>, out: &mut BTreeSet<&'a str>) {
    match ty {
        Type::Name { name, args } => {
            out.insert(name.text);
            args.iter().for_each(|a| type_names(a, out));
        }
        Type::Fn { params, result, .. } => {
            params
                .iter()
                .chain(result.as_deref())
                .for_each(|t| type_names(t, out));
        }
    }
}

/// The types `name` reaches, itself only when through a cycle.
fn reach<'a>(name: &'a str, edges: &BTreeMap<&'a str, BTreeSet<&'a str>>) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut todo: Vec<_> = edges[name].iter().copied().collect();
    while let Some(next) = todo.pop() {
        if seen.insert(next) {
            todo.extend(&edges[next]);
        }
    }
    seen
}

fn hash_group<'a>(
    name: &'a str,
    defs: &BTreeMap<&'a str, &Def<'a>>,
    reach: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    hashes: &mut BTreeMap<&'a str, Hash>,
) {
    if hashes.contains_key(name) {
        return;
    }
    let mut group: BTreeSet<_> = (reach[name].iter())
        .filter(|n| reach[*n].contains(name))
        .copied()
        .collect();
    group.insert(name);
    for out in reach[name].difference(&group) {
        hash_group(out, defs, reach, hashes);
    }
    let mut key = Key::new(hashes);
    for member in &group {
        key.layout(defs[member]);
    }
    let group_hash = sha256(key.out.as_bytes());
    for member in group {
        let text = [&group_hash[..], member.as_bytes()].concat();
        hashes.insert(member, sha256(&text));
    }
}

/// A position-free text of syntax, each part length-prefixed so no two trees share one. A
/// type named in `types` is written as its hash.
struct Key<'t> {
    out: String,
    types: &'t BTreeMap<&'t str, Hash>,
}

impl<'t> Key<'t> {
    fn new(types: &'t BTreeMap<&'t str, Hash>) -> Self {
        Key {
            out: String::new(),
            types,
        }
    }

    fn word(&mut self, text: &str) {
        self.out += &format!("{}:{text}", text.len());
    }

    fn count(&mut self, n: usize) {
        self.out += &format!("{n}#");
    }

    fn flag(&mut self, b: bool) {
        self.out.push(if b { '1' } else { '0' });
    }

    fn option<T>(&mut self, x: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.flag(x.is_some());
        if let Some(x) = x {
            f(self, x);
        }
    }

    fn list<T>(&mut self, xs: impl ExactSizeIterator<Item = T>, mut f: impl FnMut(&mut Self, T)) {
        self.count(xs.len());
        xs.for_each(|x| f(self, x));
    }

    fn def(&mut self, def: &Def) {
        self.layout(def);
        match def {
            Def::Fn {
                params,
                result,
                body,
                ..
            } => self.callable(params, result.as_ref(), body),
            Def::View {
                params,
                result,
                body,
                ..
            } => self.callable(params, Some(result), body),
            Def::Example { body, .. } => self.block(body),
            Def::App { entries, .. } => entries.iter().for_each(|e| self.entry(e)),
            Def::Type { .. } | Def::Enum { .. } => {}
        }
    }

    /// What a caller sees: each parameter's type, mark and label, and the result.
    fn signature(&mut self, params: &[Param], result: Option<&Type>) {
        self.count(params.len());
        for p in params {
            self.ty(&p.ty);
            self.flag(p.inout);
            self.word(if p.labelled { p.name.text } else { "" });
        }
        self.option(result, Key::ty);
    }

    fn callable(&mut self, params: &[Param], result: Option<&Type>, body: &Block) {
        self.list(params.iter(), |k, p| {
            k.word(p.name.text);
            k.flag(p.inout);
            k.flag(p.labelled);
            k.ty(&p.ty);
        });
        self.option(result, Key::ty);
        self.block(body);
    }

    /// A record or enum's kind, name and fields in order; other kinds only their kind and name.
    fn layout(&mut self, def: &Def) {
        self.word(kind(def));
        self.word(def.name().text);
        match def {
            Def::Type { fields, .. } => self.list(fields.iter(), Key::field),
            Def::Enum { variants, .. } => self.list(variants.iter(), |k, v| {
                k.word(v.name.text);
                k.list(v.fields.iter(), Key::field);
            }),
            _ => {}
        }
    }

    fn field(&mut self, f: &Field) {
        self.word(f.name.text);
        self.ty(&f.ty);
        self.option(f.default.as_ref(), Key::expr);
    }

    fn entry(&mut self, entry: &Entry) {
        match entry {
            Entry::State(_, n) => {
                self.word("state");
                self.word(n.text)
            }
            Entry::Start(_, e) => {
                self.word("start");
                self.expr(e)
            }
            Entry::Update(_, n) => {
                self.word("update");
                self.word(n.text)
            }
            Entry::Grants(_, ns) => {
                self.word("grants");
                self.list(ns.iter(), |k, n| k.word(n.text));
            }
            Entry::View(_, n) => {
                self.word("view");
                self.word(n.text)
            }
        }
    }

    fn ty(&mut self, ty: &Type) {
        match ty {
            Type::Name { name, args } => match self.types.get(name.text) {
                Some(hash) => self.out += &format!("#{}", hex(hash)),
                None => {
                    self.word(name.text);
                    self.list(args.iter(), Key::ty);
                }
            },
            Type::Fn { params, result, .. } => {
                self.word("fn");
                self.list(params.iter(), Key::ty);
                self.option(result.as_deref(), Key::ty);
            }
        }
    }

    fn block(&mut self, block: &Block) {
        self.list(block.stmts.iter(), Key::stmt);
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let {
                var,
                name,
                ty,
                value,
            } => {
                self.word(if *var { "var" } else { "let" });
                self.word(name.text);
                self.option(ty.as_ref(), Key::ty);
                self.expr(value);
            }
            StmtKind::Discard(e) => {
                self.word("_=");
                self.expr(e)
            }
            StmtKind::Assign { target, op, value } => {
                self.word(&format!("{op:?}="));
                self.expr(target);
                self.expr(value);
            }
            StmtKind::Return(e) => {
                self.word("return");
                self.option(e.as_ref(), Key::expr)
            }
            StmtKind::Break => self.word("break"),
            StmtKind::Continue => self.word("continue"),
            StmtKind::For { name, list, body } => {
                self.word("for");
                self.word(name.text);
                self.expr(list);
                self.block(body);
            }
            StmtKind::While { cond, body } => {
                self.word("while");
                self.expr(cond);
                self.block(body);
            }
            StmtKind::Expr(e) => {
                self.word("expr");
                self.expr(e)
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Int(s) => {
                self.word("int");
                self.word(s)
            }
            ExprKind::Float(s) => {
                self.word("float");
                self.word(s)
            }
            ExprKind::Bool(b) => {
                self.word("bool");
                self.flag(*b)
            }
            ExprKind::Str(parts) => {
                self.word("str");
                self.list(parts.iter(), |k, p| match p {
                    StrPart::Text(t) => {
                        k.word("text");
                        k.word(t)
                    }
                    StrPart::Expr(e) => {
                        k.word("expr");
                        k.expr(e)
                    }
                });
            }
            ExprKind::Hole(n) => {
                self.word("hole");
                self.word(n)
            }
            ExprKind::Name(n) => {
                self.word("name");
                self.word(n)
            }
            ExprKind::Record(n, args) => {
                self.word("record");
                self.word(n);
                self.list(args.iter(), Key::arg);
            }
            ExprKind::List(xs) => {
                self.word("list");
                self.list(xs.iter(), Key::expr)
            }
            ExprKind::Map(pairs) => {
                self.word("map");
                self.list(pairs.iter(), |k, (a, b)| {
                    k.expr(a);
                    k.expr(b);
                });
            }
            ExprKind::Unary(op, e) => {
                self.word(&format!("{op:?}"));
                self.expr(e)
            }
            ExprKind::Binary(op, _, a, b) => {
                self.word(op.text());
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Range(a, b) => {
                self.word("..<");
                self.expr(a);
                self.expr(b)
            }
            ExprKind::Index(a, b) => {
                self.word("[]");
                self.expr(a);
                self.expr(b)
            }
            ExprKind::Field(e, n) => {
                self.word(".");
                self.expr(e);
                self.word(n.text)
            }
            ExprKind::Try(e, _) => {
                self.word("?");
                self.expr(e)
            }
            ExprKind::Call(f, args) => {
                self.word("call");
                self.expr(f);
                self.list(args.iter(), Key::arg);
            }
            ExprKind::Inout(e) => {
                self.word("&");
                self.expr(e)
            }
            ExprKind::Children(call, children) => {
                self.word("children");
                self.expr(call);
                self.block(children)
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                self.word("if");
                self.list(branches.iter(), |k, (c, b)| {
                    k.expr(c);
                    k.block(b);
                });
                self.option(otherwise.as_ref(), Key::block);
            }
            ExprKind::Match { subject, arms } => {
                self.word("match");
                self.expr(subject);
                self.list(arms.iter(), Key::arm);
            }
        }
    }

    fn arg(&mut self, arg: &Arg) {
        self.option(arg.label.as_ref(), |k, n| k.word(n.text));
        self.expr(&arg.value);
    }

    fn arm(&mut self, arm: &Arm) {
        self.list(arm.patterns.iter(), Key::pat);
        match &arm.body {
            ArmBody::Block(b) => {
                self.word("block");
                self.block(b)
            }
            ArmBody::Line(s) => {
                self.word("line");
                self.stmt(s)
            }
        }
    }

    fn pat(&mut self, pat: &Pat) {
        match &pat.kind {
            PatKind::Wild => self.word("_"),
            PatKind::Int(s) => {
                self.word("int");
                self.word(s)
            }
            PatKind::Str(s) => {
                self.word("str");
                self.word(s)
            }
            PatKind::Bool(b) => {
                self.word("bool");
                self.flag(*b)
            }
            PatKind::Bind(n) => {
                self.word("bind");
                self.word(n)
            }
            PatKind::Variant(n, ps) => {
                self.word("variant");
                self.word(n);
                self.list(ps.iter(), Key::pat);
            }
        }
    }
}

fn hex(hash: &Hash) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}
