//! Expressions. A chain of postfix and left-operand links is walked by a loop, so its length
//! costs no stack (M2-02 section 1.7).

use super::{Cx, builtin_tag};
use crate::ast::{self, BinOp, ExprKind as K, Pos, StrPart, UnOp};
use crate::builtins::{self, Kind};
use crate::core_form::{
    Arg, Arith, Bin, Callee, Cmp, Compound, Expr, ExprKind, FloatArith, FnRef, Place, Root, Site,
    Step, Tag, Ty, Type, Un,
};
use crate::resolve::Binding;
use crate::sig::{Item, Node};
use paka_rt::ops::{Float, Int};

/// An arithmetic operator by the type it works on.
pub(super) enum Math {
    Int(Arith, Site),
    Float(FloatArith),
    Join(Site),
}

impl From<Math> for Bin {
    fn from(m: Math) -> Bin {
        match m {
            Math::Int(op, site) => Bin::Int(op, site),
            Math::Float(op) => Bin::Float(op),
            Math::Join(site) => Bin::Join(site),
        }
    }
}

impl From<Math> for Compound {
    fn from(m: Math) -> Compound {
        match m {
            Math::Int(op, site) => Compound::Int(op, site),
            Math::Float(op) => Compound::Float(op),
            Math::Join(site) => Compound::Join(site),
        }
    }
}

/// The operand a link of a chain works on.
fn operand<'e, 'a>(e: &'e ast::Expr<'a>) -> Option<&'e ast::Expr<'a>> {
    match &e.kind {
        K::Unary(_, x)
        | K::Try(x, _)
        | K::Field(x, _)
        | K::Index(x, _)
        | K::Call(x, _)
        | K::Binary(_, _, x, _) => Some(x),
        _ => None,
    }
}

