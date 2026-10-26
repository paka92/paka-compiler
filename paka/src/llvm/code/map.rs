//! Maps through the runtime's Map rows (`paka_rt1_map`, M6-11b). A Map is one word: null, or a
//! block of its length, capacity and entries in rising key order (docs/decisions/M4-05.md); its
//! length and its entries are read inline.

use super::{Body, Value};
use crate::core_form::{Expr, Site, Ty, Type};
use paka_rt::abi::dev::map::*;
use paka_rt::abi::table::MAP;

impl Body<'_, '_> {
    pub(super) fn map_types(&self, t: Ty) -> (Ty, Ty) {
        match self.module.core.types[t] {
            Type::Map(k, v) => (k, v),
            _ => unreachable!("a Map"),
        }
    }

    /// A Map literal of type `t`: an empty Map owned first, then each entry added as it is
    /// evaluated; a later equal key replaces the value.
    pub(super) fn map(&mut self, t: Ty, entries: &[(Expr, Expr)]) -> Option<String> {
        let v = self.map_types(t).1;
        let out = self.fresh(t);
        self.store("ptr", "null", &out);
        let out = self.temp(t, out);
        for (key, value) in entries {
            let (key, value) = (self.expr(key)??, self.expr(value)??);
            let at = self.map_row(PUT, t, &out, &key, self.site);
            let value = self.owned(v, &value);
            self.drop_at(v, &at);
            self.put(v, &value, &at);
        }
        Some(out)
    }

    /// The address of the value under `key` in the Map at `m` of type `t`: `add` makes a missing
    /// entry, else a missing key traps `out_of_bounds` at `site`.
    pub(super) fn entry(&mut self, m: &str, t: Ty, key: &str, site: Site, add: bool) -> String {
        self.map_row(if add { PUT } else { AT }, t, m, key, site)
    }

    /// Map row `name` on `x`, of types `tys`; `None` for a row that is not one.
    pub(super) fn map_builtin(
        &mut self,
        name: &str,
        x: &[&str],
        tys: &[Ty],
        result: Option<Ty>,
        site: Site,
    ) -> Option<Value> {
        let t = tys[0];
        Some(match name {
            "get" => Some(self.get_key(t, x[0], x[1], result?)),
            "has" => {
                let at = self.find(t, x[0], x[1]);
                Some(self.op(format!("icmp ne ptr {at}, null")))
            }
            "remove" => {
                let key = self.key_word(t, x[1]);
                self.map_call(REMOVE, t, x[0], &key, "null", None);
                None
            }
            "keys" | "values" => {
                let out = self.fresh(result?);
                let row = if name == "keys" { KEYS } else { VALUES };
                self.map_call(row, t, x[0], "0", &out, Some(site));
                Some(self.temp(result?, out))
            }
            _ => return None,
        })
    }

    /// `Some` of a copy of the value under `key`, else `None`, as an Option of type `opt`.
    fn get_key(&mut self, t: Ty, m: &str, key: &str, opt: Ty) -> String {
        let v = self.map_types(t).1;
        let at = self.find(t, m, key);
        let out = self.fresh(opt);
        let found = self.op(format!("icmp ne ptr {at}, null"));
        let [some, none, done] = ["b"; 3].map(|s| self.name(s));
        self.line(format!("br i1 {found}, label %{some}, label %{none}"));
        self.start(&some);
        self.store("i64", "0", &out);
        let value = self.get(v, &at);
        let value = self.copied(v, &value);
        let place = self.module.layouts.get(opt).alternatives[0][0];
        self.place_into(&out, &place, &value);
        self.line(format!("br label %{done}"));
        self.start(&none);
        self.store("i64", "1", &out);
        self.line(format!("br label %{done}"));
        self.start(&done);
        self.temp(opt, out)
    }

    /// The number of entries of the Map at `m`.
    pub(super) fn map_len(&mut self, m: &str) -> String {
        let block = self.load("ptr", m);
        self.count(&block)
    }

    fn count(&mut self, block: &str) -> String {
        let empty = self.op(format!("icmp eq ptr {block}, null"));
        let from = self.current();
        let [some, done] = ["b"; 2].map(|s| self.name(s));
        self.line(format!("br i1 {empty}, label %{done}, label %{some}"));
        self.start(&some);
        let n = self.load("i64", block);
        self.line(format!("br label %{done}"));
        self.start(&done);
        self.op(format!("phi i64 [ 0, %{from} ], [ {n}, %{some} ]"))
    }

