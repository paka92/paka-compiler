//! Moves, copies and drops, as the dev back end makes them (docs/decisions/M4-05.md section 9): a
//! fresh value is a temporary its statement owns until a binding, a field or a result takes it.
//! Taking a temporary moves it; taking anything else copies it.

use super::{Body, Module, Value};
use crate::core_form::{
    Arg, Block, Callee, Expr, ExprKind, Local, Place, Root, Step, Stmt, Ty, Type, Walk,
};
use paka::owners::{Owner, whole_local};
use paka_rt::abi::dev::records::{BOX, LIST, MAP, Part, STRING, TAGGED, VALUE};
use paka_rt::abi::table::{COPY, DROP, FREE, LIST as LIST_WORDS, STRING as STRING_WORDS};

type Dropped = Vec<(Ty, Owner<usize>)>;
type Entry = (u64, u64, Option<Ty>, u64);

impl Module<'_> {
    /// The constant `@r<t>` holding type `t`'s record (M4-05 section 10).
    pub(super) fn record(&mut self, t: Ty) -> String {
        let name = format!("@r{t}");
        if self.records.contains_key(&t) {
            return name;
        }
        self.records.insert(t, String::new());
        let spec = self.specs[t].as_ref().expect("a type that owns");
        let (size, tagged) = (spec.size, spec.tagged);
        let alternatives = self.entries(t);
        let mut words = vec![size.to_string(), "0".into(), "0".into()];
        if tagged {
            words.push((TAGGED | alternatives.len() as u64).to_string());
        }
        for alt in alternatives {
            words.push(alt.len().to_string());
            for (offset, kind, of, b) in alt {
                let of = match of {
                    Some(u) => format!("ptrtoint (ptr {} to i64)", self.record(u)),
                    None => "0".into(),
                };
                words.extend([offset.to_string(), kind.to_string(), of, b.to_string()]);
            }
        }
        let words: Vec<String> = words.iter().map(|w| format!("i64 {w}")).collect();
        let text = format!(
            "{name} = private constant [{} x i64] [{}]\n",
            words.len(),
            words.join(", ")
        );
        self.records.insert(t, text);
        name
    }

    /// Record `t`'s entries per alternative: offset, kind, the record they name when it owns
    /// anything, and the kind's last word.
    fn entries(&self, t: Ty) -> Vec<Vec<Entry>> {
        let spec = self.specs[t].as_ref().expect("a type that owns");
        let entry = |(offset, part): &(u64, Part)| {
            let (kind, of, b) = match *part {
                Part::String => (STRING, None, 0),
                Part::List { item, size } => (LIST, Some(item), size),
                Part::Map { value, shape } => (MAP, Some(value), shape),
                Part::Value(t) => (VALUE, Some(t), 0),
                Part::Box { item, size } => (BOX, Some(item), size),
            };
            (*offset, kind, of.filter(|&u| self.specs[u].is_some()), b)
        };
        (spec.alternatives.iter())
            .map(|alt| alt.iter().map(entry).collect())
            .collect()
    }

    /// `@d<t>`, which frees what a value of record `t` owns; lowered once (`Body::dropper`).
    pub(super) fn drop_fn(&mut self, t: Ty) -> String {
        if self.dropped.insert(t) {
            self.drops.push(t);
        }
        format!("@d{t}")
    }
}

impl Body<'_, '_> {
    /// `v`, fresh, now owned by the statement.
    pub(super) fn temp(&mut self, t: Ty, v: String) -> String {
        if self.module.layouts.owns(t) {
            self.temps.push(v.clone());
            self.owners.own(t, Owner::Temp(self.temps.len() - 1));
        }
        v
    }

    /// `v` for a new owner: a temporary moves, anything else is copied.
    pub(super) fn owned(&mut self, t: Ty, v: &str) -> String {
        if !self.module.layouts.owns(t) {
            return v.into();
        }
        let temp = self.temps.iter().rposition(|x| x == v);
        if temp.is_some_and(|i| self.owners.disown(i)) {
            return v.into();
        }
        self.copied(t, v)
    }

