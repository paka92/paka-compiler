//! The merge sort of `sort` and `sort_by`, one function for each item type: M2-14 section 1.13's,
//! as the interpreter's, so `before` sees the same pairs in the same order.

use super::Body;
use crate::core_form::{Ty, Type};
use paka_rt::abi::dev::words::COMPARE;

impl Body<'_, '_> {
    /// `i32 @s<item>(ptr %xs, ptr %tmp, i64 %n)`, or `@b<f>` with `i64 %id` for `sort_by` with a
    /// `before` of type `f`: sorts the `%n` items at `%xs` through `%tmp`, room for as many.
    pub(super) fn sorter(mut self, item: Ty, by: Option<Ty>) -> Option<String> {
        let (name, id) = match by {
            Some(f) => (format!("@b{f}"), ", i64 %id"),
            None => (format!("@s{item}"), ""),
        };
        let small = self.op("icmp slt i64 %n, 2".into());
        let [split, head, inner, right, left, next, rest] = ["b"; 7].map(|s| self.name(s));
        self.line(format!("br i1 {small}, label %done, label %{split}"));
        self.start(&split);
        let mid = self.divide("sdiv", self.site, "%n", "2");
        let high = self.nth("%xs", item, &mid);
        let count = self.op(format!("sub i64 %n, {mid}"));
        for (at, n) in [("%xs", &mid), (high.as_str(), &count)] {
            let status = self.op(format!("call i32 {name}(ptr {at}, ptr %tmp, i64 {n}{id})"));
            self.pass(&status);
        }
        for k in ["%i", "%j", "%k"] {
            self.slot(k, "i64");
            self.store("i64", "0", k);
        }
        self.line(format!("br label %{head}"));
        self.start(&head);
        let (i, j) = (self.load("i64", "%i"), self.load("i64", "%j"));
        let a = self.op(format!("icmp slt i64 {i}, {mid}"));
        let b = self.op(format!("icmp slt i64 {j}, {count}"));
        let both = self.op(format!("and i1 {a}, {b}"));
        self.line(format!("br i1 {both}, label %{inner}, label %{rest}"));
        self.start(&inner);
        let (l, r) = (self.nth("%xs", item, &i), self.nth(&high, item, &j));
        let first = self.before(item, by, &r, &l)?;
        self.line(format!("br i1 {first}, label %{right}, label %{left}"));
        for (label, from, at, n) in [(&right, &r, "%j", &j), (&left, &l, "%i", &i)] {
            self.start(label);
            self.take(item, from);
            let step = self.op(format!("add i64 {n}, 1"));
            self.store("i64", &step, at);
            self.line(format!("br label %{next}"));
        }
        self.start(&next);
        let k = self.load("i64", "%k");
        let step = self.op(format!("add i64 {k}, 1"));
        self.store("i64", &step, "%k");
        self.line(format!("br label %{head}"));
        // The right items left are in place already; the left ones follow those taken.
        self.start(&rest);
        let k = self.load("i64", "%k");
        let (to, from) = (self.nth("%tmp", item, &k), self.nth("%xs", item, &i));
        let left = self.op(format!("sub i64 {mid}, {i}"));
        let size = self.size(item);
        let bytes = self.op(format!("mul i64 {left}, {size}"));
        self.memmove(&to, &from, &bytes);
        let taken = self.op(format!("add i64 {k}, {left}"));
        let bytes = self.op(format!("mul i64 {taken}, {size}"));
        self.memmove("%xs", "%tmp", &bytes);
        self.line("br label %done".into());
        Some(format!(
            "\ndefine internal i32 {name}(ptr %xs, ptr %tmp, i64 %n{id}) {{\nentry:\n{}{}done:\n  ret i32 0\n}}\n",
            self.slots, self.lines
        ))
    }

    /// Whether the item at `r` must come before the one at `l`: `before(r, l)` for `sort_by`,
    /// `r < l` for `sort`.
    fn before(&mut self, item: Ty, by: Option<Ty>, r: &str, l: &str) -> Option<String> {
        let (r, l) = (self.get(item, r), self.get(item, l));
        if let Some(f) = by {
            let Type::Fn(_, result) = &self.module.core.types[f] else {
                return None;
            };
            let ty = self.ty(item);
            let args = [("i64", "%id".into()), (ty, r), (ty, l)];
            return self.invoke(&format!("@v{f}"), *result, &args)?;
        }
        Some(match self.module.core.types[item] {
            Type::Int => self.op(format!("icmp slt i64 {r}, {l}")),
            Type::Float => self.op(format!("fcmp olt double {r}, {l}")),
            _ => {
                let o = self.text(COMPARE, None, &[("ptr", &r), ("ptr", &l)]);
                self.op(format!("icmp slt i64 {o}, 0"))
            }
        })
    }

    /// Moves the item at `from` to the next place in `%tmp`.
    fn take(&mut self, item: Ty, from: &str) {
        let k = self.load("i64", "%k");
        let to = self.nth("%tmp", item, &k);
        let v = self.get(item, from);
        self.put(item, &v, &to);
    }
}
