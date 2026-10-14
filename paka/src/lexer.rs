//! Source bytes to tokens (docs/decisions/M2-02.md sections 1.5 to 1.9, M2-03.md sections 1.8 to
//! 1.12 and 2.7). Comments are skipped. A line feed becomes a `Newline` token by the rule of M2-02
//! section 1.5.

use crate::diagnostic::Diagnostic;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    App,
    Break,
    Continue,
    Else,
    Enum,
    Example,
    False,
    Fn,
    For,
    If,
    In,
    Inout,
    Let,
    Match,
    Migrate,
    Return,
    True,
    Type,
    Var,
    View,
    While,
    Upper,
    Lower,
    Int,
    Float,
    Str,
    /// `??` and the name after it.
    Hole,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    NotEq,
    AndAnd,
    OrOr,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    DotDotLt,
    Amp,
    Question,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Colon,
    Comma,
    Semi,
    Dot,
    Arrow,
    FatArrow,
    Pipe,
    Underscore,
    Newline,
    End,
}

/// `text` is the spelling; for `Str` it is the source between the quotes, escapes undecoded.
#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub kind: Kind,
    pub text: &'a [u8],
    pub line: usize,
    pub col: usize,
    /// Byte offset of the first byte, including an opening quote.
    pub start: usize,
}

/// One piece of a string literal, in written order.
#[derive(Debug, PartialEq)]
pub enum Part<'a> {
    /// Literal text with its escapes decoded.
    Text(Vec<u8>),
    /// The expression between `{` and `}`, and the byte offset where it starts.
    Expr { start: usize, text: &'a [u8] },
}

impl<'a> Token<'a> {
    /// Bytes in the source, including a string's quotes.
    pub fn len(&self) -> usize {
        self.text.len() + if self.kind == Kind::Str { 2 } else { 0 }
    }

    /// The parts of a `Str` token; the lexer has checked its escapes and braces.
    pub fn parts(&self) -> Vec<Part<'a>> {
        let (text, base) = (self.text, self.start + 1);
        let (mut parts, mut run, mut i) = (vec![], vec![], 0);
        while i < text.len() {
            match text[i] {
                b'\\' => {
                    run.push(unescape(text[i + 1]));
                    i += 2;
                }
                b'{' => {
                    let end = i + text[i..].iter().position(|&b| b == b'}').unwrap();
                    if !run.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut run)));
                    }
                    let (start, text) = (base + i + 1, &text[i + 1..end]);
                    parts.push(Part::Expr { start, text });
                    i = end + 1;
                }
                b => {
                    run.push(b);
                    i += 1;
                }
            }
        }
        if !run.is_empty() {
            parts.push(Part::Text(run));
        }
        parts
    }
}

fn unescape(b: u8) -> u8 {
    match b {
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        _ => b,
    }
}

/// Hands out one token at a time, so the parser can stop at the first error.
pub struct Lexer<'a> {
    path: &'a str,
    src: &'a [u8],
    at: usize,
    line: usize,
    line_start: usize,
    /// The kind of the last token handed out.
    last: Option<Kind>,
    /// Brackets opened and not yet closed.
    open: Vec<Kind>,
}

const KEYWORDS: &[(&str, Kind)] = &[
    ("app", Kind::App),
    ("break", Kind::Break),
    ("continue", Kind::Continue),
    ("else", Kind::Else),
    ("enum", Kind::Enum),
    ("example", Kind::Example),
    ("false", Kind::False),
    ("fn", Kind::Fn),
    ("for", Kind::For),
    ("if", Kind::If),
    ("in", Kind::In),
    ("inout", Kind::Inout),
    ("let", Kind::Let),
    ("match", Kind::Match),
    ("migrate", Kind::Migrate),
    ("return", Kind::Return),
    ("true", Kind::True),
    ("type", Kind::Type),
    ("var", Kind::Var),
    ("view", Kind::View),
    ("while", Kind::While),
];

/// Longest first, so the first match is the longest.
const SYMBOLS: &[(&str, Kind)] = &[
    ("..<", Kind::DotDotLt),
    ("==", Kind::EqEq),
    ("!=", Kind::NotEq),
    ("<=", Kind::LtEq),
    (">=", Kind::GtEq),
    ("&&", Kind::AndAnd),
    ("||", Kind::OrOr),
    ("+=", Kind::PlusEq),
    ("-=", Kind::MinusEq),
    ("*=", Kind::StarEq),
    ("/=", Kind::SlashEq),
    ("%=", Kind::PercentEq),
    ("->", Kind::Arrow),
    ("=>", Kind::FatArrow),
    ("{", Kind::LBrace),
    ("}", Kind::RBrace),
    ("(", Kind::LParen),
    (")", Kind::RParen),
    ("[", Kind::LBracket),
    ("]", Kind::RBracket),
    ("<", Kind::Lt),
    (">", Kind::Gt),
    ("+", Kind::Plus),
    ("-", Kind::Minus),
    ("*", Kind::Star),
    ("/", Kind::Slash),
    ("%", Kind::Percent),
    ("!", Kind::Bang),
    ("&", Kind::Amp),
    ("?", Kind::Question),
    ("=", Kind::Eq),
    (":", Kind::Colon),
    (",", Kind::Comma),
    (";", Kind::Semi),
    (".", Kind::Dot),
    ("|", Kind::Pipe),
];