    /// A copy of `v` that owns blocks of its own; `v` itself for a type that owns nothing.
    pub(super) fn copied(&mut self, t: Ty, v: &str) -> String {
        if !self.module.layouts.owns(t) {
            return v.into();
        }
        let to = self.fresh(t);
        self.copy_into(t, v, &to);
        to
    }

    /// Writes a copy of `v` of type `t` at `at`.
    pub(super) fn copy_into(&mut self, t: Ty, v: &str, at: &str) {
        if !self.module.layouts.owns(t) {
            return self.put(t, v, at);
        }
        self.declare(COPY);
        let record = self.module.record(t);
        let status = self.op(format!(
            "call i32 @paka_rt1_copy(ptr {record}, ptr {v}, ptr {at})"
        ));
        let refused = self.op(format!("icmp ne i32 {status}, 0"));
        self.check(&refused, "memory", self.site);
    }

    /// `v` owned where drops come before it is written: words held in memory that own nothing
    /// are copied first, as they may lie in a value dropped.
    pub(super) fn kept(&mut self, t: Ty, v: &str) -> String {
        if self.module.layouts.owns(t) || self.ty(t) != "ptr" {
            return self.owned(t, v);
        }
        let to = self.fresh(t);
        self.put(t, v, &to);
        to
    }

    pub(super) fn fresh(&mut self, t: Ty) -> String {
        let to = self.name("%c");
        self.hold(&to, t);
        to
    }

    /// Local `l`'s value at a last use, a temporary from now on, so a new owner takes it whole.
    pub(super) fn moved_out(&mut self, l: Local) -> String {
        let t = self.locals[l].expect("a local that owns");
        let to = self.fresh(t);
        let at = self.at[l].clone();
        self.put(t, &at, &to);
        self.temp(t, to)
    }

    /// Binds local `l`, which then owns `v`.
    pub(super) fn bind_owned(&mut self, l: Local, v: &str) -> Option<()> {
        let t = self.locals[l]?;
        let v = self.owned(t, v);
        let at = self.at[l].clone();
        self.put(t, &v, &at);
        if self.module.layouts.owns(t) {
            self.owners.own(t, Owner::Local(l));
        }
        Some(())
    }

    /// Binds local `l` to `v` without owning it: a pattern name that borrows from its subject.
    pub(super) fn bind_shared(&mut self, l: Local, v: &str) -> Option<()> {
        let t = self.locals[l]?;
        let at = self.at[l].clone();
        self.put(t, v, &at);
        Some(())
    }

    /// Frees what the value of type `t` at `at` owns. It drops a copy, so `at` does not escape
    /// and the optimizer keeps its words as known values, such as a list's length in a loop.
    pub(super) fn drop_at(&mut self, t: Ty, at: &str) {
        if self.module.layouts.owns(t) {
            let drop = self.module.drop_fn(t);
            let copy = self.fresh(t);
            self.put(t, at, &copy);
            self.line(format!("call void {drop}(ptr {copy})"));
        }
    }

