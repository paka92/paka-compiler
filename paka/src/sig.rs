//! The signature table of a parsed and resolved program: the type of every definition's interface
//! and the built-in rows, one entry per definition (docs/decisions/M2-05.md, M2-09.md, M2-14.md,
//! M2-15.md). Bodies, patterns and expressions are not looked at but for the defaults of fields,
//! where only the shape and the outermost type are (what a default holds is left to the
//! expression checker). Each definition's result depends on its own text and on the signatures it
//! names, so one definition's error never changes another's result.

use crate::ast::{self, Def, Entry, Pos, Program, Type};
use crate::builtins::{self, Kind};
use crate::diagnostic::Diagnostic;
use crate::resolve::{Binding, Resolved};
use std::collections::BTreeMap;

mod check;
mod default;
mod graph;
mod hash;
mod row;
#[cfg(test)]
mod tests;

type Res<T = ()> = Result<T, Diagnostic>;

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

/// Every type met, once. `Ty(i)` is the user type of definition `i`.
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
    fn new(names: Vec<&'a str>) -> Types<'a> {
        let mut types = Types {
            names,
            nodes: Vec::new(),
            ids: BTreeMap::new(),
        };
        for i in 0..types.names.len() {
            types.intern(Node::User(i));
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

    pub fn user(&self, def: usize) -> Ty {
        Ty(u32::try_from(def).expect("fewer than 2^32 definitions"))
    }

    /// The spelling of a signature: `fn(A, B) -> R`, `Map<K, V>`.
    pub fn show(&self, ty: Ty) -> String {
        let mut out = String::new();
        self.render(ty, &mut out, &|i, out| out.push_str(self.names[i]));
        out
    }

    /// Writes `ty` with each user type written by `user`.
    fn render(&self, ty: Ty, out: &mut String, user: &dyn Fn(usize, &mut String)) {
        let list =
            |out: &mut String, tys: &[Ty]| join(out, tys, |out, &t| self.render(t, out, user));
        match self.node(ty) {
            Node::Var(c) => out.push(*c),
            Node::User(i) => user(*i, out),
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
                    self.render(*r, out, user);
                }
            }
        }
    }

    /// The types of the program `ty` mentions, each with whether it is held by value: not behind
    /// a List, Map, Option, Effect or function type.
    fn refs(&self, ty: Ty, by_value: bool, out: &mut Vec<(usize, bool)>) {
        match self.node(ty) {
            Node::User(i) => out.push((*i, by_value)),
            Node::Name(name, args) => {
                let inside = by_value && *name == "Result";
                args.iter().for_each(|&a| self.refs(a, inside, out));
            }
            Node::Fn(params, result) => {
                params
                    .iter()
                    .chain(result)
                    .for_each(|&a| self.refs(a, false, out));
            }
            Node::Var(_) => {}
        }
    }

    /// Whether a function type stands in `ty`, not looking into the types of the program.
    fn has_fn(&self, ty: Ty) -> bool {
        match self.node(ty) {
            Node::Fn(..) => true,
            Node::Name(_, args) => args.iter().any(|&a| self.has_fn(a)),
            _ => false,
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
    /// The type named after `state:` and the function named after `update:`, when they name any.
    App {
        state: Option<Ty>,
        update: Option<usize>,
    },
}

impl<'a> Item<'a> {
    fn is_type(&self) -> bool {
        matches!(self, Item::Record(_) | Item::Enum(_))
    }

    /// The fields of a record, or of every variant in order.
    fn each_field(&self) -> impl Iterator<Item = &Field<'a>> {
        let (record, variants) = match self {
            Item::Record(fields) => (&fields[..], &[][..]),
            Item::Enum(variants) => (&[][..], &variants[..]),
            _ => (&[][..], &[][..]),
        };
        record.iter().chain(variants.iter().flat_map(|v| &v.fields))
    }
}

/// One entry per definition of the program, in order, and the built-in rows.
pub struct Table<'a> {
    pub types: Types<'a>,
    pub defs: Vec<Res<Item<'a>>>,
    pub fns: BTreeMap<&'static str, Vec<Sig<'static>>>,
    pub variants: BTreeMap<&'static str, Sig<'static>>,
    hashes: Vec<Option<u64>>,
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
        let mut types = Types::new(program.defs.iter().map(|d| d.name().text).collect());
        let (fns, variants) = built_ins(&mut types);
        let mut convert = Convert {
            path,
            program,
            resolved,
            variants: &variants,
            types: &mut types,
            def: 0,
        };
        let mut defs: Vec<_> = (program.defs.iter().enumerate())
            .map(|(i, d)| convert.def(i, d))
            .collect();
        let comps = graph::components(&check::edges(&types, &defs));
        let hashes = hash::all(program, &types, &defs, &comps);
        let mut rejected = check::finite_values(path, program, &types, &defs, &comps);
        rejected.extend(check::state_functions(path, program, &types, &defs));
        for (i, d) in rejected {
            defs[i] = Err(d);
        }
        Table {
            types,
            defs,
            fns,
            variants,
            hashes,
        }
    }

    /// The interface hash of definition `def`; none for an example or a rejected definition.
    pub fn hash(&self, def: usize) -> Option<u64> {
        self.defs[def].as_ref().ok().and(self.hashes[def])
    }
}