/// The spelling of a punctuation or keyword kind.
pub fn symbol(kind: Kind) -> &'static str {
    let found = SYMBOLS.iter().chain(KEYWORDS).find(|(_, k)| *k == kind);
    found.map_or("?", |(s, _)| s)
}

/// After these a line feed continues the statement (M2-02 section 1.5, rule 2); a `Newline` or the
/// end has nothing left to end.
fn continues(kind: Kind) -> bool {
    use Kind::*;
    matches!(
        kind,
        Comma
            | Eq
            | Arrow
            | FatArrow
            | Pipe
            | LBrace
            | Plus
            | Minus
            | Star
            | Slash
            | Percent
            | EqEq
            | NotEq
            | Lt
            | Gt
            | LtEq
            | GtEq
            | AndAnd
            | OrOr
            | DotDotLt
            | PlusEq
            | MinusEq
            | StarEq
            | SlashEq
            | PercentEq
            | Newline
            | End
    )
}

fn name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl<'a> Lexer<'a> {
    pub fn new(path: &'a str, src: &'a [u8]) -> Self {
        Lexer {
            path,
            src,
            at: 0,
            line: 1,
            line_start: 0,
            last: None,
            open: vec![],
        }
    }

    /// A lexer that starts at byte `at`, which is on `line` and which that line starts at `line_start`.
    pub fn at(path: &'a str, src: &'a [u8], at: usize, line: usize, line_start: usize) -> Self {
        Lexer {
            at,
            line,
            line_start,
            ..Lexer::new(path, src)
        }
    }

    /// Steps over the byte a lexical error stopped at.
    pub fn skip_byte(&mut self) {
        self.at += 1;
    }

    /// Forgets the open brackets, for a restart after an error.
    pub fn reset_brackets(&mut self) {
        self.open.clear();
    }

    fn peek(&self, ahead: usize) -> Option<u8> {
        self.src.get(self.at + ahead).copied()
    }

    fn make(&self, kind: Kind, start: usize, end: usize) -> Token<'a> {
        let src: &'a [u8] = self.src;
        Token {
            kind,
            text: &src[start..end],
            line: self.line,
            col: start - self.line_start + 1,
            start,
        }
    }

    /// The error for the `len` bytes at `index`, on the current line.
    fn fail(&self, index: usize, len: usize, code: &'static str, message: &str) -> Diagnostic {
        let position = (self.line, index - self.line_start + 1);
        Diagnostic::new(self.path, position, len, code, message)
    }

    fn invalid(&self, index: usize) -> Diagnostic {
        let message = "control byte or invalid UTF-8";
        self.fail(index, 1, "invalid-byte", message)
    }

    /// Length of the valid UTF-8 character at `index`, if there is one.
    fn char_len(&self, index: usize) -> Option<usize> {
        let chunk = &self.src[index..self.src.len().min(index + 4)];
        let valid = std::str::from_utf8(chunk)
            .or_else(|e| std::str::from_utf8(&chunk[..e.valid_up_to()]))
            .unwrap_or_default();
        valid.chars().next().map(char::len_utf8)
    }

    /// The error for a byte that no rule accepts at `index`.
    fn bad_byte(&self, index: usize) -> Diagnostic {
        let code = "unexpected-character";
        match (self.src[index], self.char_len(index)) {
            (0x20..=0x7E, _) => self.fail(index, 1, code, "no token starts with this byte"),
            (0x80.., Some(n)) => {
                self.fail(index, n, code, "only ASCII outside strings and comments")
            }
            _ => self.invalid(index),
        }
    }

    /// The index after the character at `index`, which may sit in a string or a comment.
    fn text_char(&self, index: usize) -> Result<usize, Diagnostic> {
        match self.src[index] {
            0x20..=0x7E => Ok(index + 1),
            0x80.. => (self.char_len(index).map(|n| index + n)).ok_or_else(|| self.invalid(index)),
            _ => Err(self.invalid(index)),
        }
    }

    /// Skips spaces, line feeds and comments; returns the first line feed, or the end if the
    /// last line has none.
    fn skip_trivia(&mut self) -> Result<Option<Token<'a>>, Diagnostic> {
        let mut newline = None;
        loop {
            match (self.peek(0), self.peek(1)) {
                (Some(b' '), _) => self.at += 1,
                (Some(b'\n'), _) => {
                    newline = newline.or(Some(self.make(Kind::Newline, self.at, self.at + 1)));
                    self.at += 1;
                    self.line += 1;
                    self.line_start = self.at;
                }
                (Some(b'/'), Some(b'/')) => {
                    while self.peek(0).is_some_and(|b| b != b'\n') {
                        self.at = self.text_char(self.at)?;
                    }
                }
                (None, _) => {
                    return Ok(newline.or(Some(self.make(Kind::Newline, self.at, self.at))));
                }
                _ => return Ok(newline),
            }
        }
    }

    /// Whether a line feed here ends a statement (M2-02 section 1.5, rules 1 to 3).
    fn ends_statement(&self) -> bool {
        let inside = matches!(self.open.last(), Some(Kind::LParen | Kind::LBracket));
        let closing = matches!(self.peek(0), Some(b'}' | b')' | b']'));
        self.last.is_some_and(|k| !continues(k)) && !inside && !closing
    }

    pub fn next(&mut self) -> Result<Token<'a>, Diagnostic> {
        let newline = self.skip_trivia()?;
        let token = match newline.filter(|_| self.ends_statement()) {
            Some(newline) => newline,
            None => self.token()?,
        };
        match token.kind {
            Kind::LBrace | Kind::LParen | Kind::LBracket => self.open.push(token.kind),
            Kind::RBrace | Kind::RParen | Kind::RBracket => drop(self.open.pop()),
            _ => {}
        }
        self.last = Some(token.kind);
        Ok(token)
    }

    /// The token at `self.at`, which is not space or comment.
    fn token(&mut self) -> Result<Token<'a>, Diagnostic> {
        let start = self.at;
        let rest = &self.src[start..];
        let (kind, len) = match rest {
            [] => (Kind::End, 0),
            [b'"', ..] => return self.string(),
            [b'0'..=b'9', ..] => self.number()?,
            [b'_', next, ..] if name_byte(*next) => {
                let message = "names start with a letter; `_` alone discards";
                return Err(self.fail(start, 1, "unexpected-character", message));
            }
            [b'_', ..] => (Kind::Underscore, 1),
            [b'a'..=b'z' | b'A'..=b'Z', ..] => {
                let len = rest.iter().take_while(|&&b| name_byte(b)).count();
                (self.word(&rest[..len]), len)
            }
            [b'?', b'?', b'a'..=b'z', ..] => (
                Kind::Hole,
                2 + rest[2..].iter().take_while(|&&b| name_byte(b)).count(),
            ),
            [b'.', b'0'..=b'9', ..] => {
                return Err(self.fail(start, 1, "unexpected-character", "write 0.5"));
            }
            [b'.', b'.', ..] if !rest.starts_with(b"..<") => {
                return Err(self.fail(start, 1, "unexpected-character", "write a..<b"));
            }
            _ => match SYMBOLS.iter().find(|(s, _)| rest.starts_with(s.as_bytes())) {
                Some((s, kind)) => (*kind, s.len()),
                None => return Err(self.bad_byte(start)),
            },
        };
        self.at = start + len;
        Ok(self.make(kind, start, self.at))
    }

    fn word(&self, name: &[u8]) -> Kind {
        let keyword = KEYWORDS.iter().find(|(k, _)| k.as_bytes() == name);
        match (keyword, name[0].is_ascii_uppercase()) {
            (Some((_, kind)), _) => *kind,
            (None, true) => Kind::Upper,
            (None, false) => Kind::Lower,
        }
    }

    /// An Int or Float at `self.at`: its kind and length (M2-03 section 1.12).
    fn number(&self) -> Result<(Kind, usize), Diagnostic> {
        let digits = |from: usize| {
            self.src[from..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };
        let whole = digits(self.at);
        if self.src[self.at] == b'0' && whole > 1 {
            let message = "no leading zero: write 7";
            return Err(self.fail(self.at + 1, 1, "unexpected-character", message));
        }
        let point = self.at + whole;
        let (kind, len) = match self.src.get(point..point + 2) {
            Some([b'.', b'0'..=b'9']) => (Kind::Float, whole + 1 + digits(point + 1)),
            _ => (Kind::Int, whole),
        };
        let end = self.at + len;
        let message = match (self.src.get(end), &self.src[self.at..end]) {
            (Some(b'x' | b'X'), b"0") => "no hex: write 16",
            (Some(b'e' | b'E'), _) => "no exponent: write 1000.0",
            (Some(b'_'), _) => "write 1000",
            (Some(b'.'), _) if kind == Kind::Int && self.src.get(end + 1) != Some(&b'.') => {
                "write 1.0"
            }
            _ => return Ok((kind, len)),
        };
        Err(self.fail(end, 1, "unexpected-character", message))
    }

    fn unterminated(&self, open: usize) -> Diagnostic {
        let line = &self.src[open..];
        let len = line.iter().position(|&b| b == b'\n').unwrap_or(line.len());
        self.fail(
            open,
            len,
            "unterminated-string",
            "the string has no closing quote",
        )
    }

    /// Scans a string at the opening quote (M2-03 section 2.7): the first problem found wins.
    fn string(&mut self) -> Result<Token<'a>, Diagnostic> {
        let open = self.at;
        let mut i = open + 1;
        let mut interpolation = None;
        loop {
            let Some(&b) = self.src.get(i) else {
                return Err(self.unterminated(open));
            };
            match (b, interpolation) {
                (b'\n', _) => return Err(self.unterminated(open)),
                (b'"', None) => break,
                (b'\\', None) => {
                    if !matches!(
                        self.src.get(i + 1),
                        Some(b'n' | b'r' | b't' | b'\\' | b'"' | b'{' | b'}')
                    ) {
                        let message = r#"escapes are \n \r \t \\ \" \{ \}"#;
                        return Err(self.fail(i, 1, "bad-escape", message));
                    }
                    i += 2;
                }
                (b'{', None) => {
                    interpolation = Some(i + 1);
                    i += 1;
                }
                (b'}', None) => {
                    return Err(self.fail(i, 1, "unexpected-character", r"write \}"));
                }
                (b'}', Some(from)) if from == i => {
                    return Err(self.fail(i, 1, "unexpected-token", "expected an expression"));
                }
                (b'}', Some(_)) => {
                    interpolation = None;
                    i += 1;
                }
                (b'"' | b'{', Some(_)) => {
                    let closed =
                        (self.src[i..].iter().take_while(|&&b| b != b'\n')).any(|&b| b == b'}');
                    if !closed {
                        return Err(self.unterminated(open));
                    }
                    let message = if b == b'"' {
                        "no string literal inside { }"
                    } else {
                        "no { inside { }"
                    };
                    return Err(self.fail(i, 1, "unexpected-character", message));
                }
                (0x20..=0x7E, _) => i += 1,
                (_, None) => i = self.text_char(i)?,
                (_, Some(_)) => return Err(self.bad_byte(i)),
            }
        }
        self.at = i + 1;
        let src: &'a [u8] = self.src;
        Ok(Token {
            text: &src[open + 1..i],
            ..self.make(Kind::Str, open, i + 1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Every token up to and including END, or the first error.
    fn lex(src: &[u8]) -> Result<Vec<Token<'_>>, Diagnostic> {
        let mut lexer = Lexer::new("main.paka", src);
        let mut tokens = vec![];
        loop {
            let token = lexer.next()?;
            tokens.push(token);
            if token.kind == Kind::End {
                return Ok(tokens);
            }
        }
    }

    /// `text@line:col`, with STRING, NL and END named by kind.
    fn describe(src: &[u8]) -> Vec<String> {
        let tokens = lex(src).unwrap_or_else(|d| panic!("{d}"));
        let name = |t: &Token| match t.kind {
            Kind::Str => "STRING".to_string(),
            Kind::Newline => "NL".to_string(),
            Kind::End => "END".to_string(),
            _ => String::from_utf8_lossy(t.text).into_owned(),
        };
        let at = |t: &Token| format!("{}@{}:{}", name(t), t.line, t.col);
        tokens.iter().map(at).collect()
    }

    fn kinds(src: &str) -> Vec<Kind> {
        let tokens = lex(src.as_bytes()).unwrap_or_else(|d| panic!("{d}"));
        tokens.iter().map(|t| t.kind).collect()
    }

    fn texts(src: &str) -> Vec<String> {
        let tokens = lex(src.as_bytes()).unwrap_or_else(|d| panic!("{d}"));
        let text = |t: &Token| String::from_utf8_lossy(t.text).into_owned();
        tokens.iter().map(text).collect()
    }

    fn parts(src: &str) -> Vec<Part<'_>> {
        lex(src.as_bytes()).unwrap_or_else(|d| panic!("{d}"))[0].parts()
    }

    /// The diagnostic line up to and including the code: `path:line:col: error[code]`.
    fn code(src: impl AsRef<[u8]>) -> String {
        match lex(src.as_ref()) {
            Err(d) => d
                .to_string()
                .split(": ")
                .take(2)
                .collect::<Vec<_>>()
                .join(": "),
            Ok(_) => panic!("{:?} lexed", src.as_ref()),
        }
    }

    fn message(src: &str) -> String {
        lex(src.as_bytes()).unwrap_err().message
    }

    #[test]
    fn light_paka_tokens() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/light.paka");
        let src = fs::read(path).unwrap();
        // Spec section 14, line by line, with the line ends that end a statement.
        let expected = "
            enum@1:1 Msg@1:6 {@1:10 Start@1:12 }@1:18 NL@1:19
            fn@3:1 update@3:4 (@3:10 m@3:11 :@3:12 Msg@3:14 )@3:17 ->@3:19 List@3:22 <@3:26
            Effect@3:27 <@3:33 Msg@3:34 >@3:37 >@3:38 {@3:40
            [@4:3 Print@4:4 (@4:9 STRING@4:10 )@4:30 ,@4:31 Exit@4:33 ]@4:37
            }@5:1 NL@5:2
            app@7:1 light@7:5 {@7:11
            start@8:3 :@8:8 Start@8:11 NL@8:16
            update@9:3 :@9:9 update@9:11 NL@9:17
            grants@10:3 :@10:9 [@10:11 Console@10:12 ]@10:19
            }@11:1 NL@11:2
            END@12:1";
        let expected: Vec<_> = expected.split_whitespace().collect();
        assert_eq!(expected.len(), 51);
        assert_eq!(describe(&src), expected);
    }

    #[test]
    fn keywords() {
        use Kind::*;
        let src = "app break continue else enum example false fn for if in inout let match \
                   migrate return true type var view while";
        let want = [
            App, Break, Continue, Else, Enum, Example, False, Fn, For, If, In, Inout, Let, Match,
            Migrate, Return, True, Type, Var, View, While, Newline, End,
        ];
        assert_eq!(kinds(src), want);
    }

    #[test]
    fn names() {
        use Kind::*;
        let want = [Upper, Upper, Lower, Lower, Lower, Newline, End];
        assert_eq!(kinds("A Zed9_x a b_1 z9"), want);
        // A keyword is a keyword only as a whole name, in lower case.
        let want = [Lower, Upper, Lower, Lower, Lower, Lower, Newline, End];
        assert_eq!(kinds("enumerate Match fns start state update"), want);
        assert_eq!(describe(b"a_b9"), ["a_b9@1:1", "NL@1:5", "END@1:5"]);
    }

    #[test]
    fn symbols() {
        use Kind::*;
        let src = "{ } ( ) [ ] < > <= >= == != && || + - * / % ! ..< & ? = += -= *= /= %= \
                   : , ; . -> => | _";
        let want = [
            LBrace, RBrace, LParen, RParen, LBracket, RBracket, Lt, Gt, LtEq, GtEq, EqEq, NotEq,
            AndAnd, OrOr, Plus, Minus, Star, Slash, Percent, Bang, DotDotLt, Amp, Question, Eq,
            PlusEq, MinusEq, StarEq, SlashEq, PercentEq, Colon, Comma, Semi, Dot, Arrow, FatArrow,
            Pipe, Underscore,
        ];
        let got = kinds(&format!("{{ {src} }}"));
        assert_eq!(&got[1..got.len() - 3], want);
    }

    #[test]
    fn longest_symbol_wins() {
        use Kind::*;
        assert_eq!(kinds("(>>"), [LParen, Gt, Gt, End]);
        assert_eq!(kinds("(->>"), [LParen, Arrow, Gt, End]);
        assert_eq!(kinds("(<=>"), [LParen, LtEq, Gt, End]);
        assert_eq!(kinds("(===>"), [LParen, EqEq, FatArrow, End]);
        assert_eq!(kinds("(!!="), [LParen, Bang, NotEq, End]);
        assert_eq!(kinds("(&&&"), [LParen, AndAnd, Amp, End]);
        assert_eq!(kinds("(|||"), [LParen, OrOr, Pipe, End]);
        assert_eq!(kinds("(a-1"), [LParen, Lower, Minus, Int, End]);
        assert_eq!(kinds("(a..<b"), [LParen, Lower, DotDotLt, Lower, End]);
        assert_eq!(kinds("(1..<5"), [LParen, Int, DotDotLt, Int, End]);
        assert_eq!(kinds("(a.b"), [LParen, Lower, Dot, Lower, End]);
        assert_eq!(kinds("(a.<"), [LParen, Lower, Dot, Lt, End]);
    }

    #[test]
    fn holes() {
        use Kind::*;
        assert_eq!(kinds("??rest"), [Hole, Newline, End]);
        assert_eq!(texts("??rest_2 x")[0], "??rest_2");
        assert_eq!(
            describe(b"a ??b"),
            ["a@1:1", "??b@1:3", "NL@1:6", "END@1:6"]
        );
        // Without a lower-case letter right after, `?` stays a `?`.
        assert_eq!(kinds("(??"), [LParen, Question, Question, End]);
        assert_eq!(kinds("(?? a"), [LParen, Question, Question, Lower, End]);
        assert_eq!(kinds("(??A"), [LParen, Question, Question, Upper, End]);
        assert_eq!(kinds("(?a"), [LParen, Question, Lower, End]);
    }

    #[test]
    fn numbers() {
        use Kind::*;
        assert_eq!(
            kinds("0 7 10 9223372036854775808"),
            [Int, Int, Int, Int, Newline, End]
        );
        assert_eq!(
            kinds("0.0 0.5 1.50 12.25"),
            [Float, Float, Float, Float, Newline, End]
        );
        assert_eq!(texts("0 10 1.50")[..3], ["0", "10", "1.50"]);
        // The sign is a separate token; the parser folds it.
        assert_eq!(kinds("(-7 -1.5"), [LParen, Minus, Int, Minus, Float, End]);
        assert_eq!(kinds("(1..<2"), [LParen, Int, DotDotLt, Int, End]);
    }

    #[test]
    fn other_number_spellings_fail_at_the_breaking_byte() {
        let fail = |src: &str| {
            let d = lex(src.as_bytes()).unwrap_err();
            (d.position.unwrap().1, d.code, d.message)
        };
        let at = |col, message: &str| (col, "unexpected-character", message.to_string());
        assert_eq!(fail("(1."), at(3, "write 1.0"));
        assert_eq!(fail("(1.x"), at(3, "write 1.0"));
        assert_eq!(fail("(.5"), at(2, "write 0.5"));
        assert_eq!(fail("(1.5.5"), at(5, "write 0.5"));
        assert_eq!(fail("(1e3"), at(3, "no exponent: write 1000.0"));
        assert_eq!(fail("(1.5e3"), at(5, "no exponent: write 1000.0"));
        assert_eq!(fail("(0x10"), at(3, "no hex: write 16"));
        assert_eq!(fail("(1_000"), at(3, "write 1000"));
    }

    #[test]
    fn strings() {
        assert_eq!(kinds("\"\""), [Kind::Str, Kind::Newline, Kind::End]);
        let tokens = lex(b"\"a b\" \"~ !\"").unwrap();
        assert_eq!(tokens[0].text, b"a b");
        assert_eq!(tokens[1].text, b"~ !");
        // `//`, `??`, digits and other symbols are plain bytes inside a string.
        let tokens = lex(b"\"// ?? 42 - _ ; &\"").unwrap();
        assert_eq!(tokens[0].text, b"// ?? 42 - _ ; &");
        // The text is the source between the quotes; the length counts the quotes.
        let tokens = lex(br#""a\n{x}""#).unwrap();
        assert_eq!(tokens[0].text, br#"a\n{x}"#);
        assert_eq!(tokens[0].len(), 8);
        assert_eq!(
            describe(r#""x y" z"#.as_bytes())[..2],
            ["STRING@1:1", "z@1:7"]
        );
    }

    #[test]
    fn string_parts() {
        let text = |s: &str| Part::Text(s.as_bytes().to_vec());
        assert_eq!(parts("\"\""), []);
        assert_eq!(parts("\"abc\""), [text("abc")]);
        let expr = |start, text| Part::Expr { start, text };
        assert_eq!(parts(r#""a{x}b""#), [text("a"), expr(3, b"x"), text("b")]);
        assert_eq!(parts(r#""{a}{b + 1}""#), [expr(2, b"a"), expr(5, b"b + 1")]);
        assert_eq!(parts(r#""\{x\}""#), [text("{x}")]);
        assert_eq!(parts(r#""a\n\r\t\\\"\{\}""#), [text("a\n\r\t\\\"{}")]);
        assert_eq!(
            parts("\"caf\u{e9} {n}\""),
            [text("caf\u{e9} "), expr(8, b"n")]
        );
    }

    #[test]
    fn utf8_in_strings_and_comments() {
        assert_eq!(
            lex("\"caf\u{e9} \u{1f600}\"".as_bytes()).unwrap()[0].kind,
            Kind::Str
        );
        // Columns count bytes.
        let want = ["STRING@1:1", "x@1:6", "NL@1:7", "END@1:7"];
        assert_eq!(describe("\"\u{e9}\" x".as_bytes()), want);
        assert_eq!(
            describe("// \u{e9}\nx".as_bytes()),
            ["x@2:1", "NL@2:2", "END@2:2"]
        );
    }

    #[test]
    fn comments() {
        use Kind::*;
        assert_eq!(kinds("// only a comment"), [End]);
        assert_eq!(kinds("a // c\nb"), [Lower, Newline, Lower, Newline, End]);
        assert_eq!(
            kinds("a\n// c\n\n// d\nb"),
            [Lower, Newline, Lower, Newline, End]
        );
        assert_eq!(kinds("a //"), [Lower, Newline, End]);
        assert_eq!(kinds("a ////x"), [Lower, Newline, End]);
        // A comment after a continuing token does not end the statement.
        assert_eq!(kinds("a + // c\nb"), [Lower, Plus, Lower, Newline, End]);
        assert_eq!(
            kinds("(a // c\nb)"),
            [LParen, Lower, Lower, RParen, Newline, End]
        );
        assert_eq!(kinds("/ /"), [Slash, Slash, End]);
    }

    #[test]
    fn newline_rule() {
        use Kind::*;
        assert_eq!(kinds("a\nb"), [Lower, Newline, Lower, Newline, End]);
        assert_eq!(kinds("a\n\n  \n b"), [Lower, Newline, Lower, Newline, End]);
        assert_eq!(kinds("a\n"), [Lower, Newline, End]);
        assert_eq!(kinds("a"), [Lower, Newline, End]);
        assert_eq!(kinds(""), [End]);
        assert_eq!(kinds("\n\n"), [End]);
        assert_eq!(kinds("\n\na"), [Lower, Newline, End]);
    }

    #[test]
    fn newline_continues_after() {
        use Kind::*;
        let tokens = [
            ",", "=", "->", "=>", "|", "{", "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=",
            ">=", "&&", "||", "..<", "+=", "-=", "*=", "/=", "%=",
        ];
        for token in tokens {
            let got = kinds(&format!("a {token}\nb"));
            assert!(!got[..got.len() - 2].contains(&Newline), "{token}: {got:?}");
            assert_eq!(got[got.len() - 2..], [Newline, End], "{token}");
        }
        // End of input after a continuing token adds no NEWLINE either.
        assert_eq!(kinds("a +"), [Lower, Plus, End]);
        assert_eq!(kinds("a +\n"), [Lower, Plus, End]);
        // `?` and `&` are not two-sided operators.
        assert_eq!(
            kinds("a ?\nb"),
            [Lower, Question, Newline, Lower, Newline, End]
        );
        assert_eq!(kinds("a &\nb"), [Lower, Amp, Newline, Lower, Newline, End]);
        // Neither are `;` and `.`.
        assert_eq!(
            kinds("{ a }\nb"),
            [LBrace, Lower, RBrace, Newline, Lower, Newline, End]
        );
        assert_eq!(kinds("f ;\nb"), [Lower, Semi, Newline, Lower, Newline, End]);
        assert_eq!(kinds("a.\nb"), [Lower, Dot, Newline, Lower, Newline, End]);
        // A type's `>` is the comparison token, so it continues too.
        assert_eq!(
            kinds("List<Int>\nx"),
            [Upper, Lt, Upper, Gt, Lower, Newline, End]
        );
    }

    #[test]
    fn newline_not_before_closing_bracket() {
        use Kind::*;
        assert_eq!(kinds("{ a\n}"), [LBrace, Lower, RBrace, Newline, End]);
        assert_eq!(
            kinds("{ a\n\n// c\n}"),
            [LBrace, Lower, RBrace, Newline, End]
        );
        assert_eq!(kinds("(a\n)"), [LParen, Lower, RParen, Newline, End]);
        assert_eq!(kinds("[a\n]"), [LBracket, Lower, RBracket, Newline, End]);
    }

    #[test]
    fn newline_inside_brackets() {
        use Kind::*;
        assert_eq!(
            kinds("(a\nb)"),
            [LParen, Lower, Lower, RParen, Newline, End]
        );
        assert_eq!(
            kinds("[1,\n2]"),
            [LBracket, Int, Comma, Int, RBracket, Newline, End]
        );
        assert_eq!(
            kinds("[a\nb]"),
            [LBracket, Lower, Lower, RBracket, Newline, End]
        );
        // The innermost bracket decides: a block inside parentheses has line ends again.
        let want = [
            LParen, LBrace, Lower, Newline, Lower, RBrace, RParen, Newline, End,
        ];
        assert_eq!(kinds("({ a\nb })"), want);
        let want = [
            LBrace, Lower, LParen, Lower, Lower, RParen, Newline, Lower, RBrace,
        ];
        assert_eq!(kinds("{ f(a\nb)\nc }")[..9], want);
        // After the parenthesis closes, the enclosing block counts again.
        let want = [LBrace, LParen, RParen, Newline, Lower, RBrace, Newline, End];
        assert_eq!(kinds("{ ()\nc }"), want);
    }

    #[test]
    fn newline_token_positions() {
        let want = ["a@1:1", "NL@1:3", "b@3:2", "NL@3:3", "END@3:3"];
        assert_eq!(describe(b"a \n\n b"), want);
        assert_eq!(
            describe(b"a // x\n b"),
            ["a@1:1", "NL@1:7", "b@2:2", "NL@2:3", "END@2:3"]
        );
        assert_eq!(
            describe(b"a +\n b"),
            ["a@1:1", "+@1:3", "b@2:2", "NL@2:3", "END@2:3"]
        );
    }

    #[test]
    fn positions() {
        assert_eq!(describe(b""), ["END@1:1"]);
        assert_eq!(describe(b"a\n"), ["a@1:1", "NL@1:2", "END@2:1"]);
        let want = ["a@3:3", "b@3:5", "NL@3:6", "END@3:6"];
        assert_eq!(describe(b"\n\n  a b"), want);
    }

    #[test]
    fn end_is_repeatable() {
        let mut lexer = Lexer::new("main.paka", b"a");
        assert_eq!(lexer.next().unwrap().kind, Kind::Lower);
        assert_eq!(lexer.next().unwrap().kind, Kind::Newline);
        assert_eq!(lexer.next().unwrap().kind, Kind::End);
        assert_eq!(lexer.next().unwrap().kind, Kind::End);
    }

    #[test]
    fn bytes_after_the_last_pulled_token_are_not_looked_at() {
        let mut lexer = Lexer::new("main.paka", b"enum \x01");
        assert_eq!(lexer.next().unwrap().kind, Kind::Enum);
        assert!(lexer.next().is_err());
        let mut lexer = Lexer::new("main.paka", b"enum $ ");
        assert_eq!(lexer.next().unwrap().kind, Kind::Enum);
    }

    #[test]
    fn invalid_bytes() {
        let invalid =
            |src: &str| assert_eq!(code(src), "main.paka:1:3: error[invalid-byte]", "{src:?}");
        for src in ["a \t", "a \r", "a \x00", "a \x1f", "a \x7f"] {
            invalid(src);
        }
        assert_eq!(code("a\n\t"), "main.paka:2:1: error[invalid-byte]");
        // Invalid UTF-8 is invalid-byte at its first byte, wherever it stands.
        let sources: [(&[u8], &str); 8] = [
            (b"a \xff", "1:3"),
            (b"a \x80", "1:3"),
            (b"a \xc0\x80", "1:3"),
            (b"a \xed\xa0\x80", "1:3"),
            (b"a \xe2\x82", "1:3"),
            (b"a \xf4\x90\x80\x80", "1:3"),
            (b"a \"\xff\"", "1:4"),
            (b"a //\xff", "1:5"),
        ];
        for (src, at) in sources {
            assert_eq!(
                code(src),
                format!("main.paka:{at}: error[invalid-byte]"),
                "{src:?}"
            );
        }
        // Control bytes inside strings and comments too.
        assert_eq!(code("a \"b\t\""), "main.paka:1:5: error[invalid-byte]");
        assert_eq!(code("a \"b\x7f\""), "main.paka:1:5: error[invalid-byte]");
        assert_eq!(code("a // b\t"), "main.paka:1:7: error[invalid-byte]");
        assert_eq!(code("a \"{\t}\""), "main.paka:1:5: error[invalid-byte]");
    }

    #[test]
    fn unexpected_characters() {
        for src in ["a $", "a #", "a @", "a ~", "a ^", "a `", "a '", "a \\"] {
            assert_eq!(
                code(src),
                "main.paka:1:3: error[unexpected-character]",
                "{src}"
            );
        }
        // Non-ASCII outside strings and comments, a byte order mark included.
        assert_eq!(
            code("a \u{e9}"),
            "main.paka:1:3: error[unexpected-character]"
        );
        assert_eq!(
            code("\u{feff}a"),
            "main.paka:1:1: error[unexpected-character]"
        );
        assert_eq!(
            code("a\n \u{1f600}"),
            "main.paka:2:2: error[unexpected-character]"
        );
        assert_eq!(message("\u{e9}"), "only ASCII outside strings and comments");
        assert_eq!(message("$"), "no token starts with this byte");
    }

    #[test]
    fn name_and_number_errors() {
        for src in ["a _m", "a __", "a _1"] {
            assert_eq!(
                code(src),
                "main.paka:1:3: error[unexpected-character]",
                "{src}"
            );
        }
        assert_eq!(
            message("_m"),
            "names start with a letter; `_` alone discards"
        );
        assert_eq!(code("a 01"), "main.paka:1:4: error[unexpected-character]");
        assert_eq!(
            code("a\n 007"),
            "main.paka:2:3: error[unexpected-character]"
        );
        assert_eq!(code("a 00.5"), "main.paka:1:4: error[unexpected-character]");
        assert_eq!(code("a 01.5"), "main.paka:1:4: error[unexpected-character]");
        assert_eq!(message("01"), "no leading zero: write 7");
    }

    #[test]
    fn range_errors() {
        for src in ["a..b", "a...b", "a..", "a.. <b", "a..>b"] {
            assert_eq!(
                code(src),
                "main.paka:1:2: error[unexpected-character]",
                "{src}"
            );
        }
        assert_eq!(message("a..b"), "write a..<b");
    }

    #[test]
    fn string_errors() {
        let unterminated = |src: &str, at: &str| {
            let want = format!("main.paka:{at}: error[unterminated-string]");
            assert_eq!(code(src), want, "{src:?}");
        };
        unterminated("a \"b", "1:3");
        unterminated("a \"b\nc\"", "1:3");
        unterminated("a \"{\"", "1:3");
        unterminated("a \"{x", "1:3");
        unterminated("a \"{x\ny}\"", "1:3");
        unterminated("a \"{ \"", "1:3");
        unterminated("a \"{{", "1:3");
        unterminated("a\n \"", "2:2");
        unterminated("\"a\\\\", "1:1");
        assert_eq!(code("a \"b\\q\""), "main.paka:1:5: error[bad-escape]");
        assert_eq!(code("a \"\\u{e9}\""), "main.paka:1:4: error[bad-escape]");
        assert_eq!(code("a \"b\\"), "main.paka:1:5: error[bad-escape]");
        assert_eq!(code("a \"b\\\nc\""), "main.paka:1:5: error[bad-escape]");
        assert_eq!(
            code("a \"b}\""),
            "main.paka:1:5: error[unexpected-character]"
        );
        assert_eq!(
            code("a \"{x}}\""),
            "main.paka:1:7: error[unexpected-character]"
        );
        // M2-03 section 1.10: a bare `}` is an error even after an escaped `\{`.
        assert_eq!(
            code(r#"a "\{x}""#),
            "main.paka:1:7: error[unexpected-character]"
        );
        assert_eq!(
            code("a \"{\"x\"}\""),
            "main.paka:1:5: error[unexpected-character]"
        );
        assert_eq!(
            code("a \"{{x}\""),
            "main.paka:1:5: error[unexpected-character]"
        );
        assert_eq!(code("a \"{}\""), "main.paka:1:5: error[unexpected-token]");
        assert_eq!(
            code("a \"{\u{e9}}\""),
            "main.paka:1:5: error[unexpected-character]"
        );
        // The first problem in scan order wins.
        assert_eq!(code("\"\\q}\""), "main.paka:1:2: error[bad-escape]");
        assert_eq!(
            code("\"}\\q\""),
            "main.paka:1:2: error[unexpected-character]"
        );
        assert_eq!(code("\"{}\\q\""), "main.paka:1:3: error[unexpected-token]");
        assert_eq!(code("\"\t\\q\""), "main.paka:1:2: error[invalid-byte]");
    }

    #[test]
    fn error_messages() {
        assert_eq!(message("\"\\q\""), r#"escapes are \n \r \t \\ \" \{ \}"#);
        assert_eq!(message("\"}\""), r"write \}");
        assert_eq!(message("\"{\"x}\""), "no string literal inside { }");
        assert_eq!(message("\"{{x}\""), "no { inside { }");
        assert_eq!(message("\"{}\""), "expected an expression");
        assert_eq!(message("\"x"), "the string has no closing quote");
    }

    #[test]
    fn escapes_are_the_seven() {
        for escape in ["n", "r", "t", "\\", "\"", "{", "}"] {
            assert!(
                lex(format!("\"\\{escape}\"").as_bytes()).is_ok(),
                "{escape}"
            );
        }
    }

    #[test]
    fn diagnostic_carries_path_and_message() {
        let line = Lexer::new("dir/x.paka", b"$")
            .next()
            .unwrap_err()
            .to_string();
        let message = line.strip_prefix("dir/x.paka:1:1: error[unexpected-character]: ");
        assert!(
            message.is_some_and(|m| !m.is_empty() && !m.contains('\n')),
            "{line}"
        );
    }

    /// Each lex case is decided by the lexer alone: its `reject` file names path, position and code.
    #[test]
    fn lex_cases() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/lex");
        let mut cases: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        cases.sort();
        assert_eq!(cases.len(), 5);
        for case in cases {
            let source = fs::read(case.join("main.paka")).unwrap();
            let reject = fs::read_to_string(case.join("reject")).unwrap();
            let want = reject.strip_suffix('\n').unwrap();
            let got = match lex(&source) {
                Err(d) => d.to_string(),
                Ok(_) => panic!("{} lexed", case.display()),
            };
            let rest = got
                .strip_prefix(want)
                .unwrap_or_else(|| panic!("{}: want {want}, got {got}", case.display()));
            assert!(
                rest.starts_with(": ") && rest.len() > 2,
                "{}: {got}",
                case.display()
            );
        }
    }
}