    /// `void @d<t>(ptr %v)`: frees each block a value of record `t` owns, a part's own blocks
    /// before the part's, as `rt_drop` does. A value holding a Map, or one met below the stack
    /// limit (M6-16), goes to `rt_drop`, whose walk takes no stack, so a drop never traps.
    pub(super) fn dropper(mut self, t: Ty) -> String {
        let alternatives = self.module.entries(t);
        let tagged = self.module.specs[t].as_ref().is_some_and(|s| s.tagged);
        self.declare(DROP);
        let walk = format!(
            "call void @paka_rt1_drop(ptr {}, ptr %v)\n  ret void",
            self.module.record(t)
        );
        if alternatives.iter().flatten().any(|e| e.1 == MAP) {
            return format!("\ndefine internal void @d{t}(ptr %v) {{\nentry:\n  {walk}\n}}\n");
        }
        let labels: Vec<Option<String>> = (alternatives.iter())
            .map(|alt| (!alt.is_empty()).then(|| self.name("b")))
            .collect();
        if tagged {
            let tag = self.load("i64", "%v");
            let cases: Vec<String> = (labels.iter().enumerate())
                .filter_map(|(i, l)| Some(format!("i64 {i}, label %{}", l.as_ref()?)))
                .collect();
            self.line(format!(
                "switch i64 {tag}, label %done [ {} ]",
                cases.join(" ")
            ));
        } else {
            self.line(format!("br label %{}", labels[0].as_ref().expect("a part")));
        }
        for (alt, label) in alternatives.iter().zip(&labels) {
            let Some(label) = label else { continue };
            self.start(label);
            alt.iter().for_each(|e| self.drop_part(e));
            self.line("br label %done".into());
        }
        format!(
            "\ndefine internal void @d{t}(ptr %v) {{\nentry:\n  %mark = alloca i8, align 1\n{}  \
             %mark.at = ptrtoint ptr %mark to i64\n  \
             %limit = load i64, ptr @paka_rt1_stack_limit, align 8\n  \
             %low = icmp ult i64 %mark.at, %limit\n  \
             br i1 %low, label %walk, label %body\nwalk:\n  {walk}\nbody:\n{}done:\n  ret void\n}}\n",
            self.slots, self.lines
        )
    }

    /// Frees what the part `entry` of the value at `%v` owns.
    fn drop_part(&mut self, &(offset, kind, of, b): &Entry) {
        let at = self.offset("%v", offset as usize);
        match kind {
            STRING | LIST => {
                let words = if kind == STRING {
                    STRING_WORDS
                } else {
                    LIST_WORDS
                };
                let cap_at = self.offset(&at, words.cap);
                let cap = self.load("i64", &cap_at);
                let next = self.when(&format!("icmp ne i64 {cap}, 0"));
                let ptr_at = self.offset(&at, words.ptr);
                let block = self.load("ptr", &ptr_at);
                let mut bytes = cap.clone();
                if kind == LIST {
                    if let Some(item) = of {
                        let len_at = self.offset(&at, words.len);
                        let len = self.load("i64", &len_at);
                        self.drop_items(item, &block, &len, b);
                    }
                    bytes = self.op(format!("mul i64 {cap}, {b}"));
                }
                self.free(&block, &bytes);
                self.close(&next);
            }
            VALUE => {
                if let Some(t) = of {
                    let drop = self.module.drop_fn(t);
                    self.line(format!("call void {drop}(ptr {at})"));
                }
            }
            _ => {
                let block = self.load("ptr", &at);
                let next = self.when(&format!("icmp ne ptr {block}, null"));
                if let Some(t) = of {
                    let drop = self.module.drop_fn(t);
                    self.line(format!("call void {drop}(ptr {block})"));
                }
                self.free(&block, &b.to_string());
                self.close(&next);
            }
        }
    }

    /// Drops each of the `len` items of record `item`, `stride` bytes apart, from `block`.
    fn drop_items(&mut self, item: Ty, block: &str, len: &str, stride: u64) {
        let drop = self.module.drop_fn(item);
        let from = self.current();
        let [head, body, exit] = ["b"; 3].map(|s| self.name(s));
        let [i, n] = ["%t"; 2].map(|s| self.name(s));
        self.line(format!("br label %{head}"));
        self.start(&head);
        self.line(format!("{i} = phi i64 [ 0, %{from} ], [ {n}, %{body} ]"));
        let more = self.op(format!("icmp ult i64 {i}, {len}"));
        self.line(format!("br i1 {more}, label %{body}, label %{exit}"));
        self.start(&body);
        let skip = self.op(format!("mul i64 {i}, {stride}"));
        let at = self.op(format!("getelementptr i8, ptr {block}, i64 {skip}"));
        self.line(format!("call void {drop}(ptr {at})"));
        self.line(format!("{n} = add i64 {i}, 1"));
        self.line(format!("br label %{head}"));
        self.start(&exit);
    }

