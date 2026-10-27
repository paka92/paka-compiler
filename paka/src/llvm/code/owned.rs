//! Moves, copies and drops, as the dev back end makes them (docs/decisions/M4-05.md section 9): a
//! fresh value is a temporary its statement owns until a binding, a field or a result takes it.
//! Taking a temporary moves it; taking anything else copies it.

use super::{Body, Module, Value};
use crate::core_form::{
    Arg, Block, Callee, Expr, ExprKind, Local, Place, Root, Step, Stmt, Ty, Walk,
};
use paka::owners::Owner;
use paka_rt::abi::dev::records::{BOX, LIST, MAP, Part, STRING, TAGGED, VALUE};
use paka_rt::abi::table::{COPY, DROP};

type Dropped = Vec<(Ty, Owner<usize>)>;

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
        let alternatives: Vec<Vec<_>> = (spec.alternatives.iter())
            .map(|alt| {
                (alt.iter())
                    .map(|(offset, part)| match *part {
                        Part::String => (*offset, STRING, None, 0),
                        Part::List { item, size } => (*offset, LIST, Some(item), size),
                        Part::Map { value, shape } => (*offset, MAP, Some(value), shape),
                        Part::Value(t) => (*offset, VALUE, Some(t), 0),
                        Part::Box { item, size } => (*offset, BOX, Some(item), size),
                    })
                    .collect()
            })
            .collect();
        let mut words = vec![size.to_string(), "0".into(), "0".into()];
        if tagged {
            words.push((TAGGED | alternatives.len() as u64).to_string());
        }
        for alt in alternatives {
            words.push(alt.len().to_string());
            for (offset, kind, of, b) in alt {
                let of = match of.filter(|&u| self.specs[u].is_some()) {
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
            self.declare(DROP);
            let record = self.module.record(t);
            let copy = self.fresh(t);
            self.put(t, at, &copy);
            self.line(format!(
                "call void @paka_rt1_drop(ptr {record}, ptr {copy})"
            ));
        }
    }

    fn drop_all(&mut self, dropped: Dropped) {
        for (t, owner) in dropped {
            let at = match owner {
                Owner::Local(l) => self.at[l].clone(),
                Owner::Temp(i) => self.temps[i].clone(),
            };
            self.drop_at(t, &at);
        }
    }

    pub(super) fn statements(&mut self, stmts: &[Stmt]) -> Option<()> {
        stmts.iter().try_for_each(|s| {
            let mark = self.owners.mark();
            self.stmt(s)?;
            self.drop_temps(mark);
            Some(())
        })
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
        self.statements(&block.stmts)?;
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
        self.statements(&block.stmts)?;
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

/// The local `e` reads whole, if it does.
fn whole_local(e: &Expr) -> Option<Local> {
    match &e.kind {
        ExprKind::Place(p) if p.steps.is_empty() => match p.root {
            Root::Local(l) => Some(l),
            Root::Value(_) => None,
        },
        _ => None,
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
