//! Statements and blocks. A loop runs at most `FUEL` times, and a statement inside one is charged
//! that many times.

use crate::ty::Ty;
use crate::{FUEL, Fx, Gen, LEAF, Ret};

impl Gen {
    /// `{`, the `head` lines, `n` statements and a value of type `value`, `}`. `binds` are the
    /// block's own names.
    pub(crate) fn block(
        &mut self,
        fx: &mut Fx,
        binds: Vec<(String, Ty)>,
        head: &[String],
        n: usize,
        value: Option<&Ty>,
    ) -> String {
        let mark = fx.locals.len();
        binds
            .into_iter()
            .for_each(|(name, ty)| fx.bind(name, ty, false));
        fx.indent += 1;
        let mut out = String::from("{\n");
        for line in head {
            out += &format!("{}{line}\n", fx.pad(0));
        }
        for _ in 0..n {
            out += &self.stmt(fx);
        }
        match value {
            Some(ty) => out += &format!("{}{}\n", fx.pad(0), self.typed(fx, ty, 3)),
            // A function body may not end in a `return`, even inside an `if`.
            None if fx.indent == 1 && matches!(fx.ret, Ret::Bare) => {
                out += &format!("{}{}\n", fx.pad(0), self.decl(fx));
            }
            None => {}
        }
        fx.indent -= 1;
        fx.locals.truncate(mark);
        out + &fx.pad(0) + "}"
    }

    /// One statement and its line feed, or nothing when the budget is spent.
    fn stmt(&mut self, fx: &mut Fx) -> String {
        if !fx.charge(1) {
            return String::new();
        }
        let (deep, loops) = (fx.nest < 2, fx.loops < 2 && fx.nest < 2);
        let text = match self.rng.below(16) {
            3 | 4 => self.assign(fx),
            5 if loops => Some(self.range_loop(fx)),
            6 if loops => Some(self.walk(fx)),
            7 if loops => Some(self.while_loop(fx)),
            8 | 9 if deep => Some(self.if_stmt(fx)),
            10 if deep => self.match_stmt(fx),
            11 => self.mutate(fx),
            12 => self.call_stmt(fx),
            13 => self.guard(fx),
            14 => {
                let ty = self.ty(2, true);
                Some(format!("_ = {}", self.anchored(fx, &ty, 3)))
            }
            15 if self.rng.chance(16) => Some(format!("assert({})", self.typed(fx, &Ty::Bool, 2))),
            _ => None,
        };
        let text = text.unwrap_or_else(|| self.decl(fx));
        format!("{}{text}\n", fx.pad(0))
    }

    fn decl(&mut self, fx: &mut Fx) -> String {
        let ty = self.ty(2, true);
        let name = fx.fresh();
        let writable = self.rng.chance(2);
        let kw = if writable { "var" } else { "let" };
        let text = match self.rng.chance(4) {
            true => format!("{kw} {name} = {}", self.anchored(fx, &ty, 3)),
            false => format!("{kw} {name}: {ty} = {}", self.typed(fx, &ty, 3)),
        };
        fx.bind(name, ty, writable);
        text
    }

    /// A writable place `keep` accepts: its root local, its text and its type.
    fn place(&mut self, fx: &Fx, keep: impl Fn(&Ty) -> bool) -> Option<(usize, String, Ty)> {
        let mut found = vec![];
        for (i, l) in fx.locals.iter().enumerate() {
            if !l.writable || l.frozen || l.hidden {
                continue;
            }
            for (path, ty) in self.paths(&l.ty, 2).into_iter().filter(|p| keep(&p.1)) {
                found.push((i, format!("{}{path}", l.name), ty));
            }
        }
        if found.is_empty() {
            return None;
        }
        let at = self.rng.below(found.len());
        Some(found.swap_remove(at))
    }