impl<'a> Cx<'_, 'a> {
    pub(super) fn expr(&mut self, e: &ast::Expr<'a>) -> Expr {
        let mut links = vec![];
        let mut head = e;
        while let Some(inner) = operand(head) {
            links.push(head);
            head = inner;
        }
        let mut links = links.into_iter().rev().peekable();
        let mut cur = match (&head.kind, links.peek().map(|l| &l.kind)) {
            (K::Name(_), Some(K::Call(..))) => {
                let call = links.next().expect("a peeked link");
                self.named_call(head, call)
            }
            _ => self.leaf(head),
        };
        for link in links {
            cur = self.link(link, cur);
        }
        cur
    }

    fn typed(&mut self, e: &ast::Expr<'a>, kind: ExprKind) -> Expr {
        Expr {
            ty: self.ty_of(e),
            kind,
        }
    }

    fn leaf(&mut self, e: &ast::Expr<'a>) -> Expr {
        let kind = match &e.kind {
            K::Int(text) => ExprKind::Int(Int::parse(text).expect("an Int in range")),
            K::Float(text) => ExprKind::Float(Float::parse(text).expect("a finite Float")),
            K::Bool(b) => ExprKind::Bool(*b),
            K::Str(parts) => self.string(e, parts),
            K::Hole(name) => ExprKind::Hole(self.site_of(e.at, Some(name.to_string()))),
            K::Name(_) => return self.name(e),
            K::Record(_, args) => self.record_literal(e, args),
            K::List(items) => ExprKind::List(items.iter().map(|x| self.expr(x)).collect()),
            K::Map(pairs) => {
                let pair = |(k, v): &(ast::Expr<'a>, ast::Expr<'a>)| (self.expr(k), self.expr(v));
                ExprKind::Map(pairs.iter().map(pair).collect())
            }
            K::If {
                branches,
                otherwise,
            } => return self.if_expr(e, branches, otherwise.as_ref()),
            K::Match { subject, arms } => return self.match_expr(e, subject, arms),
            _ => unreachable!("a leaf"),
        };
        self.typed(e, kind)
    }

    fn string(&mut self, e: &ast::Expr<'a>, parts: &[StrPart<'a>]) -> ExprKind {
        let text = parts.iter().map(|p| match p {
            StrPart::Text(t) => Some(t.as_str()),
            StrPart::Expr(_) => None,
        });
        if let Some(text) = text.collect::<Option<String>>() {
            return ExprKind::Str(text);
        }
        let string = self.core.ty(Type::String);
        let part = |p: &StrPart<'a>| match p {
            StrPart::Text(t) => Expr {
                ty: Some(string),
                kind: ExprKind::Str(t.clone()),
            },
            StrPart::Expr(x) => self.expr(x),
        };
        let parts = parts.iter().map(part).collect();
        ExprKind::Interp(parts, self.site(e.at))
    }

    fn name(&mut self, e: &ast::Expr<'a>) -> Expr {
        let table = self.table;
        let kind = match self.resolved.get(e.at).expect("a resolved name") {
            Binding::Local(decl) => {
                let local = self.slots[&decl];
                return Expr {
                    ty: Some(self.locals[local]),
                    kind: ExprKind::Place(Place {
                        root: Root::Local(local),
                        steps: vec![],
                    }),
                };
            }
            Binding::Def(d) => ExprKind::FnValue(FnRef::Fn(self.ids[d])),
            Binding::Variant(d, v) => {
                let Item::Enum(variants) = &table.defs[d].item else {
                    unreachable!("a variant of an enum");
                };
                if variants[v].fields.is_empty() {
                    ExprKind::Variant(Tag::User(self.ids[d], v), vec![])
                } else {
                    ExprKind::FnValue(FnRef::Variant(self.ids[d], v))
                }
            }
            Binding::Builtin(name) => ExprKind::Variant(builtin_tag(name), vec![]),
        };
        self.typed(e, kind)
    }

    fn record_literal(&mut self, e: &ast::Expr<'a>, args: &[ast::Arg<'a>]) -> ExprKind {
        let Some(Binding::Def(d)) = self.resolved.get(e.at) else {
            unreachable!("a record literal names a record");
        };
        self.record(d, args)
    }

    /// The record `d` built from `args`, which are labelled in declared order, and its defaults.
    pub(super) fn record(&mut self, d: usize, args: &[ast::Arg<'a>]) -> ExprKind {
        let program = self.program;
        let Some(ast::Def::Type { fields, .. }) = program.defs.get(d) else {
            unreachable!("a record");
        };
        let mut given = args.iter().peekable();
        let mut values = vec![];
        for field in fields {
            let value = match given
                .next_if(|a| a.label.as_ref().is_some_and(|l| l.text == field.name.text))
            {
                Some(arg) => self.expr(&arg.value),
                None => self.default(d, field),
            };
            values.push(value);
        }
        ExprKind::Record(self.ids[d], values)
    }

    /// A default is typed in the record's own definition.
    fn default(&mut self, d: usize, field: &ast::Field<'a>) -> Expr {
        let value = field.default.as_ref().expect("a field with a default");
        let outer = std::mem::replace(&mut self.def, d);
        let value = self.expr(value);
        self.def = outer;
        value
    }

    fn link(&mut self, link: &ast::Expr<'a>, inner: Expr) -> Expr {
        match &link.kind {
            K::Unary(op, _) => self.unary(link, *op, inner),
            K::Try(..) => self.try_op(link, inner),
            K::Field(base, name) => self.field(link, base, name.text, inner),
            K::Index(base, index) => self.index(link, base, index, inner),
            K::Call(..) => self.value_call(link, inner),
            K::Binary(op, at, _, right) => self.binary(link, *op, *at, inner, right),
            _ => unreachable!("a link"),
        }
    }

    fn unary(&mut self, e: &ast::Expr<'a>, op: UnOp, inner: Expr) -> Expr {
        let ty = self.ty_of(e);
        let un = match op {
            UnOp::Not => Un::Not,
            UnOp::Neg if self.is(ty, Type::Float) => Un::NegFloat,
            UnOp::Neg => Un::Neg(self.site(e.at)),
        };
        Expr {
            ty,
            kind: ExprKind::Unary(un, Box::new(inner)),
        }
    }

    fn binary(
        &mut self,
        e: &ast::Expr<'a>,
        op: BinOp,
        at: Pos,
        left: Expr,
        right: &ast::Expr<'a>,
    ) -> Expr {
        let right = self.expr(right);
        let ty = self.ty_of(e);
        let bin = match op {
            BinOp::Or => Bin::Or,
            BinOp::And => Bin::And,
            BinOp::Eq => Bin::Eq(self.site(at)),
            BinOp::Ne => Bin::Ne(self.site(at)),
            BinOp::Lt => Bin::Order(Cmp::Lt),
            BinOp::Le => Bin::Order(Cmp::Le),
            BinOp::Gt => Bin::Order(Cmp::Gt),
            BinOp::Ge => Bin::Order(Cmp::Ge),
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                self.math(op, ty, at).into()
            }
        };
        Expr {
            ty,
            kind: ExprKind::Binary(bin, Box::new(left), Box::new(right)),
        }
    }

    /// The operator `op` at `at` on operands of type `ty`.
    pub(super) fn math(&mut self, op: BinOp, ty: Option<Ty>, at: Pos) -> Math {
        if self.is(ty, Type::Int) {
            Math::Int(arith(op), self.site(at))
        } else if self.is(ty, Type::Float) {
            Math::Float(float_arith(op))
        } else {
            Math::Join(self.site(at))
        }
    }

    fn field(&mut self, e: &ast::Expr<'a>, base: &ast::Expr<'a>, name: &str, inner: Expr) -> Expr {
        let table = self.table;
        let record = self.entry(base).expect("a record");
        let Node::User(d) = table.types.node(record) else {
            unreachable!("a record");
        };
        let Item::Record(fields) = &table.defs[*d].item else {
            unreachable!("a record");
        };
        let at = fields.iter().position(|f| f.name == name);
        self.extend(e, inner, Step::Field(at.expect("a field")))
    }

    /// An index or a slice; the `[` is the first token after `base`.
    fn index(
        &mut self,
        e: &ast::Expr<'a>,
        base: &ast::Expr<'a>,
        index: &ast::Expr<'a>,
        inner: Expr,
    ) -> Expr {
        let step = if let K::Range(a, b) = &index.kind {
            let (a, b) = (self.expr(a), self.expr(b));
            Step::Slice(a, b, self.site(self.after(base)))
        } else {
            let index = self.expr(index);
            let site = self.site(self.after(base));
            if self.is_map(base) {
                Step::Key(index, site)
            } else {
                Step::Item(index, site)
            }
        };
        self.extend(e, inner, step)
    }

    fn is_map(&self, e: &ast::Expr<'a>) -> bool {
        let ty = self.entry(e).map(|t| self.table.types.node(t));
        matches!(ty, Some(Node::Name("Map", _)))
    }

    /// `inner` with one more step; a computed value roots a new place.
    fn extend(&mut self, e: &ast::Expr<'a>, inner: Expr, step: Step) -> Expr {
        let mut place = match inner.kind {
            ExprKind::Place(place) => place,
            _ => Place {
                root: Root::Value(Box::new(inner)),
                steps: vec![],
            },
        };
        place.steps.push(step);
        self.typed(e, ExprKind::Place(place))
    }

    /// `f(args)` where `f` is a name.
    fn named_call(&mut self, callee: &ast::Expr<'a>, call: &ast::Expr<'a>) -> Expr {
        let K::Call(_, args) = &call.kind else {
            unreachable!("a call");
        };
        let kind = match self.resolved.get(callee.at).expect("a resolved callee") {
            Binding::Local(_) => {
                let f = self.name(callee);
                return self.value_call(call, f);
            }
            Binding::Def(d) => self.call(Callee::Fn(self.ids[d]), args, callee.at),
            Binding::Variant(d, v) => {
                ExprKind::Variant(Tag::User(self.ids[d], v), self.values(args))
            }
            Binding::Builtin(name) if self.table.variants.contains_key(name) => {
                ExprKind::Variant(builtin_tag(name), self.values(args))
            }
            Binding::Builtin(name) => {
                let row = self.row(name, args);
                self.call(Callee::Builtin(row), args, callee.at)
            }
        };
        self.typed(call, kind)
    }

    /// A call through a function value: positional, with the site at its first token.
    fn value_call(&mut self, call: &ast::Expr<'a>, callee: Expr) -> Expr {
        let K::Call(f, args) = &call.kind else {
            unreachable!("a call");
        };
        let kind = self.call(Callee::Value(Box::new(callee)), args, f.at);
        self.typed(call, kind)
    }

    fn call(&mut self, callee: Callee, args: &[ast::Arg<'a>], at: Pos) -> ExprKind {
        let args = args.iter().map(|a| self.arg(a)).collect();
        ExprKind::Call(callee, args, self.site(at))
    }

    fn arg(&mut self, arg: &ast::Arg<'a>) -> Arg {
        match &arg.value.kind {
            K::Inout(place) => Arg::Inout(self.place(place).0),
            _ => Arg::Value(self.expr(&arg.value)),
        }
    }

    fn values(&mut self, args: &[ast::Arg<'a>]) -> Vec<Expr> {
        args.iter().map(|a| self.expr(&a.value)).collect()
    }

    /// The index in `builtins::ROWS` of the row the checker chose for a call of `name`: the first
    /// of its rows with as many parameters as arguments whose first parameter takes the first
    /// argument.
    fn row(&self, name: &str, args: &[ast::Arg<'a>]) -> usize {
        let rows = &self.table.fns[name];
        let first = args.first().and_then(|a| match &a.value.kind {
            K::Inout(x) => self.entry(x),
            _ => self.entry(&a.value),
        });
        let takes = |r: &crate::sig::Sig| {
            r.params.len() == args.len()
                && (rows.len() == 1
                    || first.is_none_or(|t| self.table.types.fits(t, r.params[0].ty)))
        };
        let nth = rows.iter().position(takes).expect("a row for the call");
        let named = |&&(kind, text): &&(Kind, &'static str)| {
            kind == Kind::Fn && builtins::name(text) == name
        };
        let all = builtins::ROWS
            .iter()
            .enumerate()
            .filter(|&(_, row)| named(&row));
        all.map(|(i, _)| i).nth(nth).expect("a row of the name")
    }
}

fn arith(op: BinOp) -> Arith {
    match op {
        BinOp::Add => Arith::Add,
        BinOp::Sub => Arith::Sub,
        BinOp::Mul => Arith::Mul,
        BinOp::Div => Arith::Div,
        _ => Arith::Rem,
    }
}

fn float_arith(op: BinOp) -> FloatArith {
    match op {
        BinOp::Add => FloatArith::Add,
        BinOp::Sub => FloatArith::Sub,
        BinOp::Mul => FloatArith::Mul,
        _ => FloatArith::Div,
    }
}