    /// The body of a Map equality function: the same length, then each pair of entries in key
    /// order, key then value, until one differs; `%no` is the function's false.
    pub(super) fn equal_entries(&mut self, t: Ty) {
        let (k, v) = self.map_types(t);
        let (bx, by) = (self.load("ptr", "%x"), self.load("ptr", "%y"));
        let (n, m) = (self.count(&bx), self.count(&by));
        let same = self.op(format!("icmp eq i64 {n}, {m}"));
        let i = self.name("%i");
        self.slot(&i, "i64");
        self.store("i64", "0", &i);
        let [head, inner, value, next, done] = ["b"; 5].map(|s| self.name(s));
        self.line(format!("br i1 {same}, label %{head}, label %no"));
        self.start(&head);
        let j = self.load("i64", &i);
        let more = self.op(format!("icmp ult i64 {j}, {n}"));
        self.line(format!("br i1 {more}, label %{inner}, label %{done}"));
        self.start(&inner);
        let (a, b) = (self.entry_at(t, &bx, &j), self.entry_at(t, &by, &j));
        for (ty, label, offset) in [(k, &value, 0), (v, &next, self.size(k))] {
            let (a, b) = (self.offset(&a, offset), self.offset(&b, offset));
            let (a, b) = (self.get(ty, &a), self.get(ty, &b));
            let eq = self.equal(ty, &a, &b);
            self.line(format!("br i1 {eq}, label %{label}, label %no"));
            self.start(label);
        }
        let step = self.op(format!("add i64 {j}, 1"));
        self.store("i64", &step, &i);
        self.line(format!("br label %{head}"));
        self.start(&done);
        self.line("ret i1 true".into());
    }

    /// The address of entry `i`'s key in the Map block `block` of type `t`; its value follows.
    fn entry_at(&mut self, t: Ty, block: &str, i: &str) -> String {
        let (k, v) = self.map_types(t);
        let size = self.size(k) + self.size(v);
        let at = self.op(format!("getelementptr [{size} x i8], ptr {block}, i64 {i}"));
        self.offset(&at, HEAD as usize)
    }

    /// The address of the value under `key`, or null.
    fn find(&mut self, t: Ty, m: &str, key: &str) -> String {
        let key = self.key_word(t, key);
        let out = self.name("%w");
        self.slot(&out, "ptr");
        self.map_call(FIND, t, m, &key, &out, None);
        self.load("ptr", &out)
    }

    /// Map row `row` that gives an address, a trap passed on from `site`.
    fn map_row(&mut self, row: u64, t: Ty, m: &str, key: &str, site: Site) -> String {
        let key = self.key_word(t, key);
        let out = self.name("%w");
        self.slot(&out, "ptr");
        self.map_call(row, t, m, &key, &out, Some(site));
        self.load("ptr", &out)
    }

    /// The word a key of the Map type `t` is passed as: an Int, or a String's address.
    fn key_word(&mut self, t: Ty, key: &str) -> String {
        match self.ty(self.map_types(t).0) {
            "ptr" => self.op(format!("ptrtoint ptr {key} to i64")),
            _ => key.into(),
        }
    }

    /// Map row `row` on the Map at `m` of type `t`, its result into `out`. With a site, a trap
    /// there is passed on; without, the row cannot trap.
    fn map_call(&mut self, row: u64, t: Ty, m: &str, key: &str, out: &str, site: Option<Site>) {
        let (k, v) = self.map_types(t);
        let string = self.module.core.types[k] == Type::String;
        let shape = self.size(v) as u64 | if string { STRING_KEY } else { 0 };
        let record = match self.module.specs[v] {
            Some(_) => self.module.record(v),
            None => "null".into(),
        };
        let at = match site {
            Some(site) => {
                let (text, len) = self.module.place(site);
                format!("ptr {text}, i64 {len}")
            }
            None => "ptr null, i64 0".into(),
        };
        self.declare(MAP);
        let status = self.op(format!(
            "call i32 @paka_rt1_map(i64 {row}, ptr {record}, i64 {}, ptr {m}, i64 {key}, ptr {out}, {at})",
            shape as i64
        ));
        if site.is_some() {
            self.pass(&status);
        }
    }
}
