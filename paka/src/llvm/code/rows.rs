//! The generic rows, lowered for each concrete type (M6-14). A List grows as the dev runtime's
//! does and its items move bitwise; `unwrap_or` and `ok_or` copy what they give.

use super::{Body, Value, stand_in};
use crate::core_form::{Site, Ty};
use paka_rt::abi::table::LIST;
use paka_rt::ops::CAP;

impl Body<'_, '_> {
    /// Row `row` on `args` of types `tys`, when it is generic; a String row otherwise.
    pub(super) fn generic(
        &mut self,
        row: &str,
        args: &[(&str, String)],
        tys: &[Ty],
        result: Option<Ty>,
        site: Site,
    ) -> Option<Value> {
        let x: Vec<&str> = args.iter().map(|(_, v)| v.as_str()).collect();
        Some(match row.split('(').next()? {
            "append" => self.append(tys[0], x[0], x[1], site),
            "extend" => self.extend(tys[0], x[0], x[1], site),
            "pop" => self.pop(tys[0], x[0], site),
            "insert" => self.insert(tys[0], x[0], x[1], x[2], site),
            "remove_at" => self.remove_at(tys[0], x[0], x[1], site),
            "swap" => self.swap(tys[0], x[0], x[1], x[2], site),
            "reverse" => self.reverse(tys[0], x[0]),
            "sort" => self.sort(tys[0], x[0], None),
            "sort_by" => self.sort(tys[0], x[0], Some((tys[1], x[1]))),
            "list_of" => Some(self.list_of(result?, x[0], x[1], site)),
            "unwrap_or" => Some(self.unwrap_or(tys[0], x[0], x[1])),
            "ok_or" => Some(self.ok_or(tys[0], result?, x[0], x[1])),
            "unreachable" => {
                self.trap("assert", site);
                let dead = self.name("b");
                self.start(&dead);
                result.map(|t| stand_in(self.ty(t)))
            }
            name @ ("get" | "has" | "remove" | "keys" | "values") => {
                return self.map_builtin(name, &x, tys, result, site);
            }
            _ => return self.text_builtin(row, args, site).map(Some),
        })
    }

    fn append(&mut self, t: Ty, xs: &str, item: &str, site: Site) -> Value {
        let item_t = self.item_ty(t);
        // Kept before the List may move, as it may lie in it.
        let v = self.kept(item_t, item);
        let len = self.room(t, xs, "1", site);
        let (ptr, _) = self.words(xs);
        let at = self.nth(&ptr, item_t, &len);
        self.put(item_t, &v, &at);
        self.resize(xs, &len, "add", "1");
        None
    }

    fn extend(&mut self, t: Ty, xs: &str, more: &str, site: Site) -> Value {
        let item = self.item_ty(t);
        let more = self.owned(t, more);
        let (from, n) = self.words(&more);
        let len = self.room(t, xs, &n, site);
        let (ptr, _) = self.words(xs);
        let at = self.nth(&ptr, item, &len);
        let bytes = self.op(format!("mul i64 {n}, {}", self.size(item)));
        self.memmove(&at, &from, &bytes);
        self.resize(xs, &len, "add", &n);
        let cap = self.offset(&more, LIST.cap);
        let cap = self.load("i64", &cap);
        self.free_block(t, &from, &cap);
        None
    }

    fn pop(&mut self, t: Ty, xs: &str, site: Site) -> Value {
        let item = self.item_ty(t);
        let (ptr, len) = self.words(xs);
        let empty = self.op(format!("icmp eq i64 {len}, 0"));
        self.check(&empty, "out_of_bounds", site);
        let last = self.resize(xs, &len, "sub", "1");
        let at = self.nth(&ptr, item, &last);
        let v = self.moved(item, &at);
        Some(self.temp(item, v))
    }

    fn insert(&mut self, t: Ty, xs: &str, item: &str, i: &str, site: Site) -> Value {
        let item_t = self.item_ty(t);
        let len = self.length(xs);
        let outside = self.op(format!("icmp ugt i64 {i}, {len}"));
        self.check(&outside, "out_of_bounds", site);
        let v = self.kept(item_t, item);
        let len = self.room(t, xs, "1", site);
        let (ptr, _) = self.words(xs);
        let at = self.nth(&ptr, item_t, i);
        let next = self.op(format!("add i64 {i}, 1"));
        let to = self.nth(&ptr, item_t, &next);
        self.shift_items(item_t, &to, &at, &len, i);
        self.put(item_t, &v, &at);
        self.resize(xs, &len, "add", "1");
        None
    }

    fn remove_at(&mut self, t: Ty, xs: &str, i: &str, site: Site) -> Value {
        let item = self.item_ty(t);
        let at = self.item(xs, t, i, site);
        let v = self.moved(item, &at);
        let (ptr, len) = self.words(xs);
        let next = self.op(format!("add i64 {i}, 1"));
        let from = self.nth(&ptr, item, &next);
        self.shift_items(item, &at, &from, &len, &next);
        self.resize(xs, &len, "sub", "1");
        Some(self.temp(item, v))
    }

