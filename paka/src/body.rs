//! The statement and expression checker: every body is checked against the signature table
//! alone, never against another definition's body (docs/decisions/M2-02.md, M2-09.md, M2-12.md).
//! A definition's result is its first error, so an error in one never changes another's.
//!
//! Ownership and overlap (M2-05) are checked after the types, in `own`.
//!
//! Not here: the error forms (M2-31), which are left unchecked.

use crate::ast::{self, Block, Def, Expr, ExprKind as K, Pos, Program};
use crate::diagnostic::Diagnostic;
use crate::resolve::Resolved;
use crate::sig::{Defs, Item, Node, Sig, Table, Ty, Types};
use std::collections::{BTreeMap, BTreeSet};
use text::{float_fix, named, render};

mod app;
mod arms;
mod branch;
mod call;
mod expr;
mod hole;
mod own;
mod pattern;
mod place;
mod settle;
mod stmt;
#[cfg(test)]
mod tests;
mod text;

/// The result of checking a program's bodies, one entry per definition.
pub struct Bodies {
    /// The first error of each definition, or none.
    pub defs: Vec<Option<Diagnostic>>,
    /// The type of every expression met, by (position, length in bytes); partial after an error.
    pub types: Vec<BTreeMap<(Pos, usize), Ty>>,
    /// The row picked for each call of a built-in that has several, by (position, length in
    /// bytes) of the name: the row's place among the rows of that name.
    pub rows: Vec<BTreeMap<(Pos, usize), usize>>,
    /// The signatures each definition read, by name (M4-09).
    pub reads: Vec<BTreeSet<String>>,
}

type Res<T = ()> = Result<T, Diagnostic>;

/// Where an error stands: the position and the length in bytes.
type Span = (Pos, usize);

fn span<K>(e: &ast::Node<K>) -> Span {
    (e.at, e.len)
}

/// What an expression gives.
#[derive(Clone, Copy)]
enum Out {
    Val(Ty),
    /// A call without a result, or a statement.
    Unit,
    /// Control leaves: `return`, `break`, `continue`, `unreachable()`.
    Never,
}

/// How the last line of a block is used.
#[derive(Clone, Copy)]
enum Use {
    Dropped,
    /// As the value of the block, of this type when it is known.
    Value(Option<Ty>),
}

impl Use {
    /// The type the value is wanted to have, when it is known.
    fn want(self) -> Option<Ty> {
        match self {
            Use::Value(want) => want,
            Use::Dropped => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum How {
    Let,
    Var,
    Param,
    Inout,
    For,
    Pattern,
}

struct Local {
    ty: Ty,
    how: How,
}

struct Cx<'t, 'a> {
    path: &'t str,
    program: &'a Program<'a>,
    resolved: &'t Resolved,
    types: &'t mut Types<'a>,
    defs: &'t Defs<'a>,
    fns: &'t BTreeMap<&'static str, Vec<Sig<'static>>>,
    variants: &'t BTreeMap<&'static str, Sig<'static>>,
    /// The definition being checked, its result and the `return`s that end it.
    def: usize,
    result: Option<Ty>,
    tails: BTreeSet<Pos>,
    /// The start of the assignment target being checked.
    writing: Option<Pos>,
    loops: usize,
    /// Every local met, by the position of its name.
    locals: BTreeMap<Pos, Local>,
    /// The place each `for` name walks, as written, when it walks a place.
    walks: BTreeMap<Pos, String>,
    info: BTreeMap<(Pos, usize), Ty>,
    picks: BTreeMap<Span, usize>,
}

pub fn check<'a>(
    path: &str,
    program: &'a Program<'a>,
    resolved: &Resolved,
    table: &mut Table<'a>,
) -> Bodies {
    let Table {
        types,
        defs,
        fns,
        variants,
    } = table;
    let mut cx = Cx {
        path,
        program,
        resolved,
        types,
        defs,
        fns,
        variants,
        def: 0,
        result: None,
        tails: BTreeSet::new(),
        writing: None,
        loops: 0,
        locals: BTreeMap::new(),
        walks: BTreeMap::new(),
        info: BTreeMap::new(),
        picks: BTreeMap::new(),
    };
    let mut bodies = Bodies {
        defs: Vec::new(),
        types: Vec::new(),
        rows: Vec::new(),
        reads: Vec::new(),
    };
    for (i, def) in program.defs.iter().enumerate() {
        cx.defs.take_reads();
        bodies.defs.push(cx.definition(i, def));
        bodies.reads.push(cx.defs.take_reads());
        bodies.types.push(std::mem::take(&mut cx.info));
        bodies.rows.push(std::mem::take(&mut cx.picks));
    }
    bodies
}

/// The error that comes first in the source.
fn earliest(a: Option<Diagnostic>, b: Option<Diagnostic>) -> Option<Diagnostic> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y.position < x.position { y } else { x }),
        (a, b) => a.or(b),
    }
}

impl<'t, 'a> Cx<'t, 'a> {
    /// The first error among the resolver's, the table's and the body's. The body is checked
    /// whatever the others found; a name or type they left unknown is silent here.
    fn definition(&mut self, i: usize, def: &'a Def<'a>) -> Option<Diagnostic> {
        (self.def, self.loops, self.result, self.writing) = (i, 0, None, None);
        self.locals.clear();
        self.walks.clear();
        let defs = self.defs;
        let found = match (def, &defs[i].item) {
            (Def::Fn { params, body, .. }, Item::Fn(sig)) => self.function(sig, params, body).err(),
            (Def::Type { fields, .. }, Item::Record(held)) => (fields.iter().zip(held))
                .find_map(|(f, h)| self.expect(f.default.as_ref()?, h.ty).err()),
            (Def::Example { body, .. }, _) => self.example(body).err(),
            (Def::App { name, entries }, Item::App { state, update }) => {
                self.app(name, entries, (*state, *update)).err()
            }
            _ => None,
        };
        earliest(defs[i].error.clone(), found)
    }