    /// Branches on `test`; what follows runs when it holds, until `close` with the label returned.
    fn when(&mut self, test: &str) -> String {
        let holds = self.op(test.into());
        let [yes, next] = ["b"; 2].map(|s| self.name(s));
        self.line(format!("br i1 {holds}, label %{yes}, label %{next}"));
        self.start(&yes);
        next
    }

    fn close(&mut self, next: &str) {
        self.line(format!("br label %{next}"));
        self.start(next);
    }

    fn free(&mut self, block: &str, bytes: &str) {
        self.declare(FREE);
        self.line(format!(
            "call void @paka_rt1_free(ptr {block}, i64 {bytes})"
        ));
    }

    pub(super) fn drop_all(&mut self, dropped: Dropped) {
        for (t, owner) in dropped {
            let at = match owner {
                Owner::Local(l) => self.at[l].clone(),
                Owner::Temp(i) if self.literals.contains(&i) => continue,
                Owner::Temp(i) => self.temps[i].clone(),
            };
            self.drop_at(t, &at);
        }
    }

    /// Runs `block`'s statements, each dropping the temporaries it made.
    pub(super) fn statements(&mut self, block: &Block) -> Option<()> {
        block.stmts.iter().try_for_each(|s| {
            let mark = self.owners.mark();
            match s {
                Stmt::Let(l, e) if self.borrows(block, *l, e) => {
                    let v = self.expr(e)??;
                    self.bind_shared(*l, &v)?;
                }
                _ => self.stmt(s)?,
            }
            self.drop_temps(mark);
            Some(())
        })
    }

    /// Whether `let l = e` in `block` may borrow, owning nothing: nothing writes `l`, and `e` is a
    /// field or list item of a read-only parameter, which stays as it is through the call.
    fn borrows(&self, block: &Block, l: Local, e: &Expr) -> bool {
        let ExprKind::Place(p) = &e.kind else {
            return false;
        };
        let Root::Local(r) = p.root else { return false };
        let Some(mut t) = self.locals[r].filter(|_| self.read_only.contains(&r)) else {
            return false;
        };
        for step in &p.steps {
            t = match (step, &self.module.core.types[t]) {
                (Step::Field(i), _) => self.module.layouts.get(t).alternatives[0][*i].ty,
                (Step::Item(..), Type::List(item)) => *item,
                _ => return false,
            };
        }
        !writes(block, l)
    }

    pub(super) fn drop_temps(&mut self, mark: usize) {
        let temps = self.owners.temps_since(mark);
        self.drop_all(temps);
    }

    /// A block in its own scope, after binding each of `binds`: its value, owned by no one.
    pub(super) fn scoped(&mut self, binds: &[(Local, String)], block: &Block) -> Option<Value> {
        self.owners.enter(false);
        for (l, v) in binds {
            self.bind_owned(*l, v)?;
        }
        self.statements(block)?;
        let v = match &block.value {
            Some(e) => self.expr(e)?.map(|v| self.yielded(e, &v)),
            None => None,
        };
        let dropped = self.owners.leave();
        self.drop_all(dropped);
        Some(v)
    }

    /// A loop body in its own scope, which `break` and `continue` leave.
    pub(super) fn loop_body(&mut self, block: &Block) -> Option<()> {
        self.owners.enter(true);
        self.statements(block)?;
        let dropped = self.owners.leave();
        self.drop_all(dropped);
        Some(())
    }

    /// Before a `break` or `continue`: drops the loop body's owners.
    pub(super) fn leave_loop(&mut self) {
        let dropped = self.owners.to_loop();
        self.drop_all(dropped);
    }

    /// Before leaving the function: drops every owner but `kept`.
    pub(super) fn leave_fn(&mut self, kept: Option<Local>) {
        let dropped = self.owners.to_fn(kept);
        self.drop_all(dropped);
    }

