//! Random Paka programs from a seed (M4-21). Each one checks and ends: a function calls only the
//! functions declared before it, every loop is bounded, and a cost bound limits how many
//! statements a run performs. The app's handler calls each function and prints what it gives.
//!
//! Every random choice goes through `Rng::below`, so a later shrinker can replay a program from
//! a recorded list of choices instead of a seed.

pub mod edit;
mod expr;
mod show;
mod stmt;
mod ty;

use std::collections::BTreeMap;
use ty::{Enum, Func, Record, Ty};

/// The iterations of a `while` or a list walk, and the longest range.
const FUEL: u64 = 6;
/// The cost one function body may reach, and the handler's.
const BUDGET: u64 = 4_000;
const TOP_BUDGET: u64 = 40_000;
/// The levels a recursive function is charged for.
const RECURSION: u64 = 101;
/// The most a function value may cost; a call through one is charged this much.
const LEAF: u64 = 40;

/// The program of `seed`, as source text.
pub fn program(seed: u64) -> String {
    Gen {
        rng: Rng(seed),
        records: vec![],
        enums: vec![],
        fns: vec![],
        holes: 0,
        shows: BTreeMap::new(),
        show_text: String::new(),
    }
    .program()
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let z = (self.0 ^ (self.0 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

struct Gen {
    rng: Rng,
    records: Vec<Record>,
    enums: Vec<Enum>,
    fns: Vec<Func>,
    holes: usize,
    /// The text function of each type printed, `s{i}`, and their definitions.
    shows: BTreeMap<Ty, usize>,
    show_text: String,
}

struct Local {
    name: String,
    ty: Ty,
    /// A `var` or an `inout` parameter the generator may change.
    writable: bool,
    /// Walked by an enclosing `for`.
    frozen: bool,
    /// Passed with `&` to the call being written.
    hidden: bool,
}

/// What `return` may give in a body.
enum Ret {
    Never,
    Bare,
    Value(Ty),
}

/// The function being written.
struct Fx {
    locals: Vec<Local>,
    ret: Ret,
    /// How many times a statement written now may run per call.
    weight: u64,
    cost: u64,
    budget: u64,
    loops: u32,
    nest: u32,
    indent: usize,
    names: usize,
    /// The locals read so far, by position, in order.
    reads: Vec<usize>,
    /// The expression being written must have a type without context: no hole.
    anchored: bool,
}

impl Fx {
    fn new(ret: Ret, budget: u64) -> Fx {
        Fx {
            locals: vec![],
            ret,
            weight: 1,
            cost: 0,
            budget,
            loops: 0,
            nest: 0,
            indent: 0,
            names: 0,
            reads: vec![],
            anchored: false,
        }
    }

    fn fresh(&mut self) -> String {
        self.names += 1;
        format!("v{}", self.names - 1)
    }

    fn bind(&mut self, name: String, ty: Ty, writable: bool) {
        let (frozen, hidden) = (false, false);
        (self.locals).push(Local {
            name,
            ty,
            writable,
            frozen,
            hidden,
        });
    }

    /// Takes `cost` per run of the current statement, if the budget allows.
    fn charge(&mut self, cost: u64) -> bool {
        let total = self.cost.saturating_add(self.weight.saturating_mul(cost));
        let fits = total <= self.budget;
        if fits {
            self.cost = total;
        }
        fits
    }

    fn pad(&self, more: usize) -> String {
        "  ".repeat(self.indent + more)
    }
}

impl Gen {
    fn program(mut self) -> String {
        let mut out = String::new();
        for _ in 0..self.rng.below(4) {
            out += &self.user_type();
        }
        for _ in 0..1 + self.rng.below(6) {
            out += &self.function();
        }
        let app = self.app();
        out + &self.show_text + &app
    }

    /// A record or an enum whose fields use only the types declared before it.
    fn user_type(&mut self) -> String {
        if self.rng.chance(2) {
            let i = self.records.len();
            let mut fields = vec![];
            let mut text = vec![];
            for j in 0..1 + self.rng.below(3) {
                let ty = self.ty(1, false);
                let default = self.default(&ty);
                text.push(match &default {
                    Some(d) => format!("x{j}: {ty} = {d}"),
                    None => format!("x{j}: {ty}"),
                });
                fields.push((ty, default.is_some()));
            }
            self.records.push(Record { fields });
            format!("type R{i} {{ {} }}\n\n", text.join(", "))
        } else {
            let i = self.enums.len();
            let mut variants = vec![];
            let mut text = vec![];
            for j in 0..1 + self.rng.below(3) {
                let fields: Vec<_> = (0..self.rng.below(3)).map(|_| self.ty(1, false)).collect();
                let named: Vec<_> = (fields.iter().enumerate())
                    .map(|(k, t)| format!("y{k}: {t}"))
                    .collect();
                text.push(match named.is_empty() {
                    true => format!("V{i}_{j}"),
                    false => format!("V{i}_{j}({})", named.join(", ")),
                });
                variants.push(fields);
            }
            self.enums.push(Enum { variants });
            format!("enum E{i} {{ {} }}\n\n", text.join(", "))
        }
    }

    fn default(&mut self, ty: &Ty) -> Option<String> {
        if self.rng.chance(2) {
            return None;
        }
        Some(match ty {
            Ty::Int => self.rng.below(10).to_string(),
            Ty::Float => "1.5".into(),
            Ty::Bool => "true".into(),
            Ty::Str => "\"d\"".into(),
            Ty::List(_) => "[]".into(),
            Ty::Map(..) => "[:]".into(),
            Ty::Opt(_) => "None".into(),
            _ => return None,
        })
    }

    /// A type at most `depth` levels deep; function types when `fns`.
    fn ty(&mut self, depth: u32, fns: bool) -> Ty {
        let inner = depth.saturating_sub(1);
        match self.rng.below(if depth == 0 { 5 } else { 15 }) {
            0 | 1 => Ty::Int,
            2 => Ty::Float,
            3 => Ty::Bool,
            4 | 5 => Ty::Str,
            6 | 7 => Ty::list(self.ty(inner, fns)),
            8 => {
                let key = if self.rng.chance(2) { Ty::Int } else { Ty::Str };
                Ty::Map(Box::new(key), Box::new(self.ty(inner, fns)))
            }
            9 => Ty::opt(self.ty(inner, fns)),
            10 => Ty::Res(Box::new(self.ty(inner, fns)), Box::new(self.ty(0, false))),
            11 if !self.records.is_empty() => Ty::Rec(self.rng.below(self.records.len())),
            12 if !self.enums.is_empty() => Ty::Enum(self.rng.below(self.enums.len())),
            13 | 14 if fns => {
                let types = self.value_types();
                match types.is_empty() {
                    true => Ty::Int,
                    false => self.rng.pick(&types).clone(),
                }
            }
            _ => Ty::Int,
        }
    }

    /// The type of each function value: a cheap function without `inout`, or a variant with
    /// fields.
    fn value_types(&self) -> Vec<Ty> {
        self.values().into_iter().map(|(_, t)| t).collect()
    }

    fn values(&self) -> Vec<(String, Ty)> {
        let fns = (self.fns.iter().enumerate())
            .filter(|(_, f)| !f.inout() && f.cost <= LEAF)
            .filter_map(|(i, f)| {
                let params = f.params.iter().map(|p| p.0.clone()).collect();
                Some((format!("f{i}"), Ty::Fn(params, Box::new(f.result.clone()?))))
            });
        let variants = self.enums.iter().enumerate().flat_map(|(i, e)| {
            (e.variants.iter().enumerate())
                .filter(|(_, fields)| !fields.is_empty())
                .map(move |(j, fields)| {
                    let ty = Ty::Fn(fields.clone(), Box::new(Ty::Enum(i)));
                    (format!("V{i}_{j}"), ty)
                })
        });
        fns.chain(variants).collect()
    }

    fn function(&mut self) -> String {
        if self.rng.chance(5) {
            return self.recursive();
        }
        let id = self.fns.len();
        let mut params = vec![];
        for _ in 0..self.rng.below(4) {
            let ty = self.ty(2, true);
            let inout = !matches!(ty, Ty::Fn(..)) && self.rng.chance(4);
            params.push((ty, inout));
        }
        let result = match params.iter().any(|p| p.1) && self.rng.chance(3) {
            true => None,
            false => Some(self.ty(2, true)),
        };
        let ret = result.clone().map_or(Ret::Bare, Ret::Value);
        let mut fx = Fx::new(ret, BUDGET);
        let mut sig = vec![];
        for (j, (ty, inout)) in params.iter().enumerate() {
            fx.bind(format!("p{j}"), ty.clone(), *inout);
            sig.push(format!("p{j}: {}{ty}", if *inout { "inout " } else { "" }));
        }
        let n = 1 + self.rng.below(5);
        let body = self.block(&mut fx, vec![], &[], n, result.as_ref());
        let arrow = result
            .as_ref()
            .map_or(String::new(), |t| format!(" -> {t}"));
        let cost = fx.cost + 1;
        self.fns.push(Func {
            params,
            result,
            cost,
        });
        format!("fn f{id}({}){arrow} {body}\n\n", sig.join(", "))
    }

    /// `fn f{id}(p0: Int, ...)`, which calls itself on `p0 - 1` until `p0` is 0, or on `p0` for good
    /// past `RECURSION` levels, which runs out of stack on every back end. A level costs at most
    /// `LEAF`.
    fn recursive(&mut self) -> String {
        let id = self.fns.len();
        let result = self.ty(1, false);
        let mut params = vec![(Ty::Int, false)];
        params.extend((0..self.rng.below(2)).map(|_| (self.ty(1, false), false)));
        let mut fx = Fx::new(Ret::Value(result.clone()), LEAF);
        let mut sig = vec![];
        for (j, (ty, _)) in params.iter().enumerate() {
            fx.bind(format!("p{j}"), ty.clone(), false);
            sig.push(format!("p{j}: {ty}"));
        }
        let base = self.typed(&mut fx, &result, 2);
        let step = format!("(if (p0 > {}) {{ p0 }} else {{ (p0 - 1) }})", RECURSION - 1);
        let args: Vec<_> = std::iter::once(step)
            .chain((1..params.len()).map(|j| format!("p{j}")))
            .collect();
        let head = [
            format!("if (p0 <= 0) {{ return {base} }}"),
            format!("let r: {result} = f{id}({})", args.join(", ")),
        ];
        let n = self.rng.below(3);
        let binds = vec![("r".into(), result.clone())];
        let body = self.block(&mut fx, binds, &head, n, Some(&result));
        let text = format!("fn f{id}({}) -> {result} {body}\n\n", sig.join(", "));
        self.fns.push(Func {
            params,
            result: Some(result),
            cost: (fx.cost + 1) * RECURSION,
        });
        text
    }

    /// The handler calls every function the budget allows and prints its result and what it
    /// changed through `inout`, a List of ordered items sometimes sorted as well.
    fn app(&mut self) -> String {
        let mut fx = Fx::new(Ret::Never, TOP_BUDGET);
        fx.indent = 1;
        let (mut lines, mut effects) = (String::new(), vec![]);
        for i in 0..self.fns.len() {
            if !fx.charge(self.fns[i].cost) {
                continue;
            }
            let (mut args, mut shown) = (vec![], vec![]);
            for (ty, inout) in self.fns[i].params.clone() {
                let arg = self.expr(&mut fx, &ty, 2);
                if inout {
                    let name = fx.fresh();
                    lines += &format!("  var {name}: {ty} = {arg}\n");
                    args.push(format!("&{name}"));
                    shown.push((name, ty));
                } else {
                    args.push(arg);
                }
            }
            let call = format!("f{i}({})", args.join(", "));
            match self.fns[i].result.clone() {
                Some(ty) => {
                    let name = fx.fresh();
                    lines += &format!("  let {name}: {ty} = {call}\n");
                    shown.insert(0, (name, ty));
                }
                None => lines += &format!("  {call}\n"),
            }
            for (name, ty) in shown {
                effects.push(format!("Print({}({name}))", self.show(&ty)));
                if matches!(&ty, Ty::List(t) if t.ordered()) && self.rng.chance(2) {
                    let sorted = fx.fresh();
                    lines += &format!("  var {sorted} = {name}\n  sort(&{sorted})\n");
                    effects.push(format!("Print({}({sorted}))", self.show(&ty)));
                }
                fx.bind(name, ty, false);
            }
        }
        effects.push(match self.rng.below(8) {
            0 => format!("ExitWith({})", self.rng.pick(&[0, 5, 9, 255])),
            _ => "Exit".into(),
        });
        format!(
            "enum Msg {{ Start }}\n\nfn update(m: Msg) -> List<Effect<Msg>> {{\n{lines}  [{}]\n}}\n\n\
             app generated {{\n  start: Start\n  update: update\n  grants: [Console]\n}}\n",
            effects.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_gives_one_program() {
        assert_eq!(program(7), program(7));
        assert_ne!(program(7), program(8));
    }
}
