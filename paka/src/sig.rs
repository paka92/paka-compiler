//! The signature table of a parsed and resolved program: the type of every definition's interface
//! and the built-in rows (docs/decisions/M2-05.md, M2-09.md, M2-14.md, M2-15.md). Bodies, patterns
//! and expressions are not looked at but for the defaults of fields, where only the shape and the
//! outermost type are (what a default holds is left to the expression checker; so is a bare
//! variant that has fields).
//!
//! A definition has an `item`, which holds its well-formed parts (a type with an error is `_`),
//! and an `error`, the earliest in source order of its name errors, signature errors and, for a
//! type, a missing finite value. An error does not make the signature unknown. An item depends on
//! the text of its definition and the names it uses; an error also, for a finite value, on the
//! types it holds.

use crate::ast::{self, Def, Entry, Pos, Program, Type};
use crate::builtins::{self, Kind};
use crate::diagnostic::Diagnostic;
use crate::resolve::{Binding, Resolved};
use std::collections::BTreeMap;

mod check;
mod default;
mod graph;
mod row;
#[cfg(test)]
mod tests;

/// A type, as an id into `Types`: equal types have equal ids, so comparing costs one compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ty(u32);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    /// A built-in type with its arguments.
    Name(&'static str, Box<[Ty]>),
    /// A type of the program, by the index of its definition.
    User(usize),
    Fn(Box<[Ty]>, Option<Ty>),
    /// A type variable of a built-in row, or the unknown part of a default.
    Var(char),
}

/// Every type met, once.
pub struct Types<'a> {
    names: Vec<&'a str>,
    nodes: Vec<Node>,
    ids: BTreeMap<Node, Ty>,
}

fn join<T>(
    out: &mut String,
    items: impl IntoIterator<Item = T>,
    mut each: impl FnMut(&mut String, T),
) {
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        each(out, item);
    }
}

impl<'a> Types<'a> {
    fn new(defs: &'a [Def<'a>]) -> Types<'a> {
        let mut types = Types {
            names: defs.iter().map(|d| d.name().text).collect(),
            nodes: Vec::new(),
            ids: BTreeMap::new(),
        };
        for (i, def) in defs.iter().enumerate() {
            if matches!(def, Def::Type { .. } | Def::Enum { .. }) {
                types.intern(Node::User(i));
            }
        }
        types
    }

    pub fn intern(&mut self, node: Node) -> Ty {
        if let Some(&ty) = self.ids.get(&node) {
            return ty;
        }
        let ty = Ty(u32::try_from(self.nodes.len()).expect("fewer than 2^32 types"));
        self.nodes.push(node.clone());
        self.ids.insert(node, ty);
        ty
    }

    fn named(&mut self, name: &'static str, args: &[Ty]) -> Ty {
        self.intern(Node::Name(name, args.into()))
    }

    pub fn node(&self, ty: Ty) -> &Node {
        &self.nodes[ty.0 as usize]
    }

    /// The type definition `def` declares; none for a definition of any other kind.
    pub fn user(&self, def: usize) -> Option<Ty> {
        self.ids.get(&Node::User(def)).copied()
    }

    /// The unknown type, which stands for a part with an error.
    fn unknown(&mut self) -> Ty {
        self.intern(Node::Var('_'))
    }

    /// The spelling of a signature: `fn(A, B) -> R`, `Map<K, V>`.
    pub fn show(&self, ty: Ty) -> String {
        let mut out = String::new();
        self.render(ty, &mut out);
        out
    }

    fn render(&self, ty: Ty, out: &mut String) {
        let list = |out: &mut String, tys: &[Ty]| join(out, tys, |out, &t| self.render(t, out));
        match self.node(ty) {
            Node::Var(c) => out.push(*c),
            Node::User(i) => out.push_str(self.names[*i]),
            Node::Name(name, args) => {
                out.push_str(name);
                if !args.is_empty() {
                    out.push('<');
                    list(out, args);
                    out.push('>');
                }
            }
            Node::Fn(params, result) => {
                out.push_str("fn(");
                list(out, params);
                out.push(')');
                if let Some(r) = result {
                    out.push_str(" -> ");
                    self.render(*r, out);
                }
            }
        }
    }

    /// Pushes the types of the program a value of `ty` holds by value: those it names, looking
    /// through a Result and nothing else. True when a value exists whatever they are: a part is
    /// an empty List, a `None`, a function or an Int; a Result needs one side to be.
    fn needs(&self, ty: Ty, out: &mut Vec<usize>) -> bool {
        match self.node(ty) {
            Node::User(i) => {
                out.push(*i);
                false
            }
            Node::Name("Result", args) => (args.iter())
                .map(|&a| self.needs(a, out))
                .fold(false, |x, y| x | y),
            _ => true,
        }
    }

    /// Whether a value of type `found` can stand where `want` does; a variable fits anything.
    fn fits(&self, found: Ty, want: Ty) -> bool {
        match (self.node(found), self.node(want)) {
            (Node::Var(_), _) | (_, Node::Var(_)) => true,
            (Node::Name(a, x), Node::Name(b, y)) => {
                a == b
                    && x.len() == y.len()
                    && x.iter().zip(y.iter()).all(|(&p, &q)| self.fits(p, q))
            }
            (a, b) => a == b,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Param<'a> {
    pub name: &'a str,
    pub inout: bool,
    pub labelled: bool,
    pub ty: Ty,
}

#[derive(Clone, Debug)]
pub struct Sig<'a> {
    pub params: Vec<Param<'a>>,
    pub result: Option<Ty>,
}