    fn swap(&mut self, t: Ty, xs: &str, i: &str, j: &str, site: Site) -> Value {
        let a = self.item(xs, t, i, site);
        let b = self.item(xs, t, j, site);
        self.exchange(self.item_ty(t), &a, &b);
        None
    }

    fn reverse(&mut self, t: Ty, xs: &str) -> Value {
        let item = self.item_ty(t);
        let (ptr, len) = self.words(xs);
        let (i, j) = (self.name("%i"), self.name("%i"));
        self.slot(&i, "i64");
        self.slot(&j, "i64");
        let last = self.op(format!("sub i64 {len}, 1"));
        self.store("i64", "0", &i);
        self.store("i64", &last, &j);
        let [head, inner, exit] = ["b"; 3].map(|s| self.name(s));
        self.line(format!("br label %{head}"));
        self.start(&head);
        let (m, n) = (self.load("i64", &i), self.load("i64", &j));
        let more = self.op(format!("icmp slt i64 {m}, {n}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{exit}"));
        self.start(&inner);
        let (a, b) = (self.nth(&ptr, item, &m), self.nth(&ptr, item, &n));
        self.exchange(item, &a, &b);
        let (m, n) = (
            self.op(format!("add i64 {m}, 1")),
            self.op(format!("sub i64 {n}, 1")),
        );
        self.store("i64", &m, &i);
        self.store("i64", &n, &j);
        self.line(format!("br label %{head}"));
        self.start(&exit);
        None
    }

    /// Sorts the List at `xs` of type `t` with the module's sort function for its items; `by` is
    /// `sort_by`'s `before`, its type and value. Under two items nothing is called.
    fn sort(&mut self, t: Ty, xs: &str, by: Option<(Ty, &str)>) -> Value {
        let item = self.item_ty(t);
        let (ptr, len) = self.words(xs);
        let small = self.op(format!("icmp slt i64 {len}, 2"));
        let [go, done] = ["b"; 2].map(|s| self.name(s));
        self.line(format!("br i1 {small}, label %{done}, label %{go}"));
        self.start(&go);
        let bytes = self.op(format!("mul i64 {len}, {}", self.size(item)));
        let scratch = self.alloc(&bytes);
        let (name, id) = match by {
            Some((f, id)) => {
                self.module.dispatched.insert(f);
                (format!("@b{f}"), format!(", i64 {id}"))
            }
            None => (format!("@s{item}"), String::new()),
        };
        self.module.sorts.insert((item, by.map(|(f, _)| f)));
        let status = self.op(format!(
            "call i32 {name}(ptr {ptr}, ptr {scratch}, i64 {len}{id})"
        ));
        self.free_block(t, &scratch, &len);
        self.pass(&status);
        self.line(format!("br label %{done}"));
        self.start(&done);
        None
    }

