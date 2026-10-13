//! The core form of the M1 subset and the lowering into it. It is owned data, so it outlives the
//! source. M2-23 redefines it.

use crate::ast::{Def, Expr, ExprKind, Program};
use crate::check::Checked;

#[derive(Debug, PartialEq)]
pub struct Core {
    pub app: String,
    /// The variants of the message enum, in declared order.
    pub variants: Vec<String>,
    /// The position of the start variant in `variants`.
    pub start: usize,
    pub grants: Vec<Grant>,
    /// The Paka name of the update function.
    pub update_name: String,
    /// The effects `update` returns, in order.
    pub update: Vec<Effect>,
}

#[derive(Debug, PartialEq)]
pub enum Grant {
    Console,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Print(String),
    Exit,
}

/// Runs on a checked program, where every lookup below succeeds.
pub fn lower(program: &Program, checked: &Checked) -> Core {
    let (variants, start) = message_enum(program, checked.start);
    Core {
        app: checked.app.into(),
        variants,
        start,
        grants: checked.grants.iter().filter_map(|g| grant(g)).collect(),
        update_name: checked.update_name.into(),
        update: effects(checked.update),
    }
}

/// The variants of the enum that holds `start`, and the position of `start` among them.
fn message_enum(program: &Program, start: &str) -> (Vec<String>, usize) {
    program
        .defs
        .iter()
        .find_map(|def| match def {
            Def::Enum { variants, .. } => {
                let at = variants.iter().position(|v| v.text == start)?;
                Some((variants.iter().map(|v| v.text.into()).collect(), at))
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn grant(name: &str) -> Option<Grant> {
    (name == "Console").then_some(Grant::Console)
}

fn effects(body: &Expr) -> Vec<Effect> {
    match &body.kind {
        ExprKind::List(items) => items.iter().filter_map(effect).collect(),
        _ => vec![],
    }
}

fn effect(e: &Expr) -> Option<Effect> {
    match &e.kind {
        ExprKind::Apply("Print", args) => match args.first()?.kind {
            ExprKind::Str(text) => Some(Effect::Print(text.into())),
            _ => None,
        },
        ExprKind::Apply("Exit", _) => Some(Effect::Exit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check::check, parser::parse};

    #[test]
    fn light_paka_lowers() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");
        let src = std::fs::read(path).unwrap();
        let program = parse("main.paka", &src).unwrap();
        let checked = check("main.paka", &program).unwrap();
        let by_hand = Core {
            app: "light".into(),
            variants: vec!["Start".into()],
            start: 0,
            grants: vec![Grant::Console],
            update_name: "update".into(),
            update: vec![Effect::Print("let there be light".into()), Effect::Exit],
        };
        assert_eq!(lower(&program, &checked), by_hand);
    }
}