    /// `place = e` or `place op= e`; `e` does not read the place's root, so nothing doubles.
    fn assign(&mut self, fx: &mut Fx) -> Option<String> {
        let (root, place, ty) = self.place(fx, |_| true)?;
        let ops: &[&str] = match ty {
            Ty::Int => &["=", "+=", "-=", "*=", "/=", "%="],
            Ty::Float => &["=", "+=", "-=", "*=", "/="],
            Ty::Str => &["=", "+="],
            _ => &["="],
        };
        let op = *self.rng.pick(ops);
        fx.locals[root].hidden = true;
        let value = self.typed(fx, &ty, 3);
        fx.locals[root].hidden = false;
        Some(format!("{place} {op} {value}"))
    }

    /// A body that runs up to `span` times per run of the statement.
    fn looped(
        &mut self,
        fx: &mut Fx,
        span: u64,
        binds: Vec<(String, Ty)>,
        head: &[String],
    ) -> String {
        let weight = fx.weight;
        fx.weight = weight.saturating_mul(span.max(1));
        (fx.loops, fx.nest) = (fx.loops + 1, fx.nest + 1);
        let n = 1 + self.rng.below(3);
        let body = self.block(fx, binds, head, n, None);
        (fx.loops, fx.nest) = (fx.loops - 1, fx.nest - 1);
        fx.weight = weight;
        body
    }

    fn range_loop(&mut self, fx: &mut Fx) -> String {
        let (range, span) = match self.rng.below(5) {
            0 => ("9223372036854775801..<9223372036854775807".to_string(), 6),
            1 => ("-9223372036854775808..<-9223372036854775805".to_string(), 3),
            2 => {
                let fallback = Ty::list(Ty::Int);
                let c = self.local_ty(fx, |t| matches!(t, Ty::Str | Ty::List(_)), fallback);
                (
                    format!("0..<min(len({}), {FUEL})", self.anchored(fx, &c, 1)),
                    FUEL,
                )
            }
            _ => {
                let (from, span) = (
                    self.rng.below(5) as i64 - 2,
                    self.rng.below(FUEL as usize + 1),
                );
                (format!("{from}..<{}", from + span as i64), span as u64)
            }
        };
        let name = fx.fresh();
        let body = self.looped(fx, span, vec![(name.clone(), Ty::Int)], &[]);
        format!("for {name} in {range} {body}")
    }

    /// A walk of a list, cut after `FUEL` items. What the list expression reads is frozen.
    fn walk(&mut self, fx: &mut Fx) -> String {
        let fallback = Ty::list(self.ty(1, false));
        let list = self.local_ty(fx, |t| matches!(t, Ty::List(_)), fallback);
        let Ty::List(item) = &list else {
            unreachable!()
        };
        let start = fx.reads.len();
        let walked = self.anchored(fx, &list, 2);
        let frozen: Vec<(usize, bool)> = fx.reads[start..]
            .iter()
            .map(|&i| (i, fx.locals[i].frozen))
            .collect();
        frozen.iter().for_each(|&(i, _)| fx.locals[i].frozen = true);
        let (fuel, name) = (fx.fresh(), fx.fresh());
        let head = [
            format!("{fuel} += 1"),
            format!("if {fuel} > {FUEL} {{"),
            "  break".into(),
            "}".into(),
        ];
        let body = self.looped(fx, FUEL, vec![(name.clone(), (**item).clone())], &head);
        frozen
            .into_iter()
            .rev()
            .for_each(|(i, was)| fx.locals[i].frozen = was);
        format!(
            "var {fuel}: Int = 0\n{}for {name} in {walked} {body}",
            fx.pad(0)
        )
    }

    fn while_loop(&mut self, fx: &mut Fx) -> String {
        let fuel = fx.fresh();
        let weight = fx.weight;
        fx.weight = weight.saturating_mul(FUEL + 1);
        let cond = self.typed(fx, &Ty::Bool, 2);
        fx.weight = weight;
        let body = self.looped(fx, FUEL, vec![], &[format!("{fuel} += 1")]);
        format!(
            "var {fuel}: Int = 0\n{}while {fuel} < {FUEL} && {cond} {body}",
            fx.pad(0)
        )
    }

