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
    use std::fs;
    use std::path::Path;

    // Hand-built trees. Positions are (line, column) from 1; `len` is bytes up to the last token.

    fn nm(text: &str, line: usize, col: usize) -> Name<'_> {
        Name {
            text,
            at: (line, col),
        }
    }

    fn node<K>(line: usize, col: usize, len: usize, kind: K) -> Node<K> {
        Node {
            at: (line, col),
            len,
            kind,
        }
    }

    /// An expression on line 2, where `stmt` and `expr` put their code.
    fn e(col: usize, len: usize, kind: ExprKind) -> Expr {
        node(2, col, len, kind)
    }

    fn var(col: usize, text: &str) -> Expr<'_> {
        e(col, text.len(), ExprKind::Name(text))
    }

    fn int(col: usize, text: &str) -> Expr<'_> {
        e(col, text.len(), ExprKind::Int(text))
    }

    fn bx(x: Expr) -> Box<Expr> {
        Box::new(x)
    }

    fn arg(value: Expr) -> Arg {
        Arg { label: None, value }
    }

    fn labelled<'a>(label: &'a str, col: usize, value: Expr<'a>) -> Arg<'a> {
        Arg {
            label: Some(nm(label, 2, col)),
            value,
        }
    }

    fn ty(text: &str, line: usize, col: usize) -> Type<'_> {
        generic(text, line, col, vec![])
    }

    fn generic<'a>(text: &'a str, line: usize, col: usize, args: Vec<Type<'a>>) -> Type<'a> {
        Type::Name {
            name: nm(text, line, col),
            args,
        }
    }

    fn plain<'a>(name: Name<'a>, ty: Type<'a>) -> Field<'a> {
        Field {
            name,
            ty,
            default: None,
        }
    }

    fn param<'a>(name: Name<'a>, ty: Type<'a>) -> Param<'a> {
        Param {
            name,
            inout: false,
            labelled: false,
            ty,
        }
    }

    fn empty(line: usize, col: usize) -> Block<'static> {
        Block {
            stmts: vec![],
            close: (line, col),
        }
    }

    fn tree(src: &str) -> Program<'_> {
        let (program, errors) = parse_all("main.paka", src.as_bytes());
        assert!(errors.is_empty(), "{errors:?}");
        program
    }

    /// The statements of `fn f() { ... }` holding `code` on line 2, column 3.
    fn stmts(code: &str, want: Vec<Stmt>) {
        let src = format!("fn f() {{\n  {code}\n}}\n");
        let program = tree(&src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        assert_eq!(body.stmts, want, "{code}");
    }

    fn stmt(code: &str, kind: StmtKind) {
        stmts(code, vec![node(2, 3, code.len(), kind)]);
    }

    fn expr(code: &str, kind: ExprKind) {
        stmt(code, StmtKind::Expr(e(3, code.len(), kind)));
    }

    /// The first error of `src`: position, code, and a part of the message.
    fn fail(src: &str, at: Pos, code: &str, message: &str) {
        let (_, errors) = parse_all("main.paka", src.as_bytes());
        let d = errors.first().expect("no error");
        assert_eq!((d.position, d.code), (Some(at), code), "{d}");
        assert!(d.message.contains(message), "{d}");
    }

    /// `fail` for `code` as the only statement, on line 2, column 3.
    fn fail_in(code: &str, col: usize, kind: &str, message: &str) {
        fail(
            &format!("fn f() {{\n  {code}\n}}\n"),
            (2, col),
            kind,
            message,
        );
    }

    fn unexpected(code: &str, col: usize, message: &str) {
        fail_in(code, col, "unexpected-token", message);
    }

    // Definitions

    #[test]
    fn type_definition() {
        let src = "type Point {\n  x: Int = 0,\n  y: List<Int>\n}\n";
        let x = Field {
            name: nm("x", 2, 3),
            ty: ty("Int", 2, 6),
            default: Some(node(2, 12, 1, ExprKind::Int("0"))),
        };
        let y = plain(nm("y", 3, 3), generic("List", 3, 6, vec![ty("Int", 3, 11)]));
        let want = Def::Type {
            name: nm("Point", 1, 6),
            fields: vec![x, y],
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn empty_type_and_empty_app() {
        let want = vec![
            Def::Type {
                name: nm("Empty", 1, 6),
                fields: vec![],
            },
            Def::App {
                name: nm("a", 2, 5),
                entries: vec![],
            },
        ];
        assert_eq!(tree("type Empty {}\napp a {\n}\n").defs, want);
    }

    #[test]
    fn enum_definition() {
        let src = "enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }\n";
        let variant = |name, fields| Variant { name, fields };
        let want = Def::Enum {
            name: nm("Shape", 1, 6),
            variants: vec![
                variant(nm("Dot", 1, 14), vec![]),
                variant(
                    nm("Circle", 1, 19),
                    vec![plain(nm("r", 1, 26), ty("Float", 1, 29))],
                ),
                variant(
                    nm("Rect", 1, 37),
                    vec![
                        plain(nm("w", 1, 42), ty("Float", 1, 45)),
                        plain(nm("h", 1, 52), ty("Float", 1, 55)),
                    ],
                ),
            ],
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn fn_with_bare_inout_and_labelled_parameters() {
        let src =
            "fn find(xs: List<Int>, log: inout List<String>; lo: Int, hi: Int) -> Bool {\n}\n";
        let xs = generic("List", 1, 13, vec![ty("Int", 1, 18)]);
        let log = generic("List", 1, 35, vec![ty("String", 1, 40)]);
        let params = vec![
            param(nm("xs", 1, 9), xs),
            Param {
                inout: true,
                ..param(nm("log", 1, 24), log)
            },
            Param {
                labelled: true,
                ..param(nm("lo", 1, 49), ty("Int", 1, 53))
            },
            Param {
                labelled: true,
                ..param(nm("hi", 1, 58), ty("Int", 1, 62))
            },
        ];
        let want = Def::Fn {
            name: nm("find", 1, 4),
            params,
            result: Some(ty("Bool", 1, 70)),
            body: empty(2, 1),
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn fn_without_parameters_or_result() {
        let want = Def::Fn {
            name: nm("f", 1, 4),
            params: vec![],
            result: None,
            body: empty(1, 9),
        };
        assert_eq!(tree("fn f() {}\n").defs, vec![want]);
    }

    #[test]
    fn function_types() {
        let src = "fn apply(f: fn(Int, String) -> Bool, g: fn()) {}\n";
        let f = Type::Fn {
            at: (1, 13),
            params: vec![ty("Int", 1, 16), ty("String", 1, 21)],
            result: Some(Box::new(ty("Bool", 1, 32))),
        };
        let g = Type::Fn {
            at: (1, 41),
            params: vec![],
            result: None,
        };
        let want = Def::Fn {
            name: nm("apply", 1, 4),
            params: vec![param(nm("f", 1, 10), f), param(nm("g", 1, 38), g)],
            result: None,
            body: empty(1, 48),
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn example_definition() {
        let want = Def::Example {
            name: nm("sorted", 1, 9),
            body: empty(2, 1),
        };
        assert_eq!(tree("example sorted {\n}\n").defs, vec![want]);
    }

    #[test]
    fn app_with_every_entry() {
        let src = "app counter {\n  state: Model\n  start: Start\n  update: step\n  grants: [Console, Clock]\n}\n";
        let entries = vec![
            Entry::State((2, 3), nm("Model", 2, 10)),
            Entry::Start((3, 3), node(3, 10, 5, ExprKind::Name("Start"))),
            Entry::Update((4, 3), nm("step", 4, 11)),
            Entry::Grants((5, 3), vec![nm("Console", 5, 12), nm("Clock", 5, 21)]),
        ];
        let want = Def::App {
            name: nm("counter", 1, 5),
            entries,
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    // Statements

    #[test]
    fn let_and_var() {
        stmt(
            "let x = 1",
            StmtKind::Let {
                var: false,
                name: nm("x", 2, 7),
                ty: None,
                value: int(11, "1"),
            },
        );
        stmt(
            "var xs: List<Int> = []",
            StmtKind::Let {
                var: true,
                name: nm("xs", 2, 7),
                ty: Some(generic("List", 2, 11, vec![ty("Int", 2, 16)])),
                value: e(23, 2, ExprKind::List(vec![])),
            },
        );
    }

    #[test]
    fn discard_return_break_continue() {
        let call = ExprKind::Call(bx(var(7, "f")), vec![]);
        stmt("_ = f()", StmtKind::Discard(e(7, 3, call)));
        stmt("return", StmtKind::Return(None));
        stmt("return 1", StmtKind::Return(Some(int(10, "1"))));
        stmt("break", StmtKind::Break);
        stmt("continue", StmtKind::Continue);
    }

    #[test]
    fn every_assignment_operator() {
        use AssignOp::*;
        let ops = [("=", Set), ("+=", Add), ("-=", Sub), ("*=", Mul)];
        for (text, op) in ops.into_iter().chain([("/=", Div), ("%=", Rem)]) {
            let code = format!("x {text} 1");
            let (target, value) = (var(3, "x"), int(code.len() + 2, "1"));
            stmt(&code, StmtKind::Assign { target, op, value });
        }
    }

    #[test]
    fn assignment_to_a_field_and_an_item() {
        let field = e(3, 3, ExprKind::Field(bx(var(3, "p")), nm("x", 2, 5)));
        let target = e(3, 6, ExprKind::Index(bx(field), bx(int(7, "0"))));
        let (op, value) = (AssignOp::Add, int(13, "1"));
        stmt("p.x[0] += 1", StmtKind::Assign { target, op, value });
    }

    #[test]
    fn loops() {
        let range = e(12, 5, ExprKind::Range(bx(int(12, "0")), bx(int(16, "3"))));
        let body = Block {
            stmts: vec![node(2, 20, 5, StmtKind::Break)],
            close: (2, 26),
        };
        let name = nm("i", 2, 7);
        stmt(
            "for i in 0..<3 { break }",
            StmtKind::For {
                name,
                list: range,
                body,
            },
        );
        let name = nm("x", 2, 7);
        let list = var(12, "xs");
        let body = empty(2, 16);
        stmt("for x in xs {}", StmtKind::For { name, list, body });
        let cond = e(
            9,
            5,
            ExprKind::Binary(BinOp::Lt, bx(var(9, "x")), bx(int(13, "3"))),
        );
        let body = empty(2, 17);
        stmt("while x < 3 { }", StmtKind::While { cond, body });
    }

    // Expressions

    #[test]
    fn literals_names_and_holes() {
        expr("42", ExprKind::Int("42"));
        expr("3.5", ExprKind::Float("3.5"));
        expr("true", ExprKind::Bool(true));
        expr("false", ExprKind::Bool(false));
        expr("??todo", ExprKind::Hole("todo"));
        expr("x", ExprKind::Name("x"));
        expr("Start", ExprKind::Name("Start"));
    }

    #[test]
    fn a_minus_touching_a_number_joins_it() {
        expr("-5", ExprKind::Int("-5"));
        expr("-2.5", ExprKind::Float("-2.5"));
        expr("- 5", ExprKind::Unary(UnOp::Neg, bx(int(5, "5"))));
        expr(
            "-(5)",
            ExprKind::Unary(UnOp::Neg, bx(e(4, 3, ExprKind::Int("5")))),
        );
        expr("-x", ExprKind::Unary(UnOp::Neg, bx(var(4, "x"))));
        expr("!x", ExprKind::Unary(UnOp::Not, bx(var(4, "x"))));
        expr(
            "--5",
            ExprKind::Unary(UnOp::Neg, bx(e(4, 2, ExprKind::Int("-5")))),
        );
    }

    #[test]
    fn strings() {
        let text = |s: &str| StrPart::Text(s.into());
        expr(r#""hi""#, ExprKind::Str(vec![text("hi")]));
        expr(r#""""#, ExprKind::Str(vec![]));
        expr(r#""a\nb\{""#, ExprKind::Str(vec![text("a\nb{")]));
        let parts = vec![text("x"), StrPart::Expr(var(6, "n")), text("y")];
        expr(r#""x{n}y""#, ExprKind::Str(parts));
        let sum = ExprKind::Binary(BinOp::Add, bx(var(5, "a")), bx(int(9, "1")));
        expr(
            r#""{a + 1}""#,
            ExprKind::Str(vec![StrPart::Expr(e(5, 5, sum))]),
        );
    }

    #[test]
    fn calls() {
        expr("f()", ExprKind::Call(bx(var(3, "f")), vec![]));
        let call = ExprKind::Call(bx(var(3, "f")), vec![arg(int(5, "1")), arg(int(8, "2"))]);
        expr("f(1, 2)", call);
        let args = vec![
            labelled("lo", 5, int(9, "1")),
            labelled("hi", 12, int(16, "2")),
        ];
        expr("f(lo: 1, hi: 2)", ExprKind::Call(bx(var(3, "f")), args));
        let args = vec![arg(int(5, "1")), labelled("hi", 8, int(12, "2"))];
        expr("f(1, hi: 2)", ExprKind::Call(bx(var(3, "f")), args));
    }

    #[test]
    fn inout_arguments() {
        let inout = |col, place| e(col, 1 + place_len(&place), ExprKind::Inout(bx(place)));
        fn place_len(place: &Expr) -> usize {
            place.len
        }
        let args = vec![arg(inout(5, var(6, "xs")))];
        expr("f(&xs)", ExprKind::Call(bx(var(3, "f")), args));
        let args = vec![labelled("a", 5, inout(8, var(9, "xs")))];
        expr("f(a: &xs)", ExprKind::Call(bx(var(3, "f")), args));
        let place = e(6, 3, ExprKind::Field(bx(var(6, "p")), nm("x", 2, 8)));
        let args = vec![arg(inout(5, place))];
        expr("f(&p.x)", ExprKind::Call(bx(var(3, "f")), args));
    }

    #[test]
    fn a_call_through_a_value() {
        let inner = e(
            3,
            4,
            ExprKind::Call(bx(var(3, "g")), vec![arg(int(5, "1"))]),
        );
        let outer = ExprKind::Call(bx(inner), vec![arg(int(8, "2"))]);
        expr("g(1)(2)", outer);
    }

    #[test]
    fn fields_items_slices_and_try() {
        expr("a.b", ExprKind::Field(bx(var(3, "a")), nm("b", 2, 5)));
        expr("a[0]", ExprKind::Index(bx(var(3, "a")), bx(int(5, "0"))));
        let range = e(5, 5, ExprKind::Range(bx(int(5, "1")), bx(int(9, "3"))));
        expr("a[1..<3]", ExprKind::Index(bx(var(3, "a")), bx(range)));
        expr("a?", ExprKind::Try(bx(var(3, "a"))));
        let field = e(3, 3, ExprKind::Field(bx(var(3, "a")), nm("b", 2, 5)));
        expr("a.b(1)", ExprKind::Call(bx(field), vec![arg(int(7, "1"))]));
    }

    #[test]
    fn every_binary_operator() {
        use BinOp::*;
        let ops = [("||", Or), ("&&", And), ("==", Eq), ("!=", Ne), ("<", Lt)];
        let rest = [("<=", Le), (">", Gt), (">=", Ge), ("+", Add), ("-", Sub)];
        for (text, op) in ops
            .into_iter()
            .chain(rest)
            .chain([("*", Mul), ("/", Div), ("%", Rem)])
        {
            let code = format!("a {text} b");
            let kind = ExprKind::Binary(op, bx(var(3, "a")), bx(var(code.len() + 2, "b")));
            expr(&code, kind);
        }
    }

    #[test]
    fn precedence_and_grouping() {
        use BinOp::*;
        let bin = |col, len, op, l, r| e(col, len, ExprKind::Binary(op, bx(l), bx(r)));
        let un = |col, op, x| e(col, 2, ExprKind::Unary(op, bx(x)));
        let a = var(3, "a");
        let want = bin(3, 11, Or, a, bin(8, 6, And, var(8, "b"), var(13, "c")));
        expr("a || b && c", want.kind);
        let want = bin(
            3,
            9,
            Add,
            var(3, "a"),
            bin(7, 5, Mul, var(7, "b"), var(11, "c")),
        );
        expr("a + b * c", want.kind);
        let want = bin(
            3,
            9,
            Sub,
            bin(3, 5, Sub, var(3, "a"), var(7, "b")),
            var(11, "c"),
        );
        expr("a - b - c", want.kind);
        let want = bin(
            3,
            10,
            Eq,
            var(3, "a"),
            bin(8, 5, Add, var(8, "b"), int(12, "1")),
        );
        expr("a == b + 1", want.kind);
        let want = bin(3, 7, And, un(3, UnOp::Not, var(4, "a")), var(9, "b"));
        expr("!a && b", want.kind);
        let want = bin(3, 6, Mul, un(3, UnOp::Neg, var(4, "a")), var(8, "b"));
        expr("-a * b", want.kind);
        let sum = bin(3, 7, Add, var(4, "a"), var(8, "b"));
        let want = bin(3, 11, Mul, sum, var(13, "c"));
        expr("(a + b) * c", want.kind);
    }

    #[test]
    fn parentheses_span_the_brackets() {
        expr("(a)", ExprKind::Name("a"));
    }

    #[test]
    fn records_lists_and_maps() {
        let fields = vec![
            labelled("x", 11, int(14, "1")),
            labelled("y", 17, int(20, "2")),
        ];
        expr("Point { x: 1, y: 2 }", ExprKind::Record("Point", fields));
        expr("Point {}", ExprKind::Record("Point", vec![]));
        expr("[]", ExprKind::List(vec![]));
        expr("[1, 2]", ExprKind::List(vec![int(4, "1"), int(7, "2")]));
        expr("[:]", ExprKind::Map(vec![]));
        let str_at = |col, text| e(col, 3, ExprKind::Str(vec![StrPart::Text(text)]));
        let rows = vec![
            (str_at(4, "a".into()), int(9, "1")),
            (str_at(12, "b".into()), int(17, "2")),
        ];
        expr(r#"["a": 1, "b": 2]"#, ExprKind::Map(rows));
    }

    #[test]
    fn if_with_else_if_and_else() {
        let block = |n, text, close| Block {
            stmts: vec![node(2, n, 1, StmtKind::Expr(int(n, text)))],
            close: (2, close),
        };
        let branches = vec![
            (var(6, "a"), block(10, "1", 12)),
            (var(22, "b"), block(26, "2", 28)),
        ];
        let kind = ExprKind::If {
            branches,
            otherwise: Some(block(37, "3", 39)),
        };
        expr("if a { 1 } else if b { 2 } else { 3 }", kind);
    }

    #[test]
    fn match_with_every_pattern_and_body() {
        let src = "fn f() {
  match x {
    0 | 1 => a
    -2 => b
    \"s\" => c
    true => d
    V => e
    W(p, _) => { f }
    _ => y = 1
  }
}
";
        let pat = |line, col, len, kind| node(line, col, len, kind);
        let said = |line, col, text| {
            let name = node(line, col, 1, ExprKind::Name(text));
            ArmBody::Line(node(line, col, 1, StmtKind::Expr(name)))
        };
        let arm = |patterns, body| Arm { patterns, body };
        let w = pat(
            8,
            5,
            7,
            PatKind::Variant(
                "W",
                vec![
                    pat(8, 7, 1, PatKind::Bind("p")),
                    pat(8, 10, 1, PatKind::Wild),
                ],
            ),
        );
        let block = Block {
            stmts: vec![node(
                8,
                18,
                1,
                StmtKind::Expr(node(8, 18, 1, ExprKind::Name("f"))),
            )],
            close: (8, 20),
        };
        let set = StmtKind::Assign {
            target: node(9, 10, 1, ExprKind::Name("y")),
            op: AssignOp::Set,
            value: node(9, 14, 1, ExprKind::Int("1")),
        };
        let arms = vec![
            arm(
                vec![
                    pat(3, 5, 1, PatKind::Int("0")),
                    pat(3, 9, 1, PatKind::Int("1")),
                ],
                said(3, 14, "a"),
            ),
            arm(vec![pat(4, 5, 2, PatKind::Int("-2"))], said(4, 11, "b")),
            arm(
                vec![pat(5, 5, 3, PatKind::Str("s".into()))],
                said(5, 12, "c"),
            ),
            arm(vec![pat(6, 5, 4, PatKind::Bool(true))], said(6, 13, "d")),
            arm(
                vec![pat(7, 5, 1, PatKind::Variant("V", vec![]))],
                said(7, 10, "e"),
            ),
            arm(vec![w], ArmBody::Block(block)),
            arm(
                vec![pat(9, 5, 1, PatKind::Wild)],
                ArmBody::Line(node(9, 10, 5, set)),
            ),
        ];
        let subject = bx(node(2, 9, 1, ExprKind::Name("x")));
        let kind = ExprKind::Match { subject, arms };
        let matched = node(2, 3, 114, StmtKind::Expr(node(2, 3, 114, kind)));
        let program = tree(src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        assert_eq!(body.stmts, vec![matched]);
    }

    #[test]
    fn an_upper_name_before_a_brace_ends_a_header() {
        let cond = e(
            6,
            12,
            ExprKind::Binary(BinOp::Eq, bx(var(6, "x")), bx(var(11, "Counter"))),
        );
        let block = |n, text, close| Block {
            stmts: vec![node(2, n, 1, StmtKind::Expr(int(n, text)))],
            close: (2, close),
        };
        let kind = ExprKind::If {
            branches: vec![(cond, block(21, "1", 23))],
            otherwise: Some(block(32, "2", 34)),
        };
        expr("if x == Counter { 1 } else { 2 }", kind);
    }

    #[test]
    fn parentheses_build_a_record_in_a_header() {
        let record = e(12, 10, ExprKind::Record("Counter", vec![]));
        let paren = ExprKind::Record("Counter", vec![]);
        assert_eq!(record.kind, paren);
        let record = e(11, 12, paren);
        let cond = e(
            6,
            17,
            ExprKind::Binary(BinOp::Eq, bx(var(6, "x")), bx(record)),
        );
        let body = Block {
            stmts: vec![node(2, 26, 1, StmtKind::Expr(int(26, "1")))],
            close: (2, 28),
        };
        let kind = ExprKind::If {
            branches: vec![(cond, body)],
            otherwise: None,
        };
        expr("if x == (Counter {}) { 1 }", kind);
    }

    #[test]
    fn a_trailing_operator_continues_the_line() {
        let src = "fn f() {\n  a +\n    b\n}\n";
        let sum = ExprKind::Binary(
            BinOp::Add,
            bx(var(3, "a")),
            bx(node(3, 5, 1, ExprKind::Name("b"))),
        );
        let want = node(2, 3, 9, StmtKind::Expr(node(2, 3, 9, sum)));
        let program = tree(src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        assert_eq!(body.stmts, vec![want]);
    }

    #[test]
    fn chains_are_loops() {
        let src = format!("fn f() {{\n  {}x\n}}\n", "!".repeat(1000));
        let program = tree(&src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        let StmtKind::Expr(first) = &body.stmts[0].kind else {
            panic!("not an expression")
        };
        let mut e = first;
        let mut depth = 0;
        while let ExprKind::Unary(UnOp::Not, inner) = &e.kind {
            e = inner;
            depth += 1;
        }
        assert_eq!(depth, 1000);
        let sum = ["a"; 2000].join(" + ");
        let src = format!("fn f() {{\n  {sum}\n}}\n");
        assert!(tree(&src).defs.len() == 1);
        let src = format!("fn f() {{\n  f{}\n}}\n", "()".repeat(1000));
        assert!(tree(&src).defs.len() == 1);
        let mut src = String::from("fn f() {\n  if a { 1 }");
        src += &" else if a { 1 }".repeat(1000);
        src += "\n}\n";
        let program = tree(&src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        let StmtKind::Expr(chain) = &body.stmts[0].kind else {
            panic!("not an expression")
        };
        let ExprKind::If {
            branches,
            otherwise,
        } = &chain.kind
        else {
            panic!("not an if")
        };
        assert_eq!((branches.len(), otherwise.is_none()), (1001, true));
    }

    #[test]
    fn the_light_program() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/light.paka");
        let src = fs::read(path).unwrap();
        let program = parse("light.paka", &src).unwrap();
        let text = StrPart::Text("let there be light".into());
        let print = node(
            4,
            4,
            27,
            ExprKind::Call(
                Box::new(node(4, 4, 5, ExprKind::Name("Print"))),
                vec![arg_at(node(4, 10, 20, ExprKind::Str(vec![text])))],
            ),
        );
        let exit = node(4, 33, 4, ExprKind::Name("Exit"));
        let effects = node(4, 3, 35, ExprKind::List(vec![print, exit]));
        let result = generic(
            "List",
            3,
            22,
            vec![generic("Effect", 3, 27, vec![ty("Msg", 3, 34)])],
        );
        let want = vec![
            Def::Enum {
                name: nm("Msg", 1, 6),
                variants: vec![Variant {
                    name: nm("Start", 1, 12),
                    fields: vec![],
                }],
            },
            Def::Fn {
                name: nm("update", 3, 4),
                params: vec![param(nm("m", 3, 11), ty("Msg", 3, 14))],
                result: Some(result),
                body: Block {
                    stmts: vec![node(4, 3, 35, StmtKind::Expr(effects))],
                    close: (5, 1),
                },
            },
            Def::App {
                name: nm("light", 7, 5),
                entries: vec![
                    Entry::Start((8, 3), node(8, 11, 5, ExprKind::Name("Start"))),
                    Entry::Update((9, 3), nm("update", 9, 11)),
                    Entry::Grants((10, 3), vec![nm("Console", 10, 12)]),
                ],
            },
        ];
        assert_eq!(program.defs, want);
    }

    fn arg_at(value: Expr) -> Arg {
        arg(value)
    }

    // Parse errors

    #[test]
    fn a_semicolon_is_not_a_line_end() {
        unexpected("x;", 4, "remove the ;: a line break ends the statement");
    }

    #[test]
    fn a_comma_before_a_closer() {
        unexpected("[1, 2,]", 9, "remove the comma before ]");
        unexpected("f(1,)", 7, "remove the comma before )");
        unexpected("Point { x: 1, }", 17, "remove the comma before }");
    }

    #[test]
    fn a_comma_after_an_arm() {
        let src = "fn f() {\n  match x {\n    _ => 1,\n  }\n}\n";
        let message = "remove the comma: one arm per line";
        fail(src, (3, 11), "unexpected-token", message);
    }

    #[test]
    fn else_sits_on_the_line_of_the_brace() {
        let src = "fn f() {\n  if a { 1 }\n  else { 2 }\n}\n";
        fail(
            src,
            (3, 3),
            "unexpected-token",
            "put else on the line of the }",
        );
    }

    #[test]
    fn names_of_the_wrong_case() {
        let upper = "type names start with an upper-case letter: write Msg";
        fail("enum msg { A }\n", (1, 6), "unexpected-token", upper);
        let lower = "function names start with a lower-case letter: write update";
        fail("fn Update() {}\n", (1, 4), "unexpected-token", lower);
    }

    #[test]
    fn comparisons_do_not_chain() {
        unexpected("a < b < c", 9, "write a < b && b < c");
    }

    #[test]
    fn inout_is_written_on_arguments_only() {
        let message = "& is written only on an argument of a call";
        fail_in("let r = &xs", 11, "unexpected-inout-mark", message);
    }

    #[test]
    fn type_parameters_interfaces_and_lambdas_are_refused() {
        let params =
            "Paka has no type parameters; write it for one type: fn first(xs: List<Int>) -> Int";
        for (src, at) in [
            ("fn first<T>() {}\n", 9),
            ("type Box<T> {}\n", 9),
            ("enum E<T> { A }\n", 7),
        ] {
            fail(src, (1, at), "unexpected-token", params);
        }
        let interface = "Paka has no interfaces; write one function per type and pass it by name: sort_by(&xs, before)";
        fail(
            "interface Shape {}\n",
            (1, 1),
            "unexpected-token",
            interface,
        );
        fail("impl Shape {}\n", (1, 1), "unexpected-token", interface);
        let lambda = "no function written in place; write it at top level and pass its name: fn before(a: T, b: T) -> Bool { ... }; a local it would have used becomes a parameter or a field";
        unexpected("let f = x => x", 13, lambda);
        unexpected("let f = (x) -> x", 15, lambda);
    }

    #[test]
    fn a_function_type_has_no_inout() {
        let src = "fn f(g: fn(inout Int)) {}\n";
        fail(
            src,
            (1, 12),
            "unexpected-token",
            "no inout in a function type",
        );
    }

    #[test]
    fn view_is_unsupported() {
        let message = "`view` is not supported";
        fail("view v() {}\n", (1, 1), "unsupported-view", message);
        fail(
            "app a {\n  view: v\n}\n",
            (2, 3),
            "unsupported-view",
            message,
        );
    }

    /// Runs `f` on a big stack: 64 nested levels do not fit the 2 MiB of a debug test thread.
    fn deep(f: impl FnOnce() + Send) {
        std::thread::scope(|s| {
            let run = std::thread::Builder::new().stack_size(32 << 20);
            run.spawn_scoped(s, f).unwrap().join().unwrap();
        });
    }

    #[test]
    fn nesting_stops_at_64_levels() {
        deep(|| {
            let message =
                "nesting deeper than 64 levels; move the inner part into a named function or local";
            let list = |n: usize| format!("fn f() {{\n  {}1{}\n}}\n", "[".repeat(n), "]".repeat(n));
            assert!(parse_all("main.paka", list(63).as_bytes()).1.is_empty());
            fail(&list(64), (2, 66), "nesting-too-deep", message);
            let types =
                |n: usize| format!("fn f(x: {}Int{}) {{}}\n", "L<".repeat(n), ">".repeat(n));
            assert!(parse_all("main.paka", types(63).as_bytes()).1.is_empty());
            fail(&types(64), (1, 136), "nesting-too-deep", message);
            let src = format!(
                "fn f() {{\n  {}\"{{x}}\"{}\n}}\n",
                "[".repeat(63),
                "]".repeat(63)
            );
            fail(&src, (2, 67), "nesting-too-deep", message);
        });
    }

    #[test]
    fn braces_of_blocks_count_too() {
        deep(|| {
            let message = "nesting deeper than 64 levels";
            let blocks = |n: usize| {
                format!(
                    "fn f() {{\n  {}{}\n}}\n",
                    "if a { ".repeat(n),
                    "}".repeat(n)
                )
            };
            assert!(parse_all("main.paka", blocks(63).as_bytes()).1.is_empty());
            fail(&blocks(64), (2, 449), "nesting-too-deep", message);
        });
    }

    #[test]
    fn a_range_stands_after_in_and_inside_brackets_only() {
        unexpected("let r = 0..<3", 12, "expected end of line, found `..<`");
    }

    #[test]
    fn an_empty_match_is_an_error_at_the_brace() {
        let src = "fn f() {\n  match x {\n  }\n}\n";
        fail(src, (3, 3), "unexpected-token", "expected a match arm");
    }

    #[test]
    fn an_empty_enum_and_empty_fields_are_errors() {
        fail(
            "enum A {}\n",
            (1, 9),
            "unexpected-token",
            "expected variant name",
        );
        fail(
            "enum A { V() }\n",
            (1, 12),
            "unexpected-token",
            "expected field name",
        );
    }

    #[test]
    fn a_bare_argument_cannot_follow_a_label() {
        unexpected(
            "f(lo: 1, 2)",
            12,
            "a bare argument cannot follow a labelled one",
        );
    }

    #[test]
    fn a_let_needs_its_equals_sign() {
        unexpected("let x 1", 9, "expected `=`, found `1`");
    }

    #[test]
    fn two_statements_need_two_lines() {
        unexpected("a b", 5, "expected end of line, found `b`");
        let src = "enum A { X } enum B { Y }\n";
        fail(
            src,
            (1, 14),
            "unexpected-token",
            "expected end of line, found `enum`",
        );
    }

    #[test]
    fn a_lexical_error_is_reported_as_it_is() {
        let src = "fn f() {\n  \"abc\n}\n";
        fail(src, (2, 3), "unterminated-string", "closing quote");
    }

    // Recovery

    fn names<'a>(program: &'a Program<'a>) -> Vec<&'a str> {
        program.defs.iter().map(|d| d.name().text).collect()
    }

    #[test]
    fn a_broken_definition_leaves_the_others_parsed() {
        let src = "enum A { X }\nfn broken( {\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes());
        assert_eq!(names(&program), ["A", "B"]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].position, Some((2, 12)));
        assert!(errors[0].detailed(src.as_bytes()).contains("text: \"{\""));
    }

    #[test]
    fn a_lexical_error_in_a_body_leaves_the_next_definition() {
        let src = "fn f() {\n  \"abc\n}\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes());
        assert_eq!(names(&program), ["B"]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "unterminated-string");
    }

    #[test]
    fn a_definition_start_ends_the_broken_one() {
        let src = "enum A {\nfn f() {}\nenum {}\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes());
        assert_eq!(names(&program), ["f"]);
        let at: Vec<_> = errors.iter().map(|d| d.position).collect();
        assert_eq!(at, [Some((2, 1)), Some((3, 6))]);
    }

    #[test]
    fn an_unclosed_bracket_ends_at_the_next_definition() {
        let src = "fn f() {\n  [1, 2\n}\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes());
        assert_eq!(names(&program), ["B"]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].position, Some((3, 1)));
    }

    #[test]
    fn parse_reports_the_first_error_only() {
        let src = "enum A {\nfn f() {}\nenum {}\n";
        let error = parse("main.paka", src.as_bytes()).unwrap_err();
        assert_eq!(error.position, Some((2, 1)));
    }
}
