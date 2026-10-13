use crate::ast::Program;
use crate::diagnostic::Diagnostic;

pub fn parse<'a>(_path: &'a str, _src: &'a [u8]) -> Result<Program<'a>, Diagnostic> {
    unimplemented!("M1-09")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use std::fs;
    use std::path::Path;

    fn name(text: &str, line: usize, col: usize) -> Name<'_> {
        Name {
            text,
            at: (line, col),
        }
    }

    fn ty<'a>(text: &'a str, line: usize, col: usize, arg: Option<Type<'a>>) -> Type<'a> {
        Type {
            name: name(text, line, col),
            arg: arg.map(Box::new),
        }
    }

    fn apply<'a>(text: &'a str, col: usize, args: Vec<Expr<'a>>) -> Expr<'a> {
        Expr {
            at: (4, col),
            kind: ExprKind::Apply(text, args),
        }
    }

    /// examples/light.paka by hand; positions from spec/m1.md section 14.
    fn light() -> Program<'static> {
        let print_arg = Expr {
            at: (4, 10),
            kind: ExprKind::Str("let there be light"),
        };
        let effects = Expr {
            at: (4, 3),
            kind: ExprKind::List(vec![
                apply("Print", 4, vec![print_arg]),
                apply("Exit", 33, vec![]),
            ]),
        };
        let msg = ty("Msg", 3, 34, None);
        let result = ty("List", 3, 22, Some(ty("Effect", 3, 27, Some(msg))));
        Program {
            defs: vec![
                Def::Enum {
                    name: name("Msg", 1, 6),
                    variants: vec![name("Start", 1, 12)],
                },
                Def::Fn {
                    name: name("update", 3, 4),
                    param: name("m", 3, 11),
                    param_type: ty("Msg", 3, 14, None),
                    result,
                    body: effects,
                },
                Def::App {
                    name: name("light", 7, 5),
                    entries: vec![
                        Entry::Start((8, 3), name("Start", 8, 11)),
                        Entry::Update((9, 3), name("update", 9, 11)),
                        Entry::Grants((10, 3), vec![name("Console", 10, 12)]),
                    ],
                },
            ],
        }
    }

    #[test]
    fn light_paka_tree() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/light.paka");
        let src = fs::read(path).unwrap();
        assert_eq!(parse("light.paka", &src).unwrap(), light());
    }
}
