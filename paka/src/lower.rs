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

pub fn lower(program: &Program, checked: &Checked) -> Core {
    Core {
        app: String::new(),
        variants: vec![],
        start: 0,
        grants: vec![],
        update: vec![],
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
            update: vec![Effect::Print("let there be light".into()), Effect::Exit],
        };
        assert_eq!(lower(&program, &checked), by_hand);
    }
}
