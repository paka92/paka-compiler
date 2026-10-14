//! Names of functions and variants, calls and record literals. A built-in row binds its type
//! letters from the arguments and then from the type asked for (M2-09 section 1.3).

use super::{Cx, Out, Res};
use crate::ast::{Arg, Expr, ExprKind as K};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Item, Node, Param, Sig, Ty};
use std::collections::BTreeMap;
use std::slice::from_ref;

type Bound = BTreeMap<char, Ty>;

impl Cx<'_, '_> {
    /// A name used as a value.
    pub(super) fn name(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        let (defs, fns, variants) = (self.defs, self.fns, self.variants);
        match self.resolved.get(e.at) {
            Some(Binding::Local(decl)) => {
                let ty = self.locals.get(&decl).map(|l| l.ty);
                Ok(Out::Val(ty.unwrap_or_else(|| self.unknown())))
            }
            Some(Binding::Def(i)) => match &defs[i].item {
                Item::Fn(sig) => self.bare(e, from_ref(sig), true, false, want),
                _ => Err(self.fail_at(e, "not-a-value", "this name is not a value")),
            },
            Some(Binding::Variant(i, j)) => {
                let sig = self.variant_sig(i, j);
                self.bare(e, &[sig], true, true, want)
            }
            Some(Binding::Builtin(b)) => match (variants.get(b), fns.get(b)) {
                (Some(sig), _) => self.bare(e, from_ref(sig), false, true, want),
                (_, Some(rows)) => self.bare(e, rows, false, false, want),
                _ => Ok(Out::Val(self.unknown())),
            },
            None => Ok(Out::Val(self.unknown())),
        }
    }

    /// A function or variant named without a call: a value for a variant without fields and for a
    /// user function without `inout`; otherwise a mistake.
    fn bare(
        &mut self,
        e: &Expr,
        rows: &[Sig<'_>],
        user: bool,
        variant: bool,
        want: Option<Ty>,
    ) -> Res<Out> {
        let sig = &rows[0];
        if let Some(result) = sig.result.filter(|_| variant && sig.params.is_empty()) {
            return Ok(Out::Val(result));
        }
        if rows.iter().any(|s| s.result.is_some() && s.result == want) {
            return Err(self.fail_at(
                e,
                "wrong-argument-count",
                "this needs its arguments: call it",
            ));
        }
        if !user || sig.params.iter().any(|p| p.inout) {
            return Err(self.fail_at(e, "not-a-value", "this cannot be used as a value"));
        }
        let params: Vec<_> = sig.params.iter().map(|p| p.ty).collect();
        Ok(Out::Val(
            self.types.intern(Node::Fn(params.into(), sig.result)),
        ))
    }

    fn variant_sig(&self, i: usize, j: usize) -> Sig<'static> {
        let fields = match &self.defs[i].item {
            Item::Enum(variants) => &variants[j].fields[..],
            _ => &[],
        };
        let param = |f: &crate::sig::Field| Param {
            name: "",
            inout: false,
            labelled: false,
            ty: f.ty,
        };
        Sig {
            params: fields.iter().map(param).collect(),
            result: self.types.user(i),
        }
    }

    /// A call whose callee is a name: what it names decides the signature.
    pub(super) fn call_named(
        &mut self,
        call: &Expr,
        callee: &Expr,
        args: &[Arg],
        want: Option<Ty>,
    ) -> Res<Out> {
        let (defs, fns, variants) = (self.defs, self.fns, self.variants);
        let name = match callee.kind {
            K::Name(name) => name,
            _ => "this",
        };
        match self.resolved.get(callee.at) {
            Some(Binding::Def(i)) => match &defs[i].item {
                Item::Fn(sig) => self.invoke(call, name, from_ref(sig), false, args, want),
                _ => Err(self.fail_at(
                    callee,
                    "not-a-function",
                    format!("`{name}` is not a function"),
                )),
            },
            Some(Binding::Variant(i, j)) => {
                let sig = self.variant_sig(i, j);
                self.invoke(call, name, &[sig], true, args, want)
            }
            Some(Binding::Builtin(b)) if variants.contains_key(b) => {
                self.invoke(call, name, from_ref(&variants[b]), true, args, want)
            }
            Some(Binding::Builtin(b)) if fns.contains_key(b) => {
                let out = self.invoke(call, name, &fns[b], false, args, want)?;
                Ok(if b == "unreachable" { Out::Never } else { out })
            }
            Some(Binding::Local(_)) => {
                let ty = self.expr_val(callee, None)?;
                self.call_value(call, callee, ty, args, want)
            }
            _ => Err(self.fail_at(
                callee,
                "not-a-function",
                format!("`{name}` is not a function"),
            )),
        }
    }

