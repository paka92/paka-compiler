//! `if`, `match` and `?`: the forms that decide between blocks.

use super::{Cx, builtin_tag};
use crate::ast::{self, ArmBody, PatKind};
use crate::core_form::{Arm, Bin, Block, Expr, ExprKind, Local, Place, Root, Stmt, Tag, Ty, Type};
use crate::resolve::Binding;
use crate::sig::{self, Item, Node};
use paka_rt::ops::Int;

impl<'a> Cx<'_, 'a> {
    pub(super) fn if_expr(
        &mut self,
        e: &ast::Expr<'a>,
        branches: &[(ast::Expr<'a>, ast::Block<'a>)],
        otherwise: Option<&ast::Block<'a>>,
    ) -> Expr {
        let valued = self.entry(e).is_some();
        let mut pairs = vec![];
        for (cond, block) in branches {
            let cond = self.expr(cond);
            pairs.push((cond, self.block(block, valued)));
        }
        let otherwise = otherwise.map(|b| self.block(b, valued));
        let ty = self.ty_of(e);
        Expr {
            ty,
            kind: if_chain(ty, valued, pairs, otherwise),
        }
    }

    pub(super) fn match_expr(
        &mut self,
        e: &ast::Expr<'a>,
        subject: &ast::Expr<'a>,
        arms: &[ast::Arm<'a>],
    ) -> Expr {
        let kind = self.entry(subject).map(|t| self.table.types.node(t));
        if matches!(
            kind,
            Some(Node::User(_) | Node::Name("Option" | "Result", _))
        ) {
            self.variant_match(e, subject, arms)
        } else {
            self.value_match(e, subject, arms)
        }
    }

    /// A match on a user enum, an Option or a Result: a core `match`.
    fn variant_match(
        &mut self,
        e: &ast::Expr<'a>,
        subject: &ast::Expr<'a>,
        arms: &[ast::Arm<'a>],
    ) -> Expr {
        let valued = self.entry(e).is_some();
        let on = self.entry(subject).expect("a matched value");
        let subject = self.expr(subject);
        let arms = arms.iter().map(|a| self.arm(on, a, valued)).collect();
        Expr {
            ty: self.ty_of(e),
            kind: ExprKind::Match(Box::new(subject), arms),
        }
    }

    fn arm(&mut self, on: sig::Ty, arm: &ast::Arm<'a>, valued: bool) -> Arm {
        let tags = arm.patterns.iter().map(|p| self.tag(p)).collect();
        let binds = match &arm.patterns[..] {
            [one] => self.binds(on, one),
            _ => vec![],
        };
        Arm {
            tags,
            binds,
            body: self.arm_body(&arm.body, valued),
        }
    }

    fn arm_body(&mut self, body: &ArmBody<'a>, valued: bool) -> Block {
        match body {
            ArmBody::Block(b) => self.block(b, valued),
            ArmBody::Line(s) => self.stmts(std::slice::from_ref(s), valued),
        }
    }

    fn tag(&self, p: &ast::Pat<'a>) -> Tag {
        match self.resolved.get(p.at).expect("a resolved variant") {
            Binding::Variant(d, v) => Tag::User(self.ids[d], v),
            Binding::Builtin(name) => builtin_tag(name),
            _ => unreachable!("a variant"),
        }
    }

    /// The local of each field a variant pattern names, in field order.
    fn binds(&mut self, on: sig::Ty, p: &ast::Pat<'a>) -> Vec<Option<Local>> {
        let PatKind::Variant(_, parts) = &p.kind else {
            unreachable!("a variant pattern");
        };
        let types = self.field_types(on, p);
        let bind = |(part, ty): (&ast::Pat<'a>, sig::Ty)| match part.kind {
            PatKind::Bind(_) => {
                let ty = self.ty(ty);
                Some(self.declare(part.at, ty))
            }
            _ => None,
        };
        parts.iter().zip(types).map(bind).collect()
    }

    /// The types of the fields of the variant `p` names, in a value of type `on`.
    fn field_types(&self, on: sig::Ty, p: &ast::Pat<'a>) -> Vec<sig::Ty> {
        let table = self.table;
        match self.resolved.get(p.at) {
            Some(Binding::Variant(d, v)) => match &table.defs[d].item {
                Item::Enum(variants) => variants[v].fields.iter().map(|f| f.ty).collect(),
                _ => unreachable!("an enum"),
            },
            Some(Binding::Builtin(name)) => {
                let Node::Name(_, args) = table.types.node(on) else {
                    unreachable!("an Option or a Result");
                };
                match name {
                    "Some" | "Ok" => vec![args[0]],
                    "Err" => vec![args[1]],
                    _ => vec![],
                }
            }
            _ => unreachable!("a variant"),
        }
    }

    /// A match on an Int, String or Bool: a `let` of the subject and a chain of `==` tests, in an
    /// `if true` that takes the match's type (docs/core.md section 7).
    fn value_match(
        &mut self,
        e: &ast::Expr<'a>,
        subject: &ast::Expr<'a>,
        arms: &[ast::Arm<'a>],
    ) -> Expr {
        let valued = self.entry(e).is_some();
        let subject = self.expr(subject);
        let ty = subject.ty.expect("a matched value");
        let held = self.local(ty);
        let (last, init) = arms.split_last().expect("a match has an arm");
        let mut branches = vec![];
        for arm in init {
            let cond = self.condition(held, ty, &arm.patterns);
            branches.push((cond, self.arm_body(&arm.body, valued)));
        }
        let otherwise = self.arm_body(&last.body, valued);
        let ty_match = self.ty_of(e);
        let mut block = if branches.is_empty() {
            otherwise
        } else {
            let kind = if_chain(ty_match, valued, branches, Some(otherwise));
            nested(valued, Expr { ty: ty_match, kind })
        };
        block.stmts.insert(0, Stmt::Let(held, subject));
        let cond = Expr {
            ty: Some(self.core.ty(Type::Bool)),
            kind: ExprKind::Bool(true),
        };
        Expr {
            ty: ty_match,
            kind: ExprKind::If(Box::new(cond), block, None),
        }
    }

    /// Whether the held subject equals one of the patterns; each `==` is at its pattern.
    fn condition(&mut self, held: Local, ty: Ty, patterns: &[ast::Pat<'a>]) -> Expr {
        let mut cond: Option<Expr> = None;
        for p in patterns {
            let right = self.literal(p, ty);
            let site = self.site(p.at);
            let left = read(held, ty);
            let eq = Expr {
                ty: Some(self.core.ty(Type::Bool)),
                kind: ExprKind::Binary(Bin::Eq(site), Box::new(left), Box::new(right)),
            };
            cond = Some(match cond {
                None => eq,
                Some(before) => Expr {
                    ty: eq.ty,
                    kind: ExprKind::Binary(Bin::Or, Box::new(before), Box::new(eq)),
                },
            });
        }
        cond.expect("a pattern")
    }

    fn literal(&mut self, p: &ast::Pat<'a>, ty: Ty) -> Expr {
        let kind = match &p.kind {
            PatKind::Int(text) => ExprKind::Int(Int::parse(text).expect("an Int in range")),
            PatKind::Str(text) => ExprKind::Str(text.clone()),
            PatKind::Bool(b) => ExprKind::Bool(*b),
            _ => unreachable!("a literal pattern"),
        };
        Expr { ty: Some(ty), kind }
    }

    /// `inner?`: a match whose `Ok` or `Some` arm gives the value and whose other arm returns
    /// the error or the `None` (docs/core.md section 7).
    pub(super) fn try_op(&mut self, e: &ast::Expr<'a>, inner: Expr) -> Expr {
        let ty = self.ty_of(e).expect("a value past `?`");
        let operand = inner.ty.expect("an Option or a Result");
        let error = match self.core.types[operand] {
            Type::Result(_, error) => Some(error),
            _ => None,
        };
        let (pass, fail, again) = match error {
            Some(_) => (Tag::Ok, Tag::Err, Tag::Err),
            None => (Tag::Some, Tag::None, Tag::None),
        };
        let value = self.local(ty);
        let error = error.map(|ty| (self.local(ty), ty));
        let early = Expr {
            ty: self.result,
            kind: ExprKind::Variant(again, error.iter().map(|&(x, ty)| read(x, ty)).collect()),
        };
        let arms = vec![
            Arm {
                tags: vec![pass],
                binds: vec![Some(value)],
                body: Block {
                    stmts: vec![],
                    value: Some(Box::new(read(value, ty))),
                },
            },
            Arm {
                tags: vec![fail],
                binds: error.iter().map(|&(x, _)| Some(x)).collect(),
                body: Block {
                    stmts: vec![Stmt::Return(Some(early))],
                    value: None,
                },
            },
        ];
        Expr {
            ty: Some(ty),
            kind: ExprKind::Match(Box::new(inner), arms),
        }
    }
}

/// A read of a whole local.
fn read(local: Local, ty: Ty) -> Expr {
    Expr {
        ty: Some(ty),
        kind: ExprKind::Place(Place {
            root: Root::Local(local),
            steps: vec![],
        }),
    }
}

/// An `if` chain: each branch is tried in order, and the last `else` is `otherwise`. `else if`
/// is an `if` inside the `else` block.
fn if_chain(
    ty: Option<Ty>,
    valued: bool,
    branches: Vec<(Expr, Block)>,
    otherwise: Option<Block>,
) -> ExprKind {
    let mut tail = otherwise;
    let mut kind = None;
    for (cond, then) in branches.into_iter().rev() {
        if let Some(inner) = kind.take() {
            tail = Some(nested(valued, Expr { ty, kind: inner }));
        }
        kind = Some(ExprKind::If(Box::new(cond), then, tail.take()));
    }
    kind.expect("an if has a branch")
}

/// A block holding only `e`.
fn nested(valued: bool, e: Expr) -> Block {
    if valued {
        Block {
            stmts: vec![],
            value: Some(Box::new(e)),
        }
    } else {
        Block {
            stmts: vec![Stmt::Expr(e)],
            value: None,
        }
    }
}