    fn function(&mut self, sig: &Sig<'_>, params: &[ast::Param<'a>], body: &'a Block<'a>) -> Res {
        for (p, s) in params.iter().zip(&sig.params) {
            let how = if p.inout { How::Inout } else { How::Param };
            self.locals.insert(p.name.at, Local { ty: s.ty, how });
        }
        self.run(body, sig.result)
    }

    /// An example is a function without parameters that gives a Bool.
    fn example(&mut self, body: &'a Block<'a>) -> Res {
        let bool = self.prim("Bool");
        self.run(body, Some(bool))
    }

    fn run(&mut self, body: &'a Block<'a>, result: Option<Ty>) -> Res {
        self.result = result;
        self.tails = stmt::tails(body);
        let used = result.map_or(Use::Dropped, |r| Use::Value(Some(r)));
        let typed = self.block(body, used).err();
        earliest(typed, self.ownership(body)).map_or(Ok(()), Err)
    }

    fn fail(&self, (at, len): Span, code: &'static str, message: impl ToString) -> Diagnostic {
        let name = self.program.defs[self.def].name().text;
        Diagnostic::new(self.path, at, len, code, message).in_definition(name)
    }

    fn fail_at(&self, e: &Expr, code: &'static str, message: impl ToString) -> Diagnostic {
        self.fail(span(e), code, message)
    }

    /// `type-mismatch` at `at`; a repair is the last words of the message.
    fn mismatch(&self, at: Span, want: &str, found: &str, repair: &str) -> Diagnostic {
        let repair = if repair.is_empty() {
            String::new()
        } else {
            format!("; {repair}")
        };
        let message = format!("expected `{want}`, found `{found}`{repair}");
        (self.fail(at, "type-mismatch", message)).with_types(&want, &found)
    }

    /// `e` has type `found` where `want` is needed; an Int and a Float name the other's repair.
    fn mismatch_at(&self, e: &Expr, want: Ty, found: Ty) -> Diagnostic {
        if self.is_function(want) {
            let by = match e.kind {
                K::Name(name) if self.is_function(found) => Some(name),
                _ => None,
            };
            return self.function_mismatch(span(e), want, found, by);
        }
        let (want, found) = (self.types.show(want), self.types.show(found));
        let repair = match (want.as_str(), found.as_str()) {
            ("Float", "Int") => float_fix(e),
            ("Int", "Float") => format!("write to_int({})", render(e)),
            _ => String::new(),
        };
        self.mismatch(span(e), &want, &found, &repair)
    }

    fn is_function(&self, ty: Ty) -> bool {
        matches!(self.types.node(ty), Node::Fn(..))
    }

    /// A function type was needed (M2-09 section 5): the types are not quoted, and a function that
    /// is named is named again.
    fn function_mismatch(&self, at: Span, want: Ty, found: Ty, by: Option<&str>) -> Diagnostic {
        let (want, found) = (self.types.show(want), self.types.show(found));
        let by = by.map_or(String::new(), |name| format!(" ({name})"));
        let message = format!("expected {want}, found {found}{by}");
        (self.fail(at, "type-mismatch", message)).with_types(&want, &found)
    }

    fn no_value(&self, e: &Expr, want: Option<Ty>) -> Diagnostic {
        let want = want.map_or("a value".to_string(), |w| self.types.show(w));
        let message = match named(e) {
            Some(name) => format!("{name} has no result"),
            None => format!("expected `{want}`, found `no value`"),
        };
        (self.fail_at(e, "type-mismatch", message)).with_types(&want, &"no value")
    }

    fn prim(&mut self, name: &'static str) -> Ty {
        self.types.named(name, &[])
    }

    fn letter(&mut self, c: char) -> Ty {
        self.types.intern(Node::Var(c))
    }

    fn unknown(&mut self) -> Ty {
        self.types.unknown()
    }

    fn is_var(&self, ty: Ty) -> bool {
        matches!(self.types.node(ty), Node::Var(_))
    }

    /// Whether `ty` is the built-in type `name`, whatever its arguments.
    fn is(&self, ty: Ty, name: &str) -> bool {
        matches!(self.types.node(ty), Node::Name(n, _) if *n == name)
    }

    /// Argument `i` of `ty` when it is the built-in type `name`.
    fn arg(&self, ty: Ty, name: &str, i: usize) -> Option<Ty> {
        match self.types.node(ty) {
            Node::Name(n, args) if *n == name => args.get(i).copied(),
            _ => None,
        }
    }

    /// Whether a letter of a built-in row is still unbound in `ty`.
    fn free(&self, ty: Ty) -> bool {
        match self.types.node(ty) {
            Node::Var(c) => *c != '_',
            Node::Name(_, args) => args.iter().any(|&t| self.free(t)),
            Node::Fn(params, result) => params.iter().chain(result).any(|&t| self.free(t)),
            Node::User(_) => false,
        }
    }

    fn note(&mut self, e: &Expr, out: &Out) {
        if let Out::Val(ty) = out {
            self.info.insert((e.at, e.len), *ty);
        }
    }
}
