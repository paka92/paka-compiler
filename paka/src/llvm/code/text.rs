//! Strings through the runtime's String rows (`paka_rt1_text`), whole-value equality, and the
//! effect list an `update` builds.

use super::{Body, order};
use crate::core_form::{Bin, Effect, Expr, ExprKind, Site, Tag, Ty, Type};
use paka_rt::abi::dev::words::*;
use paka_rt::abi::table::{self, TEXT};

/// The String builtins that are one row each.
const ROWS: &[(&str, u64)] = &[
    ("min(a: String, b: String) -> String", MIN),
    ("max(a: String, b: String) -> String", MAX),
    ("code_at(text: String, i: Int) -> Int", CODE_AT),
    ("from_code(n: Int) -> String", FROM_CODE),
    ("words(text: String) -> List<String>", WORDS),
    ("split(text: String, sep: String) -> List<String>", SPLIT),
    ("chars(text: String) -> List<String>", CHARS),
    ("lines(text: String) -> List<String>", LINES),
    ("contains(text: String, part: String) -> Bool", CONTAINS),
    (
        "starts_with(text: String, part: String) -> Bool",
        STARTS_WITH,
    ),
    ("ends_with(text: String, part: String) -> Bool", ENDS_WITH),
    ("find(text: String, part: String) -> Option<Int>", FIND),
    ("parse_int(text: String) -> Option<Int>", PARSE_INT),
    ("parse_float(text: String) -> Option<Float>", PARSE_FLOAT),
    (
        "join(parts: List<String>, sep: String) -> String",
        JOIN_PARTS,
    ),
    ("repeat(text: String, n: Int) -> String", REPEAT),
    ("pad_left(text: String, width: Int) -> String", PAD_LEFT),
    ("pad_right(text: String, width: Int) -> String", PAD_RIGHT),
    ("trim(text: String) -> String", TRIM),
    ("trim_start(text: String) -> String", TRIM_START),
    ("trim_end(text: String) -> String", TRIM_END),
    (
        "replace(text: String, old: String, new: String) -> String",
        REPLACE,
    ),
    ("upper(text: String) -> String", UPPER),
    ("lower(text: String) -> String", LOWER),
    ("fixed(x: Float; digits: Int) -> String", FIXED),
];

impl Body<'_, '_> {
    fn string(&self) -> Ty {
        let at = self
            .module
            .core
            .types
            .iter()
            .position(|t| *t == Type::String);
        at.expect("the String type")
    }

    /// A String literal: its words point at a constant, which is copied before its first write.
    pub(super) fn literal(&mut self, s: &str) -> String {
        let ptr = match s.is_empty() {
            true => "null".into(),
            false => self.module.text(s.into()).0,
        };
        let out = self.name("%h");
        self.bytes(&out, table::STRING.size);
        let words = [
            (table::STRING.ptr, "ptr", ptr),
            (table::STRING.len, "i64", s.len().to_string()),
            (table::CHARS, "i64", s.chars().count().to_string()),
            (table::STRING.cap, "i64", table::CONSTANT.to_string()),
        ];
        for (offset, ty, v) in words {
            let at = self.offset(&out, offset);
            self.store(ty, &v, &at);
        }
        self.temp(self.string(), out)
    }

    /// String row `row` on up to three operands, each `(type, value)`, a Float as its bits. With
    /// a site, a trap there is passed on; without, the row cannot trap. The result is the row's:
    /// an Int, a Bool, the address of an `Option` or a new String.
    pub(super) fn text(
        &mut self,
        row: u64,
        site: Option<Site>,
        operands: &[(&str, &str)],
    ) -> String {
        let size = match row {
            COMPARE | CODE_AT | CONTAINS..=ENDS_WITH => 8,
            FIND..=PARSE_FLOAT => 16,
            _ => 32,
        };
        let out = self.name("%w");
        self.bytes(&out, size);
        let mut words = vec![];
        for &(ty, v) in operands {
            words.push(match ty {
                "i64" => v.into(),
                "i1" => self.op(format!("zext i1 {v} to i64")),
                "ptr" => self.op(format!("ptrtoint ptr {v} to i64")),
                _ => {
                    let at = self.name("%f");
                    self.slot(&at, "double");
                    self.store("double", v, &at);
                    self.load("i64", &at)
                }
            });
        }
        words.resize(3, "0".into());
        let at = match site {
            Some(site) => {
                let (text, len) = self.module.place(site);
                format!("ptr {text}, i64 {len}")
            }
            None => "ptr null, i64 0".into(),
        };
        self.declare(TEXT);
        let [a, b, c] = [&words[0], &words[1], &words[2]];
        let status = self.op(format!(
            "call i32 @paka_rt1_text(i64 {row}, i64 {a}, i64 {b}, i64 {c}, ptr {out}, {at})"
        ));
        if site.is_some() {
            self.pass(&status);
        }
        match row {
            COMPARE | CODE_AT => self.load("i64", &out),
            CONTAINS..=ENDS_WITH => {
                let w = self.load("i64", &out);
                self.op(format!("icmp ne i64 {w}, 0"))
            }
            FIND..=PARSE_FLOAT => out,
            WORDS..=LINES => {
                let strings = Type::List(self.string());
                let t = self.module.core.types.iter().position(|t| *t == strings);
                self.temp(t.expect("the List<String> type"), out)
            }
            _ => self.temp(self.string(), out),
        }
    }

