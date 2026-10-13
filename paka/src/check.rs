//! Name resolution and type checking (spec/m1.md sections 6 to 8). The first problem in the
//! spec's fixed order is the one reported.

use crate::ast::{Expr, Program};
use crate::diagnostic::Diagnostic;

type Check<T> = Result<T, Diagnostic>;

/// A program that passed, reduced to what running it needs.
#[derive(Debug)]
pub struct Checked<'a> {
    pub app: &'a str,
    pub start: &'a str,
    /// The body of the `update` function.
    pub update: &'a Expr<'a>,
    pub grants: Vec<&'a str>,
}

pub fn check<'a>(path: &'a str, _program: &'a Program<'a>) -> Check<Checked<'a>> {
    Err(Diagnostic {
        path: path.into(),
        position: None,
        code: "unimplemented",
        message: "the checker is not written yet".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, ExprKind};
    use crate::parser::parse;

    fn checked<'a>(program: &'a Program<'a>) -> Check<Checked<'a>> {
        check("main.paka", program)
    }

    fn expr(at: (usize, usize), kind: ExprKind) -> Expr {
        Expr { at, kind }
    }

    #[test]
    fn light_paka_checks() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");
        let src = std::fs::read(path).unwrap();
        let program = parse("main.paka", &src).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!(light.app, "light");
        assert_eq!(light.start, "Start");
        assert_eq!(light.grants, ["Console"]);
        let print = expr(
            (4, 4),
            ExprKind::Apply(
                "Print",
                vec![expr((4, 10), ExprKind::Str("let there be light"))],
            ),
        );
        let exit = expr((4, 33), ExprKind::Apply("Exit", vec![]));
        let body = expr((4, 3), ExprKind::List(vec![print, exit]));
        assert_eq!(light.update, &body);
    }

    #[test]
    fn definitions_and_entries_may_come_in_any_order() {
        let src = "app light { grants: [] update: update start: Go }
            fn update(m: Msg) -> List<Effect<Msg>> { [] }
            enum Msg { Stop, Go }";
        let program = parse("main.paka", src.as_bytes()).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!((light.start, light.grants), ("Go", vec![]));
    }
}
