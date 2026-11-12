//! Recursive descent over the lexer's tokens, one token of lookahead (docs/decisions/M2-02.md
//! section 2, with M2-03, M2-05, M2-06 and M2-09). Tokens are pulled on demand, so the first lexical
//! or syntax problem of a definition, in source order, is the one reported. Where asked to
//! recover, a broken definition is skipped up to the next one.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::{Kind, Lexer, Part, Token, symbol};
use std::mem;
use std::ops::Range;

type Parse<T> = Result<T, Diagnostic>;

/// Most levels open at once: the brackets `(`, `[`, `{`, a type's `<` and an interpolation's `{`,
/// and the forms that nest with none: an `if` or `match` in a header, a `fn` type as a result.
const NESTING: usize = 64;

/// Most links in one chain of operators, calls, fields, items, `?` or prefix operators.
const CHAIN: usize = 1_000;

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

/// The program, or the first error; the rest of the file is not read.
pub fn parse<'a>(path: &'a str, src: &'a [u8]) -> Parse<Program<'a>> {
    parse_files(path, src, std::iter::once((0..src.len(), 1)))
}

/// The files of one program, which `src` holds at the given bytes, each on its given first line:
/// their definitions in that order, or the first error.
pub fn parse_files<'a>(
    path: &'a str,
    src: &'a [u8],
    files: impl IntoIterator<Item = (Range<usize>, usize)>,
) -> Parse<Program<'a>> {
    let mut program = Program::default();
    for (bytes, line) in files {
        let (file, errors) = parse_all(path, &src[..bytes.end], (bytes.start, line), false);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        program.defs.extend(file.defs);
        program.lines.push(line);
    }
    Ok(program)
}

/// The one definition `src` holds, with positions counted from its start. Its id is its bare name
/// (docs/decisions/M4-48.md).
pub fn parse_def<'a>(path: &'a str, src: &'a [u8]) -> Parse<Def<'a>> {
    let mut parser = Parser::new(path, src, Lexer::new(path, src), 0);
    let Some(def) = parser.next_def()? else {
        return Err(parser.unexpected("a definition"));
    };
    parser.skip_newlines()?;
    match parser.cur.kind {
        Kind::End => Ok(def),
        _ => Err(parser.error(
            "unexpected-token",
            "expected one definition, found a second",
        )),
    }
}

/// The migrate `src` holds from byte `start`, on `line`, to its end (docs/decisions/M5-07.md
/// section 2): `migrate(old: Old) -> New { body }`, read as a function named `migrate`.
pub fn parse_migrate<'a>(
    path: &'a str,
    src: &'a [u8],
    (start, line): (usize, usize),
) -> Parse<Def<'a>> {
    let mut parser = Parser::new(path, src, Lexer::at(path, src, start, line, start), 0);
    parser.skip_newlines()?;
    let name = parser.name(Kind::Migrate, "migrate")?;
    let def = parser.fn_rest(name)?;
    parser.skip_newlines()?;
    match parser.cur.kind {
        Kind::End => Ok(def),
        _ => Err(parser.unexpected("the end of the migrate")),
    }
}

/// Each definition of `src` as its id and its text, from its first token to its last; or the first
/// error.
pub fn split<'a>(path: &'a str, src: &'a [u8]) -> Parse<Vec<(&'a str, &'a [u8])>> {
    let mut parser = Parser::new(path, src, Lexer::new(path, src), 0);
    let mut defs = vec![];
    while let Some(def) = parser.next_def()? {
        defs.push((def.name().text, &src[parser.def_start..parser.prev_end]));
    }
    Ok(defs)
}

