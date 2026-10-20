//! Names of functions and variants, calls and record literals. A built-in row binds its type
//! letters from the arguments and then from the type asked for (M2-09 section 1.3); a letter
//! neither fixes is `cannot-infer`, unless the row's result carries it to the call's position.

use super::text::{article, float_fix, render};
use super::{Cx, Out, Res, span};
use crate::ast::{Arg, Expr, ExprKind as K};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Item, Node, Param, Sig, Ty};
use paka_rt::ops::Int;
use std::collections::BTreeMap;
use std::slice::from_ref;

pub(super) type Bound = BTreeMap<char, Ty>;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Fn,
    Variant,
    /// A function value: positional, nothing inout.
    Value,
}

/// What a call names: its words in messages and where its count errors stand.
struct Callee {
    name: String,
    at: super::Span,
    kind: Kind,
}

impl<'a> Cx<'_, 'a> {
    /// A name used as a value.
    pub(super) fn name(&mut self, e: &Expr, want: Option<Ty>) -> Res<Out> {
        let (defs, fns, variants) = (self.defs, self.fns, self.variants);
        let unknown = |this: &mut Self| Ok(Out::Val(this.unknown()));
        match self.resolved.get(e.at) {
            Some(Binding::Local(decl)) => {
                let ty = self.locals.get(&decl).map(|l| l.ty);
                Ok(Out::Val(ty.unwrap_or_else(|| self.unknown())))
            }
            Some(Binding::Def(i)) => match &defs[i].item {
                Item::Fn(sig) => self.bare(e, from_ref(sig), (true, false), want),
                Item::View(_) => Err(self.outside_view(e)),
                _ => unknown(self),
            },
            Some(Binding::Variant(i, j)) => {
                let sig = self.variant_sig(i, j);
                self.bare(e, &[sig], (true, true), want)
            }
            Some(Binding::Builtin(b)) => match (variants.get(b), fns.get(b)) {
                (Some(sig), _) => self.bare(e, from_ref(sig), (false, true), want),
                (_, Some(rows)) => self.bare(e, rows, (false, false), want),
                _ if self.elements.contains_key(b) => Err(self.outside_view(e)),
                _ => unknown(self),
            },
            None => unknown(self),
        }
    }

    /// A function or variant named without a call: a value for a variant without fields and for a
    /// user function without `inout`; otherwise a mistake.
    fn bare(
        &mut self,
        e: &Expr,
        rows: &[Sig<'_>],
        (user, variant): (bool, bool),
        want: Option<Ty>,
    ) -> Res<Out> {
        let sig = &rows[0];
        let K::Name(name) = e.kind else {
            unreachable!("a bare name is a name")
        };
        if let Some(result) = sig.result.filter(|_| variant && sig.params.is_empty()) {
            let mut bound = Bound::new();
            if let Some(w) = want {
                self.bind(result, w, &mut bound);
            }
            return Ok(Out::Val(self.subst(result, &bound)));
        }
        let kind = if variant { Kind::Variant } else { Kind::Fn };
        let callee = Callee {
            name: name.to_string(),
            at: span(e),
            kind,
        };
        if want.is_some_and(|w| {
            rows.iter()
                .any(|s| s.result.is_some_and(|r| self.bind(r, w, &mut Bound::new())))
        }) {
            return Err(self.count_error(&callee, sig, 0, true));
        }
        let message = match (user, variant) {
            (true, _) if sig.params.iter().any(|p| p.inout) => format!(
                "{name} has an inout parameter and cannot be a value; give it a result instead of inout, or call it directly"
            ),
            (true, _) => {
                let params: Vec<_> = sig.params.iter().map(|p| p.ty).collect();
                return Ok(Out::Val(
                    self.types.intern(Node::Fn(params.into(), sig.result)),
                ));
            }
            (false, true) => self.variant_wrapper(name, sig, want),
            (false, false) => format!(
                "{name} is a built-in and cannot be a value; write fn wrap(...) -> R {{ {name}(...) }} and pass wrap"
            ),
        };
        Err(self.fail_at(e, "not-a-value", message))
    }

    /// The text that offers a function to stand where a built-in variant was named; the letters
    /// of its row are filled in from the function type asked for.
    fn variant_wrapper(&mut self, name: &str, sig: &Sig<'_>, want: Option<Ty>) -> String {
        let mut bound = Bound::new();
        if let Some(w) = want
            && let Node::Fn(params, result) = self.types.node(w).clone()
        {
            for (p, &t) in sig.params.iter().zip(params.iter()) {
                self.bind(p.ty, t, &mut bound);
            }
            if let (Some(a), Some(b)) = (sig.result, result) {
                self.bind(a, b, &mut bound);
            }
        }
        let mut show = |ty| {
            let ty = self.subst(ty, &bound);
            self.types.show(ty)
        };
        let params: Vec<_> = sig
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, show(p.ty)))
            .collect();
        let result = sig.result.map(&mut show).unwrap_or_default();
        let names: Vec<_> = sig.params.iter().map(|p| p.name).collect();
        format!(
            "{name} is a built-in variant and cannot be a value; write fn {}_of({}) -> {result} {{ {name}({}) }}",
            name.to_lowercase(),
            params.join(", "),
            names.join(", ")
        )
    }

    pub(super) fn variant_sig(&self, i: usize, j: usize) -> Sig<'a> {
        let fields = match &self.defs[i].item {
            Item::Enum(variants) => &variants[j].fields[..],
            _ => &[],
        };
        let param = |f: &crate::sig::Field<'a>| Param {
            name: f.name,
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
    pub(super) fn call_named(&mut self, callee: &Expr, args: &[Arg], want: Option<Ty>) -> Res<Out> {
        let (defs, fns, variants) = (self.defs, self.fns, self.variants);
        let K::Name(name) = callee.kind else {
            unreachable!("a named call has a name")
        };
        let to = |kind| Callee {
            name: name.to_string(),
            at: span(callee),
            kind,
        };
        match self.resolved.get(callee.at) {
            Some(Binding::Def(i)) => match &defs[i].item {
                Item::Fn(sig) => self.invoke(&to(Kind::Fn), from_ref(sig), args, want),
                Item::View(_) => Err(self.outside_view(callee)),
                _ => self.unbound(args),
            },
            Some(Binding::Variant(i, j)) => {
                let sig = self.variant_sig(i, j);
                self.invoke(&to(Kind::Variant), &[sig], args, want)
            }
            Some(Binding::Builtin(b)) if variants.contains_key(b) => {
                let out = self.invoke(&to(Kind::Variant), from_ref(&variants[b]), args, want)?;
                if b == "ExitWith" {
                    self.exit_status(args)?;
                }
                Ok(out)
            }
            Some(Binding::Builtin(b)) if fns.contains_key(b) => {
                self.ui_call(b, args)?;
                let out = self.invoke(&to(Kind::Fn), &fns[b], args, want)?;
                Ok(if b == "unreachable" { Out::Never } else { out })
            }
            Some(Binding::Builtin(b)) if self.elements.contains_key(b) => {
                Err(self.outside_view(callee))
            }
            Some(Binding::Local(_)) => {
                let ty = self.expr_val(callee, None)?;
                self.call_value(callee, ty, args, want)
            }
            _ => self.unbound(args),
        }
    }

    /// A call of the function or view `callee` names, with its one signature `sig`.
    pub(super) fn call_sig(
        &mut self,
        callee: &Expr,
        sig: &Sig<'_>,
        args: &[Arg],
        want: Option<Ty>,
    ) -> Res<Out> {
        let c = Callee {
            name: render(callee),
            at: span(callee),
            kind: Kind::Fn,
        };
        self.invoke(&c, from_ref(sig), args, want)
    }

    /// An `ExitWith` written with an Int literal must name a status a program may choose.
    fn exit_status(&self, args: &[Arg]) -> Res {
        let [Arg { value, .. }] = args else {
            return Ok(());
        };
        let K::Int(text) = value.kind else {
            return Ok(());
        };
        let chosen = |n| n == Int::new(0) || (Int::new(5)..=Int::new(255)).contains(&n);
        if Int::parse(text).is_some_and(chosen) {
            return Ok(());
        }
        let message =
            format!("{text} is not an exit status a program may choose; write 0 or 5 to 255");
        Err(self.fail_at(value, "literal-out-of-range", message))
    }

    /// A call of a name that is not known: the arguments are checked, the call is silent.
    fn unbound(&mut self, args: &[Arg]) -> Res<Out> {
        for arg in args {
            self.expr_val(&arg.value, None)?;
        }
        Ok(Out::Val(self.unknown()))
    }

    /// A call of a value of type `ty`: a function value, called by position.
    pub(super) fn call_value(
        &mut self,
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
                let callee = Callee {
                    name: render(callee),
                    at: span(callee),
                    kind: Kind::Value,
                };
                self.invoke(&callee, &[sig], args, want)
            }
            Node::Var(_) => self.unbound(args),
            _ => Err(self.not_a_function(callee, ty)),
        }
    }

    fn not_a_function(&self, callee: &Expr, ty: Ty) -> Diagnostic {
        let message = format!(
            "{} is {}, not a function",
            render(callee),
            article(&self.types.show(ty))
        );
        self.fail_at(callee, "not-a-function", message)
    }

    /// The count error: "NAME takes 2 arguments (a, b), got 1"; a bare name adds the call to write.
    fn count_error(&self, c: &Callee, sig: &Sig<'_>, got: usize, bare: bool) -> Diagnostic {
        let (name, n) = (&c.name, sig.params.len());
        let names: Vec<_> = sig
            .params
            .iter()
            .map(|p| p.name)
            .filter(|p| !p.is_empty())
            .collect();
        let message = match (n, c.kind) {
            (0, Kind::Variant) => format!("{name} has no fields: write {name}"),
            (0, _) => format!("{name} takes no arguments"),
            _ => {
                let plural = if n == 1 { "" } else { "s" };
                let labels = if names.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", names.join(", "))
                };
                let write = if bare {
                    format!("; write {name}(...)")
                } else {
                    String::new()
                };
                format!("{name} takes {n} argument{plural}{labels}, got {got}{write}")
            }
        };
        self.fail(c.at, "wrong-argument-count", message)
    }

    /// Checks `args` against the row of `rows` that suits them and gives the call's outcome.
    fn invoke(&mut self, c: &Callee, rows: &[Sig<'_>], args: &[Arg], want: Option<Ty>) -> Res<Out> {
        let counts = |s: &&Sig| {
            s.params.len() == args.len() && !(c.kind == Kind::Variant && args.is_empty())
        };
        let counted: Vec<_> = rows.iter().filter(counts).collect();
        let Some(&only) = counted.first() else {
            return Err(self.count_error(c, &rows[0], args.len(), false));
        };
        let (sig, first) = match rows {
            [_] => (only, None),
            _ => {
                let (row, first) = self.pick(c, &counted, &args[0])?;
                let nth = rows.iter().position(|r| std::ptr::eq(r, row));
                self.picks.insert(c.at, nth.expect("a row of the call"));
                (row, first)
            }
        };
        self.labels(c, sig, args)?;
        self.walk(c, sig, args, first, want)
    }

    /// The first row whose first parameter takes the first argument, and that argument's type.
    fn pick<'r, 's>(
        &mut self,
        c: &Callee,
        rows: &[&'r Sig<'s>],
        arg: &Arg,
    ) -> Res<(&'r Sig<'s>, Option<Ty>)> {
        let inner = match &arg.value.kind {
            K::Inout(x) => x.as_ref(),
            _ => &arg.value,
        };
        let ty = self.expr_val(inner, None)?;
        if let Some(&row) = rows.iter().find(|r| self.types.fits(ty, r.params[0].ty)) {
            return Ok((row, Some(ty)));
        }
        let (mut shown, mut heads) = (Vec::<String>::new(), Vec::<&str>::new());
        for row in rows {
            let text = self.types.show(row.params[0].ty);
            if !shown.contains(&text) {
                shown.push(text);
            }
        }
        for text in &shown {
            let head = text.split('<').next().unwrap_or(text);
            if !heads.contains(&head) {
                heads.push(head);
            }
        }
        let list = match heads.split_last() {
            Some((last, init)) if init.len() > 1 => {
                format!("{}, {} or {last}", article(init[0]), init[1..].join(", "))
            }
            Some((last, [first])) => format!("{} or {last}", article(first)),
            Some((last, _)) => article(last),
            None => String::new(),
        };
        let message = match self.arg(ty, "List", 0) {
            Some(item) if c.name == "sort" => {
                let (item, xs) = (self.types.show(item), render(inner));
                format!(
                    "{item} has no order; write sort_by(&{xs}, before) with fn before(a: {item}, b: {item}) -> Bool"
                )
            }
            _ => format!("{} takes {list}", c.name),
        };
        let found = self.types.show(ty);
        let d = self.fail_at(&arg.value, "type-mismatch", message);
        let expected = match shown.as_slice() {
            [init @ .., last] if !init.is_empty() => format!("{} or {last}", init.join(", ")),
            _ => shown.concat(),
        };
        Err(d.with_types(&expected, &found))
    }

    /// Labels stand where the signature has them, with its names, in its order.
    fn labels(&self, c: &Callee, sig: &Sig<'_>, args: &[Arg]) -> Res {
        for (arg, p) in args.iter().zip(&sig.params) {
            let value = render(&arg.value);
            let (at, message) = match &arg.label {
                Some(l) if c.kind == Kind::Value => (
                    (l.at, l.text.len()),
                    "drop the label: calls through a function value are positional".to_string(),
                ),
                Some(l) if !p.labelled => (
                    (l.at, l.text.len()),
                    format!("{} is bare: write {value}", p.name),
                ),
                Some(l) if l.text != p.name => {
                    let elsewhere = sig.params.iter().any(|q| q.labelled && q.name == l.text);
                    let message = if elsewhere {
                        "labels come in declared order".to_string()
                    } else {
                        format!("write {}: {value}", p.name)
                    };
                    ((l.at, l.text.len()), message)
                }
                None if p.labelled => (span(&arg.value), format!("write {}: {value}", p.name)),
                _ => continue,
            };
            return Err(self.fail(at, "wrong-label", message));
        }
        Ok(())
    }

    /// Types the arguments, binding the row's letters first from them, then from `want`; the open
    /// ones are checked last, with the bindings put in. Of several arguments that do not fit, the
    /// first one is reported.
    fn walk(
        &mut self,
        c: &Callee,
        sig: &Sig<'_>,
        args: &[Arg],
        mut first: Option<Ty>,
        want: Option<Ty>,
    ) -> Res<Out> {
        let (mut bound, mut open, picked) = (Bound::new(), Vec::new(), first.is_some());
        let (mut misfit, mut waiting) = (None, Vec::new());
        for (i, (arg, param)) in args.iter().zip(&sig.params).enumerate() {
            let hint = self.subst(param.ty, &bound);
            let later = i > 0 || c.kind == Kind::Variant;
            if matches!(arg.value.kind, K::Hole(_)) && later && self.free(hint) {
                self.info.insert(span(&arg.value), hint);
                open.push((i, hint));
                continue;
            }
            // A hole deeper in waits, like a bare one, for the letters the position binds.
            if later && self.free(hint) && holds_hole(&arg.value) {
                waiting.push(i);
                continue;
            }
            let got = self.argument(c, arg, param, hint, first.take())?;
            let settled = !self.free(got);
            let right = if settled {
                self.bind(hint, got, &mut bound)
            } else {
                self.types.fits(got, hint)
            };
            if !right {
                misfit = Some((i, hint, got));
                break;
            }
            if !settled {
                open.push((i, got));
            }
        }
        if let (Some(w), Some(r)) = (want, sig.result) {
            let r = self.subst(r, &bound);
            if !self.bind(r, w, &mut bound) && misfit.is_some() && !self.free(w) {
                let r = self.subst(r, &bound);
                let (w, r) = (self.types.show(w), self.types.show(r));
                return Err(self.mismatch(c.at, &w, &r, ""));
            }
        }
        for i in waiting.into_iter().filter(|_| misfit.is_none()) {
            let hint = self.subst(sig.params[i].ty, &bound);
            let got = self.argument(c, &args[i], &sig.params[i], hint, None)?;
            if !self.free(got) {
                self.bind(hint, got, &mut bound);
            }
            open.push((i, got));
        }
        for (i, got) in open {
            let (param, value) = (sig.params[i].ty, &args[i].value);
            let hint = self.subst(param, &bound);
            if !self.types.fits(got, hint) {
                return Err(self.arg_mismatch(c, picked, args, i, (hint, got)));
            }
            if misfit.is_some() {
                continue;
            }
            if !self.free(hint) {
                self.settle(value, hint)?;
            } else if self.stuck(sig, param, &bound) {
                // The argument's own shape, with the letters the call has bound (M2-02 section 5).
                let mut known = bound.clone();
                self.unify(param, got, &mut known);
                let shape = self.subst(param, &known);
                self.info.insert(span(value), shape);
                let open = args.iter().map(|a| &a.value).filter(|v| self.is_open(v));
                return Err(self.open_part(self.needs_in(open, value), None));
            } else {
                self.unify(param, got, &mut bound);
            }
        }
        if let Some((i, hint, got)) = misfit {
            // The row's `M` is the app's message type; name it where nothing has bound it.
            let msg = (bound.get(&'M').copied()).unwrap_or_else(|| self.types.named("Msg", &[]));
            let shown = Bound::from([('M', msg)]);
            let (hint, got) = (self.subst(hint, &shown), self.subst(got, &shown));
            return Err(self.arg_mismatch(c, picked, args, i, (hint, got)));
        }
        Ok(match sig.result {
            Some(r) => Out::Val(self.subst(r, &bound)),
            None => Out::Unit,
        })
    }

    /// Argument `i` has type `got` where `hint` is needed. After the first argument of a row picked
    /// by it, an Int and a Float are two of one type that differ.
    fn arg_mismatch(
        &self,
        c: &Callee,
        picked: bool,
        args: &[Arg],
        i: usize,
        (hint, got): (Ty, Ty),
    ) -> Diagnostic {
        let (want, found) = (self.types.show(hint), self.types.show(got));
        let mixed = |fixed: &Expr| {
            let message = format!("{} takes two of one type; {}", c.name, float_fix(fixed));
            self.fail_at(&args[i].value, "type-mismatch", message)
                .with_types(&want, &found)
        };
        match (picked && i > 0, want.as_str(), found.as_str()) {
            (true, "Int", "Float") => mixed(&args[0].value),
            (true, "Float", "Int") => mixed(&args[i].value),
            _ => self.mismatch_at(&args[i].value, hint, got),
        }
    }

    /// The type of one argument, once its `&` is right for the parameter.
    fn argument(
        &mut self,
        c: &Callee,
        arg: &Arg,
        param: &Param<'_>,
        hint: Ty,
        done: Option<Ty>,
    ) -> Res<Ty> {
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
                let message = match c.kind {
                    Kind::Value => format!(
                        "{} is a function value and takes no inout; drop the &",
                        c.name
                    ),
                    _ => format!("{} is read-only in {}; drop the &", param.name, c.name),
                };
                Err(self.fail((v.at, 1), "unexpected-inout-mark", message))
            }
            (_, true) => {
                let (value, f) = (render(v), &c.name);
                Err(self.fail_at(
                    v,
                    "missing-inout-mark",
                    format!("{f} changes {value}: write &{value}"),
                ))
            }
            _ => match done {
                Some(ty) => Ok(ty),
                None => match self.expr(v, Some(hint))? {
                    Out::Never => {
                        self.info.insert(span(v), hint);
                        Ok(hint)
                    }
                    out => self.val(out, v),
                },
            },
        }
    }

    /// `Name { field: value, .. }`: fields in declaration order, those with a default may go.
    pub(super) fn record(&mut self, e: &Expr, name: &str, args: &[Arg]) -> Res<Out> {
        let defs = self.defs;
        let Some(Binding::Def(i)) = self.resolved.get(e.at) else {
            return Ok(Out::Val(self.unknown()));
        };
        let Item::Record(fields) = &defs[i].item else {
            return Ok(Out::Val(self.unknown()));
        };
        let found: Vec<_> = (args.iter())
            .map(|a| {
                a.label
                    .as_ref()
                    .and_then(|l| fields.iter().position(|f| f.name == l.text))
            })
            .collect();
        for (k, arg) in args.iter().enumerate() {
            // As in a call, the first label that stands before one it should follow is wrong.
            let ordered = |s| {
                !found[..k].contains(&Some(s)) && found[k + 1..].iter().flatten().all(|&t| t >= s)
            };
            let (at, message) = match (&arg.label, found[k]) {
                (Some(_), Some(s)) if ordered(s) => continue,
                (Some(l), None) => (
                    (l.at, l.text.len()),
                    format!("{name} has no field {}", l.text),
                ),
                (Some(l), Some(_)) => {
                    ((l.at, l.text.len()), "labels come in declared order".into())
                }
                (None, _) => {
                    let next = k.checked_sub(1).and_then(|j| found[j]).map_or(0, |s| s + 1);
                    let field = fields.get(next).map_or("NAME", |f| f.name);
                    (span(&arg.value), format!("write {field}: value"))
                }
            };
            return Err(self.fail(at, "wrong-label", message));
        }
        let slots: Vec<usize> = found.into_iter().flatten().collect();
        let mut given = slots.iter().peekable();
        for (k, f) in fields.iter().enumerate() {
            if given.next_if(|&&s| s == k).is_none() && !f.default {
                let message = format!("{name} needs {0}: write {0}: ...", f.name);
                return Err(self.fail((e.at, name.len()), "wrong-argument-count", message));
            }
        }
        for (arg, &s) in args.iter().zip(&slots) {
            self.expect(&arg.value, fields[s].ty)?;
        }
        let ty = self.types.user(i);
        Ok(Out::Val(ty.unwrap_or_else(|| self.unknown())))
    }

    /// `ty` with the letters bound so far put in.
    pub(super) fn subst(&mut self, ty: Ty, bound: &Bound) -> Ty {
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

    /// Whether `param` has a letter that nothing has fixed and the row's result does not carry.
    fn stuck(&self, sig: &Sig<'_>, param: Ty, bound: &Bound) -> bool {
        let carried = sig.result.map(|r| self.letters(r)).unwrap_or_default();
        let fixed = |c: &char| bound.get(c).is_some_and(|&t| !self.free(t));
        (self.letters(param).iter()).any(|c| !carried.contains(c) && !fixed(c))
    }

    fn letters(&self, ty: Ty) -> Vec<char> {
        let (mut found, mut stack) = (Vec::new(), vec![ty]);
        while let Some(t) = stack.pop() {
            match self.types.node(t) {
                Node::Var(c) => found.push(*c),
                Node::Name(_, args) => stack.extend(args.iter()),
                Node::Fn(params, result) => stack.extend(params.iter().chain(result)),
                Node::User(_) => {}
            }
        }
        found
    }

    /// Binds the row's letters in `row` to what the open `found` shows of them, so a message can
    /// show the call's type: `list_of([], count: 3)` is a `List<List<T>>`. A letter met again
    /// keeps what both show. Only the row's letters are bound, never a letter of `found`, so a
    /// letter of a literal that shares a name with one of the row is no alias.
    fn unify(&self, row: Ty, found: Ty, bound: &mut Bound) {
        match (self.types.node(row), self.types.node(found)) {
            (Node::Var(_), Node::Var(d)) if *d != '_' => {}
            (Node::Var(c), _) => {
                let both = bound.get(c).and_then(|&old| self.join(old, found));
                bound.insert(*c, both.unwrap_or(found));
            }
            (Node::Name(a, x), Node::Name(b, y)) if a == b && x.len() == y.len() => {
                (x.iter().zip(y.iter())).for_each(|(&r, &f)| self.unify(r, f, bound));
            }
            _ => {}
        }
    }

    /// The type two open types share, the one that shows more of it; none when they differ. The
    /// letters of neither are filled from the other, so `Ok(1)` and `Err("x")` stay open.
    pub(super) fn join(&self, a: Ty, b: Ty) -> Option<Ty> {
        self.types
            .fits(a, b)
            .then(|| if self.shows(b, a) { b } else { a })
    }

    /// Whether `specific` shows a type wherever `general` has a letter.
    fn shows(&self, specific: Ty, general: Ty) -> bool {
        match (self.types.node(general), self.types.node(specific)) {
            (Node::Var(_), _) => true,
            (Node::Name(a, x), Node::Name(b, y)) => {
                a == b
                    && x.len() == y.len()
                    && x.iter().zip(y.iter()).all(|(&g, &s)| self.shows(s, g))
            }
            (a, b) => a == b,
        }
    }

    /// Matches `found` against the shape `row`, binding its free letters; false when a letter
    /// meets two types or the shapes differ. A part that is not yet known binds nothing; one that
    /// cannot be known (`_`) binds to `_`, so nothing is said about it twice.
    pub(super) fn bind(&self, row: Ty, found: Ty, bound: &mut Bound) -> bool {
        match (self.types.node(row), self.types.node(found)) {
            (Node::Var('_'), _) => true,
            (Node::Var(_), _) if self.free(found) => true,
            (Node::Var(c), _) => self.types.fits(*bound.entry(*c).or_insert(found), found),
            (_, Node::Var('_')) => {
                self.letters(row).into_iter().for_each(|c| {
                    bound.entry(c).or_insert(found);
                });
                true
            }
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

/// Whether a hole sits in `e` as an item or an argument, at any depth.
fn holds_hole(e: &Expr) -> bool {
    match &e.kind {
        K::Hole(_) => true,
        K::List(items) => items.iter().any(holds_hole),
        K::Map(pairs) => pairs.iter().any(|(k, v)| holds_hole(k) || holds_hole(v)),
        K::Call(_, args) => args.iter().any(|a| holds_hole(&a.value)),
        _ => false,
    }
}
