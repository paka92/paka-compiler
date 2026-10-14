//! Lowering a checked program into the core form (docs/core.md). A node's type and trap site are
//! taken once its operands are lowered, so the type table and the site table fill in that order.

use crate::ast::{self, Def, Entry, Pos};
use crate::body::Bodies;
use crate::core_form::{
    self, App, Enum, Expr, ExprKind, Field, Fn, Grant, Local, Param, Program, Record, Site, Tag,
    TrapSite, Ty, Type, Variant,
};
use crate::resolve::{Binding, Resolved};
use crate::sig::{self, Item, Node, Table};
use std::collections::BTreeMap;

mod arms;
mod expr;
mod stmt;
#[cfg(test)]
mod tests;

struct Cx<'t, 'a> {
    core: Program,
    program: &'t ast::Program<'a>,
    resolved: &'t Resolved,
    table: &'t Table<'a>,
    bodies: &'t Bodies,
    source: &'t [u8],
    /// The offset of each line's start in `source`.
    starts: Vec<usize>,
    /// Each definition's place in the list of its kind: records, enums, functions, examples.
    ids: Vec<usize>,
    /// The definition being lowered, which `bodies.types` is indexed by.
    def: usize,
    locals: Vec<Ty>,
    /// The local of each name, by the position that declares it.
    slots: BTreeMap<Pos, Local>,
    result: Option<Ty>,
}

/// Runs on a program that `body::check` accepted: every lookup succeeds.
pub fn lower<'a>(
    path: &str,
    source: &[u8],
    program: &ast::Program<'a>,
    resolved: &Resolved,
    table: &Table<'a>,
    bodies: &Bodies,
) -> Program {
    let newlines = source.iter().enumerate().filter(|&(_, &b)| b == b'\n');
    let mut cx = Cx {
        core: Program {
            file: path.into(),
            types: vec![],
            records: vec![],
            enums: vec![],
            fns: vec![],
            examples: vec![],
            app: None,
            sites: vec![],
        },
        program,
        resolved,
        table,
        bodies,
        source,
        starts: std::iter::once(0)
            .chain(newlines.map(|(i, _)| i + 1))
            .collect(),
        ids: ids(program),
        def: 0,
        locals: vec![],
        slots: BTreeMap::new(),
        result: None,
    };
    for (i, def) in program.defs.iter().enumerate() {
        cx.def = i;
        cx.definition(def);
    }
    cx.core
}

fn ids(program: &ast::Program) -> Vec<usize> {
    let mut counts = [0; 4];
    let place = |def: &Def| {
        let list = match def {
            Def::Type { .. } => 0,
            Def::Enum { .. } => 1,
            Def::Fn { .. } => 2,
            Def::Example { .. } => 3,
            Def::App { .. } => return 0,
        };
        counts[list] += 1;
        counts[list] - 1
    };
    program.defs.iter().map(place).collect()
}

