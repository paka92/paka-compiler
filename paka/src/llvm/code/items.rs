//! Lists: literals, items and slices with their bounds checks inline, walks and searches. Each
//! List owns its block, so an item is written in place.

use super::Body;
use crate::core_form::{Block, Expr, Local, Site, Ty, Type};
use paka_rt::abi::table::{CONSTANT, LIST};

impl Body<'_, '_> {
    fn item_ty(&self, t: Ty) -> Ty {
        match self.module.core.types[t] {
            Type::List(item) => item,
            _ => unreachable!("a List"),
        }
    }

    fn words(&mut self, at: &str) -> (String, String) {
        let ptr = self.offset(at, LIST.ptr);
        let len = self.offset(at, LIST.len);
        (self.load("ptr", &ptr), self.load("i64", &len))
    }

    /// The address of item `i` of `ptr`'s items, each of type `item`.
    fn nth(&mut self, ptr: &str, item: Ty, i: &str) -> String {
        let size = self.size(item);
        self.op(format!("getelementptr [{size} x i8], ptr {ptr}, i64 {i}"))
    }

    /// A List literal of type `t`: a new block owned first, then each item stored as it is
    /// evaluated.
    pub(super) fn list(&mut self, t: Ty, items: &[Expr]) -> Option<String> {
        let item = self.item_ty(t);
        let ptr = match items.len() * self.size(item) {
            0 => "null".into(),
            bytes => self.alloc(bytes),
        };
        let out = self.name("%h");
        self.hold(&out, t);
        let count = items.len().to_string();
        let cap = if items.is_empty() {
            CONSTANT.to_string()
        } else {
            count.clone()
        };
        for (offset, ty, v) in [
            (LIST.ptr, "ptr", &ptr),
            (LIST.len, "i64", &count),
            (LIST.cap, "i64", &cap),
        ] {
            let at = self.offset(&out, offset);
            self.store(ty, v, &at);
        }
        let out = self.temp(t, out);
        for (i, e) in items.iter().enumerate() {
            let v = self.expr(e)??;
            let v = self.owned(item, &v);
            let at = self.nth(&ptr, item, &i.to_string());
            self.put(item, &v, &at);
        }
        Some(out)
    }

    /// The address of item `i` of the List at `at` of type `t`, after the bounds check.
    pub(super) fn item(&mut self, at: &str, t: Ty, i: &str, site: Site) -> String {
        let (ptr, len) = self.words(at);
        let outside = self.op(format!("icmp uge i64 {i}, {len}"));
        self.check(&outside, "out_of_bounds", site);
        self.nth(&ptr, self.item_ty(t), i)
    }

    /// A view of items `a` up to `b` excluded of the List at `at` of type `t`, after the bounds
    /// check. It owns nothing: taking it copies it.
    pub(super) fn slice(&mut self, at: &str, t: Ty, a: &str, b: &str, site: Site) -> String {
        let (ptr, len) = self.words(at);
        let reversed = self.op(format!("icmp ugt i64 {a}, {b}"));
        let past = self.op(format!("icmp ugt i64 {b}, {len}"));
        let outside = self.op(format!("or i1 {reversed}, {past}"));
        self.check(&outside, "out_of_bounds", site);
        let from = self.nth(&ptr, self.item_ty(t), a);
        let count = self.op(format!("sub i64 {b}, {a}"));
        let view = self.name("%h");
        self.hold(&view, t);
        let cap = CONSTANT.to_string();
        for (offset, ty, v) in [
            (LIST.ptr, "ptr", &from),
            (LIST.len, "i64", &count),
            (LIST.cap, "i64", &cap),
        ] {
            let at = self.offset(&view, offset);
            self.store(ty, v, &at);
        }
        view
    }

    pub(super) fn length(&mut self, at: &str) -> String {
        self.words(at).1
    }

    /// `for l in xs`: each item of the List as it was when the loop began, lent to `l`.
    pub(super) fn each(&mut self, l: Local, xs: &Expr, body: &Block) -> Option<()> {
        let item = self.item_ty(xs.ty?);
        let at = self.expr(xs)??;
        let (ptr, len) = self.words(&at);
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        let [head, inner, next, exit] = ["b"; 4].map(|s| self.name(s));
        self.line(format!("br label %{head}"));
        self.start(&head);
        let n = self.load("i64", &i);
        let more = self.op(format!("icmp ult i64 {n}, {len}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{exit}"));
        self.start(&inner);
        let from = self.nth(&ptr, item, &n);
        let v = self.get(item, &from);
        let local = self.at[l].clone();
        self.put(self.locals[l]?, &v, &local);
        self.looped(&next, &exit, body)?;
        self.line(format!("br label %{next}"));
        self.start(&next);
        let n = self.load("i64", &i);
        let step = self.op(format!("add i64 {n}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&exit);
        Some(())
    }

    /// The first position of `v` in the List at `xs` of type `t`, or -1.
    pub(super) fn position(&mut self, t: Ty, xs: &str, v: &str) -> String {
        let item = self.item_ty(t);
        let (ptr, len) = self.words(xs);
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        let [head, inner, found, next, done] = ["b"; 5].map(|s| self.name(s));
        self.line(format!("br label %{head}"));
        self.start(&head);
        let n = self.load("i64", &i);
        let more = self.op(format!("icmp ult i64 {n}, {len}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{done}"));
        self.start(&inner);
        let at = self.nth(&ptr, item, &n);
        let x = self.get(item, &at);
        let eq = self.equal(item, &x, v);
        self.line(format!("br i1 {eq}, label %{found}, label %{next}"));
        self.start(&next);
        let step = self.op(format!("add i64 {n}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&found);
        self.line(format!("br label %{done}"));
        self.start(&done);
        self.op(format!("phi i64 [ -1, %{head} ], [ {n}, %{found} ]"))
    }

    /// `Some(i)` when `i` is a position and `None` when it is -1, as an `Option<Int>` of type `opt`.
    pub(super) fn found(&mut self, opt: Ty, i: &str) -> String {
        let at = self.name("%h");
        self.hold(&at, opt);
        let none = self.op(format!("icmp slt i64 {i}, 0"));
        let tag = self.op(format!("zext i1 {none} to i64"));
        self.store("i64", &tag, &at);
        let payload = self.module.layouts.get(opt).alternatives[0][0].offset;
        let to = self.offset(&at, payload);
        self.store("i64", i, &to);
        at
    }

    /// The body of a List equality function: the same length, then each pair of items until one
    /// differs; `%no` is the function's false.
    pub(super) fn equal_items(&mut self, t: Ty) {
        let item = self.item_ty(t);
        let ((px, n), (py, m)) = (self.words("%x"), self.words("%y"));
        let same = self.op(format!("icmp eq i64 {n}, {m}"));
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        let [head, inner, next, done] = ["b"; 4].map(|s| self.name(s));
        self.line(format!("br i1 {same}, label %{head}, label %no"));
        self.start(&head);
        let k = self.load("i64", &i);
        let more = self.op(format!("icmp ult i64 {k}, {n}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{done}"));
        self.start(&inner);
        let (a, b) = (self.nth(&px, item, &k), self.nth(&py, item, &k));
        let (a, b) = (self.get(item, &a), self.get(item, &b));
        let eq = self.equal(item, &a, &b);
        self.line(format!("br i1 {eq}, label %{next}, label %no"));
        self.start(&next);
        let step = self.op(format!("add i64 {k}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&done);
        self.line("ret i1 true".into());
    }
}