    /// A call of a value of type `ty`: a function value, called by position.
    pub(super) fn call_value(
        &mut self,
        call: &Expr,
        callee: &Expr,
        ty: Ty,
        args: &[Arg],
        want: Option<Ty>,
    ) -> Res<Out> {
        match self.types.node(ty).clone() {
            Node::Fn(params, result) => {
                let param = |&ty| Param {
                    name: "",
                    inout: false,
                    labelled: false,
                    ty,
                };
                let sig = Sig {
                    params: params.iter().map(param).collect(),
                    result,
                };
                self.invoke(call, "this function value", &[sig], false, args, want)
            }
            Node::Var(_) => {
                for arg in args {
                    self.expr_val(&arg.value, None)?;
                }
                Ok(Out::Val(self.unknown()))
            }
            _ => {
                let message = format!("a value of type {} cannot be called", self.types.show(ty));
                Err(self.fail_at(callee, "not-a-function", message))
            }
        }
    }

    /// Checks `args` against the row of `rows` that suits them and gives the call's outcome.
    fn invoke(
        &mut self,
        call: &Expr,
        name: &str,
        rows: &[Sig<'_>],
        variant: bool,
        args: &[Arg],
        want: Option<Ty>,
    ) -> Res<Out> {
        let counts = |s: &&Sig| s.params.len() == args.len() && !(variant && args.is_empty());
        let counted: Vec<_> = rows.iter().filter(counts).collect();
        let Some(&only) = counted.first() else {
            let message = format!(
                "`{name}` takes {} arguments, not {}",
                rows[0].params.len(),
                args.len()
            );
            return Err(self.fail_at(call, "wrong-argument-count", message));
        };
        let (sig, first) = match rows {
            [_] => (only, None),
            _ => self.pick(&counted, &args[0])?,
        };
        self.labels(sig, args)?;
        self.walk(sig, args, first, want)
    }

    /// The first row whose first parameter takes the first argument, and that argument's type.
    fn pick<'r, 's>(&mut self, rows: &[&'r Sig<'s>], arg: &Arg) -> Res<(&'r Sig<'s>, Option<Ty>)> {
        let inner = match &arg.value.kind {
            K::Inout(x) => x.as_ref(),
            _ => &arg.value,
        };
        let ty = self.expr_val(inner, None)?;
        if let Some(&row) = rows.iter().find(|r| self.types.fits(ty, r.params[0].ty)) {
            return Ok((row, Some(ty)));
        }
        let mut shown: Vec<String> = Vec::new();
        for row in rows {
            let text = self.types.show(row.params[0].ty);
            if !shown.contains(&text) {
                shown.push(text);
            }
        }
        Err(self.mismatch(&arg.value, &shown.join(" or "), &self.types.show(ty), ""))
    }

    /// Labels stand where the signature has them, with its names, in its order.
    fn labels(&self, sig: &Sig<'_>, args: &[Arg]) -> Res {
        for (arg, p) in args.iter().zip(&sig.params) {
            let right = match &arg.label {
                Some(l) => p.labelled && l.text == p.name,
                None => !p.labelled,
            };
            if !right {
                let message = match p.labelled {
                    true => format!("this argument is labelled `{}:`", p.name),
                    false => "this argument takes no label".to_string(),
                };
                return Err(self.label_error(arg, message));
            }
        }
        Ok(())
    }

    fn label_error(&self, arg: &Arg, message: String) -> Diagnostic {
        match &arg.label {
            Some(l) => self.fail(l.at, l.text.len(), "wrong-label", message),
            None => self.fail_at(&arg.value, "wrong-label", message),
        }
    }

    /// Types the arguments, binding the row's letters first from them, then from `want`.
    fn walk(
        &mut self,
        sig: &Sig<'_>,
        args: &[Arg],
        mut first: Option<Ty>,
        want: Option<Ty>,
    ) -> Res<Out> {
        let mut bound = Bound::new();
        let mut open = Vec::new();
        for (i, (arg, param)) in args.iter().zip(&sig.params).enumerate() {
            let hint = self.subst(param.ty, &bound);
            let got = self.argument(arg, param, hint, first.take())?;
            let settled = !self.free(got);
            let right = if settled {
                self.bind(hint, got, &mut bound)
            } else {
                self.types.fits(got, hint)
            };
            if !right {
                return Err(self.mismatch_at(&arg.value, hint, got));
            }
            if !settled {
                open.push((i, got));
            }
        }
        if let (Some(w), Some(r)) = (want, sig.result) {
            let r = self.subst(r, &bound);
            self.bind(r, w, &mut bound);
        }
        for (i, got) in open {
            let hint = self.subst(sig.params[i].ty, &bound);
            if !self.types.fits(got, hint) {
                return Err(self.mismatch_at(&args[i].value, hint, got));
            }
            self.bind(hint, got, &mut bound);
        }
        Ok(match sig.result {
            Some(r) => Out::Val(self.subst(r, &bound)),
            None => Out::Unit,
        })
    }

    /// The type of one argument, once its `&` is right for the parameter.
    fn argument(&mut self, arg: &Arg, param: &Param<'_>, hint: Ty, done: Option<Ty>) -> Res<Ty> {
        let v = &arg.value;
        match (&v.kind, param.inout) {
            (K::Inout(x), true) => {
                let ty = match done {
                    Some(ty) => ty,
                    None => self.expr_val(x, Some(hint))?,
                };
                self.inout_target(x)?;
                self.info.insert((v.at, v.len), ty);
                Ok(ty)
            }
            (K::Inout(_), false) => {
                Err(self.fail_at(v, "unexpected-inout-mark", "this parameter is not `inout`"))
            }
            (_, true) => {
                let message = format!("`{}` is `inout`: write `&` before the argument", param.name);
                Err(self.fail_at(v, "missing-inout-mark", message))
            }
            _ => match done {
                Some(ty) => Ok(ty),
                None => self.expr_val(v, Some(hint)),
            },
        }
    }

    /// `Name { field: value, .. }`: fields in declaration order, those with a default may go.
    pub(super) fn record(&mut self, e: &Expr, args: &[Arg]) -> Res<Out> {
        let defs = self.defs;
        let Some(Binding::Def(i)) = self.resolved.get(e.at) else {
            return Ok(Out::Val(self.unknown()));
        };
        let Item::Record(fields) = &defs[i].item else {
            return Ok(Out::Val(self.unknown()));
        };
        let missing = |this: &Self, f: &crate::sig::Field| {
            let message = format!("the field `{}` has no value", f.name);
            this.fail_at(e, "wrong-argument-count", message)
        };
        let mut next = 0;
        for arg in args {
            let hit = arg
                .label
                .as_ref()
                .and_then(|l| fields[next..].iter().position(|f| f.name == l.text));
            let Some(skipped) = hit else {
                return Err(self.label_error(arg, "name the next field of the record".into()));
            };
            if let Some(f) = fields[next..next + skipped].iter().find(|f| !f.default) {
                return Err(missing(self, f));
            }
            next += skipped + 1;
            self.expect(&arg.value, fields[next - 1].ty)?;
        }
        if let Some(f) = fields[next..].iter().find(|f| !f.default) {
            return Err(missing(self, f));
        }
        let ty = self.types.user(i);
        Ok(Out::Val(ty.unwrap_or_else(|| self.unknown())))
    }

    /// `ty` with the letters bound so far put in.
    fn subst(&mut self, ty: Ty, bound: &Bound) -> Ty {
        match self.types.node(ty).clone() {
            Node::Var(c) => bound.get(&c).copied().unwrap_or(ty),
            Node::Name(name, args) => {
                let args: Vec<_> = args.iter().map(|&t| self.subst(t, bound)).collect();
                self.types.named(name, &args)
            }
            Node::Fn(params, result) => {
                let params: Vec<_> = params.iter().map(|&t| self.subst(t, bound)).collect();
                let result = result.map(|t| self.subst(t, bound));
                self.types.intern(Node::Fn(params.into(), result))
            }
            Node::User(_) => ty,
        }
    }

    /// Matches `found` against the shape `row`, binding its free letters; false when a letter
    /// meets two types or the shapes differ. A part that is not yet known binds nothing.
    fn bind(&self, row: Ty, found: Ty, bound: &mut Bound) -> bool {
        match (self.types.node(row), self.types.node(found)) {
            (Node::Var(c), _) if *c == '_' || self.is_var(found) || self.free(found) => true,
            (Node::Var(c), _) => *bound.entry(*c).or_insert(found) == found,
            (_, Node::Var(_)) => true,
            (Node::Name(a, x), Node::Name(b, y)) => a == b && self.bind_all(x, y, bound),
            (Node::Fn(p, r), Node::Fn(q, s)) => {
                let result = match (r, s) {
                    (Some(r), Some(s)) => self.bind(*r, *s, bound),
                    (r, s) => r == s,
                };
                self.bind_all(p, q, bound) & result
            }
            (a, b) => a == b,
        }
    }

    fn bind_all(&self, rows: &[Ty], found: &[Ty], bound: &mut Bound) -> bool {
        let each = |ok, (&r, &f)| self.bind(r, f, bound) & ok;
        rows.len() == found.len() && rows.iter().zip(found).fold(true, each)
    }
}
