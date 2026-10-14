//! What a program uses of the language: the syntax node kinds in its tree and the names it writes.
//! The exit gate (tests/conformance/README.md) compares this with `NODE_KINDS` and the built-in
//! table. Operators are token classes, not nodes, and have no kind.

use crate::ast::*;
use crate::builtins;
use std::collections::BTreeSet;

/// Gives each node enum a `kind()` that names its variant as `Enum::Variant`, and lists them all
/// in `NODE_KINDS`. The match has no wildcard: a variant missing here does not compile.
macro_rules! node_kinds {
    ($($ty:ident: $($variant:ident)+;)+) => {
        $(impl $ty<'_> {
            fn kind(&self) -> &'static str {
                match self {
                    $(Self::$variant { .. } => concat!(stringify!($ty), "::", stringify!($variant)),)+
                }
            }
        })+

        pub const NODE_KINDS: &[&str] =
            &[$($(concat!(stringify!($ty), "::", stringify!($variant)),)+)+];
    };
}

node_kinds! {
    Def: Type Enum Fn Example App;
    Entry: State Start Update Grants;
    Type: Name Fn;
    StmtKind: Let Discard Assign Return Break Continue For While Expr;
    ExprKind: Int Float Bool Str Hole Name Record List Map Unary Binary Range Index Field Try Call
        Inout If Match;
    StrPart: Text Expr;
    ArmBody: Block Line;
    PatKind: Wild Int Str Bool Bind Variant;
}

/// Every distinct name of the built-in table, sorted.
pub fn builtin_names() -> Vec<&'static str> {
    let rows = builtins::ROWS.iter();
    let names: BTreeSet<_> = rows.map(|(_, row)| builtins::name(row)).collect();
    names.into_iter().collect()
}

/// The node kinds of a tree, and the names it writes as a type, a name in an expression or a
/// pattern, a record name or a grant.
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
            } => {
                params.iter().for_each(|p| self.ty(&p.ty));
                result.iter().for_each(|t| self.ty(t));
                self.block(body);
            }
            Def::Example { body, .. } => self.block(body),
            Def::App { entries, .. } => entries.iter().for_each(|e| self.entry(e)),
        }
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
            Entry::State(_, name) | Entry::Update(_, name) => self.name(name.text),
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
            StmtKind::Assign { target, value, .. } => {
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
            ExprKind::Name(name) => self.name(name),
            ExprKind::Record(name, args) => {
                self.name(name);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::List(items) => items.iter().for_each(|e| self.expr(e)),
            ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            ExprKind::Unary(_, e)
            | ExprKind::Field(e, _)
            | ExprKind::Try(e, _)
            | ExprKind::Inout(e) => self.expr(e),
            ExprKind::Binary(_, _, a, b) | ExprKind::Range(a, b) | ExprKind::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee);
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
