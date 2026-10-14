//! A strict JSON reader and string quoter. Numbers stay as text, so no float is ever made.

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

/// The text as a JSON string literal.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