    /// A String builtin of one row; `None` for any other.
    pub(super) fn text_builtin(
        &mut self,
        row: &str,
        args: &[(&str, String)],
        site: Site,
    ) -> Option<String> {
        let &(_, row) = ROWS.iter().find(|(name, _)| *name == row)?;
        let operands: Vec<(&str, &str)> = args.iter().map(|(t, v)| (*t, v.as_str())).collect();
        Some(self.text(row, Some(site), &operands))
    }

    /// Interpolation: the parts joined in order, an Int in decimal and a Bool as `true` or
    /// `false`.
    pub(super) fn interpolate(&mut self, parts: &[Expr], site: Site) -> Option<String> {
        let mut text: Option<String> = None;
        for part in parts {
            let v = self.expr(part)??;
            let v = match self.module.core.types[part.ty?] {
                Type::Int => self.text(SHOW, Some(site), &[("i64", &v)]),
                Type::Bool => self.text(SHOW, Some(site), &[("i1", &v), ("i64", "1")]),
                Type::String => v,
                _ => return None,
            };
            text = Some(match text {
                None => v,
                Some(t) => self.text(JOIN, Some(site), &[("ptr", &t), ("ptr", &v)]),
            });
        }
        Some(text.unwrap_or_else(|| self.literal("")))
    }

    /// A String's count of code points.
    pub(super) fn chars(&mut self, at: &str) -> String {
        let at = self.offset(at, table::CHARS);
        self.load("i64", &at)
    }

    /// `x op y` on values held in memory: a String's join and order, any value's equality.
    pub(super) fn held_binary(&mut self, op: &Bin, x: &Expr, y: &Expr) -> Option<String> {
        let t = x.ty?;
        let (x, y) = (self.expr(x)??, self.expr(y)??);
        let string = self.module.core.types[t] == Type::String;
        Some(match op {
            Bin::Join(site) => self.text(JOIN, Some(*site), &[("ptr", &x), ("ptr", &y)]),
            Bin::Eq(_) => self.equal(t, &x, &y),
            Bin::Ne(_) => {
                let eq = self.equal(t, &x, &y);
                self.op(format!("xor i1 {eq}, true"))
            }
            Bin::Order(c) if string => {
                let o = self.text(COMPARE, None, &[("ptr", &x), ("ptr", &y)]);
                let cc = order(c, ["slt", "sle", "sgt", "sge"]);
                self.op(format!("icmp {cc} i64 {o}, 0"))
            }
            _ => return None,
        })
    }

    /// Whether `x` and `y` of type `t` are equal. Floats compare as numbers, so NaN differs from
    /// itself; a record or variant through its equality function.
    pub(super) fn equal(&mut self, t: Ty, x: &str, y: &str) -> String {
        match (&self.module.core.types[t], self.ty(t)) {
            (Type::Float, _) => self.op(format!("fcmp oeq double {x}, {y}")),
            (Type::String, _) => {
                let o = self.text(COMPARE, None, &[("ptr", x), ("ptr", y)]);
                self.op(format!("icmp eq i64 {o}, 0"))
            }
            (_, "ptr") => {
                if self.module.compared.insert(t) {
                    self.module.equalities.push(t);
                }
                self.op(format!("call i1 @q{t}(ptr {x}, ptr {y})"))
            }
            (_, ty) => self.op(format!("icmp eq {ty} {x}, {y}")),
        }
    }

    /// `i1 @q<t>(ptr %x, ptr %y)`: a List item by item, a Map entry by entry in key order, a
    /// variant's tags, then field by field until one differs.
    pub(super) fn equality(mut self, t: Ty) -> String {
        match self.module.core.types[t] {
            Type::List(_) => self.equal_items(t),
            Type::Map(..) => self.equal_entries(t),
            _ => return self.equal_fields(t),
        }
        self.equality_fn(t)
    }

