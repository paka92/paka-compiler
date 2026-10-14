//! Expressions: literals, operators and the chains they form.

use super::{Cx, Out, Res, Use};
use crate::ast::{BinOp, Expr, ExprKind as K, Name, StrPart, UnOp};
use crate::sig::{Item, Node, Ty};
use paka_rt::ops::{Float, Int};

/// The operand a chain continues from; the head of a chain has none.
fn operand<'x, 'a>(e: &'x Expr<'a>) -> Option<&'x Expr<'a>> {
    match &e.kind {
        K::Unary(_, x)
        | K::Try(x)
        | K::Field(x, _)
        | K::Binary(_, x, _)
        | K::Range(x, _)
        | K::Index(x, _)
        | K::Call(x, _) => Some(x.as_ref()),
        _ => None,
    }
}

impl Cx<'_, '_> {
    /// The outcome of `e`; `want` is a hint, the caller decides whether it must hold.
    pub(super) fn expr(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        let want = want.filter(|&w| !self.is_var(w));
        let mut links = vec![e];
        while let Some(x) = operand(links[links.len() - 1]) {
            links.push(x);
        }
        let head = links.pop().expect("a chain has a head");
        let hint = |links: &[&Expr]| if links.is_empty() { want } else { None };
        let mut prev = head;
        let mut cur = match (&head.kind, links.last().copied().map(|l| &l.kind)) {
            (K::Name(_), Some(K::Call(_, args))) => {
                prev = links.pop().expect("a call");
                self.call_named(prev, head, args, hint(&links))?
            }
            _ => self.leaf(head, hint(&links))?,
        };
        self.note(prev, &cur);
        while let Some(link) = links.pop() {
            cur = self.link(link, prev, cur, hint(&links))?;
            self.note(link, &cur);
            prev = link;
        }
        Ok(cur)
    }

    /// The type of `e`, which must give a value.
    pub(super) fn expr_val(&mut self, e: &Expr, want: Option<Ty>) -> Res<Ty> {
        let out = self.expr(e, want)?;
        self.val(out, e)
    }

    pub(super) fn val(&mut self, out: Out, e: &Expr) -> Res<Ty> {
        match out {
            Out::Val(ty) => Ok(ty),
            Out::Never => Ok(self.unknown()),
            Out::Unit => Err(self.no_value(e, None)),
        }
    }

    /// `e` must fit `want`; a value that leaves control gives `Never`.
    pub(super) fn expect_out(&mut self, e: &Expr, want: Ty) -> Res<Out> {
        match self.expr(e, Some(want))? {
            Out::Val(ty) if self.types.fits(ty, want) => {
                let open = self.free(ty) && !self.free(want);
                Ok(Out::Val(if open { want } else { ty }))
            }
            Out::Val(ty) => Err(self.mismatch_at(e, want, ty)),
            Out::Unit => Err(self.no_value(e, Some(want))),
            Out::Never => Ok(Out::Never),
        }
    }

    pub(super) fn expect(&mut self, e: &Expr, want: Ty) -> Res<Ty> {
        match self.expect_out(e, want)? {
            Out::Val(ty) => Ok(ty),
            _ => Ok(want),
        }
    }

    fn leaf(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        let ty = match &e.kind {
            K::Int(text) => {
                if Int::parse(text).is_none() {
                    return Err(self.out_of_range(e, "Int"));
                }
                self.int()
            }
            K::Float(text) => {
                let finite = Float::parse(text).is_some_and(|f| (f.bits() >> 52) & 0x7ff != 0x7ff);
                if !finite {
                    return Err(self.out_of_range(e, "Float"));
                }
                self.prim("Float")
            }
            K::Bool(_) => self.prim("Bool"),
            K::Str(parts) => {
                for part in parts {
                    if let StrPart::Expr(x) = part {
                        let ty = self.expr_val(x, None)?;
                        self.allow(x, ty, &["Int", "Bool", "String"], "Int, Bool or String")?;
                    }
                }
                self.prim("String")
            }
            K::Hole(_) => return Ok(self.hole(Use::Value(want))),
            K::Name(_) => return self.name(e, want),
            K::Record(_, args) => return self.record(e, args),
            K::List(items) => self.list(items, want)?,
            K::Map(pairs) => self.map(pairs, want)?,
            K::If {
                branches,
                otherwise,
            } => return self.if_expr(e, branches, otherwise.as_ref(), Use::Value(want)),
            K::Match { subject, .. } => return self.match_expr(subject, Use::Value(want)),
            K::Inout(x) => return self.expr(x, want),
            _ => self.unknown(),
        };
        Ok(Out::Val(ty))
    }

