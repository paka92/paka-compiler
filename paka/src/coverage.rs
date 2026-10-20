//! What a program uses of the language: the syntax node kinds in its tree and the names it writes.
//! The exit gate (tests/conformance/README.md) compares this with `NODE_KINDS` and the built-in
//! table.

use crate::ast::*;
use crate::builtins;
use std::collections::BTreeSet;

/// Gives each node enum a `kind()` that names its variant as `Enum::Variant`, and lists them all
/// in `$all`. The match has no wildcard: a variant missing here does not compile.
macro_rules! node_kinds {
    ($all:ident; $($ty:ident $(<$lt:lifetime>)?: $($variant:ident)+;)+) => {
        $(impl $ty $(<$lt>)? {
            fn kind(&self) -> &'static str {
                match self {
                    $(Self::$variant { .. } => concat!(stringify!($ty), "::", stringify!($variant)),)+
                }
            }
        })+

        pub const $all: &[&str] =
            &[$($(concat!(stringify!($ty), "::", stringify!($variant)),)+)+];
    };
}

node_kinds! {
    NODE_KINDS;
    Def<'_>: Type Enum Fn Example App View;
    Entry<'_>: State Start Update Grants View;
    Type<'_>: Name Fn;
    StmtKind<'_>: Let Discard Assign Return Break Continue For While Expr;
    ExprKind<'_>: Int Float Bool Str Hole Name Record List Map Unary Binary Range Index Field Try Call
        Inout If Match Children;
    StrPart<'_>: Text Expr;
    ArmBody<'_>: Block Line;
    PatKind<'_>: Wild Int Str Bool Bind Variant;
    AssignOp: Set Add Sub Mul Div Rem;
    UnOp: Neg Not;
    BinOp: Or And Eq Ne Lt Le Gt Ge Add Sub Mul Div Rem;
}

/// Node kinds that parse but no command accepts yet, so no passing case can hold them: views
/// are checked from M5-13.
pub const NOT_YET_RUN: &[&str] = &["Def::View", "Entry::View", "ExprKind::Children"];

/// Every distinct name of the built-in table, sorted.
pub fn builtin_names() -> Vec<&'static str> {
    let rows = builtins::ROWS.iter();
    let names: BTreeSet<_> = rows.map(|(_, row)| builtins::name(row)).collect();
    names.into_iter().collect()
}

/// The node kinds of a tree, and the names it writes: an upper-case name as a type, a name in an
/// expression or a pattern, a record name or a grant; a lower-case name only as a callee, since a
/// local name may hide a built-in function.
#[derive(Default)]
pub struct Uses {
    pub kinds: BTreeSet<&'static str>,
    pub names: BTreeSet<String>,
}

pub fn uses(program: &Program) -> Uses {
    let mut uses = Uses::default();
    program.defs.iter().for_each(|d| uses.def(d));
    uses
}

impl Uses {
    pub fn merge(&mut self, other: Uses) {
        self.kinds.extend(other.kinds);
        self.names.extend(other.names);
    }

    fn name(&mut self, name: &str) {
        if !self.names.contains(name) {
            self.names.insert(name.to_string());
        }
    }

    fn def(&mut self, def: &Def) {
        self.kinds.insert(def.kind());
        match def {
            Def::Type { fields, .. } => self.fields(fields),
            Def::Enum { variants, .. } => variants.iter().for_each(|v| self.fields(&v.fields)),
            Def::Fn {
                params,
                result,
                body,
                ..
            } => self.callable(params, result.as_ref(), body),
            Def::View {
                params,
                result,
                body,
                ..
            } => self.callable(params, Some(result), body),
            Def::Example { body, .. } => self.block(body),
            Def::App { entries, .. } => entries.iter().for_each(|e| self.entry(e)),
        }
    }

    fn callable(&mut self, params: &[Param], result: Option<&Type>, body: &Block) {
        params.iter().for_each(|p| self.ty(&p.ty));
        result.iter().for_each(|t| self.ty(t));
        self.block(body);
    }

    fn fields(&mut self, fields: &[Field]) {
        for field in fields {
            self.ty(&field.ty);
            field.default.iter().for_each(|e| self.expr(e));
        }
    }

    fn entry(&mut self, entry: &Entry) {
        self.kinds.insert(entry.kind());
        match entry {
            Entry::State(_, name) => self.name(name.text),
            Entry::Update(..) | Entry::View(..) => {}
            Entry::Start(_, expr) => self.expr(expr),
            Entry::Grants(_, grants) => grants.iter().for_each(|g| self.name(g.text)),
        }
    }