    fn inner(&mut self, fx: &mut Fx, binds: Vec<(String, Ty)>) -> String {
        fx.nest += 1;
        let n = 1 + self.rng.below(3);
        let body = self.block(fx, binds, &[], n, None);
        fx.nest -= 1;
        body
    }

    fn if_stmt(&mut self, fx: &mut Fx) -> String {
        let cond = self.typed(fx, &Ty::Bool, 2);
        let mut out = format!("if {cond} {}", self.inner(fx, vec![]));
        if self.rng.chance(3) {
            let cond = self.typed(fx, &Ty::Bool, 2);
            out += &format!(" else if {cond} {}", self.inner(fx, vec![]));
        }
        if self.rng.chance(2) {
            out += &format!(" else {}", self.inner(fx, vec![]));
        }
        out
    }

    fn match_stmt(&mut self, fx: &mut Fx) -> Option<String> {
        let matchable = |t: &Ty| {
            matches!(
                t,
                Ty::Bool | Ty::Int | Ty::Str | Ty::Opt(_) | Ty::Res(..) | Ty::Enum(_)
            )
        };
        let fallback = match self.rng.below(4) {
            0 => Ty::Bool,
            1 => Ty::Int,
            2 => Ty::Str,
            _ => Ty::opt(Ty::Int),
        };
        let ty = self.local_ty(fx, matchable, fallback);
        let subject = self.anchored(fx, &ty, 1);
        // Each arm: its pattern and the names it binds.
        let arms: Vec<(String, Vec<(String, Ty)>)> = match &ty {
            Ty::Bool if self.rng.chance(2) => vec![("true".into(), vec![]), ("_".into(), vec![])],
            Ty::Bool => vec![("true".into(), vec![]), ("false".into(), vec![])],
            Ty::Int => vec![
                ("0".into(), vec![]),
                ("-1".into(), vec![]),
                ("_".into(), vec![]),
            ],
            Ty::Str => vec![
                (r#""a""#.into(), vec![]),
                (r#""""#.into(), vec![]),
                ("_".into(), vec![]),
            ],
            Ty::Opt(t) => {
                let v = fx.fresh();
                vec![
                    (format!("Some({v})"), vec![(v, (**t).clone())]),
                    ("None".into(), vec![]),
                ]
            }
            Ty::Res(t, e) => {
                let (v, w) = (fx.fresh(), fx.fresh());
                let ok = (format!("Ok({v})"), vec![(v, (**t).clone())]);
                vec![ok, (format!("Err({w})"), vec![(w, (**e).clone())])]
            }
            Ty::Enum(i) => self.enum_arms(fx, *i),
            _ => return None,
        };
        let mut out = format!("match {subject} {{\n");
        for (pat, binds) in arms {
            out += &format!("{}  {pat} => {}\n", fx.pad(0), self.arm(fx, binds));
        }
        Some(out + &fx.pad(0) + "}")
    }

    fn enum_arms(&mut self, fx: &mut Fx, i: usize) -> Vec<(String, Vec<(String, Ty)>)> {
        let variants = self.enums[i].variants.clone();
        let bare: Vec<usize> = (0..variants.len())
            .filter(|&j| variants[j].is_empty())
            .collect();
        let joined = bare.len() > 1 && self.rng.chance(2);
        let mut arms = vec![];
        if joined {
            let pats: Vec<_> = bare.iter().map(|j| format!("V{i}_{j}")).collect();
            arms.push((pats.join(" | "), vec![]));
        }
        for (j, fields) in variants.iter().enumerate() {
            if fields.is_empty() {
                if !joined {
                    arms.push((format!("V{i}_{j}"), vec![]));
                }
                continue;
            }
            let (mut subs, mut binds) = (vec![], vec![]);
            for t in fields {
                match self.rng.chance(3) {
                    true => subs.push("_".to_string()),
                    false => {
                        let v = fx.fresh();
                        subs.push(v.clone());
                        binds.push((v, t.clone()));
                    }
                }
            }
            arms.push((format!("V{i}_{j}({})", subs.join(", ")), binds));
        }
        arms
    }

    /// An arm body: nothing, an assignment on the arm's line, or a block.
    fn arm(&mut self, fx: &mut Fx, binds: Vec<(String, Ty)>) -> String {
        if self.rng.chance(4) {
            return "{}".into();
        }
        if self.rng.chance(3) {
            let mark = fx.locals.len();
            binds
                .iter()
                .for_each(|(n, t)| fx.bind(n.clone(), t.clone(), false));
            let line = self.assign(fx);
            fx.locals.truncate(mark);
            if let Some(line) = line {
                return line;
            }
        }
        fx.indent += 1;
        let body = self.inner(fx, binds);
        fx.indent -= 1;
        body
    }

    /// A built-in that changes a List or a Map place.
    fn mutate(&mut self, fx: &mut Fx) -> Option<String> {
        let (root, place, ty) = self.place(fx, |t| matches!(t, Ty::List(_) | Ty::Map(..)))?;
        fx.locals[root].hidden = true;
        let text = match &ty {
            Ty::List(t) => match self.rng.below(10) {
                0 | 1 => format!("append(&{place}, {})", self.item(fx, root, &place, t)),
                2 => {
                    let more = self.beside(fx, &[(root, place.clone())], &ty);
                    let more = more.unwrap_or_else(|| self.with_leaf(fx, &ty));
                    format!("extend(&{place}, {more})")
                }
                3 => format!(
                    "insert(&{place}, {}, at: {})",
                    self.item(fx, root, &place, t),
                    self.rng.below(2)
                ),
                4 => format!("swap(&{place}, 0, {})", self.rng.below(2)),
                5 => format!("reverse(&{place})"),
                6 if t.ordered() => format!("sort(&{place})"),
                7 => self
                    .sort_by(fx, &place, t)
                    .unwrap_or_else(|| format!("reverse(&{place})")),
                8 => format!("_ = pop(&{place})"),
                _ => format!("_ = remove_at(&{place}, 0)"),
            },
            Ty::Map(k, _) => format!("remove(&{place}, {})", self.anchored(fx, k, 1)),
            _ => unreachable!(),
        };
        fx.locals[root].hidden = false;
        Some(text)
    }

    /// A literal or a local, no larger than what is already there.
    pub(crate) fn with_leaf(&mut self, fx: &mut Fx, ty: &Ty) -> String {
        let was = std::mem::replace(&mut fx.anchored, true);
        let e = self.leaf(fx, ty);
        fx.anchored = was;
        e
    }

    fn sort_by(&mut self, fx: &mut Fx, place: &str, item: &Ty) -> Option<String> {
        let want = Ty::Fn(vec![item.clone(), item.clone()], Box::new(Ty::Bool));
        let names: Vec<_> = self.values().into_iter().filter(|v| v.1 == want).collect();
        if names.is_empty() || !fx.charge(LEAF * 64) {
            return None;
        }
        Some(format!("sort_by(&{place}, {})", self.rng.pick(&names).0))
    }

    /// A call of an earlier function, its `inout` arguments on distinct locals.
    fn call_stmt(&mut self, fx: &mut Fx) -> Option<String> {
        if self.fns.is_empty() {
            return None;
        }
        let i = self.rng.below(self.fns.len());
        let (params, result, cost) = {
            let f = &self.fns[i];
            (f.params.clone(), f.result.clone(), f.cost)
        };
        let mut places = vec![];
        for (ty, _) in params.iter().filter(|p| p.1) {
            let Some((root, place, _)) = self.place(fx, |t| t == ty) else {
                break;
            };
            fx.locals[root].hidden = true;
            places.push((root, place));
        }
        let ready = places.len() == params.iter().filter(|p| p.1).count() && fx.charge(cost);
        let mut args = vec![];
        let mut inout = places.iter();
        for (ty, by_ref) in params.iter().filter(|_| ready) {
            if *by_ref {
                args.push(format!("&{}", inout.next().unwrap().1));
                continue;
            }
            let near = self.beside(fx, &places, ty);
            args.push(near.unwrap_or_else(|| self.typed(fx, ty, 2)));
        }
        places
            .iter()
            .for_each(|&(root, _)| fx.locals[root].hidden = false);
        if !ready {
            return None;
        }
        let call = format!("f{i}({})", args.join(", "));
        Some(match result {
            None => call,
            Some(_) if self.rng.chance(3) => format!("_ = {call}"),
            Some(ty) => {
                let name = fx.fresh();
                let text = format!("let {name}: {ty} = {call}");
                fx.bind(name, ty, false);
                text
            }
        })
    }

    /// In a program with overlaps, an argument of type `ty` beside one of the `&` places, a part
    /// of its local: any part, or, as often, one M2-05 accepts (an Int, Float, Bool or function
    /// value anywhere in it, else a part apart at a field); a List part is sometimes sliced.
    pub(crate) fn beside(
        &mut self,
        fx: &mut Fx,
        places: &[(usize, String)],
        ty: &Ty,
    ) -> Option<String> {
        if !self.overlaps || places.is_empty() {
            return None;
        }
        let (root, place) = &places[self.rng.below(places.len())];
        let (root, l) = (*root, &fx.locals[*root]);
        let own = &place[l.name.len()..];
        let any = self.rng.chance(2) || matches!(ty, Ty::Int | Ty::Float | Ty::Bool | Ty::Fn(..));
        let mut found = vec![];
        for (path, t) in self.paths(&l.ty, 2) {
            if t != *ty || !(any || apart(own, &path)) {
                continue;
            }
            let read = format!("{}{path}", l.name);
            if matches!(t, Ty::List(_)) {
                found.push(format!("{read}[0..<min(1, len({read}))]"));
            }
            found.push(read);
        }
        if found.is_empty() {
            return None;
        }
        fx.reads.push(root);
        Some(found.swap_remove(self.rng.below(found.len())))
    }

    /// The item `append` or `insert` puts into `&place`.
    fn item(&mut self, fx: &mut Fx, root: usize, place: &str, ty: &Ty) -> String {
        let near = self.beside(fx, &[(root, place.to_string())], ty);
        near.unwrap_or_else(|| self.anchored(fx, ty, 2))
    }

    /// `if c {`, a `return`, `break` or `continue`, `}`.
    fn guard(&mut self, fx: &mut Fx) -> Option<String> {
        let exit = match (fx.loops > 0, self.rng.below(3)) {
            (true, 0) => "break".to_string(),
            (true, 1) => "continue".to_string(),
            _ => match &fx.ret {
                Ret::Value(t) => {
                    let t = t.clone();
                    format!("return {}", self.typed(fx, &t, 2))
                }
                Ret::Bare => "return".to_string(),
                Ret::Never => return None,
            },
        };
        let cond = self.typed(fx, &Ty::Bool, 2);
        let p = fx.pad(0);
        Some(format!("if {cond} {{\n{p}  {exit}\n{p}}}"))
    }
}

/// Whether two paths below one local part at two different fields before any index step
/// (M2-05 section 1.3).
fn apart(a: &str, b: &str) -> bool {
    for (x, y) in steps(a).into_iter().zip(steps(b)) {
        if !x.starts_with('.') || !y.starts_with('.') {
            return false;
        }
        if x != y {
            return true;
        }
    }
    false
}

/// The steps of a path such as `.x0[0]["a"]`.
fn steps(path: &str) -> Vec<&str> {
    let starts: Vec<usize> = (path.match_indices(['.', '[']).map(|(i, _)| i))
        .chain([path.len()])
        .collect();
    starts.windows(2).map(|w| &path[w[0]..w[1]]).collect()
}