    fn out_of_range(&self, e: &Expr, ty: &str) -> crate::diagnostic::Diagnostic {
        self.fail_at(e, "literal-out-of-range", format!("not a {ty} that exists"))
    }

    /// The built-in type of `ty` is one of `allowed`, or not yet known.
    pub(super) fn allow(&self, at: &Expr, ty: Ty, allowed: &[&str], text: &str) -> Res {
        if self.is_var(ty) || allowed.iter().any(|a| self.is(ty, a)) {
            return Ok(());
        }
        Err(self.mismatch(at, text, &self.types.show(ty), ""))
    }

    fn link(&mut self, e: &Expr, from: &Expr, cur: Out, want: Option<Ty>) -> Res<Out> {
        let ty = self.val(cur, from)?;
        match &e.kind {
            K::Unary(op, _) => self.unary(*op, from, ty),
            K::Binary(op, _, r) => self.binary(*op, from, ty, r),
            K::Range(_, r) => {
                self.allow(from, ty, &["Int"], "Int")?;
                let int = self.int();
                self.expect(r, int)?;
                Ok(Out::Val(self.types.named("List", &[ty])))
            }
            K::Index(_, i) => self.index(from, ty, i),
            K::Field(_, name) => self.field(from, ty, name),
            K::Try(_) => self.question(e, from, ty),
            K::Call(_, args) => self.call_value(e, from, ty, args, want),
            _ => Ok(Out::Val(ty)),
        }
    }

    fn unary(&mut self, op: UnOp, from: &Expr, ty: Ty) -> Res<Out> {
        match op {
            UnOp::Neg => self.allow(from, ty, &["Int", "Float"], "Int or Float")?,
            UnOp::Not => self.allow(from, ty, &["Bool"], "Bool")?,
        }
        Ok(Out::Val(ty))
    }

    fn binary(&mut self, op: BinOp, from: &Expr, lt: Ty, r: &Expr) -> Res<Out> {
        use BinOp::{Add, And, Div, Eq, Ge, Gt, Le, Lt, Mul, Ne, Or, Rem, Sub};
        let (allowed, text): (&[&str], _) = match op {
            Or | And => (&["Bool"], "Bool"),
            Eq | Ne => (&[], ""),
            Lt | Le | Gt | Ge | Add => (&["Int", "Float", "String"], "Int, Float or String"),
            Sub | Mul | Div => (&["Int", "Float"], "Int or Float"),
            Rem => (&["Int"], "Int"),
        };
        if !allowed.is_empty() {
            self.allow(from, lt, allowed, text)?;
        }
        let rt = self.expect(r, lt)?;
        let ty = match op {
            Add | Sub | Mul | Div | Rem if self.is_var(lt) => rt,
            Add | Sub | Mul | Div | Rem => lt,
            _ => self.prim("Bool"),
        };
        Ok(Out::Val(ty))
    }

    pub(super) fn int(&mut self) -> Ty {
        self.prim("Int")
    }

    fn index(&mut self, base: &Expr, bt: Ty, i: &Expr) -> Res<Out> {
        let slice = matches!(i.kind, K::Range(..));
        let ty = match self.types.node(bt).clone() {
            Node::Name("List" | "String", args) => {
                if slice {
                    self.expr(i, None)?;
                    bt
                } else {
                    let int = self.int();
                    self.expect(i, int)?;
                    args.first().copied().unwrap_or(bt)
                }
            }
            Node::Name("Map", args) => {
                self.expect(i, args[0])?;
                args[1]
            }
            Node::Var(_) => {
                self.expr(i, None)?;
                self.unknown()
            }
            _ => return Err(self.mismatch(base, "List, Map or String", &self.types.show(bt), "")),
        };
        Ok(Out::Val(ty))
    }

