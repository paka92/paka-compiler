//! A strict JSON reader and a compact writer, shared by the compiler and the harness. Numbers stay
//! as text, so no float is ever made; objects keep their keys in the order given.

use std::fmt::{self, Display, Write};

/// Levels of nesting allowed; deeper is an error, not a stack overflow.
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// The number as written.
    Num(String),
    Str(String),
    Array(Vec<Value>),
    /// Fields in the order written; a repeated key is an error.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// An object from keys and values in that order.
    pub fn object<'k>(fields: impl IntoIterator<Item = (&'k str, Value)>) -> Value {
        Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// A whole number from 0 to `u64::MAX`; a fraction, an exponent or a sign gives `None`.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Num(n) => n.parse().ok(),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { text, at: 0 };
    let value = p.value(0)?;
    p.skip_space();
    match p.peek() {
        None => Ok(value),
        Some(_) => p.fail("text after the value"),
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Value {
        Value::Num(n.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::Str(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Str(s)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Value {
        v.map_or(Value::Null, Into::into)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Value {
        Value::Array(items.into_iter().map(Into::into).collect())
    }
}

/// Compact: no space anywhere.
impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Num(n) => f.write_str(n),
            Value::Str(s) => write_quoted(f, s),
            Value::Array(items) => {
                f.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_char(']')
            }
            Value::Object(fields) => {
                f.write_char('{')?;
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write_quoted(f, key)?;
                    write!(f, ":{value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

/// A quoted string; control characters are escaped, everything else is written as is.
fn write_quoted(out: &mut impl Write, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if c < ' ' => write!(out, "\\u{:04x}", u32::from(c))?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

/// The text as a JSON string literal.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    write_quoted(&mut out, text).expect("writing to a String");
    out
}

/// A JSON object from keys and values already written as JSON.
pub fn object<'a>(fields: impl IntoIterator<Item = (&'a str, String)>) -> String {
    let fields: Vec<String> = fields
        .into_iter()
        .map(|(key, value)| format!("{}:{value}", quote(key)))
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// A JSON array from values already written as JSON.
pub fn array(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

type Parse<T> = Result<T, String>;

impl Parser<'_> {
    fn fail<T>(&self, what: &str) -> Parse<T> {
        Err(format!("{what} at byte {}", self.at))
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        self.at += usize::from(found);
        found
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn value(&mut self, depth: usize) -> Parse<Value> {
        if depth >= MAX_DEPTH {
            return self.fail("nesting too deep");
        }
        self.skip_space();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::Str),
            Some(b't') => self.word("true", Value::Bool(true)),
            Some(b'f') => self.word("false", Value::Bool(false)),
            Some(b'n') => self.word("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => self.fail("no value"),
        }
    }

    fn word(&mut self, word: &str, value: Value) -> Parse<Value> {
        if !self.text[self.at..].starts_with(word) {
            return self.fail("unknown word");
        }
        self.at += word.len();
        Ok(value)
    }

    fn array(&mut self, depth: usize) -> Parse<Value> {
        self.at += 1;
        let mut items = vec![];
        self.skip_space();
        if self.eat(b']') {
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            if self.eat(b']') {
                return Ok(Value::Array(items));
            }
            if !self.eat(b',') {
                return self.fail("expected , or ]");
            }
        }
    }

    fn object(&mut self, depth: usize) -> Parse<Value> {
        self.at += 1;
        let mut fields: Vec<(String, Value)> = vec![];
        self.skip_space();
        if self.eat(b'}') {
            return Ok(Value::Object(fields));
        }
        loop {
            self.skip_space();
            if self.peek() != Some(b'"') {
                return self.fail("expected a key");
            }
            let key = self.string()?;
            if fields.iter().any(|(k, _)| *k == key) {
                return self.fail("repeated key");
            }
            self.skip_space();
            if !self.eat(b':') {
                return self.fail("expected :");
            }
            fields.push((key, self.value(depth + 1)?));
            self.skip_space();
            if self.eat(b'}') {
                return Ok(Value::Object(fields));
            }
            if !self.eat(b',') {
                return self.fail("expected , or }");
            }
        }
    }

    fn number(&mut self) -> Parse<Value> {
        let start = self.at;
        self.eat(b'-');
        if !self.eat(b'0') {
            self.digits()?;
        }
        if self.eat(b'.') {
            self.digits()?;
        }
        if self.eat(b'e') || self.eat(b'E') {
            let _ = self.eat(b'+') || self.eat(b'-');
            self.digits()?;
        }
        Ok(Value::Num(self.text[start..self.at].into()))
    }

    fn digits(&mut self) -> Parse<()> {
        let start = self.at;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        if self.at == start {
            return self.fail("expected a digit");
        }
        Ok(())
    }

    /// Reads from the opening quote; plain runs are copied whole, so UTF-8 is kept as it is.
    fn string(&mut self) -> Parse<String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let run = self.at;
            while self
                .peek()
                .is_some_and(|b| b != b'"' && b != b'\\' && b >= b' ')
            {
                self.at += 1;
            }
            out.push_str(&self.text[run..self.at]);
            match self.peek() {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    out.push(self.escape()?);
                }
                _ => return self.fail("unterminated string or raw control character"),
            }
        }
    }

    fn escape(&mut self) -> Parse<char> {
        let Some(byte) = self.peek() else {
            return self.fail("unterminated escape");
        };
        self.at += 1;
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode(),
            _ => return self.fail("unknown escape"),
        })
    }

    /// After `\u`: one code unit, or a high and a low surrogate that make one character.
    fn unicode(&mut self) -> Parse<char> {
        let unit = self.hex4()?;
        let code = match unit {
            0xd800..=0xdbff => {
                if !(self.eat(b'\\') && self.eat(b'u')) {
                    return self.fail("unpaired surrogate");
                }
                let low = self.hex4()?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return self.fail("unpaired surrogate");
                }
                0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00)
            }
            0xdc00..=0xdfff => return self.fail("unpaired surrogate"),
            _ => unit,
        };
        char::from_u32(code).map_or_else(|| self.fail("bad code point"), Ok)
    }

    fn hex4(&mut self) -> Parse<u32> {
        let digits = self.text.get(self.at..self.at + 4);
        match digits.filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit())) {
            Some(d) => {
                self.at += 4;
                Ok(u32::from_str_radix(d, 16).expect("four hex digits"))
            }
            None => self.fail("expected four hex digits"),
        }
    }
}
