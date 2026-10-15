//! Expressions: literals, operators and the chains they form.

use super::text::{article, float_fix, plain, render};
use super::{Cx, Out, Res, Span, Use, span};
use crate::ast::{BinOp, Def, Expr, ExprKind as K, Name, Pos, StrPart, UnOp};
use crate::diagnostic::Diagnostic;
use crate::resolve::{habit, suggestion};
use crate::sig::{Checked, Field, Item, Node, Ty};
use paka_rt::ops::{Float, Int};

/// The operand a chain continues from; the head of a chain has none.
fn operand<'x, 'a>(e: &'x Expr<'a>) -> Option<&'x Expr<'a>> {
    match &e.kind {
        K::Unary(_, x)
        | K::Try(x, _)
        | K::Field(x, _)
        | K::Binary(_, _, x, _)
        | K::Range(x, _)
        | K::Index(x, _)
        | K::Call(x, _) => Some(x.as_ref()),
        _ => None,
    }
}

impl Cx<'_, '_> {
    /// The outcome of `e`; `want` is a hint, the caller decides whether it must hold.
    pub(super) fn expr(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        let (raw, want) = (want, want.filter(|&w| !self.is_var(w)));
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
                let out = self.call_named(head, args, hint(&links))?;
                if let (Out::Never, Some(ty)) = (&out, hint(&links)) {
                    self.info.insert(span(prev), ty);
                }
                out
            }
            (K::Hole(_), Some(K::Binary(BinOp::And | BinOp::Or, ..) | K::Unary(UnOp::Not, _))) => {
                Out::Val(self.prim("Bool"))
            }
            (K::Hole(_), Some(K::Binary(..) | K::Range(..))) => Out::Val(self.letter('T')),
            (K::Hole(_), _) => self.hole(head, if links.is_empty() { raw } else { None })?,
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
                if self.free(ty) && !self.free(want) {
                    self.settle(e, want)?;
                    return Ok(Out::Val(want));
                }
                Ok(Out::Val(ty))
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
                    return Err(self.out_of_range(plain(e), "Int", text));
                }
                self.int()
            }
            K::Float(text) => {
                let finite = Float::parse(text).is_some_and(|f| (f.bits() >> 52) & 0x7ff != 0x7ff);
                if !finite {
                    return Err(self.out_of_range(plain(e), "Float", text));
                }
                self.prim("Float")
            }
            K::Bool(_) => self.prim("Bool"),
            K::Str(parts) => {
                for part in parts {
                    if let StrPart::Expr(x) = part {
                        let out = self.expr(x, None)?;
                        let ty = self.fixed(out, x)?;
                        self.text_of(x, ty)?;
                    }
                }
                self.prim("String")
            }
            K::Name(_) => return self.name(e, want),
            K::Record(name, args) => return self.record(e, name, args),
            K::List(items) => self.list(e, items, want)?,
            K::Map(pairs) => self.map(e, pairs, want)?,
            K::If {
                branches,
                otherwise,
            } => return self.if_expr(e, branches, otherwise.as_ref(), Use::Value(want)),
            K::Match { .. } => return self.match_expr(e, Use::Value(want)),
            K::Inout(x) => return self.expr(x, want),
            _ => self.unknown(),
        };
        Ok(Out::Val(ty))
    }

    /// A number `text` that is no `ty`; a negative Int has no limit named.
    pub(super) fn out_of_range(&self, at: Span, ty: &str, text: &str) -> Diagnostic {
        let message = match (ty, text.starts_with('-')) {
            ("Int", false) => format!("largest Int is {}", i64::MAX),
            _ => format!("not a {ty} that exists"),
        };
        self.fail(at, "literal-out-of-range", message)
    }

    /// The built-in type of `ty` is one of `allowed`, or not yet known.
    pub(super) fn allow(&self, at: Span, ty: Ty, allowed: &[&str], text: &str) -> Res {
        if self.is_var(ty) || allowed.iter().any(|a| self.is(ty, a)) {
            return Ok(());
        }
        Err(self.mismatch(at, text, &self.types.show(ty), ""))
    }

    /// `x`, of type `ty`, goes into a text: only an Int, a Bool or a String does.
    fn text_of(&self, x: &Expr, ty: Ty) -> Res {
        if self.is_var(ty) || ["Int", "Bool", "String"].iter().any(|n| self.is(ty, n)) {
            return Ok(());
        }
        let (shown, value) = (self.types.show(ty), render(x));
        let strings = self
            .arg(ty, "List", 0)
            .is_some_and(|t| self.is(t, "String"));
        let repair = match self.types.node(ty) {
            Node::Fn(..) => None,
            Node::Name("Float", _) => Some(format!("write {{fixed({value}, digits: 2)}}")),
            Node::Name("List", _) if strings => Some(format!("write {{join({value}, \", \")}}")),
            Node::Name("List" | "Map", _) => Some("build the text in a loop".to_string()),
            Node::Name(..) => Some("match it first".to_string()),
            Node::User(i) => Some(match &self.defs[*i].item {
                Item::Record(fields) => {
                    let each = |f: &Field| format!("{{{value}.{}}}", f.name);
                    let list: Vec<_> = fields.iter().take(2).map(each).collect();
                    format!("write its fields: {}", list.join(", "))
                }
                _ => "match it and write each variant's text".to_string(),
            }),
            Node::Var(_) => None,
        };
        let message = match repair {
            Some(r) => format!("{} has no text here: {r}", article(&shown)),
            None => "a function has no text".to_string(),
        };
        let d = self.fail_at(x, "type-mismatch", message);
        Err(d.with_types(&"Int, Bool or String", &shown))
    }

    /// The type of `from`, a position that fixes no type: an open value there is `cannot-infer`.
    fn fixed(&mut self, out: Out, from: &Expr) -> Res<Ty> {
        let ty = self.val(out, from)?;
        match out {
            Out::Never => Err(self.open_value(from, None)),
            _ if self.free(ty) => Err(self.open_value(from, None)),
            _ => Ok(ty),
        }
    }

    fn link(&mut self, e: &Expr, from: &Expr, cur: Out, want: Option<Ty>) -> Res<Out> {
        let ty = match &e.kind {
            K::Field(..) | K::Index(..) | K::Call(..) | K::Unary(UnOp::Neg, _) => {
                self.fixed(cur, from)?
            }
            _ => self.val(cur, from)?,
        };
        match &e.kind {
            K::Unary(op, _) => self.unary(*op, e, from, ty),
            K::Binary(op, at, _, r) => self.binary(*op, *at, from, ty, r),
            K::Range(_, r) => self.range(from, ty, r),
            K::Index(_, i) => self.index(from, ty, i),
            K::Field(_, name) => self.field(e, ty, name),
            K::Try(_, at) => self.question((*at, 1), from, ty),
            K::Call(_, args) => self.call_value(from, ty, args, want),
            _ => Ok(Out::Val(ty)),
        }
    }

    /// `from..<r`: both sides are Int, so an open one takes Int.
    fn range(&mut self, from: &Expr, ty: Ty, r: &Expr) -> Res<Out> {
        let int = self.int();
        self.allow(span(from), ty, &["Int"], "Int")?;
        let item = if self.free(ty) {
            self.settle(from, int)?;
            int
        } else {
            ty
        };
        self.expect(r, int)?;
        Ok(Out::Val(self.types.named("List", &[item])))
    }

    fn unary(&mut self, op: UnOp, e: &Expr, from: &Expr, ty: Ty) -> Res<Out> {
        match op {
            UnOp::Neg => self.allow((e.at, 1), ty, &["Int", "Float"], "Int or Float")?,
            UnOp::Not => {
                self.allow((e.at, 1), ty, &["Bool"], "Bool")?;
                if self.free(ty) {
                    let bool = self.prim("Bool");
                    self.settle(from, bool)?;
                    return Ok(Out::Val(bool));
                }
            }
        }
        Ok(Out::Val(ty))
    }

    fn binary(&mut self, op: BinOp, at: Pos, left: &Expr, lt: Ty, right: &Expr) -> Res<Out> {
        let (lt, rt) = if matches!(left.kind, K::Hole(_)) && self.free(lt) {
            self.beside_hole(left, right)?
        } else {
            let hint = Some(lt).filter(|&t| !self.free(t));
            (lt, self.expr_val(right, hint)?)
        };
        self.operands((at, op.text().len()), op, (left, lt), (right, rt))?;
        match (self.free(lt), self.free(rt)) {
            (true, true) => return Err(self.open_value(left, None)),
            (true, false) => self.settle(left, rt)?,
            (false, true) => self.settle(right, lt)?,
            _ => {}
        }
        let arithmetic = matches!(
            op,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
        );
        let ty = match (arithmetic, self.is_var(lt)) {
            (true, true) => rt,
            (true, false) => lt,
            _ => self.prim("Bool"),
        };
        Ok(Out::Val(ty))
    }

    /// Whether the two operands suit `op`; errors stand at the operator.
    fn operands(&self, at: Span, op: BinOp, (l, lt): (&Expr, Ty), (r, rt): (&Expr, Ty)) -> Res {
        use BinOp::{Add, And, Div, Ge, Gt, Le, Lt, Mul, Or, Rem, Sub};
        let (shown_l, shown_r) = (self.types.show(lt), self.types.show(rt));
        let text = |message: String, want: &str, found: &str| {
            (self.fail(at, "type-mismatch", message)).with_types(&want, &found)
        };
        let orderable = "Int, Float or String";
        let either = |name| self.is(lt, name) || self.is(rt, name);
        let plain = |name| matches!(name, "Int" | "Bool");
        match op {
            Add if self.is(lt, "List") => {
                let message = format!("no + on List; write extend(&{}, {})", render(l), render(r));
                return Err(text(message, orderable, &shown_l));
            }
            Add if either("String") && [&shown_l, &shown_r].iter().any(|t| plain(t.as_str())) => {
                let other = if self.is(lt, "String") { r } else { l };
                let message = format!(
                    "no + between {shown_l} and {shown_r}; write an interpolation: \"{{{}}}\"",
                    render(other)
                );
                return Err(text(message, &shown_l, &shown_r));
            }
            Rem if either("Float") => {
                let message = "no % on Float: write x - y * trunc(x / y)".to_string();
                return Err(text(message, "Int", "Float"));
            }
            Lt | Le | Gt | Ge
                if !self.is_var(lt)
                    && !["Int", "Float", "String"].iter().any(|n| self.is(lt, n)) =>
            {
                return Err(text(self.no_order(op, l, r, lt), orderable, &shown_l));
            }
            _ => {}
        }
        let (allowed, expected): (&[&str], _) = match op {
            Or | And => (&["Bool"], "Bool"),
            Add => (&["Int", "Float", "String"], orderable),
            Sub | Mul | Div => (&["Int", "Float"], "Int or Float"),
            Rem => (&["Int"], "Int"),
            _ => (&[], ""),
        };
        if !allowed.is_empty() {
            self.allow(at, lt, allowed, expected)?;
        }
        if self.types.fits(rt, lt) {
            return Ok(());
        }
        let repair = match (shown_l.as_str(), shown_r.as_str()) {
            ("Int", "Float") => float_fix(l),
            ("Float", "Int") => float_fix(r),
            _ => String::new(),
        };
        if self.is_function(lt) {
            return Err(self.function_mismatch(at, lt, rt, None));
        }
        Err(self.mismatch(at, &shown_l, &shown_r, &repair))
    }

    /// "T has no order", with a field to compare when T is a record.
    fn no_order(&self, op: BinOp, l: &Expr, r: &Expr, ty: Ty) -> String {
        let head = format!("{} has no order", self.types.show(ty));
        let field = match self.types.node(ty) {
            Node::User(i) => match &self.defs[*i].item {
                Item::Record(fields) => fields.first().map(|f| f.name),
                _ => None,
            },
            _ => None,
        };
        match field {
            Some(f) => format!(
                "{head}; compare a field: {}.{f} {} {}.{f}",
                render(l),
                op.text(),
                render(r)
            ),
            None => format!("{head}; write a function that says which comes first"),
        }
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
            _ => {
                return Err(self.mismatch(
                    span(base),
                    "List, Map or String",
                    &self.types.show(bt),
                    "",
                ));
            }
        };
        Ok(Out::Val(ty))
    }

    /// `e` is `base.name` with `base` of type `bt`.
    fn field(&mut self, e: &Expr, bt: Ty, name: &Name) -> Res<Out> {
        let defs = self.defs;
        let at = (name.at, name.text.len());
        let (shown, item) = match self.types.node(bt) {
            Node::User(i) => (self.types.show(bt), &defs[*i].item),
            Node::Var(_) => return Ok(Out::Val(bt)),
            _ => {
                if let Some(message) = habit(name.text).filter(|_| !self.defined(name.text)) {
                    return Err(self.fail(at, "unknown-name", message));
                }
                let message = format!("{} has no fields", self.types.show(bt));
                return Err((self.fail(at, "type-mismatch", message))
                    .with_types(&"a record", &self.types.show(bt)));
            }
        };
        if let Item::Record(fields) = item
            && let Some(f) = fields.iter().find(|f| f.name == name.text)
        {
            return Ok(Out::Val(f.ty));
        }
        if let Item::Enum(variants) = item
            && let Some(v) = variants
                .iter()
                .find(|v| v.fields.iter().any(|f| f.name == name.text))
        {
            return Err(self.variant_field(e, at, &shown, v.name, name.text));
        }
        let head = format!("{shown} has no field {}", name.text);
        let names = item.each_field().map(|f| f.name);
        let d = suggestion(
            self.path,
            "unknown-name",
            (name.at, name.text),
            &head,
            names,
        );
        Err(d.in_definition(self.program.defs[self.def].name().text))
    }

    /// A variant's field is read by matching and changed by building the variant again.
    fn variant_field(
        &self,
        e: &Expr,
        at: Span,
        shown: &str,
        variant: &str,
        field: &str,
    ) -> crate::diagnostic::Diagnostic {
        if self.writing == Some(e.at)
            && let K::Field(subject, _) = &e.kind
        {
            let message = format!(
                "{field} is a field of the variant {variant}; build {variant}(..) and assign {}",
                render(subject)
            );
            return self.fail_at(e, "bad-assignment-target", message);
        }
        let message =
            format!("{field} is a field of the variant {variant}; match on the value to read it");
        (self.fail(at, "type-mismatch", message)).with_types(&"a record", &shown)
    }

    /// `e?` where `from` has type `ty`: the function's own result must be of the same family.
    fn question(&mut self, at: Span, from: &Expr, ty: Ty) -> Res<Out> {
        let enclosing = self.result;
        let name = self.program.defs[self.def].name().text;
        let (up, make, want, agrees, value) = if let Some(error) = self.arg(ty, "Result", 1) {
            let theirs = enclosing.and_then(|r| self.arg(r, "Result", 1));
            let agrees = theirs.is_some_and(|x| self.types.fits(x, error));
            let shown = self.types.show(error);
            let make = format!("make it Result<T, {shown}> or match it");
            (
                format!("Err({shown})"),
                make,
                format!("Result<T, {shown}>"),
                agrees,
                self.arg(ty, "Result", 0),
            )
        } else if self.is(ty, "Option") {
            let agrees = enclosing.is_some_and(|r| self.is(r, "Option"));
            let result = enclosing.is_some_and(|r| self.is(r, "Result"));
            let make = if result {
                "make it Option<T>, write ok_or(..., error)? or match it"
            } else {
                "make it Option<T> or match it"
            };
            (
                "None".to_string(),
                make.to_string(),
                "Option<T>".to_string(),
                agrees,
                self.arg(ty, "Option", 0),
            )
        } else if self.is_var(ty) {
            return match self.free(ty) {
                true => Err(self.open_value(from, None)),
                false => Ok(Out::Val(ty)),
            };
        } else {
            let (shown, value) = (self.types.show(ty), render(from));
            let message = format!("? needs an Option or Result; {value} is {shown}; drop the ?");
            return Err(
                (self.fail(at, "type-mismatch", message)).with_types(&"Option or Result", &shown)
            );
        };
        if agrees {
            self.fill_error(from, ty, enclosing)?;
            let value = value.unwrap_or(ty);
            if self.free(value) {
                return Err(self.open_value(from, None));
            }
            return Ok(Out::Val(value));
        }
        let (found, returns) = match enclosing {
            Some(r) => (
                self.types.show(r),
                format!("returns {}", self.types.show(r)),
            ),
            None => ("no result".to_string(), "has no result".to_string()),
        };
        let make = if enclosing.is_some() && !self.is_update() {
            make
        } else {
            "match it".to_string()
        };
        let message = if matches!(self.program.defs[self.def], Def::Example { .. }) {
            "an example has no ?; match it".to_string()
        } else {
            format!("? passes {up} up, but {name} {returns}; {make}")
        };
        Err((self.fail(at, "type-mismatch", message)).with_types(&want, &found))
    }

    /// Whether the program or the built-ins define `name`.
    fn defined(&self, name: &str) -> bool {
        (self.program.defs.iter()).any(|d| d.name().text == name)
            || self.fns.contains_key(name)
            || self.variants.contains_key(name)
    }

    /// Whether the definition being checked is the `update` of an app.
    fn is_update(&self) -> bool {
        let update =
            |d: &Checked| matches!(d.item, Item::App { update: Some(u), .. } if u == self.def);
        self.defs.iter().any(update)
    }

    /// The operand of `?` takes the function's error type where its own is open: `Ok(1)?` in a
    /// function that returns `Result<Int, String>` is a `Result<Int, String>`.
    fn fill_error(&mut self, from: &Expr, ty: Ty, enclosing: Option<Ty>) -> Res {
        let theirs = enclosing.and_then(|r| self.arg(r, "Result", 1));
        let (Some(value), Some(error)) = (self.arg(ty, "Result", 0), theirs) else {
            return Ok(());
        };
        let filled = self.types.named("Result", &[value, error]);
        match self.free(filled) {
            true => drop(self.info.insert(span(from), filled)),
            false if self.free(ty) => self.settle(from, filled)?,
            false => {}
        }
        Ok(())
    }

    fn list(&mut self, e: &Expr, items: &[Expr], want: Option<Ty>) -> Res<Ty> {
        let hint = want.and_then(|w| self.arg(w, "List", 0));
        let elem = self.items(e, items.len(), false, |i| &items[i], hint)?;
        let elem = elem.unwrap_or_else(|| self.letter('T'));
        Ok(self.types.named("List", &[elem]))
    }

    fn map(&mut self, e: &Expr, pairs: &[(Expr, Expr)], want: Option<Ty>) -> Res<Ty> {
        let hint = |i| want.and_then(|w| self.arg(w, "Map", i));
        let (key_hint, value_hint) = (hint(0), hint(1));
        let key = self.items(e, pairs.len(), true, |i| &pairs[i].0, key_hint)?;
        let value = self.items(e, pairs.len(), false, |i| &pairs[i].1, value_hint)?;
        let key = key.unwrap_or_else(|| self.letter('K'));
        let value = value.unwrap_or_else(|| self.letter('V'));
        Ok(self.types.named("Map", &[key, value]))
    }
}
