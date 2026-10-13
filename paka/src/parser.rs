//! Recursive descent over the lexer's tokens, one token of lookahead (docs/decisions/M2-02.md
//! section 2, with M2-03, M2-05, M2-06 and M2-09). Tokens are pulled on demand, so the first lexical
//! or syntax problem of a definition, in source order, is the one reported. A broken definition is
//! skipped up to the next one.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::{Kind, Lexer, Part, Token, symbol};
use std::mem;

type Parse<T> = Result<T, Diagnostic>;

/// Most brackets open at once: `(`, `[`, `{`, a type's `<` and an interpolation's `{`.
const NESTING: usize = 64;

const INTERFACES: [&str; 5] = ["interface", "trait", "protocol", "impl", "extension"];

/// Binary operators from the loosest level to the tightest.
const LEVELS: [&[(Kind, BinOp)]; 5] = [
    &[(Kind::OrOr, BinOp::Or)],
    &[(Kind::AndAnd, BinOp::And)],
    &[
        (Kind::EqEq, BinOp::Eq),
        (Kind::NotEq, BinOp::Ne),
        (Kind::Lt, BinOp::Lt),
        (Kind::LtEq, BinOp::Le),
        (Kind::Gt, BinOp::Gt),
        (Kind::GtEq, BinOp::Ge),
    ],
    &[(Kind::Plus, BinOp::Add), (Kind::Minus, BinOp::Sub)],
    &[
        (Kind::Star, BinOp::Mul),
        (Kind::Slash, BinOp::Div),
        (Kind::Percent, BinOp::Rem),
    ],
];
const COMPARE: usize = 2;
const ADD: usize = 3;

/// The first error only.
pub fn parse<'a>(path: &'a str, src: &'a [u8]) -> Parse<Program<'a>> {
    let (program, errors) = parse_all(path, src);
    errors.into_iter().next().map_or(Ok(program), Err)
}

/// The definitions that parsed and one error per definition that did not.
fn parse_all<'a>(path: &'a str, src: &'a [u8]) -> (Program<'a>, Vec<Diagnostic>) {
    let mut parser = Parser::new(path, src, Lexer::new(path, src), 0);
    let (mut defs, mut errors) = (vec![], vec![]);
    loop {
        match parser.next_def() {
            Ok(Some(def)) => defs.push(def),
            Ok(None) => return (Program { defs }, errors),
            Err(error) => {
                errors.push(error);
                parser.recover();
            }
        }
    }
}

fn text<'a>(token: Token<'a>) -> &'a str {
    std::str::from_utf8(token.text).expect("the lexer yields UTF-8")
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("the lexer yields UTF-8")
}

fn pos(token: Token) -> Pos {
    (token.line, token.col)
}

fn definition_start(kind: Kind) -> bool {
    use Kind::*;
    matches!(kind, Fn | Enum | Type | App | Example | View)
}

fn assign_op(kind: Kind) -> Option<AssignOp> {
    Some(match kind {
        Kind::Eq => AssignOp::Set,
        Kind::PlusEq => AssignOp::Add,
        Kind::MinusEq => AssignOp::Sub,
        Kind::StarEq => AssignOp::Mul,
        Kind::SlashEq => AssignOp::Div,
        Kind::PercentEq => AssignOp::Rem,
        _ => return None,
    })
}

fn too_deep(path: &str, at: Pos, len: usize) -> Diagnostic {
    let message = format!(
        "nesting deeper than {NESTING} levels; move the inner part into a named function or local"
    );
    Diagnostic::new(path, at, len, "nesting-too-deep", message)
}

struct Parser<'a> {
    path: &'a str,
    src: &'a [u8],
    lexer: Lexer<'a>,
    cur: Token<'a>,
    /// Byte offset just after the last token taken.
    prev_end: usize,
    /// Brackets open at the current token.
    depth: usize,
    /// In an `if`, `while`, `for` or `match` header, where `Upper {` ends the header.
    header: bool,
    /// Byte offset of the definition being parsed.
    def_start: usize,
}

impl<'a> Parser<'a> {
    /// Call `advance` once to load the first token.
    fn new(path: &'a str, src: &'a [u8], lexer: Lexer<'a>, depth: usize) -> Self {
        let cur = Token {
            kind: Kind::Newline,
            text: b"",
            line: 1,
            col: 1,
            start: 0,
        };
        Parser {
            path,
            src,
            lexer,
            cur,
            prev_end: 0,
            depth,
            header: false,
            def_start: 0,
        }
    }

