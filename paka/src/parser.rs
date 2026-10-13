//! Recursive descent over the lexer's tokens, one token of lookahead (spec/m1.md sections 4 and 5).
//! Tokens are pulled on demand, so the first lexical or syntax problem in source order is the one
//! reported.

use crate::ast::{Def, Entry, Expr, ExprKind, Name, Pos, Program, Type};
use crate::diagnostic::Diagnostic;
use crate::lexer::{Kind, Lexer, Token};

type Parse<T> = Result<T, Diagnostic>;

/// Most brackets open at once: `(`, `[`, a type's `<` and a function body's `{`.
const NESTING: usize = 64;

pub fn parse<'a>(path: &'a str, src: &'a [u8]) -> Parse<Program<'a>> {
    let mut lexer = Lexer::new(path, src);
    let cur = significant(&mut lexer)?;
    Parser {
        path,
        lexer,
        cur,
        prev_end: 0,
        depth: 0,
    }
    .program()
}

/// The next token that is not a line end: this parser reads a statement by its brackets alone.
fn significant<'a>(lexer: &mut Lexer<'a>) -> Parse<Token<'a>> {
    loop {
        match lexer.next()? {
            token if token.kind != Kind::Newline => return Ok(token),
            _ => {}
        }
    }
}

fn text<'a>(token: Token<'a>) -> &'a str {
    std::str::from_utf8(token.text).expect("the lexer yields UTF-8")
}

fn pos(token: Token) -> Pos {
    (token.line, token.col)
}

struct Parser<'a> {
    path: &'a str,
    lexer: Lexer<'a>,
    cur: Token<'a>,
    /// Byte offset just after the last token taken.
    prev_end: usize,
    /// Brackets open at the current token.
    depth: usize,
}