impl<'a> Cx<'_, 'a> {
    fn definition(&mut self, def: &ast::Def<'a>) {
        let table = self.table;
        match (def, &table.defs[self.def].item) {
            (Def::Type { name, .. }, Item::Record(fields)) => {
                let fields = self.fields(fields);
                let name = name.text.into();
                self.core.records.push(Record { name, fields });
            }
            (Def::Enum { name, .. }, Item::Enum(variants)) => {
                let variants = variants.iter().map(|v| Variant {
                    name: v.name.into(),
                    fields: self.fields(&v.fields),
                });
                let variants = variants.collect();
                let name = name.text.into();
                self.core.enums.push(Enum { name, variants });
            }
            (
                Def::Fn {
                    name, params, body, ..
                },
                Item::Fn(sig),
            ) => {
                let f = self.function(name, params, Some(sig), body);
                self.core.fns.push(f);
            }
            (Def::Example { name, body }, _) => {
                let f = self.function(name, &[], None, body);
                self.core.examples.push(f);
            }
            (Def::App { name, entries }, _) => {
                let app = self.app(name, entries);
                self.core.app = Some(app);
            }
            _ => unreachable!("a definition and its table row agree"),
        }
    }

    fn fields(&mut self, fields: &[sig::Field]) -> Vec<Field> {
        let field = |f: &sig::Field| Field {
            name: f.name.into(),
            ty: self.ty(f.ty),
        };
        fields.iter().map(field).collect()
    }

    /// A function, or with no `sig` an example: no parameters and a Bool result.
    fn function(
        &mut self,
        name: &ast::Name,
        params: &[ast::Param],
        sig: Option<&sig::Sig>,
        body: &ast::Block<'a>,
    ) -> Fn {
        self.locals.clear();
        self.slots.clear();
        let site = self.site(name.at);
        let held = sig.map_or(&[][..], |s| &s.params[..]);
        let params: Vec<Param> = params
            .iter()
            .zip(held)
            .map(|(p, h)| {
                let ty = self.ty(h.ty);
                self.declare(p.name.at, ty);
                Param { ty, inout: p.inout }
            })
            .collect();
        self.result = match sig {
            Some(s) => s.result.map(|r| self.ty(r)),
            None => Some(self.core.ty(Type::Bool)),
        };
        let body = self.block(body, self.result.is_some());
        Fn {
            name: name.text.into(),
            site,
            params,
            locals: std::mem::take(&mut self.locals),
            result: self.result,
            body,
        }
    }

    fn app(&mut self, name: &ast::Name, entries: &[Entry<'a>]) -> App {
        let mut app = App {
            name: name.text.into(),
            state: None,
            start: 0,
            update: 0,
            grants: Default::default(),
        };
        for entry in entries {
            match entry {
                Entry::State(_, n) => app.state = Some(self.state(n)),
                Entry::Start(_, e) => {
                    let Some(Binding::Variant(_, v)) = self.resolved.get(e.at) else {
                        unreachable!("start names a variant");
                    };
                    app.start = v;
                }
                Entry::Update(_, n) => {
                    let Some(Binding::Def(f)) = self.resolved.get(n.at) else {
                        unreachable!("update names a function");
                    };
                    app.update = self.ids[f];
                }
                Entry::Grants(_, names) => app.grants = names.iter().map(grant).collect(),
            }
        }
        app
    }

    /// The record `n` names, built from its defaults.
    fn state(&mut self, n: &ast::Name) -> Expr {
        let Some(Binding::Def(d)) = self.resolved.get(n.at) else {
            unreachable!("state names a record");
        };
        let kind = self.record(d, &[]);
        let ExprKind::Record(id, _) = kind else {
            unreachable!("a record literal");
        };
        Expr {
            ty: Some(self.core.ty(Type::Record(id))),
            kind,
        }
    }

    /// The core type of `t`, or none for a type still open.
    fn conv(&mut self, t: sig::Ty) -> Option<Ty> {
        let table = self.table;
        let node = match table.types.node(t) {
            Node::Name(name, args) => {
                let a = args
                    .iter()
                    .map(|&x| self.conv(x))
                    .collect::<Option<Vec<_>>>()?;
                match (*name, &a[..]) {
                    ("Int", []) => Type::Int,
                    ("Float", []) => Type::Float,
                    ("Bool", []) => Type::Bool,
                    ("String", []) => Type::String,
                    ("List", &[x]) => Type::List(x),
                    ("Map", &[k, v]) => Type::Map(k, v),
                    ("Option", &[x]) => Type::Option(x),
                    ("Result", &[x, e]) => Type::Result(x, e),
                    ("Effect", &[m]) => Type::Effect(m),
                    _ => unreachable!("a built-in type and its arguments"),
                }
            }
            Node::User(d) => match table.defs[*d].item {
                Item::Record(_) => Type::Record(self.ids[*d]),
                _ => Type::Enum(self.ids[*d]),
            },
            Node::Fn(params, result) => {
                let params = params
                    .iter()
                    .map(|&p| self.conv(p))
                    .collect::<Option<_>>()?;
                let result = match result {
                    Some(r) => Some(self.conv(*r)?),
                    None => None,
                };
                Type::Fn(params, result)
            }
            Node::Var(_) => return None,
        };
        Some(self.core.ty(node))
    }

    /// The core type of a declared or bound type, which is complete.
    fn ty(&mut self, t: sig::Ty) -> Ty {
        self.conv(t).expect("a declared type is complete")
    }

    /// The type the checker gave `e`, when it gives a value.
    fn entry(&self, e: &ast::Expr) -> Option<sig::Ty> {
        self.bodies.types[self.def].get(&(e.at, e.len)).copied()
    }

    fn ty_of(&mut self, e: &ast::Expr) -> Option<Ty> {
        self.entry(e).and_then(|t| self.conv(t))
    }

    fn is(&self, ty: Option<Ty>, want: Type) -> bool {
        ty.is_some_and(|t| self.core.types[t] == want)
    }

    fn local(&mut self, ty: Ty) -> Local {
        self.locals.push(ty);
        self.locals.len() - 1
    }

    /// A local named at `at`.
    fn declare(&mut self, at: Pos, ty: Ty) -> Local {
        let local = self.local(ty);
        self.slots.insert(at, local);
        local
    }

    fn site(&mut self, at: Pos) -> Site {
        self.site_of(at, None)
    }

    fn site_of(&mut self, (line, col): Pos, hole: Option<String>) -> Site {
        self.core.sites.push(TrapSite { line, col, hole });
        self.core.sites.len() - 1
    }

    /// The position of the first token after `e`: the `[` of an index, the operator of `op=`.
    fn after(&self, e: &ast::Expr) -> Pos {
        let mut at = self.offset(e.at) + e.len;
        loop {
            match self.source.get(at..) {
                Some([b' ' | b'\n', ..]) => at += 1,
                Some(rest @ [b'/', b'/', ..]) => {
                    at += rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
                }
                _ => return self.position(at),
            }
        }
    }

    fn offset(&self, (line, col): Pos) -> usize {
        self.starts[line - 1] + col - 1
    }

    fn position(&self, offset: usize) -> Pos {
        let line = self.starts.partition_point(|&s| s <= offset);
        (line, offset - self.starts[line - 1] + 1)
    }
}

fn grant(name: &ast::Name) -> Grant {
    match name.text {
        "Console" => Grant::Console,
        "ReadFiles" => Grant::ReadFiles,
        "WriteFiles" => Grant::WriteFiles,
        _ => unreachable!("a checked grant"),
    }
}

/// The tag of a built-in variant.
fn builtin_tag(name: &str) -> Tag {
    use core_form::Effect::{Exit, ExitWith, Print, ReadFile, WriteFile};
    match name {
        "Some" => Tag::Some,
        "None" => Tag::None,
        "Ok" => Tag::Ok,
        "Err" => Tag::Err,
        "Print" => Tag::Effect(Print),
        "ReadFile" => Tag::Effect(ReadFile),
        "WriteFile" => Tag::Effect(WriteFile),
        "Exit" => Tag::Effect(Exit),
        "ExitWith" => Tag::Effect(ExitWith),
        _ => unreachable!("a built-in variant"),
    }
}