    fn field(&mut self, base: &Expr, bt: Ty, name: &Name) -> Res<Out> {
        let defs = self.defs;
        let record = match self.types.node(bt) {
            Node::User(i) => match &defs[*i].item {
                Item::Record(fields) => Some(fields),
                _ => None,
            },
            _ => None,
        };
        if let Some(fields) = record {
            let Some(f) = fields.iter().find(|f| f.name == name.text) else {
                let message = format!("`{}` has no field `{}`", self.types.show(bt), name.text);
                return Err(self.fail(name.at, name.text.len(), "unknown-name", message));
            };
            return Ok(Out::Val(f.ty));
        }
        if self.is_var(bt) {
            return Ok(Out::Val(bt));
        }
        Err(self.mismatch(base, "a record", &self.types.show(bt), ""))
    }

    /// `e?` where `from` has type `ty`: the function's own result must be of the same family.
    fn question(&mut self, e: &Expr, from: &Expr, ty: Ty) -> Res<Out> {
        let enclosing = self.result;
        let (want, agrees, value) = if let Some(error) = self.arg(ty, "Result", 1) {
            let theirs = enclosing.and_then(|r| self.arg(r, "Result", 1));
            let agrees = theirs.is_some_and(|x| self.types.fits(x, error));
            let want = format!("Result<T, {}>", self.types.show(error));
            (want, agrees, self.arg(ty, "Result", 0))
        } else if self.is(ty, "Option") {
            let agrees = enclosing.is_some_and(|r| self.is(r, "Option"));
            ("Option<T>".to_string(), agrees, self.arg(ty, "Option", 0))
        } else if self.is_var(ty) {
            return Ok(Out::Val(ty));
        } else {
            return Err(self.mismatch(from, "Option or Result", &self.types.show(ty), ""));
        };
        if agrees {
            return Ok(Out::Val(value.unwrap_or(ty)));
        }
        let found = enclosing.map_or("no result".to_string(), |r| self.types.show(r));
        Err(self.mismatch(e, &want, &found, ""))
    }

    fn list(&mut self, items: &[Expr], want: Option<Ty>) -> Res<Ty> {
        let hint = want.and_then(|w| self.arg(w, "List", 0));
        let mut elem = None;
        for item in items {
            self.join(&mut elem, item, hint)?;
        }
        let elem = elem.or(hint).unwrap_or_else(|| self.letter('T'));
        Ok(self.types.named("List", &[elem]))
    }

    fn map(&mut self, pairs: &[(Expr, Expr)], want: Option<Ty>) -> Res<Ty> {
        let hint = |i| want.and_then(|w| self.arg(w, "Map", i));
        let (key_hint, value_hint) = (hint(0), hint(1));
        let (mut key, mut value) = (None, None);
        for (i, (k, v)) in pairs.iter().enumerate() {
            self.join(&mut key, k, key_hint)?;
            if let (0, Some(ty)) = (i, key) {
                self.allow(k, ty, &["Int", "String"], "Int or String")?;
            }
            self.join(&mut value, v, value_hint)?;
        }
        let mut or = |found: Option<Ty>, hint: Option<Ty>, c| {
            found.or(hint).unwrap_or_else(|| self.letter(c))
        };
        let (key, value) = (or(key, key_hint, 'K'), or(value, value_hint, 'V'));
        Ok(self.types.named("Map", &[key, value]))
    }

    /// Adds `e` to a run of values that must agree: the first sets the type, the rest must fit it.
    fn join(&mut self, acc: &mut Option<Ty>, e: &Expr, hint: Option<Ty>) -> Res {
        let ty = match *acc {
            Some(a) => self.expect(e, a)?,
            None => self.expr_val(e, hint)?,
        };
        if acc.is_none_or(|a| self.free(a)) {
            *acc = Some(ty);
        }
        Ok(())
    }
}