    /// `count` copies of `item` in a new List of type `t`, the last the item itself.
    fn list_of(&mut self, t: Ty, item: &str, count: &str, site: Site) -> String {
        let item_t = self.item_ty(t);
        let negative = self.op(format!("icmp slt i64 {count}, 0"));
        self.check(&negative, "out_of_bounds", site);
        let huge = self.op(format!("icmp sgt i64 {count}, {CAP}"));
        self.check(&huge, "memory", site);
        let [make, head, inner, join] = ["b"; 4].map(|s| self.name(s));
        let zero = self.op(format!("icmp eq i64 {count}, 0"));
        let from = self.current();
        self.line(format!("br i1 {zero}, label %{join}, label %{make}"));
        self.start(&make);
        let bytes = self.op(format!("mul i64 {count}, {}", self.size(item_t)));
        let block = self.alloc(&bytes);
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        self.line(format!("br label %{head}"));
        self.start(&head);
        let n = self.load("i64", &i);
        let more = self.op(format!("icmp slt i64 {n}, {count}"));
        let filled = self.current();
        self.line(format!("br i1 {more}, label %{inner}, label %{join}"));
        self.start(&inner);
        let at = self.nth(&block, item_t, &n);
        self.copy_into(item_t, item, &at);
        let step = self.op(format!("add i64 {n}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&join);
        let ptr = self.op(format!("phi ptr [ null, %{from} ], [ {block}, %{filled} ]"));
        let out = self.list_words(t, &ptr, count, count);
        self.temp(t, out)
    }

    /// The payload of the Option at `opt` of type `o`, or `fallback`, copied.
    fn unwrap_or(&mut self, o: Ty, opt: &str, fallback: &str) -> String {
        let place = self.module.layouts.get(o).alternatives[0][0];
        let tag = self.load("i64", opt);
        let some = self.op(format!("icmp eq i64 {tag}, 0"));
        let payload = self.field(&place, opt);
        let ty = self.ty(place.ty);
        let v = self.op(format!("select i1 {some}, {ty} {payload}, {ty} {fallback}"));
        let v = self.copied(place.ty, &v);
        self.temp(place.ty, v)
    }

    /// `Ok` of the payload of the Option at `opt` of type `o`, or `Err(error)`, as a new Result of
    /// type `r`.
    fn ok_or(&mut self, o: Ty, r: Ty, opt: &str, error: &str) -> String {
        let payload = self.module.layouts.get(o).alternatives[0][0];
        let result = self.module.layouts.get(r).alternatives.clone();
        let out = self.name("%h");
        self.hold(&out, r);
        let tag = self.load("i64", opt);
        let some = self.op(format!("icmp eq i64 {tag}, 0"));
        let [ok, err, merge] = ["b"; 3].map(|s| self.name(s));
        self.line(format!("br i1 {some}, label %{ok}, label %{err}"));
        for (alt, label) in [ok, err].iter().enumerate() {
            self.start(label);
            self.store("i64", &alt.to_string(), &out);
            let place = result[alt][0];
            let v = match alt {
                0 => self.field(&payload, opt),
                _ => error.into(),
            };
            let v = self.copied(place.ty, &v);
            self.place_into(&out, &place, &v);
            self.line(format!("br label %{merge}"));
        }
        self.start(&merge);
        self.temp(r, out)
    }

    /// Makes room for `more` items past the length of the List at `xs` of type `t`, and gives that
    /// length; a `memory` trap at `site` past `CAP`. A new block is as the dev runtime's.
    fn room(&mut self, t: Ty, xs: &str, more: &str, site: Site) -> String {
        let size = self.size(self.item_ty(t));
        let (ptr, len) = self.words(xs);
        let cap_at = self.offset(xs, LIST.cap);
        let cap = self.load("i64", &cap_at);
        let need = self.op(format!("add i64 {len}, {more}"));
        let over = self.op(format!("icmp ugt i64 {need}, {CAP}"));
        self.check(&over, "memory", site);
        let short = self.op(format!("icmp ugt i64 {need}, {cap}"));
        let [grow, done] = ["b"; 2].map(|s| self.name(s));
        self.line(format!("br i1 {short}, label %{grow}, label %{done}"));
        self.start(&grow);
        let double = self.op(format!("add i64 {cap}, {cap}"));
        let new = self.unsigned("ugt", &need, &double);
        let new = self.unsigned("ugt", &new, "4");
        let new = self.unsigned("ult", &new, &CAP.to_string());
        let bytes = self.op(format!("mul i64 {new}, {size}"));
        let block = self.alloc(&bytes);
        let used = self.op(format!("mul i64 {len}, {size}"));
        self.memmove(&block, &ptr, &used);
        self.free_block(t, &ptr, &cap);
        let ptr_at = self.offset(xs, LIST.ptr);
        self.store("ptr", &block, &ptr_at);
        self.store("i64", &new, &cap_at);
        self.line(format!("br label %{done}"));
        self.start(&done);
        len
    }

    fn unsigned(&mut self, cc: &str, x: &str, y: &str) -> String {
        let first = self.op(format!("icmp {cc} i64 {x}, {y}"));
        self.op(format!("select i1 {first}, i64 {x}, i64 {y}"))
    }

    /// Stores `len op n` as the length of the List at `xs`, and gives it.
    fn resize(&mut self, xs: &str, len: &str, op: &str, n: &str) -> String {
        let new = self.op(format!("{op} i64 {len}, {n}"));
        let at = self.offset(xs, LIST.len);
        self.store("i64", &new, &at);
        new
    }

    /// Frees the block at `ptr` of `cap` items a List of type `t` had, not its items.
    fn free_block(&mut self, t: Ty, ptr: &str, cap: &str) {
        let words = self.list_words(t, ptr, "0", cap);
        self.drop_at(t, &words);
    }

    /// Moves the items from the one at `from`, item `first`, up to item `len` to `to`.
    fn shift_items(&mut self, item: Ty, to: &str, from: &str, len: &str, first: &str) {
        let rest = self.op(format!("sub i64 {len}, {first}"));
        let bytes = self.op(format!("mul i64 {rest}, {}", self.size(item)));
        self.memmove(to, from, &bytes);
    }

    /// The value of type `t` at `at` in a place of its own, which then holds it.
    fn moved(&mut self, t: Ty, at: &str) -> String {
        match self.ty(t) {
            "ptr" => {
                let to = self.fresh(t);
                self.put(t, at, &to);
                to
            }
            _ => self.get(t, at),
        }
    }

    /// Swaps the values of type `t` at `a` and `b`.
    fn exchange(&mut self, t: Ty, a: &str, b: &str) {
        let (x, y) = (self.moved(t, a), self.get(t, b));
        self.put(t, &y, a);
        self.put(t, &x, b);
    }
}
