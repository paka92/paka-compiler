//! Lowering a checked M1 program into the core form, and the view of it the M1 back ends run.

use crate::ast;
use crate::check::Checked;
use crate::core_form::{self, Program};

/// What the M1 back ends run: the effects of an `update` whose body is a constant list.
#[derive(Debug, PartialEq)]
pub struct M1 {
    pub app: String,
    /// The start variant's position in the message enum.
    pub start: usize,
    pub console: bool,
    /// The Paka name of the update function.
    pub update_name: String,
    /// The effects `update` returns, in order.
    pub update: Vec<Effect>,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Print(String),
    Exit,
}

/// Runs on a checked program, where every lookup succeeds and every node is of the M1 subset.
pub fn lower(path: &str, program: &ast::Program, checked: &Checked) -> Program {
    let _ = (path, program, checked);
    todo!()
}

/// The M1 view of `core`, or `None` when it holds what the M1 back ends cannot run.
pub fn m1(core: &Program) -> Option<M1> {
    let _ = core;
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_form::{
        App, Block, Enum, Expr, ExprKind, Fn, Grant, Param, Stmt, Tag, TrapSite, Type, Variant,
    };
    use crate::{check::check, parser::parse};
    use std::collections::BTreeSet;

    fn light() -> Program {
        let program = parse("main.paka", include_bytes!("../../examples/light.paka")).unwrap();
        let checked = check("main.paka", &program).unwrap();
        lower("main.paka", &program, &checked)
    }

    /// An effect of `Effect<Msg>`, type 1 in light.paka.
    fn effect(effect: core_form::Effect, fields: Vec<Expr>) -> Expr {
        let kind = ExprKind::Variant(Tag::Effect(effect), fields);
        Expr { ty: Some(1), kind }
    }

    #[test]
    fn light_paka_lowers() {
        let text = Expr {
            ty: Some(3),
            kind: ExprKind::Str("let there be light".into()),
        };
        let effects = vec![
            effect(core_form::Effect::Print, vec![text]),
            effect(core_form::Effect::Exit, vec![]),
        ];
        let list = Expr {
            ty: Some(2),
            kind: ExprKind::List(effects),
        };
        let update = Fn {
            name: "update".into(),
            site: 0,
            params: vec![Param {
                ty: 0,
                inout: false,
            }],
            result: Some(2),
            body: Block {
                stmts: vec![],
                value: Some(Box::new(list)),
            },
        };
        let start = Variant {
            name: "Start".into(),
            fields: vec![],
        };
        let by_hand = Program {
            file: "main.paka".into(),
            types: vec![Type::Enum(0), Type::Effect(0), Type::List(1), Type::String],
            records: vec![],
            enums: vec![Enum {
                name: "Msg".into(),
                variants: vec![start],
            }],
            fns: vec![update],
            examples: vec![],
            app: Some(App {
                name: "light".into(),
                state: None,
                start: 0,
                update: 0,
                grants: BTreeSet::from([Grant::Console]),
            }),
            sites: vec![TrapSite {
                line: 3,
                col: 4,
                hole: None,
            }],
        };
        assert_eq!(light(), by_hand);
    }

    #[test]
    fn light_paka_reaches_the_m1_back_ends_as_before() {
        let by_hand = M1 {
            app: "light".into(),
            start: 0,
            console: true,
            update_name: "update".into(),
            update: vec![Effect::Print("let there be light".into()), Effect::Exit],
        };
        assert_eq!(m1(&light()), Some(by_hand));
    }

    #[test]
    fn m1_refuses_what_its_back_ends_cannot_run() {
        let mut state = light();
        let empty = Expr {
            ty: None,
            kind: ExprKind::Record(0, vec![]),
        };
        state.app.as_mut().unwrap().state = Some(empty);
        let mut grant = light();
        grant.app.as_mut().unwrap().grants.insert(Grant::ReadFiles);
        let mut statement = light();
        statement.fns[0].body.stmts.push(Stmt::Break);
        let mut computed = light();
        let hole = Expr {
            ty: Some(2),
            kind: ExprKind::Hole(0),
        };
        computed.fns[0].body.value = Some(Box::new(hole));
        for core in [state, grant, statement, computed] {
            assert_eq!(m1(&core), None);
        }
    }
}