#[derive(Clone, Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: Ty,
    pub default: bool,
}

#[derive(Clone, Debug)]
pub struct Variant<'a> {
    pub name: &'a str,
    pub fields: Vec<Field<'a>>,
}

#[derive(Clone, Debug)]
pub enum Item<'a> {
    Record(Vec<Field<'a>>),
    Enum(Vec<Variant<'a>>),
    Fn(Sig<'a>),
    Example,
    /// The type after the first `state:` and the function after the first `update:`, when those
    /// name any; the rules of an app are the app task's (M2-32).
    App {
        state: Option<Ty>,
        update: Option<usize>,
    },
}

impl<'a> Item<'a> {
    fn is_type(&self) -> bool {
        matches!(self, Item::Record(_) | Item::Enum(_))
    }

    /// The field lists a value of the type picks one of: the record's, or one for each variant.
    fn alternatives(&self) -> impl Iterator<Item = &[Field<'a>]> {
        let (record, variants) = match self {
            Item::Record(fields) => (Some(&fields[..]), &[][..]),
            Item::Enum(variants) => (None, &variants[..]),
            _ => (None, &[][..]),
        };
        record
            .into_iter()
            .chain(variants.iter().map(|v| &v.fields[..]))
    }

    fn each_field(&self) -> impl Iterator<Item = &Field<'a>> {
        self.alternatives().flatten()
    }
}

/// A definition's `item` and its `error`, if any.
pub struct Checked<'a> {
    pub item: Item<'a>,
    pub error: Option<Diagnostic>,
}

/// One entry per definition of the program, in order, and the built-in rows.
pub struct Table<'a> {
    pub types: Types<'a>,
    pub defs: Vec<Checked<'a>>,
    pub fns: BTreeMap<&'static str, Vec<Sig<'static>>>,
    pub variants: BTreeMap<&'static str, Sig<'static>>,
}

/// Keeps `d` in `slot` when it comes before what is there in the source.
fn earliest(slot: &mut Option<Diagnostic>, d: Diagnostic) {
    if slot.as_ref().is_none_or(|e| d.position < e.position) {
        *slot = Some(d);
    }
}

type BuiltIns = (
    BTreeMap<&'static str, Vec<Sig<'static>>>,
    BTreeMap<&'static str, Sig<'static>>,
);

fn built_ins(types: &mut Types) -> BuiltIns {
    let (mut fns, mut variants) = (BTreeMap::new(), BTreeMap::new());
    for &(kind, text) in builtins::ROWS {
        let name = builtins::name(text);
        match kind {
            Kind::Fn => fns
                .entry(name)
                .or_insert_with(Vec::new)
                .push(row::sig(types, text)),
            Kind::Variant => drop(variants.insert(name, row::sig(types, text))),
            _ => {}
        }
    }
    (fns, variants)
}

impl<'a> Table<'a> {
    pub fn build(path: &str, program: &'a Program<'a>, resolved: &Resolved) -> Table<'a> {
        let mut types = Types::new(&program.defs);
        let (fns, variants) = built_ins(&mut types);
        let mut convert = Convert {
            path,
            program,
            resolved,
            named: type_names(program),
            variants: &variants,
            types: &mut types,
            def: 0,
            lost: 0,
            error: None,
        };
        let mut defs: Vec<_> = (program.defs.iter().enumerate())
            .map(|(i, d)| convert.def(i, d))
            .collect();
        for (i, d) in check::finite_values(path, program, &types, &defs) {
            earliest(&mut defs[i].error, d);
        }
        Table {
            types,
            defs,
            fns,
            variants,
        }
    }
}

/// What each name of a type stands for. A name is looked up here, not in the resolver's uses,
/// because the resolver binds no name after its first error in a definition.
fn type_names<'a>(program: &'a Program<'a>) -> BTreeMap<&'a str, Binding> {
    let built_in = (builtins::ROWS.iter())
        .filter(|r| r.0 == Kind::Type)
        .map(|r| (builtins::name(r.1), Binding::Builtin(builtins::name(r.1))));
    let declared = (program.defs.iter().enumerate())
        .filter(|(_, d)| matches!(d, Def::Type { .. } | Def::Enum { .. }))
        .map(|(i, d)| (d.name().text, Binding::Def(i)));
    built_in.chain(declared).collect()
}

