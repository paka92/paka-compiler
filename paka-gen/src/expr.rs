//! Expressions of a wanted type. A binary expression and a record literal are always bracketed,
//! so precedence and headers never matter.

use crate::ty::Ty;
use crate::{Fx, Gen, LEAF, Ret};

const INTS: &[&str] = &["0", "1", "2", "3", "7", "-1", "-42", "100", "65"];
/// The edges, picked one time in eight.
const EDGE_INTS: &[&str] = &[
    "9223372036854775807",
    "-9223372036854775808",
    "4611686018427387904",
    "-4611686018427387905",
    "9223372036854775806",
    "-9223372036854775807",
    "3037000500",
    "-3037000500",
];
const FLOATS: &[&str] = &[
    "0.0",
    "1.5",
    "-2.25",
    "-1.5",
    "-3.0",
    "0.1",
    "3.0",
    "1000000.0",
    "123456789.125",
    "-0.0",
    "(0.0 / 0.0)",
    "(1.0 / 0.0)",
    "(-1.0 / 0.0)",
];
const STRS: &[&str] = &[
    r#""""#,
    r#""a""#,
    r#""b""#,
    r#""hello world""#,
    r#""a,b,,c""#,
    r#""line1\nline2\n""#,
    r#""tab\there""#,
    r#""q\"uote""#,
    r#""back\\slash\r""#,
    r#""\{brace\}""#,
    r#""日本語""#,
    r#""é""#,
    r#""  pad  ""#,
    r#""12""#,
    r#""-7""#,
    r#""3.5""#,
];
const INT_KEYS: &[&str] = &["0", "1", "-5"];
const STR_KEYS: &[&str] = &[r#""a""#, r#""b""#, r#""""#];

impl Gen {
    /// An expression of type `ty` at most `depth` levels deep.
    pub(crate) fn expr(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> String {
        if depth == 0 || self.rng.chance(3) {
            return self.leaf(fx, ty);
        }
        let d = depth - 1;
        let tried = match self.rng.below(10) {
            0 => self.read(fx, ty),
            1 => self.call(fx, ty, d),
            2 => self.call_value(fx, ty, d),
            3 => Some(self.inline_if(fx, ty, d)),
            4 => self.try_(fx, ty, d),
            5 => self.pick_out(fx, ty, d),
            _ => self.form(fx, ty, d),
        };
        tried.unwrap_or_else(|| self.leaf(fx, ty))
    }

    /// An expression whose type needs no context.
    pub(crate) fn anchored(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> String {
        self.with(fx, true, |g, fx| g.expr(fx, ty, depth))
    }

    /// An expression whose type its position gives.
    pub(crate) fn typed(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> String {
        self.with(fx, false, |g, fx| g.expr(fx, ty, depth))
    }

    fn with(
        &mut self,
        fx: &mut Fx,
        anchored: bool,
        f: impl FnOnce(&mut Gen, &mut Fx) -> String,
    ) -> String {
        let was = std::mem::replace(&mut fx.anchored, anchored);
        let e = f(self, fx);
        fx.anchored = was;
        e
    }

    /// A literal or a local.
    pub(crate) fn leaf(&mut self, fx: &mut Fx, ty: &Ty) -> String {
        if self.rng.chance(3)
            && let Some(name) = self.local(fx, ty)
        {
            return name;
        }
        if !fx.anchored && self.rng.chance(1500) {
            self.holes += 1;
            return format!("??h{}", self.holes - 1);
        }
        if !fx.anchored && self.rng.chance(800) {
            return "unreachable()".into();
        }
        let free = !fx.anchored;
        match ty {
            Ty::Int if self.rng.chance(8) => self.rng.pick(EDGE_INTS).to_string(),
            Ty::Int => self.rng.pick(INTS).to_string(),
            Ty::Float => self.rng.pick(FLOATS).to_string(),
            Ty::Bool => self.rng.pick(&["true", "false"]).to_string(),
            Ty::Str => self.rng.pick(STRS).to_string(),
            Ty::List(_) if free && self.rng.chance(4) => "[]".into(),
            Ty::List(t) => {
                let items: Vec<_> = (0..1 + self.rng.below(3))
                    .map(|_| self.leaf(fx, t))
                    .collect();
                format!("[{}]", items.join(", "))
            }
            Ty::Map(..) if free && self.rng.chance(2) => "[:]".into(),
            Ty::Map(k, v) => {
                let keys = if **k == Ty::Int { INT_KEYS } else { STR_KEYS };
                let first = self.rng.below(keys.len());
                let n = 1 + self.rng.below(keys.len());
                let entries: Vec<_> = (0..n)
                    .map(|j| format!("{}: {}", keys[(first + j) % keys.len()], self.leaf(fx, v)))
                    .collect();
                format!("[{}]", entries.join(", "))
            }
            Ty::Opt(_) if free && self.rng.chance(3) => "None".into(),
            Ty::Opt(t) => format!("Some({})", self.with_leaf(fx, t)),
            Ty::Res(t, e) if free => match self.rng.chance(2) {
                true => format!("Ok({})", self.leaf(fx, t)),
                false => format!("Err({})", self.leaf(fx, e)),
            },
            Ty::Res(t, e) => format!("ok_or(Some({}), {})", self.leaf(fx, t), self.leaf(fx, e)),
            Ty::Rec(i) => self.record(fx, *i, 0),
            Ty::Enum(i) => self.variant(fx, *i, 0),
            Ty::Fn(..) => {
                let names: Vec<_> = self.values().into_iter().filter(|v| v.1 == *ty).collect();
                self.rng.pick(&names).0.clone()
            }
        }
    }

    /// A visible local of type `ty`.
    fn local(&mut self, fx: &mut Fx, ty: &Ty) -> Option<String> {
        let found: Vec<usize> = (0..fx.locals.len())
            .filter(|&i| !fx.locals[i].hidden && fx.locals[i].ty == *ty)
            .collect();
        if found.is_empty() {
            return None;
        }
        let i = *self.rng.pick(&found);
        fx.reads.push(i);
        Some(fx.locals[i].name.clone())
    }

    /// A local, or a field, item or entry of one, of type `ty`; an item may be near a List's end.
    pub(crate) fn read(&mut self, fx: &mut Fx, ty: &Ty) -> Option<String> {
        let mut found = vec![];
        for (i, l) in fx.locals.iter().enumerate().filter(|(_, l)| !l.hidden) {
            for (path, t) in self.paths(&l.ty, 2) {
                if t == *ty {
                    found.push((i, format!("{}{path}", l.name)));
                }
            }
            if let Ty::List(t) = &l.ty
                && **t == *ty
            {
                let back = self.rng.pick(&[1, 1, 2, 0]);
                found.push((i, format!("{0}[len({0}) - {back}]", l.name)));
            }
        }
        if found.is_empty() {
            return None;
        }
        let (i, text) = found.swap_remove(self.rng.below(found.len()));
        fx.reads.push(i);
        Some(text)
    }

    /// The steps from a value of `ty`, at most `depth`, with the type each reaches; the first is
    /// no step.
    pub(crate) fn paths(&self, ty: &Ty, depth: u32) -> Vec<(String, Ty)> {
        let mut out = vec![(String::new(), ty.clone())];
        if depth == 0 {
            return out;
        }
        let steps: Vec<(String, Ty)> = match ty {
            Ty::Rec(i) => (self.records[*i].fields.iter().enumerate())
                .map(|(j, f)| (format!(".x{j}"), f.0.clone()))
                .collect(),
            Ty::List(t) => vec![("[0]".into(), (**t).clone())],
            Ty::Map(k, v) => {
                let key = if **k == Ty::Int { "[0]" } else { r#"["a"]"# };
                vec![(key.into(), (**v).clone())]
            }
            _ => vec![],
        };
        for (step, t) in steps {
            let deeper = self.paths(&t, depth - 1).into_iter();
            out.extend(deeper.map(|(p, t)| (format!("{step}{p}"), t)));
        }
        out
    }

    fn record(&mut self, fx: &mut Fx, i: usize, depth: u32) -> String {
        let fields = self.records[i].fields.clone();
        let mut text = vec![];
        for (j, (ty, default)) in fields.iter().enumerate() {
            if !(*default && self.rng.chance(2)) {
                text.push(format!("x{j}: {}", self.typed(fx, ty, depth)));
            }
        }
        format!("(R{i} {{ {} }})", text.join(", "))
    }

    fn variant(&mut self, fx: &mut Fx, i: usize, depth: u32) -> String {
        let j = self.rng.below(self.enums[i].variants.len());
        let fields = self.enums[i].variants[j].clone();
        if fields.is_empty() {
            return format!("V{i}_{j}");
        }
        let args: Vec<_> = fields.iter().map(|t| self.typed(fx, t, depth)).collect();
        format!("V{i}_{j}({})", args.join(", "))
    }

    /// A call of an earlier function without `inout`, if the budget allows it.
    fn call(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> Option<String> {
        let found: Vec<usize> = (0..self.fns.len())
            .filter(|&i| !self.fns[i].inout() && self.fns[i].result.as_ref() == Some(ty))
            .collect();
        if found.is_empty() {
            return None;
        }
        let i = *self.rng.pick(&found);
        if !fx.charge(self.fns[i].cost) {
            return None;
        }
        let params: Vec<Ty> = self.fns[i].params.iter().map(|p| p.0.clone()).collect();
        Some(format!("f{i}({})", self.args(fx, &params, depth)))
    }

    /// A call through a local function value.
    fn call_value(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> Option<String> {
        let found: Vec<usize> = (0..fx.locals.len())
            .filter(|&i| !fx.locals[i].hidden)
            .filter(|&i| matches!(&fx.locals[i].ty, Ty::Fn(_, r) if **r == *ty))
            .collect();
        if found.is_empty() || !fx.charge(LEAF) {
            return None;
        }
        let i = *self.rng.pick(&found);
        fx.reads.push(i);
        let Ty::Fn(params, _) = fx.locals[i].ty.clone() else {
            unreachable!()
        };
        let name = fx.locals[i].name.clone();
        Some(format!("{name}({})", self.args(fx, &params, depth)))
    }

    fn args(&mut self, fx: &mut Fx, params: &[Ty], depth: u32) -> String {
        let args: Vec<_> = params.iter().map(|t| self.typed(fx, t, depth)).collect();
        args.join(", ")
    }

    fn inline_if(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> String {
        let cond = self.typed(fx, &Ty::Bool, depth);
        let mut out = format!("(if {cond} {{ {} }}", self.expr(fx, ty, depth));
        if self.rng.chance(3) {
            let cond = self.typed(fx, &Ty::Bool, depth);
            out += &format!(" else if {cond} {{ {} }}", self.expr(fx, ty, depth));
        }
        out + &format!(" else {{ {} }})", self.expr(fx, ty, depth))
    }

    /// `e?`, where the function's result allows it.
    fn try_(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> Option<String> {
        let inner = match &fx.ret {
            Ret::Value(Ty::Opt(_)) => Ty::opt(ty.clone()),
            Ret::Value(Ty::Res(_, e)) => Ty::Res(Box::new(ty.clone()), e.clone()),
            _ => return None,
        };
        Some(format!("({})?", self.anchored(fx, &inner, depth)))
    }

    /// An item, entry or field of a value that is not a local.
    fn pick_out(&mut self, fx: &mut Fx, ty: &Ty, depth: u32) -> Option<String> {
        match self.rng.below(3) {
            0 => {
                let list = self.anchored(fx, &Ty::list(ty.clone()), depth);
                Some(format!("{list}[0]"))
            }
            1 => {
                let key = self.rng.pick(STR_KEYS).to_string();
                Some(format!("[{key}: {}][{key}]", self.anchored(fx, ty, depth)))
            }
            _ => {
                let found: Vec<(usize, usize)> = (self.records.iter().enumerate())
                    .flat_map(|(i, r)| r.fields.iter().enumerate().map(move |(j, f)| (i, j, f)))
                    .filter(|(_, _, f)| f.0 == *ty)
                    .map(|(i, j, _)| (i, j))
                    .collect();
                if found.is_empty() {
                    return None;
                }
                let (i, j) = *self.rng.pick(&found);
                Some(format!("{}.x{j}", self.record(fx, i, depth)))
            }
        }
    }

    /// A type a visible local has and `keep` accepts, else `fallback`.
    pub(crate) fn local_ty(&mut self, fx: &Fx, keep: impl Fn(&Ty) -> bool, fallback: Ty) -> Ty {
        let found: Vec<&Ty> = fx
            .locals
            .iter()
            .map(|l| &l.ty)
            .filter(|t| keep(t))
            .collect();
        match found.is_empty() || self.rng.chance(3) {
            true => fallback,
            false => (*self.rng.pick(&found)).clone(),
        }
    }

    /// A String, List or Map type.
    fn container(&mut self, fx: &Fx) -> Ty {
        let fallback = match self.rng.below(3) {
            0 => Ty::Str,
            1 => Ty::list(self.ty(0, false)),
            _ => Ty::Map(Box::new(Ty::Str), Box::new(self.ty(0, false))),
        };
        self.local_ty(
            fx,
            |t| matches!(t, Ty::Str | Ty::List(_) | Ty::Map(..)),
            fallback,
        )
    }

    fn call1(&mut self, fx: &mut Fx, name: &str, ty: &Ty, depth: u32) -> String {
        format!("{name}({})", self.anchored(fx, ty, depth))
    }

    fn call2(&mut self, fx: &mut Fx, name: &str, a: &Ty, b: &Ty, depth: u32) -> String {
        let a = self.anchored(fx, a, depth);
        format!("{name}({a}, {})", self.anchored(fx, b, depth))
    }

    fn binary(&mut self, fx: &mut Fx, op: &str, ty: &Ty, depth: u32) -> String {
        let a = self.anchored(fx, ty, depth);
        format!("({a} {op} {})", self.typed(fx, ty, depth))
    }

    /// A built-in form that gives `ty`.
    fn form(&mut self, fx: &mut Fx, ty: &Ty, d: u32) -> Option<String> {
        let (int, float, str) = (&Ty::Int, &Ty::Float, &Ty::Str);
        Some(match ty {
            Ty::Int => match self.rng.below(13) {
                0..=3 => {
                    let op = *self.rng.pick(&["+", "-", "*", "/", "%"]);
                    self.binary(fx, op, int, d)
                }
                4 => format!("-({})", self.anchored(fx, int, d)),
                5 => {
                    let c = self.container(fx);
                    self.call1(fx, "len", &c, d)
                }
                6 => {
                    let f = *self.rng.pick(&["abs", "bit_not"]);
                    self.call1(fx, f, int, d)
                }
                7 => {
                    let f = [
                        "min", "max", "wrap_add", "wrap_sub", "wrap_mul", "bit_and", "bit_or",
                        "bit_xor",
                    ];
                    let f = *self.rng.pick(&f);
                    self.call2(fx, f, int, int, d)
                }
                8 => {
                    let f = *self.rng.pick(&["shift_left", "shift_right"]);
                    let count = self.rng.below(65);
                    format!("{f}({}, {count})", self.anchored(fx, int, d))
                }
                9 if self.rng.chance(3) => self.call1(fx, "to_int", float, d),
                10 => format!("code_at({}, 0)", self.anchored(fx, str, d)),
                _ => self.call2(fx, "unwrap_or", &Ty::opt(Ty::Int), int, d),
            },
            Ty::Float => match self.rng.below(8) {
                0..=2 => {
                    let op = *self.rng.pick(&["+", "-", "*", "/"]);
                    self.binary(fx, op, float, d)
                }
                3 => format!("-({})", self.anchored(fx, float, d)),
                4 => self.call1(fx, "to_float", int, d),
                5 => {
                    let f = ["sqrt", "floor", "ceil", "trunc", "round", "abs"];
                    let f = *self.rng.pick(&f);
                    self.call1(fx, f, float, d)
                }
                6 => {
                    let f = *self.rng.pick(&["min", "max"]);
                    self.call2(fx, f, float, float, d)
                }
                _ => format!("unwrap_or(parse_float({}), 0.5)", self.anchored(fx, str, d)),
            },
            Ty::Bool => match self.rng.below(9) {
                0 | 1 => {
                    let t = self.local_ty(fx, Ty::ordered, Ty::Int);
                    let op = *self.rng.pick(&["<", "<=", ">", ">="]);
                    self.binary(fx, op, &t, d)
                }
                2 | 3 => {
                    let fallback = self.ty(1, true);
                    let t = self.local_ty(fx, |_| true, fallback);
                    let op = *self.rng.pick(&["==", "!="]);
                    self.binary(fx, op, &t, d)
                }
                4 => format!("!({})", self.anchored(fx, &Ty::Bool, d)),
                5 => {
                    let op = *self.rng.pick(&["&&", "||"]);
                    self.binary(fx, op, &Ty::Bool, d)
                }
                6 => {
                    let f = *self.rng.pick(&["contains", "starts_with", "ends_with"]);
                    self.call2(fx, f, str, str, d)
                }
                7 => {
                    let t = self.local_ty(fx, |t| matches!(t, Ty::List(_)), Ty::list(Ty::Int));
                    let Ty::List(item) = &t else { unreachable!() };
                    self.call2(fx, "contains", &t, &item.clone(), d)
                }
                _ => {
                    let c = self.container(fx);
                    match &c {
                        Ty::Map(k, _) if self.rng.chance(2) => {
                            self.call2(fx, "has", &c, &k.clone(), d)
                        }
                        _ => self.call1(fx, "is_empty", &c, d),
                    }
                }
            },
            Ty::Str => match self.rng.below(12) {
                0..=2 => self.binary(fx, "+", str, d),
                3 => self.interp(fx),
                4 => {
                    let digits = if self.rng.chance(30) {
                        21
                    } else {
                        self.rng.below(21)
                    };
                    format!("fixed({}, digits: {digits})", self.anchored(fx, float, d))
                }
                5 => self.call2(fx, "join", &Ty::list(Ty::Str), str, d),
                6 => {
                    let f = *self.rng.pick(&["repeat", "pad_left", "pad_right"]);
                    format!("{f}({}, {})", self.anchored(fx, str, d), self.rng.below(4))
                }
                7 => {
                    let f = ["trim", "trim_start", "trim_end", "upper", "lower"];
                    let f = *self.rng.pick(&f);
                    self.call1(fx, f, str, d)
                }
                8 => {
                    let (s, old) = (self.anchored(fx, str, d), self.anchored(fx, str, d));
                    format!("replace({s}, {old}, {})", self.anchored(fx, str, d))
                }
                9 => format!(
                    "from_code({})",
                    self.rng.pick(&["65", "233", "26085", "128512", "97", "48"])
                ),
                10 => format!("{}[0..<1]", self.anchored(fx, str, d)),
                _ => format!("{}[0]", self.anchored(fx, str, d)),
            },
            Ty::List(t) => match self.rng.below(6) {
                0 if **t == Ty::Str => {
                    let f = *self.rng.pick(&["chars", "lines", "words"]);
                    self.call1(fx, f, str, d)
                }
                1 if **t == Ty::Str => self.call2(fx, "split", str, str, d),
                2 if matches!(**t, Ty::Int | Ty::Str) => {
                    let m = Ty::Map(t.clone(), Box::new(Ty::Int));
                    self.call1(fx, "keys", &m, d)
                }
                3 => self.call1(fx, "values", &Ty::Map(Box::new(Ty::Str), t.clone()), d),
                4 => {
                    let n = if self.rng.chance(20) {
                        -1
                    } else {
                        self.rng.below(4) as i64
                    };
                    format!("list_of({}, count: {n})", self.anchored(fx, t, d))
                }
                5 => match self.local(fx, ty) {
                    Some(xs) => format!("{xs}[0..<min(1, len({xs}))]"),
                    None => format!("{}[0..<1]", self.anchored(fx, ty, d)),
                },
                _ => {
                    let items: Vec<_> = (0..1 + self.rng.below(3))
                        .map(|_| self.expr(fx, t, d))
                        .collect();
                    format!("[{}]", items.join(", "))
                }
            },
            Ty::Opt(t) => match self.rng.below(4) {
                0 => {
                    let m = Ty::Map(Box::new(Ty::Str), t.clone());
                    self.call2(fx, "get", &m, str, d)
                }
                1 if **t == Ty::Int => match self.rng.chance(2) {
                    true => self.call2(fx, "find", str, str, d),
                    false => self.call1(fx, "parse_int", str, d),
                },
                2 if **t == Ty::Float => self.call1(fx, "parse_float", str, d),
                _ => format!("Some({})", self.anchored(fx, t, d)),
            },
            Ty::Res(t, e) => match self.rng.below(3) {
                0 => self.call2(fx, "ok_or", &Ty::opt((**t).clone()), e, d),
                1 if !fx.anchored => format!("Ok({})", self.expr(fx, t, d)),
                _ if !fx.anchored => format!("Err({})", self.expr(fx, e, d)),
                _ => return None,
            },
            Ty::Rec(i) => self.record(fx, *i, d),
            Ty::Enum(i) => self.variant(fx, *i, d),
            Ty::Map(..) | Ty::Fn(..) => return None,
        })
    }

    /// Text with an Int, Bool or String local or a number put in.
    fn interp(&mut self, fx: &mut Fx) -> String {
        let mut out = String::from("\"");
        for k in 0..1 + self.rng.below(3) {
            let ty = self.rng.pick(&[Ty::Int, Ty::Bool, Ty::Str]).clone();
            let part = self.local(fx, &ty).unwrap_or_else(|| k.to_string());
            out += &format!("t{k} {{{part}}}");
        }
        out + "\""
    }
}