    /// Takes the current token and lexes the next one.
    fn advance(&mut self) -> Parse<Token<'a>> {
        self.prev_end = self.cur.start + self.cur.len();
        let next = self.lexer.next()?;
        Ok(mem::replace(&mut self.cur, next))
    }

    fn skip_newlines(&mut self) -> Parse<()> {
        while self.cur.kind == Kind::Newline {
            self.advance()?;
        }
        Ok(())
    }

    fn accept(&mut self, kind: Kind) -> Parse<bool> {
        let hit = self.cur.kind == kind;
        if hit {
            self.advance()?;
        }
        Ok(hit)
    }

    fn eat(&mut self, kind: Kind) -> Parse<Token<'a>> {
        if self.cur.kind != kind {
            return Err(self.unexpected(&format!("`{}`", symbol(kind))));
        }
        self.advance()
    }

    /// A name of `kind` (`Upper` or `Lower`) for a `noun` such as "type" or "parameter".
    fn name(&mut self, kind: Kind, noun: &str) -> Parse<Name<'a>> {
        let token = self.cur;
        let word = text(token);
        let wrong_case = matches!(
            (kind, token.kind),
            (Kind::Upper, Kind::Lower) | (Kind::Lower, Kind::Upper)
        );
        if wrong_case {
            let (rule, fixed) = match kind {
                Kind::Upper => ("an upper", word[..1].to_uppercase() + &word[1..]),
                _ => ("a lower", word[..1].to_lowercase() + &word[1..]),
            };
            let message = format!("{noun} names start with {rule}-case letter: write {fixed}");
            return Err(self.error("unexpected-token", message));
        }
        if token.kind != kind {
            return Err(self.unexpected(&format!("{noun} name")));
        }
        self.advance()?;
        Ok(Name {
            text: word,
            at: pos(token),
        })
    }

    fn describe(&self) -> String {
        match self.cur.kind {
            Kind::Newline => "end of line".into(),
            Kind::End => "end of file".into(),
            _ => {
                let bytes = &self.src[self.cur.start..][..self.cur.len()];
                format!("`{}`", String::from_utf8_lossy(bytes))
            }
        }
    }

    fn error(&self, code: &'static str, message: impl ToString) -> Diagnostic {
        Diagnostic::new(self.path, pos(self.cur), self.cur.len(), code, message)
    }

    /// The error for the current token, which fits nowhere; `what` is what the grammar wanted.
    fn unexpected(&self, what: &str) -> Diagnostic {
        if what == "end of line" && matches!(self.cur.kind, Kind::FatArrow | Kind::Arrow) {
            let message = "no function written in place; write it at top level and pass its name: \
                fn before(a: T, b: T) -> Bool { ... }; a local it would have used becomes a \
                parameter or a field";
            return self.error("unexpected-token", message);
        }
        let message = format!("expected {what}, found {}", self.describe());
        self.error("unexpected-token", message)
    }

    fn unsupported_view(&self) -> Diagnostic {
        self.error("unsupported-view", "`view` is not supported")
    }

    /// The text from byte `from` to the last token taken.
    fn spelling(&self, from: usize) -> &'a str {
        let src: &'a [u8] = self.src;
        std::str::from_utf8(&src[from..self.prev_end]).expect("the lexer yields UTF-8")
    }

    /// A node that starts at `first` and ends with the last token taken.
    fn node<K>(&self, first: Token, kind: K) -> Node<K> {
        Node {
            at: pos(first),
            len: self.prev_end - first.start,
            kind,
        }
    }

    fn with_header<T>(&mut self, on: bool, f: impl FnOnce(&mut Self) -> Parse<T>) -> Parse<T> {
        let saved = mem::replace(&mut self.header, on);
        let result = f(self);
        self.header = saved;
        result
    }

    /// `open body close`, one level deeper and out of any header.
    fn between<T>(
        &mut self,
        open: Kind,
        close: Kind,
        body: impl FnOnce(&mut Self) -> Parse<T>,
    ) -> Parse<T> {
        if self.cur.kind == open && self.depth == NESTING {
            return Err(too_deep(self.path, pos(self.cur), self.cur.len()));
        }
        self.eat(open)?;
        self.depth += 1;
        let inner = self.with_header(false, body)?;
        self.eat(close)?;
        self.depth -= 1;
        Ok(inner)
    }

    /// `[ item { "," item } ]` up to, not including, one of `closers`; no comma before it.
    fn items<T>(
        &mut self,
        closers: &[Kind],
        mut item: impl FnMut(&mut Self) -> Parse<T>,
    ) -> Parse<Vec<T>> {
        let mut items = vec![];
        while !closers.contains(&self.cur.kind) {
            items.push(item(self)?);
            if !self.accept(Kind::Comma)? {
                break;
            }
            if closers.contains(&self.cur.kind) {
                let message = format!("remove the comma before {}", symbol(self.cur.kind));
                return Err(self.error("unexpected-token", message));
            }
        }
        Ok(items)
    }

    /// `items` with at least one item.
    fn items1<T>(
        &mut self,
        close: Kind,
        what: &str,
        item: impl FnMut(&mut Self) -> Parse<T>,
    ) -> Parse<Vec<T>> {
        if self.cur.kind == close {
            return Err(self.unexpected(what));
        }
        self.items(&[close], item)
    }

    /// Items on lines of their own, up to the closing `}`.
    fn lines<T>(&mut self, mut item: impl FnMut(&mut Self) -> Parse<T>) -> Parse<Vec<T>> {
        let mut items = vec![];
        loop {
            self.skip_newlines()?;
            if matches!(self.cur.kind, Kind::RBrace | Kind::End) {
                return Ok(items);
            }
            items.push(item(self)?);
            match self.cur.kind {
                Kind::Newline | Kind::RBrace | Kind::End => {}
                Kind::Semi => {
                    let message = "remove the ;: a line break ends the statement";
                    return Err(self.error("unexpected-token", message));
                }
                _ => return Err(self.unexpected("end of line")),
            }
        }
    }

    // Program

    fn next_def(&mut self) -> Parse<Option<Def<'a>>> {
        self.skip_newlines()?;
        if self.cur.kind == Kind::End {
            return Ok(None);
        }
        self.def_start = self.cur.start;
        let def = match self.cur.kind {
            Kind::Type => self.type_def()?,
            Kind::Enum => self.enum_def()?,
            Kind::Fn => self.fn_def()?,
            Kind::Example => self.example_def()?,
            Kind::App => self.app_def()?,
            Kind::View => return Err(self.unsupported_view()),
            Kind::Lower if INTERFACES.contains(&text(self.cur)) => {
                let message = "Paka has no interfaces; write one function per type and pass it by \
                    name: sort_by(&xs, before)";
                return Err(self.error("unexpected-token", message));
            }
            _ => return Err(self.unexpected("a definition")),
        };
        match self.cur.kind {
            Kind::Newline | Kind::End => Ok(Some(def)),
            _ => Err(self.unexpected("end of line")),
        }
    }

    /// Skips to the next definition: a definition keyword in column 1 that is past the start of
    /// the broken one.
    fn recover(&mut self) {
        while self.cur.kind != Kind::End {
            let fresh = self.cur.col == 1 && self.cur.start > self.def_start;
            if fresh && definition_start(self.cur.kind) {
                break;
            }
            match self.lexer.next() {
                Ok(token) => self.cur = token,
                Err(_) => self.lexer.skip_byte(),
            }
        }
        self.lexer.reset_brackets();
        (self.depth, self.header) = (0, false);
    }

    // Definitions

    fn type_params(&self) -> Parse<()> {
        if self.cur.kind != Kind::Lt {
            return Ok(());
        }
        let message = "Paka has no type parameters; write it for one type: \
            fn first(xs: List<Int>) -> Int";
        Err(self.error("unexpected-token", message))
    }

    fn type_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Upper, "type")?;
        self.type_params()?;
        let fields = self.between(Kind::LBrace, Kind::RBrace, |p| {
            p.items(&[Kind::RBrace], |p| p.field_def(true))
        })?;
        Ok(Def::Type { name, fields })
    }

    fn field_def(&mut self, with_default: bool) -> Parse<Field<'a>> {
        let name = self.name(Kind::Lower, "field")?;
        self.eat(Kind::Colon)?;
        let ty = self.ty()?;
        let default = if with_default && self.accept(Kind::Eq)? {
            Some(self.expr()?)
        } else {
            None
        };
        Ok(Field { name, ty, default })
    }

    fn enum_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Upper, "type")?;
        self.type_params()?;
        let variants = self.between(Kind::LBrace, Kind::RBrace, |p| {
            p.items1(Kind::RBrace, "variant name", Self::variant)
        })?;
        Ok(Def::Enum { name, variants })
    }

    fn variant(&mut self) -> Parse<Variant<'a>> {
        let name = self.name(Kind::Upper, "variant")?;
        let fields = if self.cur.kind == Kind::LParen {
            self.between(Kind::LParen, Kind::RParen, |p| {
                p.items1(Kind::RParen, "field name", |p| p.field_def(false))
            })?
        } else {
            vec![]
        };
        Ok(Variant { name, fields })
    }

    fn fn_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "function")?;
        self.type_params()?;
        let params = self.between(Kind::LParen, Kind::RParen, Self::params)?;
        let result = if self.accept(Kind::Arrow)? {
            Some(self.ty()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(Def::Fn {
            name,
            params,
            result,
            body,
        })
    }

    /// Bare parameters, then after a `;` the labelled ones.
    fn params(&mut self) -> Parse<Vec<Param<'a>>> {
        let mut params = self.items(&[Kind::RParen, Kind::Semi], |p| p.param(false))?;
        if self.accept(Kind::Semi)? {
            let labelled = self.items1(Kind::RParen, "labelled parameter", |p| p.param(true))?;
            params.extend(labelled);
        }
        Ok(params)
    }

    fn param(&mut self, labelled: bool) -> Parse<Param<'a>> {
        let name = self.name(Kind::Lower, "parameter")?;
        self.eat(Kind::Colon)?;
        let inout = self.accept(Kind::Inout)?;
        let ty = self.ty()?;
        Ok(Param {
            name,
            inout,
            labelled,
            ty,
        })
    }

    fn example_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "example")?;
        let body = self.block()?;
        Ok(Def::Example { name, body })
    }

    fn app_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "app")?;
        let entries = self.between(Kind::LBrace, Kind::RBrace, |p| p.lines(Self::entry))?;
        Ok(Def::App { name, entries })
    }

    fn entry(&mut self) -> Parse<Entry<'a>> {
        let key = self.cur;
        if key.kind == Kind::View {
            return Err(self.unsupported_view());
        }
        if !matches!(key.text, b"state" | b"start" | b"update" | b"grants") {
            return Err(self.unexpected("state, start, update or grants"));
        }
        self.advance()?;
        self.eat(Kind::Colon)?;
        let at = pos(key);
        Ok(match key.text {
            b"state" => Entry::State(at, self.name(Kind::Upper, "type")?),
            b"start" => Entry::Start(at, self.expr()?),
            b"update" => Entry::Update(at, self.name(Kind::Lower, "function")?),
            _ => {
                let grants = self.between(Kind::LBracket, Kind::RBracket, |p| {
                    p.items(&[Kind::RBracket], |p| p.name(Kind::Upper, "grant"))
                })?;
                Entry::Grants(at, grants)
            }
        })
    }

    // Types

    fn ty(&mut self) -> Parse<Type<'a>> {
        if self.cur.kind == Kind::Fn {
            return self.fn_type();
        }
        let name = self.name(Kind::Upper, "type")?;
        let args = if self.cur.kind == Kind::Lt {
            self.between(Kind::Lt, Kind::Gt, |p| p.items1(Kind::Gt, "type", Self::ty))?
        } else {
            vec![]
        };
        Ok(Type::Name { name, args })
    }

    fn fn_type(&mut self) -> Parse<Type<'a>> {
        let at = pos(self.advance()?);
        let params = self.between(Kind::LParen, Kind::RParen, |p| {
            p.items(&[Kind::RParen], |p| match p.cur.kind {
                Kind::Inout => Err(p.error("unexpected-token", "no inout in a function type")),
                _ => p.ty(),
            })
        })?;
        let result = if self.accept(Kind::Arrow)? {
            Some(Box::new(self.ty()?))
        } else {
            None
        };
        Ok(Type::Fn { at, params, result })
    }

    // Statements

    fn block(&mut self) -> Parse<Block<'a>> {
        self.between(Kind::LBrace, Kind::RBrace, |p| {
            let stmts = p.lines(Self::stmt)?;
            Ok(Block {
                stmts,
                close: pos(p.cur),
            })
        })
    }

    fn stmt(&mut self) -> Parse<Stmt<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Let | Kind::Var => {
                self.advance()?;
                let name = self.name(Kind::Lower, "variable")?;
                let ty = if self.accept(Kind::Colon)? {
                    Some(self.ty()?)
                } else {
                    None
                };
                self.eat(Kind::Eq)?;
                let value = self.expr()?;
                StmtKind::Let {
                    var: first.kind == Kind::Var,
                    name,
                    ty,
                    value,
                }
            }
            Kind::Underscore => {
                self.advance()?;
                self.eat(Kind::Eq)?;
                StmtKind::Discard(self.expr()?)
            }
            Kind::Return => {
                self.advance()?;
                let bare = matches!(self.cur.kind, Kind::Newline | Kind::RBrace | Kind::End);
                StmtKind::Return(if bare { None } else { Some(self.expr()?) })
            }
            Kind::Break | Kind::Continue => {
                self.advance()?;
                match first.kind {
                    Kind::Break => StmtKind::Break,
                    _ => StmtKind::Continue,
                }
            }
            Kind::For => {
                self.advance()?;
                let name = self.name(Kind::Lower, "variable")?;
                self.eat(Kind::In)?;
                let list = self.with_header(true, Self::range_or_expr)?;
                let body = self.block()?;
                StmtKind::For { name, list, body }
            }
            Kind::While => {
                self.advance()?;
                let cond = self.with_header(true, Self::expr)?;
                let body = self.block()?;
                StmtKind::While { cond, body }
            }
            Kind::Else => {
                return Err(self.error("unexpected-token", "put else on the line of the }"));
            }
            _ => self.assign_or_expr()?,
        };
        Ok(self.node(first, kind))
    }

    /// An expression, or an assignment when an assignment operator follows it.
    fn assign_or_expr(&mut self) -> Parse<StmtKind<'a>> {
        let target = self.expr()?;
        let Some(op) = assign_op(self.cur.kind) else {
            return Ok(StmtKind::Expr(target));
        };
        self.advance()?;
        let value = self.expr()?;
        Ok(StmtKind::Assign { target, op, value })
    }

    // Expressions

    fn expr(&mut self) -> Parse<Expr<'a>> {
        self.binary(0)
    }

    /// Left-associative operators of one level; comparisons do not chain.
    fn binary(&mut self, level: usize) -> Parse<Expr<'a>> {
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let first = self.cur;
        let mut left = self.binary(level + 1)?;
        let mut folded = false;
        while let Some(&(_, op)) = ops.iter().find(|(kind, _)| *kind == self.cur.kind) {
            if level == COMPARE && folded {
                let message = "comparisons do not chain; write a < b && b < c";
                return Err(self.error("unexpected-token", message));
            }
            self.advance()?;
            let right = self.binary(level + 1)?;
            left = self.node(first, ExprKind::Binary(op, Box::new(left), Box::new(right)));
            folded = true;
        }
        Ok(left)
    }

    /// Prefix operators, applied innermost first. A `-` right before a number is part of it.
    fn unary(&mut self) -> Parse<Expr<'a>> {
        let mut marks = vec![];
        let mut operand = loop {
            let first = self.cur;
            let op = match first.kind {
                Kind::Minus => UnOp::Neg,
                Kind::Bang => UnOp::Not,
                _ => break self.postfix(first)?,
            };
            self.advance()?;
            let number = matches!(self.cur.kind, Kind::Int | Kind::Float);
            if op == UnOp::Neg && number && self.cur.start == first.start + 1 {
                break self.postfix(first)?;
            }
            marks.push((first, op));
        };
        for (first, op) in marks.into_iter().rev() {
            operand = self.node(first, ExprKind::Unary(op, Box::new(operand)));
        }
        Ok(operand)
    }

    fn postfix(&mut self, first: Token<'a>) -> Parse<Expr<'a>> {
        let operand = self.primary(first)?;
        self.suffixes(first, operand, false)
    }

    /// Calls, items, fields and `?`; a place takes only fields and items.
    fn suffixes(&mut self, first: Token<'a>, mut e: Expr<'a>, place: bool) -> Parse<Expr<'a>> {
        loop {
            let kind = match self.cur.kind {
                Kind::LParen if !place => {
                    let args = self.between(Kind::LParen, Kind::RParen, Self::call_args)?;
                    ExprKind::Call(Box::new(e), args)
                }
                Kind::LBracket => {
                    let index =
                        self.between(Kind::LBracket, Kind::RBracket, Self::range_or_expr)?;
                    ExprKind::Index(Box::new(e), Box::new(index))
                }
                Kind::Dot => {
                    self.advance()?;
                    ExprKind::Field(Box::new(e), self.name(Kind::Lower, "field")?)
                }
                Kind::Question if !place => {
                    self.advance()?;
                    ExprKind::Try(Box::new(e))
                }
                _ => return Ok(e),
            };
            e = self.node(first, kind);
        }
    }

    /// Bare arguments, then labelled ones.
    fn call_args(&mut self) -> Parse<Vec<Arg<'a>>> {
        let mut labelled = false;
        self.items(&[Kind::RParen], |p| {
            let arg = p.arg()?;
            labelled |= arg.label.is_some();
            if arg.label.is_none() && labelled {
                let (at, len) = (arg.value.at, arg.value.len);
                let message = "a bare argument cannot follow a labelled one; write the labels last";
                return Err(Diagnostic::new(
                    p.path,
                    at,
                    len,
                    "unexpected-token",
                    message,
                ));
            }
            Ok(arg)
        })
    }

    /// `[ LOWER ":" ] [ "&" place | expr ]`; a bare lower-case name before `:` is the label.
    fn arg(&mut self) -> Parse<Arg<'a>> {
        let value = self.inout_or_expr()?;
        let label = match value.kind {
            ExprKind::Name(n) if value.len == n.len() && n.starts_with(char::is_lowercase) => n,
            _ => return Ok(Arg { label: None, value }),
        };
        if self.cur.kind != Kind::Colon {
            return Ok(Arg { label: None, value });
        }
        self.advance()?;
        let label = Name {
            text: label,
            at: value.at,
        };
        Ok(Arg {
            label: Some(label),
            value: self.inout_or_expr()?,
        })
    }

    fn inout_or_expr(&mut self) -> Parse<Expr<'a>> {
        let amp = self.cur;
        if amp.kind != Kind::Amp {
            return self.expr();
        }
        self.advance()?;
        let first = self.cur;
        let root = self.name(Kind::Lower, "variable")?;
        let root = self.node(first, ExprKind::Name(root.text));
        let place = self.suffixes(first, root, true)?;
        Ok(self.node(amp, ExprKind::Inout(Box::new(place))))
    }

    /// A record field: `LOWER ":" expr`.
    fn field_init(&mut self) -> Parse<Arg<'a>> {
        let label = self.name(Kind::Lower, "field")?;
        self.eat(Kind::Colon)?;
        let value = self.expr()?;
        Ok(Arg {
            label: Some(label),
            value,
        })
    }

    /// `a..<b` or an expression; only after `in` and inside `[ ]`.
    fn range_or_expr(&mut self) -> Parse<Expr<'a>> {
        let first = self.cur;
        let from = self.expr()?;
        if !self.accept(Kind::DotDotLt)? {
            return Ok(from);
        }
        let to = self.binary(ADD)?;
        Ok(self.node(first, ExprKind::Range(Box::new(from), Box::new(to))))
    }

    fn primary(&mut self, first: Token<'a>) -> Parse<Expr<'a>> {
        let t = self.cur;
        let kind = match t.kind {
            Kind::LParen => {
                let inner = self.between(Kind::LParen, Kind::RParen, Self::expr)?;
                return Ok(self.node(first, inner.kind));
            }
            Kind::LBracket => self.collection()?,
            Kind::If => self.if_expr()?,
            Kind::Match => self.match_expr()?,
            Kind::Str => self.string(t)?,
            Kind::Int | Kind::Float | Kind::True | Kind::False | Kind::Hole | Kind::Lower => {
                self.advance()?;
                match t.kind {
                    Kind::Int => ExprKind::Int(self.spelling(first.start)),
                    Kind::Float => ExprKind::Float(self.spelling(first.start)),
                    Kind::Hole => ExprKind::Hole(&text(t)[2..]),
                    Kind::Lower => ExprKind::Name(text(t)),
                    _ => ExprKind::Bool(t.kind == Kind::True),
                }
            }
            Kind::Upper => {
                self.advance()?;
                if self.cur.kind == Kind::LBrace && !self.header {
                    let fields = self.between(Kind::LBrace, Kind::RBrace, |p| {
                        p.items(&[Kind::RBrace], Self::field_init)
                    })?;
                    ExprKind::Record(text(t), fields)
                } else {
                    ExprKind::Name(text(t))
                }
            }
            Kind::Amp => {
                let message = "& is written only on an argument of a call";
                return Err(self.error("unexpected-inout-mark", message));
            }
            _ => return Err(self.unexpected("an expression")),
        };
        Ok(self.node(first, kind))
    }

    /// A list, a map or `[:]`.
    fn collection(&mut self) -> Parse<ExprKind<'a>> {
        self.between(Kind::LBracket, Kind::RBracket, |p| {
            if p.accept(Kind::Colon)? {
                return Ok(ExprKind::Map(vec![]));
            }
            let rows = p.items(&[Kind::RBracket], |p| {
                let key = p.expr()?;
                let value = if p.accept(Kind::Colon)? {
                    Some(p.expr()?)
                } else {
                    None
                };
                Ok((key, value))
            })?;
            let map = rows.first().is_some_and(|row| row.1.is_some());
            if let Some((odd, _)) = rows.iter().find(|row| row.1.is_some() != map) {
                let message = "a list item and a map row cannot mix";
                let error = Diagnostic::new(p.path, odd.at, odd.len, "unexpected-token", message);
                return Err(error);
            }
            Ok(if map {
                let pairs = rows.into_iter().filter_map(|(k, v)| Some((k, v?)));
                ExprKind::Map(pairs.collect())
            } else {
                ExprKind::List(rows.into_iter().map(|row| row.0).collect())
            })
        })
    }

    fn string(&mut self, token: Token<'a>) -> Parse<ExprKind<'a>> {
        let mut parts = vec![];
        for part in token.parts() {
            parts.push(match part {
                Part::Text(bytes) => StrPart::Text(utf8(bytes)),
                Part::Expr { start, text } => {
                    StrPart::Expr(self.interpolation(token, start, text.len())?)
                }
            });
        }
        self.advance()?;
        Ok(ExprKind::Str(parts))
    }

    /// The expression of the `len` bytes at `start` in the string `token`, one level deeper.
    fn interpolation(&self, token: Token, start: usize, len: usize) -> Parse<Expr<'a>> {
        let line_start = token.start - (token.col - 1);
        if self.depth == NESTING {
            return Err(too_deep(self.path, (token.line, start - line_start), 1));
        }
        let whole: &'a [u8] = self.src;
        let src = &whole[..start + len + 1];
        let lexer = Lexer::at(self.path, src, start, token.line, line_start);
        let mut inner = Parser::new(self.path, src, lexer, self.depth + 1);
        inner.advance()?;
        let e = inner.expr()?;
        inner.eat(Kind::RBrace)?;
        Ok(e)
    }

    fn if_expr(&mut self) -> Parse<ExprKind<'a>> {
        let mut branches = vec![];
        loop {
            self.advance()?;
            let cond = self.with_header(true, Self::expr)?;
            branches.push((cond, self.block()?));
            if !self.accept(Kind::Else)? {
                return Ok(ExprKind::If {
                    branches,
                    otherwise: None,
                });
            }
            if self.cur.kind != Kind::If {
                let otherwise = Some(self.block()?);
                return Ok(ExprKind::If {
                    branches,
                    otherwise,
                });
            }
        }
    }

    fn match_expr(&mut self) -> Parse<ExprKind<'a>> {
        self.advance()?;
        let subject = Box::new(self.with_header(true, Self::expr)?);
        let arms = self.between(Kind::LBrace, Kind::RBrace, |p| {
            let arms = p.lines(Self::arm)?;
            if arms.is_empty() {
                return Err(p.unexpected("a match arm"));
            }
            Ok(arms)
        })?;
        Ok(ExprKind::Match { subject, arms })
    }

    fn arm(&mut self) -> Parse<Arm<'a>> {
        let mut patterns = vec![self.pattern()?];
        while self.accept(Kind::Pipe)? {
            patterns.push(self.pattern()?);
        }
        self.eat(Kind::FatArrow)?;
        let first = self.cur;
        let body = if first.kind == Kind::LBrace {
            ArmBody::Block(self.block()?)
        } else {
            let kind = self.assign_or_expr()?;
            ArmBody::Line(self.node(first, kind))
        };
        if self.cur.kind == Kind::Comma {
            let message = "remove the comma: one arm per line";
            return Err(self.error("unexpected-token", message));
        }
        Ok(Arm { patterns, body })
    }

    // Patterns

    fn pattern(&mut self) -> Parse<Pat<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Underscore => {
                self.advance()?;
                PatKind::Wild
            }
            Kind::Minus | Kind::Int => {
                let signed = first.kind == Kind::Minus;
                if signed {
                    self.advance()?;
                }
                if self.cur.kind != Kind::Int || (signed && self.cur.start != first.start + 1) {
                    return Err(self.unexpected("an integer"));
                }
                self.advance()?;
                PatKind::Int(self.spelling(first.start))
            }
            Kind::Str => {
                let mut bytes = vec![];
                for part in first.parts() {
                    match part {
                        Part::Text(run) => bytes.extend(run),
                        Part::Expr { .. } => {
                            let message = "a string pattern has no { }";
                            return Err(self.error("unexpected-token", message));
                        }
                    }
                }
                self.advance()?;
                PatKind::Str(utf8(bytes))
            }
            Kind::True | Kind::False => {
                self.advance()?;
                PatKind::Bool(first.kind == Kind::True)
            }
            Kind::Upper => {
                self.advance()?;
                let binds = if self.cur.kind == Kind::LParen {
                    self.between(Kind::LParen, Kind::RParen, |p| {
                        p.items1(Kind::RParen, "a name or `_`", Self::binding)
                    })?
                } else {
                    vec![]
                };
                PatKind::Variant(text(first), binds)
            }
            _ => return Err(self.unexpected("a pattern")),
        };
        Ok(self.node(first, kind))
    }

    fn binding(&mut self) -> Parse<Pat<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Underscore => PatKind::Wild,
            Kind::Lower => PatKind::Bind(text(first)),
            _ => return Err(self.unexpected("a name or `_`")),
        };
        self.advance()?;
        Ok(self.node(first, kind))
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
        parse("main.paka", src.as_bytes()).unwrap()
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
        let d = parse("main.paka", src.as_bytes()).unwrap_err();
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

    const LAMBDA: &str = "no function written in place; write it at top level and pass its name: fn before(a: T, b: T) -> Bool { ... }; a local it would have used becomes a parameter or a field";
    const SEMICOLON: &str = "remove the ;: a line break ends the statement";

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
        unexpected("x;", 4, SEMICOLON);
    }

    #[test]
    fn a_semicolon_is_refused_wherever_it_stands() {
        unexpected("Print(m;)", 10, SEMICOLON);
        unexpected("[1; 2]", 5, SEMICOLON);
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
        unexpected("let f = x => x", 13, LAMBDA);
        unexpected("let f = (x) -> x", 15, LAMBDA);
    }

    #[test]
    fn a_lambda_in_a_call_is_refused_where_it_starts_or_arrows() {
        let bar = "sort_by(&ps, |a, b| a.score > b.score)";
        fail_in(bar, 16, "unexpected-character", LAMBDA);
        unexpected("sort_by(&ps, a => a)", 18, LAMBDA);
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

    /// Runs `f` on the 8 MiB of a program's main thread; a debug test thread has only 2 MiB.
    fn deep(f: impl FnOnce() + Send) {
        std::thread::scope(|s| {
            let run = std::thread::Builder::new().stack_size(8 << 20);
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

    #[test]
    fn an_app_key_is_a_name_not_a_string() {
        let src = "app a {\n  \"start\": Start\n}\n";
        fail(src, (2, 3), "unexpected-token", "expected state, start");
    }

    #[test]
    fn a_label_after_a_list_item_is_unexpected_at_the_colon() {
        unexpected("[Exit, then: Exit]", 14, "expected `]`, found `:`");
        unexpected("[1: 2, 3]", 11, "expected `:`, found `]`");
    }

    #[test]
    fn a_range_binds_tighter_than_a_comparison() {
        let range = e(16, 7, ExprKind::Range(bx(var(16, "b")), bx(var(22, "c"))));
        let list = e(
            12,
            11,
            ExprKind::Binary(BinOp::Lt, bx(var(12, "a")), bx(range)),
        );
        let name = nm("i", 2, 7);
        let body = empty(2, 26);
        stmt(
            "for i in a < b ..< c { }",
            StmtKind::For { name, list, body },
        );
    }

    #[test]
    fn a_type_list_closes_before_an_equals_sign() {
        stmt(
            "let x: List<Int>= []",
            StmtKind::Let {
                var: false,
                name: nm("x", 2, 7),
                ty: Some(generic("List", 2, 10, vec![ty("Int", 2, 15)])),
                value: e(21, 2, ExprKind::List(vec![])),
            },
        );
    }

    #[test]
    fn an_if_or_match_in_a_header_is_a_level() {
        deep(|| {
            let message = "nesting deeper than 64 levels";
            let chain = |word: &str, n: usize| {
                let (open, close) = match word {
                    "if" => ("if ", " { 1 } else { 2 }"),
                    _ => ("match ", " { _ => 1 }"),
                };
                format!("fn f() {{\n  {}a{}\n}}\n", open.repeat(n), close.repeat(n))
            };
            for (word, width) in [("if", 3), ("match", 6)] {
                assert!(parse("main.paka", chain(word, 63).as_bytes()).is_ok());
                fail(
                    &chain(word, 65),
                    (2, 3 + 64 * width),
                    "nesting-too-deep",
                    message,
                );
                fail(
                    &chain(word, 1_800),
                    (2, 3 + 64 * width),
                    "nesting-too-deep",
                    message,
                );
            }
        });
    }

    #[test]
    fn a_function_type_as_a_result_is_a_level() {
        deep(|| {
            let message = "nesting deeper than 64 levels";
            let chain = |n: usize| format!("fn f() -> {}fn() {{}}\n", "fn() -> ".repeat(n - 1));
            assert!(parse("main.paka", chain(64).as_bytes()).is_ok());
            fail(&chain(65), (1, 525), "nesting-too-deep", message);
            fail(&chain(22_000), (1, 525), "nesting-too-deep", message);
        });
    }

    #[test]
    fn a_chain_stops_at_1000_links() {
        deep(|| {
            let message = "chain longer than 1,000 links; split it with a local";
            let kinds: [(fn(usize) -> String, usize); 5] = [
                (|n| format!("{}a", "-".repeat(n)), 1003),
                (|n| format!("a{}", "+a".repeat(n)), 2004),
                (|n| format!("a{}", ".b".repeat(n)), 2004),
                (|n| format!("a{}", "()".repeat(n)), 2004),
                (|n| format!("a{}", "?".repeat(n)), 1004),
            ];
            for (chain, col) in kinds {
                let ok = format!("fn f() {{\n  {}\n}}\n", chain(1_000));
                assert!(parse("main.paka", ok.as_bytes()).is_ok());
                fail_in(&chain(1_001), col, "nesting-too-deep", message);
                fail_in(&chain(130_000), col, "nesting-too-deep", message);
            }
        });
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