    /// A record's or variant's equality.
    fn equal_fields(mut self, t: Ty) -> String {
        let layout = self.module.layouts.get(t);
        let (tagged, alternatives) = (layout.tagged, layout.alternatives.clone());
        let mut labels = vec![];
        if tagged {
            let (x, y) = (self.load("i64", "%x"), self.load("i64", "%y"));
            let same = self.op(format!("icmp eq i64 {x}, {y}"));
            self.line(format!("br i1 {same}, label %tags, label %no"));
            self.start("tags");
            labels = (alternatives.iter()).map(|_| self.name("b")).collect();
            let cases: Vec<String> = (labels.iter().enumerate())
                .map(|(i, label)| format!("i64 {i}, label %{label}"))
                .collect();
            self.line(format!("switch i64 {x}, label %no [ {} ]", cases.join(" ")));
        }
        for (i, fields) in alternatives.iter().enumerate() {
            if let Some(label) = labels.get(i) {
                self.start(label);
            }
            for place in fields {
                let (x, y) = (self.field(place, "%x"), self.field(place, "%y"));
                let eq = self.equal(place.ty, &x, &y);
                let next = self.name("b");
                self.line(format!("br i1 {eq}, label %{next}, label %no"));
                self.start(&next);
            }
            self.line("ret i1 true".into());
        }
        self.equality_fn(t)
    }

    fn equality_fn(&self, t: Ty) -> String {
        format!(
            "\ndefine internal i1 @q{t}(ptr %x, ptr %y) {{\nentry:\n{}{}no:\n  ret i1 false\n}}\n",
            self.slots, self.lines
        )
    }

    /// The effect list `update` returns, built in a new block: each `Print` takes its String's
    /// bytes, which are freed once performed.
    pub(super) fn effects(&mut self, value: &Expr) -> Option<()> {
        let ExprKind::List(items) = &value.kind else {
            return self.effect_values(value);
        };
        let fx = match items.len() {
            0 => "null".into(),
            n => self.alloc(n * 24),
        };
        for (i, item) in items.iter().enumerate() {
            let at = self.offset(&fx, i * 24);
            let (tag, ptr, len) = match &item.kind {
                ExprKind::Variant(Tag::Effect(Effect::Print), fields) => {
                    let v = self.expr(fields.first()?)??;
                    let v = self.owned(self.string(), &v);
                    let ptr = self.load("ptr", &v);
                    let len = self.offset(&v, table::STRING.len);
                    (0, ptr, self.load("i64", &len))
                }
                ExprKind::Variant(Tag::Effect(Effect::Exit), _) => (1, "null".into(), "0".into()),
                _ => return None,
            };
            self.effect(&at, tag, &ptr, &len);
        }
        self.leave_fn(None);
        self.store("ptr", &fx, "%out");
        let count = self.offset("%out", 8);
        self.store("i64", &items.len().to_string(), &count);
        self.line("ret i32 0".into());
        Some(())
    }

    /// The effect list from `value`, a computed `List<Effect<Msg>>` of `Print` and `Exit` only
    /// (`alternative`): each `Print` takes a copy of its String.
    fn effect_values(&mut self, value: &Expr) -> Option<()> {
        let Type::List(effect) = self.module.core.types[value.ty?] else {
            return None;
        };
        let at = self.expr(value)??;
        let (ptr, len) = self.words(&at);
        let bytes = self.op(format!("mul i64 {len}, 24"));
        let fx = self.alloc(&bytes);
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        let [head, inner, print, exit, next, done] = ["b"; 6].map(|s| self.name(s));
        self.line(format!("br label %{head}"));
        self.start(&head);
        let n = self.load("i64", &i);
        let more = self.op(format!("icmp ult i64 {n}, {len}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{done}"));
        self.start(&inner);
        let item = self.nth(&ptr, effect, &n);
        let to = self.op(format!("getelementptr [24 x i8], ptr {fx}, i64 {n}"));
        let tag = self.load("i64", &item);
        let printed = self.op(format!("icmp eq i64 {tag}, {}", Effect::Print as usize));
        self.line(format!("br i1 {printed}, label %{print}, label %{exit}"));
        self.start(&print);
        let text = self.module.layouts.get(effect).alternatives[0][0].offset;
        let text = self.offset(&item, text);
        let text = self.owned(self.string(), &text);
        let bytes = self.load("ptr", &text);
        let count = self.offset(&text, table::STRING.len);
        let count = self.load("i64", &count);
        self.effect(&to, 0, &bytes, &count);
        self.line(format!("br label %{next}"));
        self.start(&exit);
        self.effect(&to, 1, "null", "0");
        self.line(format!("br label %{next}"));
        self.start(&next);
        let step = self.op(format!("add i64 {n}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&done);
        self.leave_fn(None);
        self.store("ptr", &fx, "%out");
        let count = self.offset("%out", 8);
        self.store("i64", &len, &count);
        self.line("ret i32 0".into());
        Some(())
    }

    /// Writes the M1 effect `tag` with text `ptr` and `len` at `at` (docs/abi/m1.md section 3).
    fn effect(&mut self, at: &str, tag: u64, ptr: &str, len: &str) {
        self.store("i64", &tag.to_string(), at);
        let to = self.offset(at, 8);
        self.store("ptr", ptr, &to);
        let to = self.offset(at, 16);
        self.store("i64", len, &to);
    }
}