/// The definitions from byte `start`, on `line`, that parsed and, when `recover`, one error per
/// definition that did not; otherwise it stops at the first error.
fn parse_all<'a>(
    path: &'a str,
    src: &'a [u8],
    (start, line): (usize, usize),
    recover: bool,
) -> (Program<'a>, Vec<Diagnostic>) {
    let lexer = Lexer::at(path, src, start, line, start);
    let mut parser = Parser::new(path, src, lexer, 0);
    let (mut defs, mut errors) = (vec![], vec![]);
    loop {
        match parser.next_def() {
            Ok(Some(def)) => defs.push(def),
            Ok(None) => break,
            Err(error) => {
                errors.push(error);
                if !recover {
                    break;
                }
                parser.recover();
            }
        }
    }
    let lines = vec![line];
    (Program { defs, lines }, errors)
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
    matches!(kind, Fn | Enum | Type | App | Example | View | Foreign)
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

const LAMBDA: &str = "no function written in place; write it at top level and pass its name: \
    fn before(a: T, b: T) -> Bool { ... }; a local it would have used becomes a parameter or a field";

const NO_IF_LET: &str = "Paka has no `if let` or `while let`; match it";

const DEFAULT: &str = "`??` starts a hole; write `unwrap_or(e, d)`";

const RETURNING_ARM: &str =
    "an arm that returns is a block: write `=> { return Err(...) }` over three lines";

/// Whether a token of this kind can begin an expression.
fn starts_value(kind: Kind) -> bool {
    use Kind::*;
    let literal = matches!(kind, Int | Float | Str | True | False | Hole);
    let name = matches!(kind, Lower | Upper | If | Match);
    literal || name || matches!(kind, LParen | LBracket | Minus | Bang)
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
    /// Byte offset just after the last expression parsed.
    expr_end: Option<usize>,
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
            expr_end: None,
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
        let message = match self.cur.kind {
            Kind::Semi => "remove the ;: a line break ends the statement".into(),
            Kind::FatArrow | Kind::Arrow if self.in_expression(what) => LAMBDA.into(),
            _ => format!("expected {what}, found {}", self.describe()),
        };
        self.error("unexpected-token", message)
    }

    /// Whether the current token stands where an expression was wanted or just ended.
    fn in_expression(&self, what: &str) -> bool {
        what == "an expression" || self.expr_end == Some(self.prev_end)
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
        self.close(close)?;
        self.depth -= 1;
        Ok(inner)
    }

    /// Takes the closing `kind`; the `>` of a `>=` closes a type list and leaves the `=`.
    fn close(&mut self, kind: Kind) -> Parse<()> {
        if kind != Kind::Gt || self.cur.kind != Kind::GtEq {
            return self.eat(kind).map(drop);
        }
        let eq = Token {
            kind: Kind::Eq,
            text: &self.cur.text[1..],
            col: self.cur.col + 1,
            start: self.cur.start + 1,
            ..self.cur
        };
        self.prev_end = eq.start;
        self.cur = eq;
        Ok(())
    }

    /// `f` one level deeper when `counted`: a form with no bracket of its own that nests in another.
    fn nested<T>(&mut self, counted: bool, f: impl FnOnce(&mut Self) -> Parse<T>) -> Parse<T> {
        if !counted {
            return f(self);
        }
        if self.depth == NESTING {
            return Err(too_deep(self.path, pos(self.cur), self.cur.len()));
        }
        self.depth += 1;
        let inner = f(self)?;
        self.depth -= 1;
        Ok(inner)
    }

    /// Counts one more link of a chain, which `links` holds; the 1,001st is refused.
    fn link(&self, links: &mut usize) -> Parse<()> {
        *links += 1;
        if *links > CHAIN {
            let message = "chain longer than 1,000 links; split it with a local";
            return Err(self.error("nesting-too-deep", message));
        }
        Ok(())
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
            Kind::View => self.view_def()?,
            Kind::Foreign => self.foreign_def()?,
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
            Some(self.constant()?)
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
        self.fn_rest(name)
    }

    /// A function after its name.
    fn fn_rest(&mut self, name: Name<'a>) -> Parse<Def<'a>> {
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
        let named = matches!(key.text, b"state" | b"start" | b"update" | b"grants");
        if !(key.kind == Kind::Lower && named || key.kind == Kind::View) {
            return Err(self.unexpected("state, start, update, view or grants"));
        }
        self.advance()?;
        self.eat(Kind::Colon)?;
        let at = pos(key);
        Ok(match key.text {
            b"state" => Entry::State(at, self.name(Kind::Upper, "type")?),
            b"start" => Entry::Start(at, self.expr()?),
            b"update" => Entry::Update(at, self.name(Kind::Lower, "function")?),
            b"view" => Entry::View(at, self.name(Kind::Lower, "view")?),
            _ => {
                let grants = self.between(Kind::LBracket, Kind::RBracket, |p| {
                    p.items(&[Kind::RBracket], |p| p.name(Kind::Upper, "grant"))
                })?;
                Entry::Grants(at, grants)
            }
        })
    }

    fn view_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "view")?;
        self.type_params()?;
        let params = self.between(Kind::LParen, Kind::RParen, Self::params)?;
        self.eat(Kind::Arrow)?;
        let result = self.ty()?;
        let body = self.view_block(true)?;
        Ok(Def::View {
            name,
            params,
            result,
            body,
        })
    }

    fn foreign_def(&mut self) -> Parse<Def<'a>> {
        self.advance()?;
        let name = self.name(Kind::Upper, "foreign")?;
        let lines = self.between(Kind::LBrace, Kind::RBrace, |p| p.lines(Self::foreign_line))?;
        Ok(Def::Foreign { name, lines })
    }

    fn foreign_line(&mut self) -> Parse<ForeignLine<'a>> {
        if self.cur.kind == Kind::Upper {
            let name = self.name(Kind::Upper, "effect")?;
            let params = self.between(Kind::LParen, Kind::RParen, |p| {
                p.items(&[Kind::RParen], |p| p.param(false))
            })?;
            let result = if self.accept(Kind::Arrow)? {
                Some(self.ty()?)
            } else {
                None
            };
            self.eat(Kind::Eq)?;
            let symbol = self.quoted()?;
            return Ok(ForeignLine::Function {
                name,
                params,
                result,
                symbol,
            });
        }
        let named = matches!(self.cur.text, b"library" | b"source" | b"header");
        if !(self.cur.kind == Kind::Lower && named) {
            return Err(self.unexpected("library, source, header or an effect"));
        }
        let key = self.name(Kind::Lower, "line")?;
        self.eat(Kind::Colon)?;
        Ok(ForeignLine::File(key, self.quoted()?))
    }

    /// A string's text as written between its quotes, at its first character; escapes stay.
    fn quoted(&mut self) -> Parse<Name<'a>> {
        let token = self.cur;
        if token.kind != Kind::Str {
            return Err(self.unexpected("a quoted name"));
        }
        self.advance()?;
        Ok(Name {
            text: text(token),
            at: (token.line, token.col + 1),
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
            let counted = self.cur.kind == Kind::Fn;
            Some(Box::new(self.nested(counted, Self::ty)?))
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
            Kind::For => self.for_loop(Self::block)?,
            Kind::While => {
                self.advance()?;
                let cond = self.condition()?;
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

    /// `for name in list` and the body that `body` reads.
    fn for_loop(&mut self, body: fn(&mut Self) -> Parse<Block<'a>>) -> Parse<StmtKind<'a>> {
        self.advance()?;
        let name = self.name(Kind::Lower, "variable")?;
        self.eat(Kind::In)?;
        let list = self.with_header(true, Self::range_or_expr)?;
        let body = body(self)?;
        Ok(StmtKind::For { name, list, body })
    }

    /// A discard `_ = e`, an expression, or an assignment when an assignment operator follows it.
    fn assign_or_expr(&mut self) -> Parse<StmtKind<'a>> {
        if self.accept(Kind::Underscore)? {
            self.eat(Kind::Eq)?;
            return Ok(StmtKind::Discard(self.expr()?));
        }
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
        self.binary(0, false)
    }

    /// Left-associative operators of one level; comparisons do not chain. With `ranges`, a range
    /// stands as an operand of a comparison.
    fn binary(&mut self, level: usize, ranges: bool) -> Parse<Expr<'a>> {
        if ranges && level == ADD {
            return self.range();
        }
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let first = self.cur;
        let mut left = self.binary(level + 1, ranges)?;
        let mut links = 0;
        while let Some(&(_, op)) = ops.iter().find(|(kind, _)| *kind == self.cur.kind) {
            if level == COMPARE && links > 0 {
                let message = "comparisons do not chain; write a < b && b < c";
                return Err(self.error("unexpected-token", message));
            }
            self.link(&mut links)?;
            let at = pos(self.advance()?);
            let right = self.binary(level + 1, ranges)?;
            let kind = ExprKind::Binary(op, at, Box::new(left), Box::new(right));
            left = self.node(first, kind);
        }
        Ok(left)
    }

    /// Prefix operators, applied innermost first. A `-` right before a number is part of it.
    fn unary(&mut self) -> Parse<Expr<'a>> {
        let (mut marks, mut links) = (vec![], 0);
        let mut operand = loop {
            let first = self.cur;
            let op = match first.kind {
                Kind::Minus => UnOp::Neg,
                Kind::Bang => UnOp::Not,
                _ => break self.postfix(first)?,
            };
            self.link(&mut links)?;
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
        let mut links = 0;
        loop {
            if matches!(
                self.cur.kind,
                Kind::LParen | Kind::LBracket | Kind::Dot | Kind::Question
            ) {
                self.link(&mut links)?;
            }
            let kind = match self.cur.kind {
                Kind::LParen if !place => {
                    let args =
                        self.between(Kind::LParen, Kind::RParen, |p| p.call_args(Self::arg))?;
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
                    let mark = self.advance()?;
                    self.no_default(mark)?;
                    ExprKind::Try(Box::new(e), pos(mark))
                }
                _ => {
                    self.expr_end = Some(self.prev_end);
                    return Ok(e);
                }
            };
            e = self.node(first, kind);
        }
    }

    /// Bare arguments, then labelled ones, each read by `arg`.
    fn call_args(&mut self, arg: fn(&mut Self) -> Parse<Arg<'a>>) -> Parse<Vec<Arg<'a>>> {
        let mut labelled = false;
        self.items(&[Kind::RParen], |p| {
            let arg = arg(p)?;
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

    /// A record field: `LOWER ":" value`.
    fn field_init(&mut self, value: fn(&mut Self) -> Parse<Expr<'a>>) -> Parse<Arg<'a>> {
        let label = self.name(Kind::Lower, "field")?;
        self.eat(Kind::Colon)?;
        let value = value(self)?;
        Ok(Arg {
            label: Some(label),
            value,
        })
    }

    /// A `constant` of the grammar (M2-02 section 2): the default of a field.
    fn constant(&mut self) -> Parse<Expr<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Int | Kind::Float | Kind::True | Kind::False => return self.primary(first),
            Kind::Str if first.parts().iter().all(|p| matches!(p, Part::Text(_))) => {
                return self.primary(first);
            }
            Kind::Minus => {
                self.advance()?;
                if !matches!(self.cur.kind, Kind::Int | Kind::Float) {
                    return Err(self.unexpected("a number"));
                }
                if self.cur.start == first.start + 1 {
                    return self.primary(first);
                }
                ExprKind::Unary(UnOp::Neg, Box::new(self.primary(self.cur)?))
            }
            Kind::LBracket => self.between(Kind::LBracket, Kind::RBracket, |p| {
                Ok(match p.accept(Kind::Colon)? {
                    true => ExprKind::Map(vec![]),
                    false => ExprKind::List(vec![]),
                })
            })?,
            Kind::Upper => {
                self.advance()?;
                let name = text(first);
                match self.cur.kind {
                    Kind::LBrace => {
                        let fields = self.between(Kind::LBrace, Kind::RBrace, |p| {
                            p.items(&[Kind::RBrace], |p| p.field_init(Self::constant))
                        })?;
                        ExprKind::Record(name, fields)
                    }
                    Kind::LParen => {
                        let callee = Box::new(self.node(first, ExprKind::Name(name)));
                        let args = self.between(Kind::LParen, Kind::RParen, |p| {
                            p.call_args(Self::constant_arg)
                        })?;
                        ExprKind::Call(callee, args)
                    }
                    _ => ExprKind::Name(name),
                }
            }
            _ => return Err(self.unexpected("a constant")),
        };
        Ok(self.node(first, kind))
    }

    /// A variant argument of a constant: `[ LOWER ":" ] constant`.
    fn constant_arg(&mut self) -> Parse<Arg<'a>> {
        let name = self.cur;
        let after = &self.src[name.start + name.len()..];
        if name.kind == Kind::Lower && after.trim_ascii_start().starts_with(b":") {
            return self.field_init(Self::constant);
        }
        Ok(Arg {
            label: None,
            value: self.constant()?,
        })
    }

    /// An expression in which a range may stand; only after `in` and inside `[ ]`.
    fn range_or_expr(&mut self) -> Parse<Expr<'a>> {
        self.binary(0, true)
    }

    /// `add_expr [ "..<" add_expr ]`, binding tighter than a comparison.
    fn range(&mut self) -> Parse<Expr<'a>> {
        let first = self.cur;
        let from = self.binary(ADD, false)?;
        if !self.accept(Kind::DotDotLt)? {
            return Ok(from);
        }
        let to = self.binary(ADD, false)?;
        Ok(self.node(first, ExprKind::Range(Box::new(from), Box::new(to))))
    }

    /// Refuses `e ?? d`: a second `?` right after the first, with a value following.
    fn no_default(&self, mark: Token) -> Parse<()> {
        let doubled = self.src[..mark.start].ends_with(b"?");
        if doubled && starts_value(self.cur.kind) {
            let at = (mark.line, mark.col - 1);
            return Err(Diagnostic::new(
                self.path,
                at,
                2,
                "unexpected-token",
                DEFAULT,
            ));
        }
        Ok(())
    }

    /// At the first `,` of `(a, b) => ...`: the lambda error at its arrow (M2-09 section 2).
    fn tuple_lambda(&self) -> Option<Diagnostic> {
        let t = self.cur;
        let mut lexer = Lexer::at(self.path, self.src, t.start, t.line, t.start + 1 - t.col);
        let mut token = lexer.next().ok()?;
        while token.kind == Kind::Comma {
            (lexer.next().ok()?.kind == Kind::Lower).then_some(())?;
            token = lexer.next().ok()?;
        }
        let arrow = lexer.next().ok()?;
        let lambda = t.kind == Kind::Comma
            && token.kind == Kind::RParen
            && matches!(arrow.kind, Kind::FatArrow | Kind::Arrow);
        lambda.then(|| {
            Diagnostic::new(
                self.path,
                pos(arrow),
                arrow.len(),
                "unexpected-token",
                LAMBDA,
            )
        })
    }

    fn primary(&mut self, first: Token<'a>) -> Parse<Expr<'a>> {
        let t = self.cur;
        let kind = match t.kind {
            Kind::LParen => {
                let inner = self.between(Kind::LParen, Kind::RParen, Self::expr);
                let inner = inner.map_err(|e| self.tuple_lambda().unwrap_or(e))?;
                return Ok(self.node(first, inner.kind));
            }
            Kind::LBracket => self.collection()?,
            Kind::If => self.nested(self.header, |p| p.if_expr(Self::block))?,
            Kind::Match => self.nested(self.header, |p| p.match_expr(Self::arm_body))?,
            Kind::Pipe => return Err(self.error("unexpected-character", LAMBDA)),
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
                        p.items(&[Kind::RBrace], |p| p.field_init(Self::expr))
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
            let mut map = None;
            let rows = p.items(&[Kind::RBracket], |p| {
                let key = p.expr()?;
                if !*map.get_or_insert(p.cur.kind == Kind::Colon) {
                    return Ok((key, None));
                }
                p.eat(Kind::Colon)?;
                Ok((key, Some(p.expr()?)))
            })?;
            Ok(if map == Some(true) {
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

    /// The expression after `if` or `while`.
    fn condition(&mut self) -> Parse<Expr<'a>> {
        if self.cur.kind == Kind::Let {
            return Err(self.error("unexpected-token", NO_IF_LET));
        }
        self.with_header(true, Self::expr)
    }

    /// An `if` whose bodies `block` reads.
    fn if_expr(&mut self, block: impl Fn(&mut Self) -> Parse<Block<'a>>) -> Parse<ExprKind<'a>> {
        let mut branches = vec![];
        loop {
            self.advance()?;
            let cond = self.condition()?;
            branches.push((cond, block(self)?));
            if !self.accept(Kind::Else)? {
                return Ok(ExprKind::If {
                    branches,
                    otherwise: None,
                });
            }
            if self.cur.kind != Kind::If {
                let otherwise = Some(block(self)?);
                return Ok(ExprKind::If {
                    branches,
                    otherwise,
                });
            }
        }
    }

    /// A `match` whose arm bodies `body` reads.
    fn match_expr(
        &mut self,
        body: impl Fn(&mut Self) -> Parse<ArmBody<'a>>,
    ) -> Parse<ExprKind<'a>> {
        self.advance()?;
        let subject = Box::new(self.with_header(true, Self::expr)?);
        let arms = self.between(Kind::LBrace, Kind::RBrace, |p| {
            let arms = p.lines(|p| p.arm(&body))?;
            if arms.is_empty() {
                return Err(p.unexpected("a match arm"));
            }
            Ok(arms)
        })?;
        Ok(ExprKind::Match { subject, arms })
    }

    fn arm(&mut self, body: &impl Fn(&mut Self) -> Parse<ArmBody<'a>>) -> Parse<Arm<'a>> {
        let mut patterns = vec![self.pattern()?];
        while self.cur.kind == Kind::Pipe {
            self.binds_nothing(&patterns)?;
            self.advance()?;
            patterns.push(self.pattern()?);
        }
        if patterns.len() > 1 {
            self.binds_nothing(&patterns)?;
        }
        self.eat(Kind::FatArrow)?;
        let body = body(self)?;
        if self.cur.kind == Kind::Comma {
            let message = "remove the comma: one arm per line";
            return Err(self.error("unexpected-token", message));
        }
        Ok(Arm { patterns, body })
    }

    fn arm_body(&mut self) -> Parse<ArmBody<'a>> {
        let first = self.cur;
        if matches!(first.kind, Kind::Return | Kind::Break | Kind::Continue) {
            return Err(self.error("unexpected-token", RETURNING_ARM));
        }
        if first.kind == Kind::LBrace {
            return Ok(ArmBody::Block(self.block()?));
        }
        let kind = self.assign_or_expr()?;
        Ok(ArmBody::Line(self.node(first, kind)))
    }

    // Views (docs/decisions/M5-01.md)

    /// A view body when `root`: `let` lines, then one element expression. Otherwise a child block.
    fn view_block(&mut self, root: bool) -> Parse<Block<'a>> {
        self.between(Kind::LBrace, Kind::RBrace, |p| {
            let mut done = false;
            let stmts = p.lines(|p| {
                if done {
                    let message = format!(
                        "a view gives one element, found {} after it; \
                        wrap the elements in a Col or Row",
                        p.describe()
                    );
                    return Err(p.error("unexpected-token", message));
                }
                let line = p.view_line(root)?;
                done = root && !matches!(line.kind, StmtKind::Let { .. });
                Ok(line)
            })?;
            if root && !done {
                return Err(p.unexpected("an element"));
            }
            Ok(Block {
                stmts,
                close: pos(p.cur),
            })
        })
    }

    /// A `let`, a `for` in a child block, or an element expression.
    fn view_line(&mut self, root: bool) -> Parse<Stmt<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::Let | Kind::Else => return self.stmt(),
            Kind::For if !root => self.for_loop(|p| p.view_block(false))?,
            Kind::For => {
                let message = "a view gives one element, not a `for`; wrap it in a Col or Row";
                return Err(self.error("unexpected-token", message));
            }
            Kind::Var
            | Kind::While
            | Kind::Return
            | Kind::Break
            | Kind::Continue
            | Kind::Underscore => return Err(self.not_in_view()),
            _ => StmtKind::Expr(self.element(root)?),
        };
        if assign_op(self.cur.kind).is_some() {
            return Err(self.not_in_view());
        }
        Ok(self.node(first, kind))
    }

    fn not_in_view(&self) -> Diagnostic {
        let message = format!(
            "a view holds no {}; compute values in a function the view calls",
            self.describe()
        );
        self.error("unexpected-token", message)
    }

    /// Where an element may stand: an `if` or `match` whose bodies are view blocks, or an
    /// expression; a call takes a child block after it.
    fn element(&mut self, root: bool) -> Parse<Expr<'a>> {
        let first = self.cur;
        let kind = match first.kind {
            Kind::If => self.if_expr(|p| p.view_block(root))?,
            Kind::Match => self.match_expr(|p| p.view_arm(root))?,
            _ => {
                let e = self.expr()?;
                if self.cur.kind != Kind::LBrace || !matches!(e.kind, ExprKind::Call(..)) {
                    return Ok(e);
                }
                ExprKind::Children(Box::new(e), self.view_block(false)?)
            }
        };
        Ok(self.node(first, kind))
    }

    fn view_arm(&mut self, root: bool) -> Parse<ArmBody<'a>> {
        let first = self.cur;
        if first.kind == Kind::LBrace {
            return Ok(ArmBody::Block(self.view_block(root)?));
        }
        let e = self.element(root)?;
        Ok(ArmBody::Line(self.node(first, StmtKind::Expr(e))))
    }

    // Patterns

    /// Alternatives bind no names; `patterns` ends with the one just read.
    fn binds_nothing(&self, patterns: &[Pat<'a>]) -> Parse<()> {
        let Some(PatKind::Variant(_, parts)) = patterns.last().map(|p| &p.kind) else {
            return Ok(());
        };
        let name = parts.iter().find(|p| matches!(p.kind, PatKind::Bind(_)));
        let message = "alternatives bind no names; write one arm per variant";
        match name {
            Some(p) => Err(Diagnostic::new(
                self.path,
                p.at,
                p.len,
                "unexpected-token",
                message,
            )),
            None => Ok(()),
        }
    }

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

    /// `l op r` written with single spaces: the operator sits one space after `l`.
    fn binary<'a>(op: BinOp, l: Expr<'a>, r: Expr<'a>) -> ExprKind<'a> {
        let at = (l.at.0, l.at.1 + l.len + 1);
        ExprKind::Binary(op, at, bx(l), bx(r))
    }

    /// `x?`: the mark sits right after `x`.
    fn try_of(x: Expr) -> ExprKind {
        let at = (x.at.0, x.at.1 + x.len);
        ExprKind::Try(bx(x), at)
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
    fn a_default_is_a_constant() {
        let src = [
            "enum Color { Red, Blue(shade: Int) }",
            "type P { x: Int = 0 }",
            "type A {",
            "  a: Int = -1, b: Float = -1.5, c: String = \"hi\", d: Bool = true,",
            "  e: List<Int> = [], f: Map<String, Int> = [:], g: Color = Red, h: Color = Blue(2),",
            "  i: Option<Int> = Some(-1), j: P = P { x: 1 }, k: Option<Option<Int>> = Some(Some(1))",
            "}",
            "",
        ];
        assert_eq!(tree(&src.join("\n")).defs.len(), 3);
    }

    #[test]
    fn a_variant_default_keeps_its_arguments() {
        let src = "type A { c: Color = Blue(2, Red) }\n";
        let Def::Type { fields, .. } = &tree(src).defs[0] else {
            panic!("not a type")
        };
        let call = ExprKind::Call(
            bx(node(1, 21, 4, ExprKind::Name("Blue"))),
            vec![
                arg(node(1, 26, 1, ExprKind::Int("2"))),
                arg(node(1, 29, 3, ExprKind::Name("Red"))),
            ],
        );
        assert_eq!(fields[0].default, Some(node(1, 21, 12, call)));
    }

    /// `type A { ... }` holding `pattern` without its `^`, which marks the first token that is not
    /// part of a constant.
    fn bad_default(pattern: &str) {
        let (before, after) = pattern.split_once('^').unwrap();
        let src = format!("type A {{ {before}{after} }}\n");
        let col = "type A { ".len() + before.len() + 1;
        fail(&src, (1, col), "unexpected-token", "expected");
    }

    #[test]
    fn a_default_that_is_not_a_constant_is_refused_at_its_first_bad_token() {
        bad_default("n: Int = ^len(xs)");
        bad_default("n: Int = 1 ^+ 2");
        bad_default("n: Int = -^x");
        bad_default("n: Int = ^!true");
        bad_default("n: Int = ^(1)");
        bad_default("n: Int = 1 ^== 1");
        bad_default("xs: List<Int> = [^1, 2]");
        bad_default("xs: List<Int> = [^f(1)]");
        bad_default("m: Map<String, Int> = [^\"a\": 1]");
        bad_default("s: String = ^\"a{1}\"");
        bad_default("g: fn(Int) = ^f");
        bad_default("e: E = ^f(1)");
        bad_default("e: E = Blue(^x)");
        bad_default("e: E = Blue(1, ^f(2))");
        bad_default("e: E = Blue([^1])");
        bad_default("p: P = P { x: ^len(a) }");
        bad_default("n: Int = ^if a { 1 } else { 2 }");
        let src = "type A { n: Int = }\n";
        fail(
            src,
            (1, 19),
            "unexpected-token",
            "expected a constant, found `}`",
        );
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
        let cond = e(9, 5, binary(BinOp::Lt, var(9, "x"), int(13, "3")));
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
        let sum = binary(BinOp::Add, var(5, "a"), int(9, "1"));
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
        expr("a?", try_of(var(3, "a")));
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
            let kind = binary(op, var(3, "a"), var(code.len() + 2, "b"));
            expr(&code, kind);
        }
    }

    #[test]
    fn precedence_and_grouping() {
        use BinOp::*;
        let bin = |col, len, op, l, r| e(col, len, binary(op, l, r));
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

    fn only_arm<'x, 'a>(program: &'x Program<'a>) -> &'x ArmBody<'a> {
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        let StmtKind::Expr(Node {
            kind: ExprKind::Match { arms, .. },
            ..
        }) = &body.stmts[0].kind
        else {
            panic!("not a match")
        };
        &arms[0].body
    }

    /// An arm's line is the statement the same code makes inside a block, at the same place.
    #[test]
    fn an_arm_line_is_a_statement_of_a_block() {
        for code in ["_ = f()?", "x = 1", "xs[i] -= 1", "p.f = g()"] {
            let line = format!("fn f() {{\n  match x {{\n    V =>   {code}\n  }}\n}}\n");
            let block = format!("fn f() {{\n  match x {{\n    V => {{ {code} }}\n  }}\n}}\n");
            let (line, block) = (tree(&line), tree(&block));
            let (ArmBody::Line(s), ArmBody::Block(b)) = (only_arm(&line), only_arm(&block)) else {
                panic!("{code}: not a line and a block")
            };
            assert_eq!(s, &b.stmts[0], "{code}");
        }
    }

    #[test]
    fn alternatives_bind_no_names() {
        let message = "alternatives bind no names; write one arm per variant";
        let in_arm = |pattern: &str, name: &str| {
            let src = format!("fn f() {{\n  match x {{\n    {pattern} => 1\n  }}\n}}\n");
            let col = pattern.find(name).unwrap() + 5;
            fail(&src, (3, col), "unexpected-token", message);
        };
        in_arm("A(n) | B", "n");
        in_arm("A | B(n)", "n");
        in_arm("A(_, n) | B(m)", "n");
        in_arm("A | B | C(n, m)", "n");
        // the name comes before a later syntax error
        in_arm("A(n) | )", "n");
        let fine =
            "fn f() {\n  match x {\n    A(_) | B(_, _) | C | 1 => 1\n    D(n) => n\n  }\n}\n";
        assert!(parse("main.paka", fine.as_bytes()).is_ok());
    }

    #[test]
    fn an_upper_name_before_a_brace_ends_a_header() {
        let cond = e(6, 12, binary(BinOp::Eq, var(6, "x"), var(11, "Counter")));
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
        let cond = e(6, 17, binary(BinOp::Eq, var(6, "x"), record));
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
            (2, 5),
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
        let sum = ["a"; 1001].join(" + ");
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
        unexpected("sort_by(&ps, (a, b) => a)", 23, LAMBDA);
        unexpected("let f = (a, b, c) -> a", 21, LAMBDA);
        unexpected("let f = (a, 1) => a", 13, "expected `)`, found `,`");
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

    // Views

    fn view_body(src: &str) -> Block<'_> {
        match tree(src).defs.pop() {
            Some(Def::View { body, .. }) => body,
            _ => panic!("not a view"),
        }
    }

    fn line_of(e: Expr) -> Stmt {
        node(e.at.0, e.at.1, e.len, StmtKind::Expr(e))
    }

    #[test]
    fn a_view_with_a_child_block_a_key_and_a_hole() {
        let src = "view row(t: Todo) -> View<Msg> {
  Row(gap: 8) {
    Text(t.title, key: 1)
    ??more
  }
}
";
        let callee =
            |line, col, name: &'static str| bx(node(line, col, name.len(), ExprKind::Name(name)));
        let gap = Arg {
            label: Some(nm("gap", 2, 7)),
            value: node(2, 12, 1, ExprKind::Int("8")),
        };
        let row = node(2, 3, 11, ExprKind::Call(callee(2, 3, "Row"), vec![gap]));
        let title = ExprKind::Field(bx(node(3, 10, 1, ExprKind::Name("t"))), nm("title", 3, 12));
        let key = Arg {
            label: Some(nm("key", 3, 19)),
            value: node(3, 24, 1, ExprKind::Int("1")),
        };
        let text = ExprKind::Call(callee(3, 5, "Text"), vec![arg(node(3, 10, 7, title)), key]);
        let children = Block {
            stmts: vec![
                line_of(node(3, 5, 21, text)),
                line_of(node(4, 5, 6, ExprKind::Hole("more"))),
            ],
            close: (5, 3),
        };
        let want = Def::View {
            name: nm("row", 1, 6),
            params: vec![param(nm("t", 1, 10), ty("Todo", 1, 13))],
            result: generic("View", 1, 22, vec![ty("Msg", 1, 27)]),
            body: Block {
                stmts: vec![line_of(node(
                    2,
                    3,
                    54,
                    ExprKind::Children(bx(row), children),
                ))],
                close: (6, 1),
            },
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn a_child_block_holds_let_if_for_and_match() {
        let src = "view v(s: S) -> View<Msg> {
  let n = 1
  Col() {
    if n == 0 { ??a }
    for t in s.items {
      row(t, key: t)
    }
    match n {
      0 => Text(\"z\")
      _ => {}
    }
  }
}
";
        let name =
            |line, col, text: &'static str| node(line, col, text.len(), ExprKind::Name(text));
        let cond = ExprKind::Binary(
            BinOp::Eq,
            (4, 10),
            bx(name(4, 8, "n")),
            bx(node(4, 13, 1, ExprKind::Int("0"))),
        );
        let hole = Block {
            stmts: vec![line_of(node(4, 17, 3, ExprKind::Hole("a")))],
            close: (4, 21),
        };
        let branches = vec![(node(4, 8, 6, cond), hole)];
        let check = ExprKind::If {
            branches,
            otherwise: None,
        };
        let items = ExprKind::Field(bx(name(5, 14, "s")), nm("items", 5, 16));
        let key = Arg {
            label: Some(nm("key", 6, 14)),
            value: name(6, 19, "t"),
        };
        let call = ExprKind::Call(bx(name(6, 7, "row")), vec![arg(name(6, 11, "t")), key]);
        let each = StmtKind::For {
            name: nm("t", 5, 9),
            list: node(5, 14, 7, items),
            body: Block {
                stmts: vec![line_of(node(6, 7, 14, call))],
                close: (7, 5),
            },
        };
        let z = node(9, 17, 3, ExprKind::Str(vec![StrPart::Text("z".into())]));
        let text = ExprKind::Call(bx(name(9, 12, "Text")), vec![arg(z)]);
        let arms = vec![
            Arm {
                patterns: vec![node(9, 7, 1, PatKind::Int("0"))],
                body: ArmBody::Line(line_of(node(9, 12, 9, text))),
            },
            Arm {
                patterns: vec![node(10, 7, 1, PatKind::Wild)],
                body: ArmBody::Block(empty(10, 13)),
            },
        ];
        let choose = ExprKind::Match {
            subject: bx(name(8, 11, "n")),
            arms,
        };
        let col = node(3, 3, 5, ExprKind::Call(bx(name(3, 3, "Col")), vec![]));
        let children = Block {
            stmts: vec![
                line_of(node(4, 5, 17, check)),
                node(5, 5, 45, each),
                line_of(node(8, 5, 50, choose)),
            ],
            close: (12, 3),
        };
        let first = node(
            2,
            3,
            9,
            StmtKind::Let {
                var: false,
                name: nm("n", 2, 7),
                ty: None,
                value: node(2, 11, 1, ExprKind::Int("1")),
            },
        );
        let root = node(3, 3, 138, ExprKind::Children(bx(col), children));
        let want = Block {
            stmts: vec![first, line_of(root)],
            close: (13, 1),
        };
        assert_eq!(view_body(src), want);
    }

    #[test]
    fn the_app_names_its_view() {
        let src = "app counter {\n  view: counter_view\n}\n";
        let want = Def::App {
            name: nm("counter", 1, 5),
            entries: vec![Entry::View((2, 3), nm("counter_view", 2, 9))],
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn a_root_if_or_match_ends_each_branch_in_an_element() {
        let src = "view v(s: S) -> View<Msg> {
  match s.screen {
    Login => login_view(s)
    Home => {
      let n = 1
      home_view(s, key: n)
    }
  }
}

view w(s: S) -> View<Msg> {
  if s.ok { Text(\"yes\") } else { ??no }
}
";
        assert_eq!(tree(src).defs.len(), 2);
    }

    #[test]
    fn a_brace_after_a_call_opens_a_child_block_only_where_an_element_stands() {
        let body =
            view_body("view v() -> View<M> {\n  Col() {\n    if ok(s) { Row() {} }\n  }\n}\n");
        let StmtKind::Expr(Expr {
            kind: ExprKind::Children(_, children),
            ..
        }) = &body.stmts[0].kind
        else {
            panic!("no child block")
        };
        let StmtKind::Expr(Expr {
            kind: ExprKind::If { branches, .. },
            ..
        }) = &children.stmts[0].kind
        else {
            panic!("no if")
        };
        assert!(matches!(branches[0].0.kind, ExprKind::Call(..)));
        let src = "view v() -> View<M> {\n  Text(Col() {})\n}\n";
        fail(src, (2, 14), "unexpected-token", "expected `)`, found `{`");
        let src = "view v() -> View<M> {\n  let x = Col() {}\n}\n";
        fail(
            src,
            (2, 17),
            "unexpected-token",
            "expected end of line, found `{`",
        );
    }

    /// The first error of a view whose body holds `code` from line 2, column 3.
    fn view_fail(code: &str, at: Pos, message: &str) {
        let src = format!("view v() -> View<M> {{\n  {code}\n}}\n");
        fail(&src, at, "unexpected-token", message);
    }

    #[test]
    fn a_view_gives_one_element_after_its_lets() {
        let one = "a view gives one element, found";
        view_fail(
            "Text(\"a\")\n  Text(\"b\")",
            (3, 3),
            &format!("{one} `Text` after it"),
        );
        fail(
            "view v() -> View<M> {\n  ??a\n  let x = 1\n}\n",
            (3, 3),
            "unexpected-token",
            &format!("{one} `let` after it"),
        );
        fail(
            "view v() -> View<M> {}\n",
            (1, 22),
            "unexpected-token",
            "expected an element, found `}`",
        );
        fail(
            "view v() -> View<M> {\n  if a { let x = 1 } else { ??b }\n}\n",
            (2, 20),
            "unexpected-token",
            "expected an element, found `}`",
        );
        view_fail(
            "for x in xs {}",
            (2, 3),
            "not a `for`; wrap it in a Col or Row",
        );
        fail(
            "view v() {}\n",
            (1, 10),
            "unexpected-token",
            "expected `->`, found `{`",
        );
    }

    #[test]
    fn a_view_holds_no_state_changes_or_jumps() {
        let no = "a view holds no";
        view_fail("var x = 1", (2, 3), &format!("{no} `var`"));
        view_fail("return", (2, 3), &format!("{no} `return`"));
        view_fail("_ = f(x)", (2, 3), &format!("{no} `_`"));
        view_fail(
            "Col() {\n    while a {}\n  }",
            (3, 5),
            &format!("{no} `while`"),
        );
        view_fail("Col() {\n    break\n  }", (3, 5), &format!("{no} `break`"));
        view_fail("Col() {\n    x = 1\n  }", (3, 7), &format!("{no} `=`"));
        view_fail("Col() {\n    x += 1\n  }", (3, 7), &format!("{no} `+=`"));
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
            assert!(
                parse_all("main.paka", list(63).as_bytes(), (0, 1), true)
                    .1
                    .is_empty()
            );
            fail(&list(64), (2, 66), "nesting-too-deep", message);
            let types =
                |n: usize| format!("fn f(x: {}Int{}) {{}}\n", "L<".repeat(n), ">".repeat(n));
            assert!(
                parse_all("main.paka", types(63).as_bytes(), (0, 1), true)
                    .1
                    .is_empty()
            );
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
            assert!(
                parse_all("main.paka", blocks(63).as_bytes(), (0, 1), true)
                    .1
                    .is_empty()
            );
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
        let list = e(12, 11, binary(BinOp::Lt, var(12, "a"), range));
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
            type Chain = fn(usize) -> String;
            let kinds: [(Chain, usize); 5] = [
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

    // Generic types, function values and error handling (docs/decisions/M2-09.md, M2-12.md)

    fn call<'a>(col: usize, len: usize, name: &'a str, args: Vec<Arg<'a>>) -> Expr<'a> {
        e(col, len, ExprKind::Call(bx(var(col, name)), args))
    }

    fn tried(col: usize, len: usize, of: Expr) -> Expr {
        e(col, len, try_of(of))
    }

    fn bin<'a>(col: usize, len: usize, op: BinOp, l: Expr<'a>, r: Expr<'a>) -> Expr<'a> {
        e(col, len, binary(op, l, r))
    }

    fn string(col: usize, text: &str) -> Expr<'static> {
        let part = StrPart::Text(text.into());
        e(col, text.len() + 2, ExprKind::Str(vec![part]))
    }

    fn fn_type<'a>(at: Pos, params: Vec<Type<'a>>, result: Option<Type<'a>>) -> Type<'a> {
        let result = result.map(Box::new);
        Type::Fn { at, params, result }
    }

    /// `let NAME[: TYPE] = value`, the name in column 7.
    fn let_of<'a>(name: &'a str, ty: Option<Type<'a>>, value: Expr<'a>) -> StmtKind<'a> {
        StmtKind::Let {
            var: false,
            name: nm(name, 2, 7),
            ty,
            value,
        }
    }

    const CHAINED: &str = "comparisons do not chain; write a < b && b < c";
    const HOLE: &str = "`??` starts a hole; write `unwrap_or(e, d)`";

    #[test]
    fn nested_type_arguments_close_one_bracket_each() {
        let lists = |col| {
            let inner = generic("List", 2, col + 5, vec![ty("Int", 2, col + 10)]);
            generic("List", 2, col, vec![inner])
        };
        let map = generic("Map", 2, 10, vec![ty("String", 2, 14), lists(22)]);
        let value = e(41, 3, ExprKind::Map(vec![]));
        stmt(
            "let g: Map<String, List<List<Int>>> = [:]",
            let_of("g", Some(map), value),
        );
        let value = e(27, 2, ExprKind::List(vec![]));
        stmt(
            "let g: List<List<Int>>= []",
            let_of("g", Some(lists(10)), value),
        );
    }

    #[test]
    fn option_and_result_types() {
        let src = "fn f(a: Option<Int>) -> Result<Int, String> {}\n";
        let option = generic("Option", 1, 9, vec![ty("Int", 1, 16)]);
        let result = generic("Result", 1, 25, vec![ty("Int", 1, 32), ty("String", 1, 37)]);
        let want = Def::Fn {
            name: nm("f", 1, 4),
            params: vec![param(nm("a", 1, 6), option)],
            result: Some(result),
            body: empty(1, 46),
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn a_function_type_as_a_field() {
        let src = "type Job { done: fn(Int) -> Msg }\n\
                   enum Msg { Later(then: fn(String) -> Msg) }\n";
        let done = fn_type((1, 18), vec![ty("Int", 1, 21)], Some(ty("Msg", 1, 29)));
        let then = fn_type((2, 24), vec![ty("String", 2, 27)], Some(ty("Msg", 2, 38)));
        let want = vec![
            Def::Type {
                name: nm("Job", 1, 6),
                fields: vec![plain(nm("done", 1, 12), done)],
            },
            Def::Enum {
                name: nm("Msg", 2, 6),
                variants: vec![Variant {
                    name: nm("Later", 2, 12),
                    fields: vec![plain(nm("then", 2, 18), then)],
                }],
            },
        ];
        assert_eq!(tree(src).defs, want);
    }

    #[test]
    fn a_function_type_as_a_list_item_and_a_map_value() {
        let item = fn_type((2, 16), vec![ty("Int", 2, 19)], Some(ty("Int", 2, 27)));
        let value = e(34, 2, ExprKind::List(vec![]));
        stmt(
            "let fs: List<fn(Int) -> Int> = []",
            StmtKind::Let {
                var: false,
                name: nm("fs", 2, 7),
                ty: Some(generic("List", 2, 11, vec![item])),
                value,
            },
        );
        let args = vec![ty("String", 2, 14), fn_type((2, 22), vec![], None)];
        let value = e(30, 3, ExprKind::Map(vec![]));
        stmt(
            "let m: Map<String, fn()> = [:]",
            let_of("m", Some(generic("Map", 2, 10, args)), value),
        );
    }

    #[test]
    fn function_types_inside_function_types() {
        let src = "fn pick(k: fn(fn(Int) -> Bool, List<Int>) -> fn()) -> fn(Int) -> Int {}\n";
        let test = fn_type((1, 15), vec![ty("Int", 1, 18)], Some(ty("Bool", 1, 26)));
        let list = generic("List", 1, 32, vec![ty("Int", 1, 37)]);
        let k = fn_type(
            (1, 12),
            vec![test, list],
            Some(fn_type((1, 46), vec![], None)),
        );
        let result = fn_type((1, 55), vec![ty("Int", 1, 58)], Some(ty("Int", 1, 66)));
        let want = Def::Fn {
            name: nm("pick", 1, 4),
            params: vec![param(nm("k", 1, 9), k)],
            result: Some(result),
            body: empty(1, 71),
        };
        assert_eq!(tree(src).defs, vec![want]);
    }

    #[test]
    fn types_print_in_the_canonical_spelling() {
        let src =
            "fn f(a: fn(Int,String)->Bool, b: fn(Int), c: fn(), d: Map<String,List<Int>>) {}\n";
        let program = tree(src);
        let Def::Fn { params, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        let spelled: Vec<_> = params.iter().map(|p| p.ty.to_string()).collect();
        let want = [
            "fn(Int, String) -> Bool",
            "fn(Int)",
            "fn()",
            "Map<String, List<Int>>",
        ];
        assert_eq!(spelled, want);
    }

    #[test]
    fn a_broken_type_list_is_an_error_at_its_token() {
        unexpected("let x: List<> = []", 15, "expected type, found `>`");
        unexpected("let x: List<Int,> = []", 19, "remove the comma before >");
        unexpected("let x: List<Int = []", 19, "expected `>`, found `=`");
        unexpected("let x: List<Int>> = []", 19, "expected `=`, found `>`");
        let case = "type names start with an upper-case letter: write Int";
        unexpected("let x: List<int> = []", 15, case);
        unexpected("let f: fn<T>(T) = g", 12, "expected `(`, found `<`");
        let src = "fn f() {\n  let x: List<Int\n  > = []\n}\n";
        let message = "expected `>`, found end of line";
        fail(src, (2, 18), "unexpected-token", message);
    }

    #[test]
    fn no_inout_anywhere_in_a_function_type() {
        let src = "fn f(g: fn(Int, inout Int)) {}\n";
        let message = "no inout in a function type";
        fail(src, (1, 17), "unexpected-token", message);
    }

    #[test]
    fn every_interface_word_is_refused_at_a_definition_start() {
        let message = "Paka has no interfaces; write one function per type and pass it by name: \
            sort_by(&xs, before)";
        for word in ["interface", "trait", "protocol", "impl", "extension"] {
            let src = format!("{word} Shape {{ fn area(s: Shape) -> Float }}\n");
            fail(&src, (1, 1), "unexpected-token", message);
        }
        let value = int(15, "1");
        stmt("let trait = 1", let_of("trait", None, value));
    }

    #[test]
    fn a_refused_interface_leaves_the_next_definition() {
        let src = "interface Shape {\n  fn area(s: Shape) -> Float\n}\nfn f() {}\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes(), (0, 1), true);
        assert_eq!(names(&program), ["f"]);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn type_parameters_are_refused_at_the_bracket() {
        let message =
            "Paka has no type parameters; write it for one type: fn first(xs: List<Int>) -> Int";
        let src = "fn first<T>(xs: List<T>) -> T { xs[0] }\n";
        fail(src, (1, 9), "unexpected-token", message);
        fail("fn first <T>() {}\n", (1, 10), "unexpected-token", message);
        fail(
            "type Pair<A, B> { a: A }\n",
            (1, 10),
            "unexpected-token",
            message,
        );
    }

    #[test]
    fn a_bracket_in_an_expression_is_a_comparison() {
        let lt = |col| bin(col, 5, BinOp::Lt, var(col, "a"), var(col + 4, "b"));
        let gt = |col, d| bin(col, d - col + 1, BinOp::Gt, var(col, "c"), var(d, "d"));
        let args = vec![arg(lt(5)), arg(gt(12, 16))];
        expr("f(a < b, c > d)", ExprKind::Call(bx(var(3, "f")), args));
        expr("[a < b, c > d]", ExprKind::List(vec![lt(4), gt(11, 15)]));
        let grouped = e(16, 3, ExprKind::Name("d"));
        let wide = bin(12, 7, BinOp::Gt, var(12, "c"), grouped);
        let args = vec![arg(lt(5)), arg(wide)];
        expr("f(a < b, c > (d))", ExprKind::Call(bx(var(3, "f")), args));
    }

    #[test]
    fn type_arguments_in_an_expression_are_refused() {
        unexpected("f<Int>(x)", 8, CHAINED);
        unexpected("a < b > c", 9, CHAINED);
        unexpected("Some<Int>(3)", 11, CHAINED);
        unexpected("f<List<Int>>(x)", 9, CHAINED);
        unexpected("if a < b > c { 1 }", 12, CHAINED);
        unexpected("a >> b", 6, "expected an expression, found `>`");
        unexpected("f::<Int>(x)", 4, "expected end of line, found `:`");
        let comma = "expected end of line, found `,`";
        unexpected("let e = Err<Int, String>(\"no\")", 18, comma);
    }

    #[test]
    fn a_comparison_stands_in_a_header_without_brackets() {
        let block = |col| Block {
            stmts: vec![node(2, col, 1, StmtKind::Expr(int(col, "1")))],
            close: (2, col + 2),
        };
        let cond = bin(6, 5, BinOp::Lt, var(6, "a"), var(10, "b"));
        let kind = ExprKind::If {
            branches: vec![(cond, block(14))],
            otherwise: None,
        };
        expr("if a < b { 1 }", kind);
        let cond = bin(6, 9, BinOp::Gt, var(6, "n"), var(10, "Limit"));
        let kind = ExprKind::If {
            branches: vec![(cond, block(18))],
            otherwise: None,
        };
        expr("if n > Limit { 1 }", kind);
        let len = call(13, 7, "len", vec![arg(var(17, "xs"))]);
        let cond = bin(9, 11, BinOp::Lt, var(9, "i"), len);
        let body = empty(2, 23);
        stmt("while i < len(xs) { }", StmtKind::While { cond, body });
    }

    #[test]
    fn a_comparison_and_a_try_stand_in_an_interpolation() {
        let lt = |col| bin(col, 5, BinOp::Lt, var(col, "a"), var(col + 4, "b"));
        expr(r#""{a < b}""#, ExprKind::Str(vec![StrPart::Expr(lt(5))]));
        let gt = bin(14, 5, BinOp::Gt, var(14, "c"), var(18, "d"));
        let f = call(5, 15, "f", vec![arg(lt(7)), arg(gt)]);
        expr(
            r#""{f(a < b, c > d)}""#,
            ExprKind::Str(vec![StrPart::Expr(f)]),
        );
        let get = call(5, 9, "get", vec![arg(var(9, "m")), arg(var(12, "k"))]);
        let part = StrPart::Expr(tried(5, 10, get));
        expr(r#""{get(m, k)?}""#, ExprKind::Str(vec![part]));
        unexpected(r#""{f<Int>(x)}""#, 10, CHAINED);
        unexpected(r#""{a ?? 0}""#, 7, HOLE);
    }

    #[test]
    fn a_trailing_comparison_continues_the_line_and_a_leading_one_does_not() {
        let src = "fn f() {\n  a <\n    b\n}\n";
        let b = node(3, 5, 1, ExprKind::Name("b"));
        let less = binary(BinOp::Lt, var(3, "a"), b);
        let want = node(2, 3, 9, StmtKind::Expr(node(2, 3, 9, less)));
        let program = tree(src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        assert_eq!(body.stmts, vec![want]);
        let src = "fn f() {\n  a\n  < b\n}\n";
        let message = "expected an expression, found `<`";
        fail(src, (3, 3), "unexpected-token", message);
    }

    #[test]
    fn a_function_value_is_a_bare_name() {
        let out = e(11, 4, ExprKind::Inout(bx(var(12, "out"))));
        let args = vec![arg(out), arg(var(17, "shorter"))];
        expr(
            "sort_by(&out, shorter)",
            ExprKind::Call(bx(var(3, "sort_by")), args),
        );
        let args = vec![
            arg(var(12, "p")),
            labelled("then", 15, var(21, "Loaded")),
            labelled("or", 29, var(33, "Failed")),
        ];
        expr(
            "ReadFile(p, then: Loaded, or: Failed)",
            ExprKind::Call(bx(var(3, "ReadFile")), args),
        );
        stmt("let f = by_length", let_of("f", None, var(11, "by_length")));
        let same = bin(3, 16, BinOp::Eq, var(3, "Loaded"), var(13, "Failed"));
        expr("Loaded == Failed", same.kind);
        let list = ExprKind::List(vec![var(4, "Print"), var(11, "Exit")]);
        expr("[Print, Exit]", list);
    }

    #[test]
    fn a_call_through_an_item_or_a_field() {
        let item = e(3, 5, ExprKind::Index(bx(var(3, "xs")), bx(int(6, "0"))));
        expr("xs[0](y)", ExprKind::Call(bx(item), vec![arg(var(9, "y"))]));
        let field = ExprKind::Field(bx(var(3, "s")), nm("on_done", 2, 5));
        let args = vec![arg(var(13, "x"))];
        expr("s.on_done(x)", ExprKind::Call(bx(e(3, 9, field)), args));
    }

    #[test]
    fn a_function_written_in_place_is_refused_at_its_first_mark() {
        fail_in("let f = |x| x", 11, "unexpected-character", LAMBDA);
        fail_in("|x| x", 3, "unexpected-character", LAMBDA);
        let src = "fn f() {\n  sort_by(&ps, |a, b|\n    a.score > b.score)\n}\n";
        fail(src, (2, 16), "unexpected-character", LAMBDA);
        let src = "fn f() {\n  let f = x =>\n    x\n}\n";
        fail(src, (2, 13), "unexpected-token", LAMBDA);
    }

    #[test]
    fn try_binds_with_calls_items_and_fields() {
        let parse_int = |col| call(col, 12, "parse_int", vec![arg(var(col + 10, "s"))]);
        expr("parse_int(s)?", tried(3, 13, parse_int(3)).kind);
        let item = e(3, 5, ExprKind::Index(bx(var(3, "xs")), bx(var(6, "i"))));
        expr("xs[i]?", try_of(item));
        let f = tried(3, 5, call(3, 4, "f", vec![arg(var(5, "x"))]));
        expr("f(x)?.field", ExprKind::Field(bx(f), nm("field", 2, 9)));
        let p = tried(3, 2, var(3, "p"));
        expr("p?.x", ExprKind::Field(bx(p), nm("x", 2, 6)));
        let h = tried(5, 5, call(5, 4, "h", vec![arg(var(7, "x"))]));
        expr("g(h(x)?)?", tried(3, 9, call(3, 8, "g", vec![arg(h)])).kind);
        let ok = call(3, 17, "Ok", vec![arg(tried(6, 13, parse_int(6)))]);
        expr("Ok(parse_int(s)?)", ok.kind);
        expr("a??", try_of(tried(3, 2, var(3, "a"))));
    }

    #[test]
    fn try_binds_tighter_than_every_operator() {
        let a = |col| tried(col, 2, var(col, "a"));
        expr("-a?", ExprKind::Unary(UnOp::Neg, bx(a(4))));
        expr("!a?", ExprKind::Unary(UnOp::Not, bx(a(4))));
        let b = tried(8, 2, var(8, "b"));
        expr("a? + b?", bin(3, 7, BinOp::Add, a(3), b).kind);
        let parsed = call(16, 12, "parse_int", vec![arg(var(26, "f"))]);
        let args = vec![arg(parsed), arg(string(30, "no"))];
        let value = tried(10, 26, call(10, 25, "ok_or", args));
        let (target, op) = (var(3, "sum"), AssignOp::Add);
        stmt(
            "sum += ok_or(parse_int(f), \"no\")?",
            StmtKind::Assign { target, op, value },
        );
    }

    #[test]
    fn a_try_at_a_line_end_ends_the_statement() {
        let args = vec![arg(var(17, "a")), arg(var(20, "b"))];
        let ratio = tried(11, 12, call(11, 11, "ratio", args));
        let half = ExprKind::Binary(
            BinOp::Div,
            (3, 8),
            bx(node(3, 6, 1, ExprKind::Name("r"))),
            bx(node(3, 10, 1, ExprKind::Int("2"))),
        );
        let ok = ExprKind::Call(
            bx(node(3, 3, 2, ExprKind::Name("Ok"))),
            vec![arg(node(3, 6, 5, half))],
        );
        let want = vec![
            node(2, 3, 20, let_of("r", None, ratio)),
            node(3, 3, 9, StmtKind::Expr(node(3, 3, 9, ok))),
        ];
        stmts("let r = ratio(a, b)?\n  Ok(r / 2)", want);
        unexpected("a? b", 6, "expected end of line, found `b`");
        let src = "fn f() {\n  f()?\n  .x\n}\n";
        let message = "expected an expression, found `.`";
        fail(src, (3, 3), "unexpected-token", message);
    }

    #[test]
    fn a_question_mark_is_not_an_expression() {
        unexpected("let x = ?", 11, "expected an expression, found `?`");
    }

    #[test]
    fn two_question_marks_between_two_values_are_refused() {
        unexpected("let v = get(m, k) ?? 0", 21, HOLE);
        unexpected("let v = x ?? d", 13, HOLE);
        unexpected("let v = x ?? (a + b)", 13, HOLE);
        unexpected("let v = x ?? []", 13, HOLE);
        unexpected("let v = x ?? -1", 13, HOLE);
        unexpected("let v = x ?? \"none\"", 13, HOLE);
        unexpected("let v = x??0", 12, HOLE);
        unexpected("let v = x ??d", 13, "expected end of line, found `??d`");
    }

    #[test]
    fn every_start_of_a_value_after_two_question_marks_is_refused() {
        unexpected("let v = x ?? !b", 13, HOLE);
        unexpected("let v = x ?? D", 13, HOLE);
        unexpected("let v = x ?? true", 13, HOLE);
        unexpected("let v = x ?? 1.5", 13, HOLE);
        unexpected("let v = x ?? if c { 1 } else { 2 }", 13, HOLE);
        unexpected("let v = x ?? match y { A => 1 }", 13, HOLE);
        unexpected("let v = x ?? ??h", 13, HOLE);
    }

    #[test]
    fn the_refusal_of_two_question_marks_marks_both() {
        let src = "fn f() {\n  x ?? 0\n}\n";
        let d = parse("main.paka", src.as_bytes()).unwrap_err();
        assert!(d.detailed(src.as_bytes()).contains("text: \"??\""));
    }

    #[test]
    fn two_question_marks_before_a_call_an_item_or_a_minus_are_refused() {
        unexpected("a??(x)", 4, HOLE);
        unexpected("a??[0]", 4, HOLE);
        unexpected("a?? - 1", 4, HOLE);
        unexpected("a??-1", 4, HOLE);
    }

    #[test]
    fn two_question_marks_before_an_operator_a_field_or_in_brackets_are_accepted() {
        let twice = |col| e(col, 3, try_of(tried(col, 2, var(col, "a"))));
        expr("a?? + 1", bin(3, 7, BinOp::Add, twice(3), int(9, "1")).kind);
        expr("a??.x", ExprKind::Field(bx(twice(3)), nm("x", 2, 7)));
        let grouped = e(3, 5, try_of(tried(4, 2, var(4, "a"))));
        expr(
            "(a??)(x)",
            ExprKind::Call(bx(grouped), vec![arg(var(9, "x"))]),
        );
    }

    #[test]
    fn constructors_are_names_and_calls() {
        let option = generic("Option", 2, 10, vec![ty("Int", 2, 17)]);
        stmt(
            "let a: Option<Int> = None",
            let_of("a", Some(option), var(24, "None")),
        );
        let err = call(10, 21, "Err", vec![arg(string(14, "divide by zero"))]);
        stmt(
            "return Err(\"divide by zero\")",
            StmtKind::Return(Some(err)),
        );
        let ratio = bin(6, 5, BinOp::Div, var(6, "a"), var(10, "b"));
        expr("Ok(a / b)", call(3, 9, "Ok", vec![arg(ratio)]).kind);
        let block = |col, value: Expr<'static>, close| Block {
            stmts: vec![node(2, col, value.len, StmtKind::Expr(value))],
            close: (2, close),
        };
        let some = call(19, 7, "Some", vec![arg(var(24, "n"))]);
        let kind = ExprKind::If {
            branches: vec![(var(14, "ok"), block(19, some, 27))],
            otherwise: Some(block(36, var(36, "None"), 41)),
        };
        stmt(
            "let b = if ok { Some(n) } else { None }",
            let_of("b", None, e(11, 31, kind)),
        );
    }

    #[test]
    fn unreachable_is_a_call() {
        expr("unreachable()", call(3, 13, "unreachable", vec![]).kind);
        let args = vec![arg(string(15, "why"))];
        expr(
            "unreachable(\"why\")",
            call(3, 18, "unreachable", args).kind,
        );
    }

    #[test]
    fn a_match_on_an_option_or_a_result() {
        let src = "fn f() {
  match get(m, k) {
    Some(v) => v
    None => {
      return Err(\"missing\")
    }
    Err(_) => unreachable()
  }
}
";
        let name =
            |line, col, text: &'static str| node(line, col, text.len(), ExprKind::Name(text));
        let line = |value: Expr<'static>| {
            ArmBody::Line(node(
                value.at.0,
                value.at.1,
                value.len,
                StmtKind::Expr(value),
            ))
        };
        let variant = |line, len, name, binds| node(line, 5, len, PatKind::Variant(name, binds));
        let arm = |pattern, body| Arm {
            patterns: vec![pattern],
            body,
        };
        let missing = node(
            5,
            18,
            9,
            ExprKind::Str(vec![StrPart::Text("missing".into())]),
        );
        let err = ExprKind::Call(bx(name(5, 14, "Err")), vec![arg(missing)]);
        let leave = Block {
            stmts: vec![node(5, 7, 21, StmtKind::Return(Some(node(5, 14, 14, err))))],
            close: (6, 5),
        };
        let unreachable = ExprKind::Call(bx(name(7, 15, "unreachable")), vec![]);
        let arms = vec![
            arm(
                variant(3, 7, "Some", vec![node(3, 10, 1, PatKind::Bind("v"))]),
                line(name(3, 16, "v")),
            ),
            arm(variant(4, 4, "None", vec![]), ArmBody::Block(leave)),
            arm(
                variant(7, 6, "Err", vec![node(7, 9, 1, PatKind::Wild)]),
                line(node(7, 15, 13, unreachable)),
            ),
        ];
        let args = vec![arg(name(2, 13, "m")), arg(name(2, 16, "k"))];
        let subject = bx(node(2, 9, 9, ExprKind::Call(bx(name(2, 9, "get")), args)));
        let kind = ExprKind::Match { subject, arms };
        let want = node(2, 3, 114, StmtKind::Expr(node(2, 3, 114, kind)));
        let program = tree(src);
        let Def::Fn { body, .. } = &program.defs[0] else {
            panic!("not a fn")
        };
        assert_eq!(body.stmts, vec![want]);
    }

    #[test]
    fn an_arm_that_returns_is_a_block() {
        let src = "fn f() {\n  match x {\n    None => return Err(\"missing\")\n  }\n}\n";
        let message =
            "an arm that returns is a block: write `=> { return Err(...) }` over three lines";
        fail(src, (3, 13), "unexpected-token", message);
    }

    #[test]
    fn an_arm_that_breaks_or_continues_is_a_block() {
        let message =
            "an arm that returns is a block: write `=> { return Err(...) }` over three lines";
        for (word, col) in [("break", 13), ("continue", 13)] {
            let src = format!("fn f() {{\n  match x {{\n    None => {word}\n  }}\n}}\n");
            fail(&src, (3, col), "unexpected-token", message);
        }
    }

    #[test]
    fn a_wrong_arrow_outside_an_expression_is_a_plain_error() {
        let arm = "fn f() {\n  match x {\n    A -> 1\n  }\n}\n";
        fail(arm, (3, 7), "unexpected-token", "expected `=>`, found `->`");
        let fat = "expected `=`, found `=>`";
        unexpected("let g: fn(Int) => Int = h", 18, fat);
        let long = "expected `=`, found `->`";
        unexpected("let g: fn(Int) -> Int -> Int = h", 25, long);
        let header = "expected `{`, found `=>`";
        fail("fn f() => Int {\n}\n", (1, 8), "unexpected-token", header);
    }

    #[test]
    fn a_pattern_is_flat() {
        let src = "fn f() {\n  match x {\n    Some(Ok(v)) => v\n  }\n}\n";
        let message = "expected a name or `_`, found `Ok`";
        fail(src, (3, 10), "unexpected-token", message);
    }

    #[test]
    fn an_if_takes_no_let() {
        let message = "Paka has no `if let` or `while let`; match it";
        unexpected("if let Some(v) = get(m, k) { }", 6, message);
        unexpected("while let Some(v) = get(m, k) { }", 9, message);
        unexpected("if a { } else if let Some(v) = g { }", 20, message);
    }

    #[test]
    fn the_two_decision_samples_define_these_names() {
        let src = r#"enum Msg { Start(path: String), Loaded(text: String), Failed(why: String) }

type Job { done: fn(Int) -> Msg }

fn run(j: Job) -> Msg { j.done(3) }

fn shorter(a: String, b: String) -> Bool { len(a) < len(b) || len(a) == len(b) && a < b }

fn by_length(ws: List<String>) -> List<String> {
  var out = ws
  sort_by(&out, shorter)
  out
}

fn ratio(a: Int, b: Int) -> Result<Int, String> {
  if b == 0 {
    return Err("divide by zero")
  }
  Ok(a / b)
}

fn half_ratio(a: Int, b: Int) -> Result<Int, String> {
  let r = ratio(a, b)?
  Ok(r / 2)
}

fn total(fields: List<String>) -> Result<Int, String> {
  var sum = 0
  for f in fields {
    sum += ok_or(parse_int(f), "not a number: {f}")?
  }
  Ok(sum)
}

fn sign_name(n: Int) -> String {
  assert(n >= -1 && n <= 1)
  match n {
    -1 => "minus"
    0 => "zero"
    1 => "plus"
    _ => unreachable()
  }
}

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path) => [ReadFile(path, then: Loaded, or: Failed)]
    Loaded(text) => match total(records(text)) {
      Ok(sum) => [Print("{sum}"), Exit]
      Err(why) => [Print(why), ExitWith(20)]
    }
    Failed(why) => [Print("cannot read: {why}"), ExitWith(20)]
  }
}

fn count_words(words: List<String>) -> Map<String, Int> {
  var counts: Map<String, Int> = [:]
  for w in words {
    counts[w] = unwrap_or(get(counts, w), 0) + 1
  }
  counts
}

fn first_two(words: List<String>) -> Option<String> {
  let a = first_word(words)?
  let b = first_word(words[1..<len(words)])?
  Some("{a} {b}")
}

example total_rejects_bad_score {
  total([["a", "3"], ["b", "x"]]) == Err("line 2: bad score x")
}
"#;
        let program = tree(src);
        let want = ["Msg", "Job", "run", "shorter", "by_length", "ratio"];
        let rest = ["half_ratio", "total", "sign_name", "update", "count_words"];
        let last = ["first_two", "total_rejects_bad_score"];
        let all: Vec<_> = want.into_iter().chain(rest).chain(last).collect();
        assert_eq!(names(&program), all);
    }

    // Recovery

    fn names<'a>(program: &'a Program<'a>) -> Vec<&'a str> {
        program.defs.iter().map(|d| d.name().text).collect()
    }

    #[test]
    fn a_broken_definition_leaves_the_others_parsed() {
        let src = "enum A { X }\nfn broken( {\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes(), (0, 1), true);
        assert_eq!(names(&program), ["A", "B"]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].position, Some((2, 12)));
        assert!(errors[0].detailed(src.as_bytes()).contains("text: \"{\""));
    }

    #[test]
    fn a_lexical_error_in_a_body_leaves_the_next_definition() {
        let src = "fn f() {\n  \"abc\n}\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes(), (0, 1), true);
        assert_eq!(names(&program), ["B"]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "unterminated-string");
    }

    #[test]
    fn a_definition_start_ends_the_broken_one() {
        let src = "enum A {\nfn f() {}\nenum {}\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes(), (0, 1), true);
        assert_eq!(names(&program), ["f"]);
        let at: Vec<_> = errors.iter().map(|d| d.position).collect();
        assert_eq!(at, [Some((2, 1)), Some((3, 6))]);
    }

    #[test]
    fn an_unclosed_bracket_ends_at_the_next_definition() {
        let src = "fn f() {\n  [1, 2\n}\nenum B { Y }\n";
        let (program, errors) = parse_all("main.paka", src.as_bytes(), (0, 1), true);
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

    // Single definitions

    const DEFS: &str = "enum Mode { Up, Down }

fn step(x: Int, m: Mode) -> Int {
  match m {
    Up => x + 1
    Down => x - 1
  }
}

example steps_up {
  step(1, Up) == 2
}
";

    fn parts(src: &str) -> Vec<(&str, &str)> {
        let defs = split("main.paka", src.as_bytes()).unwrap();
        let text = |t| std::str::from_utf8(t).unwrap();
        defs.into_iter().map(|(id, t)| (id, text(t))).collect()
    }

    fn join(parts: &[(&str, &str)]) -> String {
        let texts: Vec<_> = parts.iter().map(|p| p.1).collect();
        texts.join("\n\n") + "\n"
    }

    fn ids<'a>(parts: &[(&'a str, &str)]) -> Vec<&'a str> {
        parts.iter().map(|p| p.0).collect()
    }

    #[test]
    fn ids_survive_body_edits_and_reordering() {
        let edited = DEFS.replace("x + 1", "x + 2");
        assert_eq!(ids(&parts(&edited)), ["Mode", "step", "steps_up"]);
        let mut reversed = parts(DEFS);
        reversed.reverse();
        assert_eq!(ids(&parts(&join(&reversed))), ["steps_up", "step", "Mode"]);
    }

    #[test]
    fn a_program_rebuilt_from_its_definitions_is_the_same() {
        let parts = parts(DEFS);
        assert_eq!(join(&parts), DEFS);
        for (id, text) in parts {
            let def = parse_def("main.paka", text.as_bytes()).unwrap();
            assert_eq!(def.name().text, id);
        }
    }

    #[test]
    fn one_definition_reparses_alone() {
        let before = parts(DEFS);
        let step = before[1].1.replace("x - 1", "x - 2");
        let alone = parse_def("main.paka", step.as_bytes()).unwrap();
        let edited = DEFS.replace("x - 1", "x - 2");
        let after = parts(&edited);
        assert_eq!(after[1].1, step);
        assert_eq!(
            parse_def("main.paka", after[1].1.as_bytes()).unwrap(),
            alone
        );
        assert_eq!((after[0], after[2]), (before[0], before[2]));
    }

    #[test]
    fn a_single_definition_is_exactly_one() {
        let error = |src: &str| parse_def("main.paka", src.as_bytes()).unwrap_err();
        assert_eq!(error("\n").position, Some((2, 1)));
        let second = error("enum A { X }\n\nenum B { Y }\n");
        assert_eq!(
            (second.code, second.position),
            ("unexpected-token", Some((3, 1)))
        );
        assert_eq!(error("enum A { X } x").position, Some((1, 14)));
    }

    #[test]
    fn foreign_parses_and_is_reserved() {
        unexpected("let foreign = 1", 7, "found `foreign`");
        fail(
            "foreign Sum {\n  source: sum.c\n}\n",
            (2, 11),
            "unexpected-token",
            "",
        );
        let program = parse("main.paka", b"foreign Sum {\n  Add(a: Int) = \"add\"\n}\n");
        assert!(
            matches!(&program.unwrap().defs[0], Def::Foreign { lines, .. } if lines.len() == 1)
        );
    }
}