impl<'a> Parser<'a> {
    /// Takes the current token and lexes the next one.
    fn advance(&mut self) -> Parse<Token<'a>> {
        self.prev_end = self.cur.start + self.cur.len();
        Ok(std::mem::replace(
            &mut self.cur,
            significant(&mut self.lexer)?,
        ))
    }

    fn eat(&mut self, kind: Kind, what: &str) -> Parse<Token<'a>> {
        if self.cur.kind != kind {
            return Err(self.unexpected(what));
        }
        self.advance()
    }

    fn name(&mut self, kind: Kind, what: &str) -> Parse<Name<'a>> {
        let token = self.eat(kind, what)?;
        Ok(Name {
            text: text(token),
            at: pos(token),
        })
    }

    fn error(&self, code: &'static str, message: String) -> Diagnostic {
        Diagnostic::new(self.path, pos(self.cur), self.cur.len(), code, message)
    }

    /// The error for the current token, which fits nowhere; `what` is what the grammar wanted.
    fn unexpected(&self, what: &str) -> Diagnostic {
        let code = match self.cur.kind {
            Kind::Type => "unsupported-type",
            Kind::View => "unsupported-view",
            Kind::Example => "unsupported-example",
            Kind::Match => "unsupported-match",
            _ => return self.error("unexpected-token", format!("expected {what}")),
        };
        self.error(code, format!("`{}` is not supported", text(self.cur)))
    }

    /// Takes an opening bracket; one more than `NESTING` open at once is an error at that bracket.
    fn open(&mut self, kind: Kind, what: &str) -> Parse<()> {
        if self.cur.kind == kind && self.depth == NESTING {
            let message = format!(
                "nesting deeper than {NESTING} levels; move the inner part into a named function"
            );
            return Err(self.error("nesting-too-deep", message));
        }
        self.eat(kind, what)?;
        self.depth += 1;
        Ok(())
    }

    fn close(&mut self, kind: Kind, what: &str) -> Parse<()> {
        self.eat(kind, what)?;
        self.depth -= 1;
        Ok(())
    }

    /// `item { "," item }`
    fn comma_list<T>(&mut self, mut item: impl FnMut(&mut Self) -> Parse<T>) -> Parse<Vec<T>> {
        let mut items = vec![item(self)?];
        while self.cur.kind == Kind::Comma {
            self.advance()?;
            items.push(item(self)?);
        }
        Ok(items)
    }

    /// `"[" [ item { "," item } ] "]"`
    fn bracketed<T>(&mut self, item: impl FnMut(&mut Self) -> Parse<T>) -> Parse<Vec<T>> {
        self.open(Kind::LBracket, "`[`")?;
        let items = match self.cur.kind {
            Kind::RBracket => vec![],
            _ => self.comma_list(item)?,
        };
        self.close(Kind::RBracket, "`]` or `,`")?;
        Ok(items)
    }

    fn program(&mut self) -> Parse<Program<'a>> {
        let mut defs = vec![];
        while self.cur.kind != Kind::End {
            defs.push(match self.cur.kind {
                Kind::Enum => self.enum_def()?,
                Kind::Fn => self.fn_def()?,
                Kind::App => self.app_def()?,
                _ => return Err(self.unexpected("a definition")),
            });
        }
        Ok(Program { defs })
    }

    fn enum_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Upper, "an enum name")?;
        self.eat(Kind::LBrace, "`{`")?;
        let variants = self.comma_list(|p| p.name(Kind::Upper, "a variant name"))?;
        self.eat(Kind::RBrace, "`}` or `,`")?;
        Ok(Def::Enum { name, variants })
    }

    fn fn_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "a function name")?;
        self.eat(Kind::LParen, "`(`")?;
        let param = self.name(Kind::Lower, "a parameter name")?;
        self.eat(Kind::Colon, "`:`")?;
        let param_type = self.ty()?;
        self.eat(Kind::RParen, "`)`")?;
        self.eat(Kind::Arrow, "`->`")?;
        let result = self.ty()?;
        self.open(Kind::LBrace, "`{`")?;
        let body = self.expr()?;
        self.close(Kind::RBrace, "`}`")?;
        Ok(Def::Fn {
            name,
            param,
            param_type,
            result,
            body,
        })
    }

    fn ty(&mut self) -> Parse<Type<'a>> {
        let name = self.name(Kind::Upper, "a type name")?;
        let mut arg = None;
        if self.cur.kind == Kind::Lt {
            self.open(Kind::Lt, "`<`")?;
            arg = Some(Box::new(self.ty()?));
            self.close(Kind::Gt, "`>`")?;
        }
        Ok(Type { name, arg })
    }

    fn expr(&mut self) -> Parse<Expr<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Str => {
                self.advance()?;
                ExprKind::Str(text(first))
            }
            Kind::LBracket => ExprKind::List(self.bracketed(Self::expr)?),
            Kind::Upper => {
                self.advance()?;
                ExprKind::Apply(text(first), self.args()?)
            }
            _ => return Err(self.unexpected("an expression")),
        };
        Ok(Expr {
            at: pos(first),
            len: self.prev_end - first.start,
            kind,
        })
    }

    /// `[ "(" arg { "," arg } ")" ]`
    fn args(&mut self) -> Parse<Vec<Expr<'a>>> {
        if self.cur.kind != Kind::LParen {
            return Ok(vec![]);
        }
        self.open(Kind::LParen, "`(`")?;
        let args = self.comma_list(Self::arg)?;
        self.close(Kind::RParen, "`)` or `,`")?;
        Ok(args)
    }

    /// An argument is an expression; a LOWER followed by `:` is a label. Only the token after
    /// the LOWER is looked at: a lexical error there only means it is not `:`. The lookahead
    /// token is dropped without updating `cur`, which is sound only because both paths return
    /// an error.
    fn arg(&mut self) -> Parse<Expr<'a>> {
        if self.cur.kind != Kind::Lower {
            return self.expr();
        }
        if significant(&mut self.lexer).is_ok_and(|t| t.kind == Kind::Colon) {
            return Err(self.error(
                "unsupported-labelled-argument",
                "labelled arguments are not supported".into(),
            ));
        }
        Err(self.unexpected("an expression"))
    }

    fn app_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "an app name")?;
        self.eat(Kind::LBrace, "`{`")?;
        let mut entries = vec![];
        while self.cur.kind != Kind::RBrace {
            entries.push(self.entry()?);
        }
        self.advance()?;
        Ok(Def::App { name, entries })
    }

    fn entry(&mut self) -> Parse<Entry<'a>> {
        let key = self.cur;
        if key.kind != Kind::Lower {
            return Err(self.unexpected("an app entry"));
        }
        let value: fn(&mut Self, Pos) -> Parse<Entry<'a>> = match key.text {
            b"start" => |p, at| Ok(Entry::Start(at, p.name(Kind::Upper, "a variant name")?)),
            b"update" => |p, at| Ok(Entry::Update(at, p.name(Kind::Lower, "a function name")?)),
            b"grants" => |p, at| {
                Ok(Entry::Grants(
                    at,
                    p.bracketed(|p| p.name(Kind::Upper, "a grant"))?,
                ))
            },
            _ => {
                return Err(self.error(
                    "unsupported-app-entry",
                    format!("the app entry `{}` is not supported", text(key)),
                ));
            }
        };
        self.advance()?;
        self.eat(Kind::Colon, "`:`")?;
        value(self, pos(key))
    }
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

    fn apply<'a>(text: &'a str, col: usize, len: usize, args: Vec<Expr<'a>>) -> Expr<'a> {
        Expr {
            at: (4, col),
            len,
            kind: ExprKind::Apply(text, args),
        }
    }

    /// examples/light.paka by hand; positions from spec/m1.md section 14.
    fn light() -> Program<'static> {
        let print_arg = Expr {
            at: (4, 10),
            len: 20,
            kind: ExprKind::Str("let there be light"),
        };
        let effects = Expr {
            at: (4, 3),
            len: 35,
            kind: ExprKind::List(vec![
                apply("Print", 4, 27, vec![print_arg]),
                apply("Exit", 33, 4, vec![]),
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