/// Turns the definitions' signatures into table entries.
struct Convert<'t, 'a> {
    path: &'t str,
    program: &'a Program<'a>,
    resolved: &'t Resolved,
    variants: &'t BTreeMap<&'static str, Sig<'static>>,
    types: &'t mut Types<'a>,
    /// The definition being converted.
    def: usize,
}

const NOT_CONSTANT: &str =
    "a default is a constant: a literal, [], [:], a variant or a record built from constants";

impl<'a> Convert<'_, 'a> {
    fn error(&self, at: Pos, len: usize, code: &'static str, message: impl ToString) -> Diagnostic {
        let name = self.program.defs[self.def].name().text;
        Diagnostic::new(self.path, at, len, code, message).in_definition(name)
    }

    /// The resolver's error for the definition: it names the first unknown name or wrong count.
    fn rejected(&self) -> Diagnostic {
        self.resolved.defs[self.def]
            .clone()
            .expect_err("the resolver rejected the definition")
    }

    fn def(&mut self, i: usize, def: &'a Def<'a>) -> Res<Item<'a>> {
        self.def = i;
        match def {
            Def::Type { fields, .. } => Ok(Item::Record(self.fields(fields)?)),
            Def::Enum { variants, .. } => {
                let variant = |v: &'a ast::Variant<'a>| {
                    Ok(Variant {
                        name: v.name.text,
                        fields: self.fields(&v.fields)?,
                    })
                };
                Ok(Item::Enum(
                    variants.iter().map(variant).collect::<Res<_>>()?,
                ))
            }
            Def::Fn { params, result, .. } => self.sig(params, result.as_ref()).map(Item::Fn),
            Def::Example { .. } => Ok(Item::Example),
            Def::App { entries, .. } => Ok(self.app(entries)),
        }
    }

    fn app(&self, entries: &[Entry]) -> Item<'a> {
        let named = |n: &ast::Name| match self.resolved.get(n.at) {
            Some(Binding::Def(i)) => Some(i),
            _ => None,
        };
        let (mut state, mut update) = (None, None);
        for entry in entries {
            match entry {
                Entry::State(_, n) => state = named(n).map(|i| self.types.user(i)),
                Entry::Update(_, n) => update = named(n),
                _ => {}
            }
        }
        Item::App { state, update }
    }

    fn sig(&mut self, params: &[ast::Param<'a>], result: Option<&Type<'a>>) -> Res<Sig<'a>> {
        let param = |p: &ast::Param<'a>| {
            Ok(Param {
                name: p.name.text,
                inout: p.inout,
                labelled: p.labelled,
                ty: self.ty(&p.ty)?,
            })
        };
        let params = params.iter().map(param).collect::<Res<_>>()?;
        Ok(Sig {
            params,
            result: result.map(|t| self.ty(t)).transpose()?,
        })
    }

    fn fields(&mut self, fields: &[ast::Field<'a>]) -> Res<Vec<Field<'a>>> {
        self.resolved.defs[self.def].clone()?;
        let field = |f: &ast::Field<'a>| {
            let ty = self.ty(&f.ty)?;
            f.default
                .iter()
                .try_for_each(|d| self.default(f.name.text, ty, d))?;
            Ok(Field {
                name: f.name.text,
                ty,
                default: f.default.is_some(),
            })
        };
        fields.iter().map(field).collect()
    }

    fn tys(&mut self, types: &[Type<'a>]) -> Res<Box<[Ty]>> {
        types.iter().map(|t| self.ty(t)).collect()
    }

    /// The type `t` names. A name the resolver did not bind, or bound with the wrong number of
    /// arguments, is the resolver's error for the definition.
    fn ty(&mut self, t: &Type<'a>) -> Res<Ty> {
        let node = match t {
            Type::Name { name, args } => match self.resolved.get(name.at) {
                Some(Binding::Def(i)) if args.is_empty() => Node::User(i),
                Some(Binding::Builtin(b)) if args.len() == builtins::arity(b) => {
                    let tys = self.tys(args)?;
                    if b == "Map" {
                        self.key(tys[0], &args[0])?;
                    }
                    Node::Name(b, tys)
                }
                _ => return Err(self.rejected()),
            },
            Type::Fn { params, result, .. } => {
                let params = self.tys(params)?;
                Node::Fn(params, result.as_deref().map(|r| self.ty(r)).transpose()?)
            }
        };
        Ok(self.types.intern(node))
    }

    /// A Map key is an Int or a String (M2-14 section 1.1).
    fn key(&self, key: Ty, written: &Type) -> Res {
        if matches!(self.types.node(key), Node::Name("Int" | "String", _)) {
            return Ok(());
        }
        let (at, len) = match written {
            Type::Name { name, .. } => (name.at, name.text.len()),
            Type::Fn { at, .. } => (*at, 2),
        };
        let message = "a Map key is an Int or a String";
        Err(self
            .error(at, len, "type-mismatch", message)
            .with_types(&"Int or String", &self.types.show(key)))
    }
}