    fn ty(&mut self, ty: &Type) {
        self.kinds.insert(ty.kind());
        match ty {
            Type::Name { name, args } => {
                self.name(name.text);
                args.iter().for_each(|t| self.ty(t));
            }
            Type::Fn { params, result, .. } => {
                params.iter().for_each(|t| self.ty(t));
                result.iter().for_each(|t| self.ty(t));
            }
        }
    }

    fn block(&mut self, block: &Block) {
        block.stmts.iter().for_each(|s| self.stmt(s));
    }

    fn stmt(&mut self, stmt: &Stmt) {
        self.kinds.insert(stmt.kind.kind());
        match &stmt.kind {
            StmtKind::Let { ty, value, .. } => {
                ty.iter().for_each(|t| self.ty(t));
                self.expr(value);
            }
            StmtKind::Discard(value) | StmtKind::Expr(value) => self.expr(value),
            StmtKind::Assign { target, op, value } => {
                self.kinds.insert(op.kind());
                self.expr(target);
                self.expr(value);
            }
            StmtKind::Return(value) => value.iter().for_each(|e| self.expr(e)),
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::For { list, body, .. } => {
                self.expr(list);
                self.block(body);
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        self.kinds.insert(expr.kind.kind());
        match &expr.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Hole(_) => {}
            ExprKind::Str(parts) => {
                for part in parts {
                    self.kinds.insert(part.kind());
                    if let StrPart::Expr(e) = part {
                        self.expr(e);
                    }
                }
            }
            ExprKind::Name(name) if name.starts_with(char::is_uppercase) => self.name(name),
            ExprKind::Name(_) => {}
            ExprKind::Record(name, args) => {
                self.name(name);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::List(items) => items.iter().for_each(|e| self.expr(e)),
            ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            ExprKind::Unary(op, e) => {
                self.kinds.insert(op.kind());
                self.expr(e);
            }
            ExprKind::Field(e, _) | ExprKind::Try(e, _) | ExprKind::Inout(e) => self.expr(e),
            ExprKind::Binary(op, _, a, b) => {
                self.kinds.insert(op.kind());
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Range(a, b) | ExprKind::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee);
                if let ExprKind::Name(name) = &callee.kind {
                    self.name(name);
                }
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::If {
                branches,
                otherwise,
            } => {
                for (cond, block) in branches {
                    self.expr(cond);
                    self.block(block);
                }
                otherwise.iter().for_each(|b| self.block(b));
            }
            ExprKind::Match { subject, arms } => {
                self.expr(subject);
                arms.iter().for_each(|a| self.arm(a));
            }
            ExprKind::Children(call, children) => {
                self.expr(call);
                self.block(children);
            }
        }
    }

    fn arm(&mut self, arm: &Arm) {
        arm.patterns.iter().for_each(|p| self.pat(p));
        self.kinds.insert(arm.body.kind());
        match &arm.body {
            ArmBody::Block(block) => self.block(block),
            ArmBody::Line(stmt) => self.stmt(stmt),
        }
    }

    fn pat(&mut self, pat: &Pat) {
        self.kinds.insert(pat.kind.kind());
        if let PatKind::Variant(name, subs) = &pat.kind {
            self.name(name);
            subs.iter().for_each(|p| self.pat(p));
        }
    }
}

/// The node kinds of a core-form program (M4-21), effects and grants aside.
pub mod core {
    use crate::core_form::*;
    use std::collections::BTreeSet;

    node_kinds! {
        CORE_KINDS;
        Type: Int Float Bool String List Map Option Result Effect Fn Record Enum;
        Stmt: Let Assign Return Break Continue For While Expr;
        Walk: List Range;
        ExprKind: Int Float Bool Str Interp Hole Place Record List Map Variant FnValue Unary Binary
            Call If Match;
        Root: Local Value;
        Step: Field Item Key Slice;
        Tag: User Some None Ok Err Effect;
        FnRef: Fn Variant;
        Callee: Fn Builtin Value;
        Arg: Value Inout;
        Un: Neg NegFloat Not;
        Bin: Int Float Join Eq Ne Order And Or;
        Arith: Add Sub Mul Div Rem;
        FloatArith: Add Sub Mul Div;
        Compound: Int Float Join;
        Cmp: Lt Le Gt Ge;
    }

    #[derive(Default)]
    pub struct Kinds(pub BTreeSet<&'static str>);

    pub fn kinds(program: &Program) -> Kinds {
        let mut k = Kinds::default();
        program.types.iter().for_each(|t| k.add(t.kind()));
        let bodies = program.fns.iter().chain(&program.examples);
        bodies.for_each(|f| k.block(&f.body));
        if let Some(state) = program.app.as_ref().and_then(|a| a.state.as_ref()) {
            k.expr(state);
        }
        k
    }

    impl Kinds {
        fn add(&mut self, kind: &'static str) {
            self.0.insert(kind);
        }

        fn block(&mut self, block: &Block) {
            block.stmts.iter().for_each(|s| self.stmt(s));
            block.value.iter().for_each(|e| self.expr(e));
        }

        fn stmt(&mut self, stmt: &Stmt) {
            self.add(stmt.kind());
            match stmt {
                Stmt::Let(_, e) | Stmt::Expr(e) => self.expr(e),
                Stmt::Assign(place, op, e) => {
                    self.place(place);
                    op.iter().for_each(|op| self.compound(op));
                    self.expr(e);
                }
                Stmt::Return(e) => e.iter().for_each(|e| self.expr(e)),
                Stmt::Break | Stmt::Continue => {}
                Stmt::For(_, walk, body) => {
                    self.add(walk.kind());
                    match walk {
                        Walk::List(e) => self.expr(e),
                        Walk::Range(a, b) => {
                            self.expr(a);
                            self.expr(b);
                        }
                    }
                    self.block(body);
                }
                Stmt::While(cond, body) => {
                    self.expr(cond);
                    self.block(body);
                }
            }
        }

        fn compound(&mut self, op: &Compound) {
            self.add(op.kind());
            match op {
                Compound::Int(a, _) => self.add(a.kind()),
                Compound::Float(a) => self.add(a.kind()),
                Compound::Join(_) => {}
            }
        }

        fn place(&mut self, place: &Place) {
            self.add(place.root.kind());
            if let Root::Value(e) = &place.root {
                self.expr(e);
            }
            for step in &place.steps {
                self.add(step.kind());
                match step {
                    Step::Field(_) => {}
                    Step::Item(e, _) | Step::Key(e, _) => self.expr(e),
                    Step::Slice(a, b, _) => {
                        self.expr(a);
                        self.expr(b);
                    }
                }
            }
        }

        fn expr(&mut self, expr: &Expr) {
            self.add(expr.kind.kind());
            match &expr.kind {
                ExprKind::Int(_)
                | ExprKind::Float(_)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Hole(_) => {}
                ExprKind::Interp(parts, _) | ExprKind::List(parts) | ExprKind::Record(_, parts) => {
                    parts.iter().for_each(|e| self.expr(e))
                }
                ExprKind::Place(place) => self.place(place),
                ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
                    self.expr(k);
                    self.expr(v);
                }),
                ExprKind::Variant(tag, args) => {
                    self.add(tag.kind());
                    args.iter().for_each(|e| self.expr(e));
                }
                ExprKind::FnValue(f) => self.add(f.kind()),
                ExprKind::Unary(op, e) => {
                    self.add(op.kind());
                    self.expr(e);
                }
                ExprKind::Binary(op, a, b) => {
                    self.add(op.kind());
                    match op {
                        Bin::Int(a, _) => self.add(a.kind()),
                        Bin::Float(a) => self.add(a.kind()),
                        Bin::Order(c) => self.add(c.kind()),
                        _ => {}
                    }
                    self.expr(a);
                    self.expr(b);
                }
                ExprKind::Call(callee, args, _) => {
                    self.add(callee.kind());
                    if let Callee::Value(e) = callee {
                        self.expr(e);
                    }
                    for arg in args {
                        self.add(arg.kind());
                        match arg {
                            Arg::Value(e) => self.expr(e),
                            Arg::Inout(p) => self.place(p),
                        }
                    }
                }
                ExprKind::If(cond, then, otherwise) => {
                    self.expr(cond);
                    self.block(then);
                    otherwise.iter().for_each(|b| self.block(b));
                }
                ExprKind::Match(subject, arms) => {
                    self.expr(subject);
                    for arm in arms {
                        arm.tags.iter().for_each(|t| self.add(t.kind()));
                        self.block(&arm.body);
                    }
                }
            }
        }
    }
}
