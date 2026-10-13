use crate::diagnostic::Diagnostic;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Enum,
    Fn,
    App,
    Type,
    View,
    Example,
    Match,
    Upper,
    Lower,
    Str,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Colon,
    Comma,
    Arrow,
    End,
}

/// `text` is the spelling; for `Str` it is the value, without the quotes.
#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub kind: Kind,
    pub text: &'a [u8],
    pub line: usize,
    pub col: usize,
    /// Byte offset of the first byte, including an opening quote.
    pub start: usize,
}

impl Token<'_> {
    /// Bytes in the source, including a string's quotes.
    pub fn len(&self) -> usize {
        self.text.len() + if self.kind == Kind::Str { 2 } else { 0 }
    }
}

/// Hands out one token at a time, so the parser can stop at the first error.
pub struct Lexer<'a> {
    path: &'a str,
    src: &'a [u8],
    at: usize,
    line: usize,
    line_start: usize,
}

fn printable(b: u8) -> bool {
    (0x20..=0x7E).contains(&b)
}

fn keyword(name: &[u8]) -> Option<Kind> {
    Some(match name {
        b"enum" => Kind::Enum,
        b"fn" => Kind::Fn,
        b"app" => Kind::App,
        b"type" => Kind::Type,
        b"view" => Kind::View,
        b"example" => Kind::Example,
        b"match" => Kind::Match,
        _ => return None,
    })
}

fn punctuation(b: u8) -> Option<Kind> {
    Some(match b {
        b'{' => Kind::LBrace,
        b'}' => Kind::RBrace,
        b'(' => Kind::LParen,
        b')' => Kind::RParen,
        b'[' => Kind::LBracket,
        b']' => Kind::RBracket,
        b'<' => Kind::Lt,
        b'>' => Kind::Gt,
        b':' => Kind::Colon,
        b',' => Kind::Comma,
        _ => return None,
    })
}

impl<'a> Lexer<'a> {
    pub fn new(path: &'a str, src: &'a [u8]) -> Self {
        Lexer {
            path,
            src,
            at: 0,
            line: 1,
            line_start: 0,
        }
    }

    fn peek(&self, ahead: usize) -> Option<u8> {
        self.src.get(self.at + ahead).copied()
    }

    /// The error for the `len` bytes at `index`, on the current line.
    fn fail(&self, index: usize, len: usize, code: &'static str, message: &str) -> Diagnostic {
        let position = (self.line, index - self.line_start + 1);
        Diagnostic::new(self.path, position, len, code, message)
    }

    /// The error for a byte that no rule accepts at `index`.
    fn bad_byte(&self, index: usize) -> Diagnostic {
        if printable(self.src[index]) {
            self.fail(
                index,
                1,
                "unexpected-character",
                "no token starts with this byte",
            )
        } else {
            self.fail(
                index,
                1,
                "invalid-byte",
                "only printable ASCII and line feed are allowed",
            )
        }
    }

    fn skip_space(&mut self) {
        while let Some(b @ (b' ' | b'\n')) = self.peek(0) {
            self.at += 1;
            if b == b'\n' {
                self.line += 1;
                self.line_start = self.at;
            }
        }
    }

    pub fn next(&mut self) -> Result<Token<'a>, Diagnostic> {
        self.skip_space();
        let start = self.at;
        let (line, col) = (self.line, start - self.line_start + 1);
        let token = |kind, text| Token {
            kind,
            text,
            line,
            col,
            start,
        };
        let Some(first) = self.peek(0) else {
            return Ok(token(Kind::End, &[]));
        };
        if first == b'"' {
            return self.string().map(|text| token(Kind::Str, text));
        }
        let kind = match (first, self.peek(1)) {
            (b'a'..=b'z' | b'A'..=b'Z', _) => {
                let len = (self.src[start..].iter())
                    .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_')
                    .count();
                self.at += len;
                let name = &self.src[start..self.at];
                let default = if first.is_ascii_uppercase() {
                    Kind::Upper
                } else {
                    Kind::Lower
                };
                return Ok(token(keyword(name).unwrap_or(default), name));
            }
            (b'-', Some(b'>')) => {
                self.at += 1;
                Kind::Arrow
            }
            (b'/', Some(b'/')) => {
                return Err(self.fail(
                    start,
                    2,
                    "unsupported-comment",
                    "comments are not supported",
                ));
            }
            (b'?', Some(b'?')) => {
                return Err(self.fail(start, 2, "unsupported-hole", "holes are not supported"));
            }
            (b'0'..=b'9', _) => {
                let len = (self.src[start..].iter())
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                return Err(self.fail(
                    start,
                    len,
                    "unsupported-integer",
                    "integers are not supported",
                ));
            }
            _ => match punctuation(first) {
                Some(kind) => kind,
                None => return Err(self.bad_byte(start)),
            },
        };
        self.at += 1;
        Ok(token(kind, &self.src[start..self.at]))
    }

    /// Scans a string at the opening quote and returns its value.
    fn string(&mut self) -> Result<&'a [u8], Diagnostic> {
        let open = self.at;
        for (i, &b) in self.src.iter().enumerate().skip(open + 1) {
            match b {
                b'"' => {
                    self.at = i + 1;
                    return Ok(&self.src[open + 1..i]);
                }
                b'\n' => break,
                b'\\' => {
                    return Err(self.fail(i, 1, "unsupported-escape", "escapes are not supported"));
                }
                b'{' | b'}' => {
                    return Err(self.fail(
                        i,
                        1,
                        "unsupported-interpolation",
                        "braces are not supported in strings",
                    ));
                }
                _ if !printable(b) => return Err(self.bad_byte(i)),
                _ => {}
            }
        }
        let line_end = (self.src[open..].iter().position(|&b| b == b'\n')).map(|n| open + n);
        Err(self.fail(
            open,
            line_end.unwrap_or(self.src.len()) - open,
            "unterminated-string",
            "the string has no closing quote",
        ))
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
        // No `1.`, `.5`, exponent, hex or underscore: the rest is other tokens.
        assert_eq!(kinds("(1."), [LParen, Int, Dot, End]);
        assert_eq!(kinds("(.5"), [LParen, Dot, Int, End]);
        assert_eq!(kinds("(1.x"), [LParen, Int, Dot, Lower, End]);
        assert_eq!(kinds("(1.5.5"), [LParen, Float, Dot, Int, End]);
        assert_eq!(kinds("(1e3"), [LParen, Int, Lower, End]);
        assert_eq!(kinds("(0x1"), [LParen, Int, Lower, End]);
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