/// Turns the definitions' signatures into table entries.
struct Convert<'t, 'a> {
    path: &'t str,
    program: &'a Program<'a>,
    resolved: &'t Resolved,
    named: BTreeMap<&'a str, Binding>,
    variants: &'t BTreeMap<&'static str, Sig<'static>>,
    types: &'t mut Types<'a>,
    /// The definition being converted.
    def: usize,
    /// How many types so far have an error and are taken for the unknown type.
    lost: usize,
    /// The earliest error found in the definition, the resolver's apart.
    error: Option<Diagnostic>,
}

impl<'a> Convert<'_, 'a> {
    fn error(&self, at: Pos, len: usize, code: &'static str, message: impl ToString) -> Diagnostic {
        let name = self.program.defs[self.def].name().text;
        Diagnostic::new(self.path, at, len, code, message).in_definition(name)
    }

    fn def(&mut self, i: usize, def: &'a Def<'a>) -> Checked<'a> {
        (self.def, self.error) = (i, None);
        let item = match def {
            Def::Type { fields, .. } => Item::Record(self.fields(fields)),
            Def::Enum { variants, .. } => Item::Enum(
                (variants.iter())
                    .map(|v| Variant {
                        name: v.name.text,
                        fields: self.fields(&v.fields),
                    })
                    .collect(),
            ),
            Def::Fn { params, result, .. } => Item::Fn(self.sig(params, result.as_ref())),
            Def::Example { .. } => Item::Example,
            Def::App { entries, .. } => self.app(entries),
        };
        if let Err(d) = &self.resolved.defs[i] {
            earliest(&mut self.error, d.clone());
        }
        Checked {
            item,
            error: self.error.take(),
        }
    }

    fn app(&self, entries: &[Entry]) -> Item<'a> {
        let def_of = |n: &ast::Name| match self.resolved.get(n.at) {
            Some(Binding::Def(i)) => Some(i),
            _ => None,
        };
        let state = entries.iter().find_map(|e| match e {
            Entry::State(_, n) => Some(n),
            _ => None,
        });
        let update = entries.iter().find_map(|e| match e {
            Entry::Update(_, n) => Some(n),
            _ => None,
        });
        Item::App {
            state: state.and_then(def_of).and_then(|i| self.types.user(i)),
            update: update.and_then(def_of),
        }
    }

    fn sig(&mut self, params: &[ast::Param<'a>], result: Option<&Type<'a>>) -> Sig<'a> {
        let param = |p: &ast::Param<'a>| Param {
            name: p.name.text,
            inout: p.inout,
            labelled: p.labelled,
            ty: self.ty(&p.ty),
        };
        Sig {
            params: params.iter().map(param).collect(),
            result: result.map(|t| self.ty(t)),
        }
    }

    fn fields(&mut self, fields: &[ast::Field<'a>]) -> Vec<Field<'a>> {
        let field = |f: &ast::Field<'a>| {
            let ty = self.ty(&f.ty);
            if let Some(d) = &f.default {
                self.default(f.name.text, ty, d);
            }
            Field {
                name: f.name.text,
                ty,
                default: f.default.is_some(),
            }
        };
        fields.iter().map(field).collect()
    }

    /// The type `t` names, or the unknown type when it has an error of its own.
    fn ty(&mut self, t: &Type<'a>) -> Ty {
        let lost = self.lost;
        let node = match t {
            Type::Name { name, args } => match self.named.get(name.text) {
                Some(&Binding::Def(i)) if args.is_empty() => Node::User(i),
                Some(&Binding::Builtin(b)) if args.len() == builtins::arity(b) => {
                    let mut tys = Vec::new();
                    for arg in args {
                        let before = self.lost;
                        let ty = self.ty(arg);
                        if b == "Map" && tys.is_empty() && self.lost == before {
                            self.key(ty, arg);
                        }
                        tys.push(ty);
                    }
                    Node::Name(b, tys.into())
                }
                _ => {
                    self.lost += 1;
                    return self.types.unknown();
                }
            },
            Type::Fn { params, result, .. } => Node::Fn(
                params.iter().map(|p| self.ty(p)).collect(),
                result.as_deref().map(|r| self.ty(r)),
            ),
        };
        if self.lost > lost {
            return self.types.unknown();
        }
        self.types.intern(node)
    }

    /// A Map key is an Int or a String (M2-14 section 1.1).
    fn key(&mut self, key: Ty, written: &Type) {
        if matches!(self.types.node(key), Node::Name("Int" | "String", _)) {
            return;
        }
        let (at, len) = match written {
            Type::Name { name, .. } => (name.at, name.text.len()),
            Type::Fn { at, .. } => (*at, 2),
        };
        let message = "a Map key is an Int or a String";
        let d = (self.error(at, len, "type-mismatch", message))
            .with_types(&"Int or String", &self.types.show(key));
        earliest(&mut self.error, d);
        self.lost += 1;
    }
}
