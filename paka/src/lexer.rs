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
}

/// Hands out one token at a time, so the parser can stop at the first error.
pub struct Lexer<'a> {
    path: &'a str,
    src: &'a [u8],
}

impl<'a> Lexer<'a> {
    pub fn new(path: &'a str, src: &'a [u8]) -> Self {
        Lexer { path, src }
    }

    pub fn next(&mut self) -> Result<Token<'a>, Diagnostic> {
        let _ = (self.path, self.src);
        todo!()
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

    /// `text@line:col`, with STRING and END named by kind.
    fn describe(src: &[u8]) -> Vec<String> {
        let tokens = lex(src).unwrap_or_else(|d| panic!("{d}"));
        let name = |t: &Token| match t.kind {
            Kind::Str => "STRING".to_string(),
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

    /// The first two parts of the diagnostic line: `path:line:col` and `error[code]`.
    fn code(src: &str) -> String {
        match lex(src.as_bytes()) {
            Err(d) => d
                .to_string()
                .split(": ")
                .take(2)
                .collect::<Vec<_>>()
                .join(": "),
            Ok(_) => panic!("{src:?} lexed"),
        }
    }

    #[test]
    fn light_paka_tokens() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/light.paka");
        let src = fs::read(path).unwrap();
        // Spec section 14, line by line.
        let expected = "
            enum@1:1 Msg@1:6 {@1:10 Start@1:12 }@1:18
            fn@3:1 update@3:4 (@3:10 m@3:11 :@3:12 Msg@3:14 )@3:17 ->@3:19 List@3:22 <@3:26
            Effect@3:27 <@3:33 Msg@3:34 >@3:37 >@3:38 {@3:40
            [@4:3 Print@4:4 (@4:9 STRING@4:10 )@4:30 ,@4:31 Exit@4:33 ]@4:37
            }@5:1
            app@7:1 light@7:5 {@7:11
            start@8:3 :@8:8 Start@8:11
            update@9:3 :@9:9 update@9:11
            grants@10:3 :@10:9 [@10:11 Console@10:12 ]@10:19
            }@11:1
            END@12:1";
        let expected: Vec<_> = expected.split_whitespace().collect();
        assert_eq!(expected.len(), 46);
        assert_eq!(describe(&src), expected);
    }

    #[test]
    fn light_paka_string_value() {
        let tokens = lex(b"Print(\"let there be light\")").unwrap();
        assert_eq!(tokens[2].kind, Kind::Str);
        assert_eq!(tokens[2].text, b"let there be light");
    }

    #[test]
    fn keywords() {
        use Kind::*;
        let want = [Enum, Fn, App, Type, View, Example, Match, End];
        assert_eq!(kinds("enum fn app type view example match"), want);
    }

    #[test]
    fn names() {
        use Kind::*;
        assert_eq!(
            kinds("A Zed9_x a b_1 z9"),
            [Upper, Upper, Lower, Lower, Lower, End]
        );
        // A keyword is a keyword only as a whole name.
        assert_eq!(kinds("enumerate Match fns"), [Lower, Upper, Lower, End]);
        assert_eq!(describe(b"a_b9"), ["a_b9@1:1", "END@1:5"]);
    }

    #[test]
    fn punctuation() {
        use Kind::*;
        let want = [
            LBrace, RBrace, LParen, RParen, LBracket, RBracket, Lt, Gt, Colon, Comma, End,
        ];
        assert_eq!(kinds("{}()[]<>:,"), want);
        assert_eq!(kinds("->"), [Arrow, End]);
        assert_eq!(kinds(">>"), [Gt, Gt, End]);
        assert_eq!(kinds("->>"), [Arrow, Gt, End]);
    }

    #[test]
    fn strings() {
        assert_eq!(kinds("\"\""), [Kind::Str, Kind::End]);
        let tokens = lex(b"\"a b\" \"~ !\"").unwrap();
        assert_eq!(tokens[0].text, b"a b");
        assert_eq!(tokens[1].text, b"~ !");
        // `//`, `??` and digits are plain bytes inside a string.
        assert_eq!(lex(b"\"// ?? 42 - _\"").unwrap()[0].text, b"// ?? 42 - _");
    }

    #[test]
    fn positions() {
        assert_eq!(describe(b""), ["END@1:1"]);
        assert_eq!(describe(b"a\n"), ["a@1:1", "END@2:1"]);
        assert_eq!(describe(b"\n\n  a b"), ["a@3:3", "b@3:5", "END@3:6"]);
        assert_eq!(
            describe(b"  \"x y\" z"),
            ["STRING@1:3", "z@1:9", "END@1:10"]
        );
    }

    #[test]
    fn end_is_repeatable() {
        let mut lexer = Lexer::new("main.paka", b"a");
        assert_eq!(lexer.next().unwrap().kind, Kind::Lower);
        assert_eq!(lexer.next().unwrap().kind, Kind::End);
        assert_eq!(lexer.next().unwrap().kind, Kind::End);
    }

    #[test]
    fn bytes_after_the_last_pulled_token_are_not_looked_at() {
        let mut lexer = Lexer::new("main.paka", b"enum \x01");
        assert_eq!(lexer.next().unwrap().kind, Kind::Enum);
        assert!(lexer.next().is_err());
        let mut lexer = Lexer::new("main.paka", b"enum ; ");
        assert_eq!(lexer.next().unwrap().kind, Kind::Enum);
    }

    #[test]
    fn error_codes_and_columns() {
        assert_eq!(code("a // b"), "main.paka:1:3: error[unsupported-comment]");
        assert_eq!(code("a ??b"), "main.paka:1:3: error[unsupported-hole]");
        assert_eq!(code("a 7"), "main.paka:1:3: error[unsupported-integer]");
        assert_eq!(code("a\n 7a"), "main.paka:2:2: error[unsupported-integer]");
        assert_eq!(code("a\t"), "main.paka:1:2: error[invalid-byte]");
        assert_eq!(code("a\r\n"), "main.paka:1:2: error[invalid-byte]");
        assert_eq!(code("a \u{e9}"), "main.paka:1:3: error[invalid-byte]");
        assert_eq!(code("a \x7f"), "main.paka:1:3: error[invalid-byte]");
        for src in [
            "a -", "a /", "a ?", "a _", "a =", "a ;", "a .", "a &", "a ?/",
        ] {
            assert_eq!(
                code(src),
                "main.paka:1:3: error[unexpected-character]",
                "{src}"
            );
        }
        assert_eq!(code("a \"b"), "main.paka:1:3: error[unterminated-string]");
        assert_eq!(
            code("a \"b\nc\""),
            "main.paka:1:3: error[unterminated-string]"
        );
        assert_eq!(
            code("a \"b\\\""),
            "main.paka:1:5: error[unsupported-escape]"
        );
        assert_eq!(
            code("a \"b{\""),
            "main.paka:1:5: error[unsupported-interpolation]"
        );
        assert_eq!(
            code("a \"b}\""),
            "main.paka:1:5: error[unsupported-interpolation]"
        );
        assert_eq!(code("a \"b\t\""), "main.paka:1:5: error[invalid-byte]");
    }

    #[test]
    fn diagnostic_carries_path_and_message() {
        let line = Lexer::new("dir/x.paka", b";")
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
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases-pending/lex");
        let mut cases: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        cases.sort();
        assert_eq!(cases.len(), 18);
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