    /// The value of a block's last expression: its own local moves out, anything else is owned.
    fn yielded(&mut self, e: &Expr, v: &str) -> String {
        let t = e.ty.expect("a value has a type");
        match whole_local(e) {
            Some(l) if self.module.layouts.owns(t) && self.owners.disown_local(l) => v.into(),
            _ => self.kept(t, v),
        }
    }

    /// Writes `e`'s value `v` as the result, drops every owner but the local it returns, and
    /// returns status 0.
    pub(super) fn give(&mut self, e: Option<&Expr>, v: Value) {
        let kept = e.and_then(whole_local).filter(|&l| self.owners.holds(l));
        let result = self.result.zip(v).map(|(t, v)| match kept {
            Some(_) => (t, v),
            None => (t, self.kept(t, &v)),
        });
        self.leave_fn(kept);
        if let Some((t, v)) = result {
            self.put(t, &v, "%out");
        }
        self.ended("ret i32 0".into());
    }
}

/// Whether `block` writes, by assignment or `&`, a place rooted at `root` anywhere inside it.
pub(super) fn writes(block: &Block, root: Local) -> bool {
    let stmt = |s: &Stmt| match s {
        Stmt::Assign(p, _, e) => written(p, root) || expr_writes(e, root),
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => expr_writes(e, root),
        Stmt::For(_, Walk::List(e), b) | Stmt::While(e, b) => {
            expr_writes(e, root) || writes(b, root)
        }
        Stmt::For(_, Walk::Range(x, y), b) => {
            expr_writes(x, root) || expr_writes(y, root) || writes(b, root)
        }
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => false,
    };
    block.stmts.iter().any(stmt) || block.value.as_ref().is_some_and(|e| expr_writes(e, root))
}

fn written(p: &Place, root: Local) -> bool {
    p.root == Root::Local(root) || place_writes(p, root)
}

fn place_writes(p: &Place, root: Local) -> bool {
    let in_root = matches!(&p.root, Root::Value(e) if expr_writes(e, root));
    in_root
        || p.steps.iter().any(|s| match s {
            Step::Field(_) => false,
            Step::Item(e, _) | Step::Key(e, _) => expr_writes(e, root),
            Step::Slice(a, b, _) => expr_writes(a, root) || expr_writes(b, root),
        })
}

fn expr_writes(e: &Expr, root: Local) -> bool {
    let any = |es: &[Expr]| es.iter().any(|e| expr_writes(e, root));
    match &e.kind {
        ExprKind::Interp(es, _)
        | ExprKind::Record(_, es)
        | ExprKind::List(es)
        | ExprKind::Variant(_, es) => any(es),
        ExprKind::Map(entries) => {
            (entries.iter()).any(|(k, v)| expr_writes(k, root) || expr_writes(v, root))
        }
        ExprKind::Place(p) => place_writes(p, root),
        ExprKind::Unary(_, x) => expr_writes(x, root),
        ExprKind::Binary(_, x, y) => expr_writes(x, root) || expr_writes(y, root),
        ExprKind::Call(callee, args, _) => {
            let callee = matches!(callee, Callee::Value(f) if expr_writes(f, root));
            callee
                || args.iter().any(|a| match a {
                    Arg::Value(e) => expr_writes(e, root),
                    Arg::Inout(p) => written(p, root),
                })
        }
        ExprKind::If(c, then, otherwise) => {
            expr_writes(c, root)
                || writes(then, root)
                || otherwise.as_ref().is_some_and(|b| writes(b, root))
        }
        ExprKind::Match(subject, arms) => {
            expr_writes(subject, root) || arms.iter().any(|a| writes(&a.body, root))
        }
        ExprKind::Root(b) => writes(b, root),
        ExprKind::Element(_, es, b, _) => any(es) || b.as_ref().is_some_and(|b| writes(b, root)),
        ExprKind::Child(c) => {
            c.key.as_ref().is_some_and(|k| expr_writes(k, root)) || expr_writes(&c.node, root)
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Hole(_)
        | ExprKind::FnValue(_)
        | ExprKind::Placeholder(_) => false,
    }
}
