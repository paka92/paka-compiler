use std::collections::BTreeMap;
use std::{env, fs, process};

#[derive(Clone, Copy)]
struct Pos {
    line: usize,
    col: usize,
}

/// An error with the position it is reported at.
type Fail = (Pos, String);

#[derive(Clone, PartialEq)]
enum Kind {
    Name(String),
    Str(String),
    Symbol(char),
}

struct Token {
    kind: Kind,
    pos: Pos,
}

enum Value {
    Str(String),
    List(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

/// Reads characters while tracking line and column.
struct Cursor {
    chars: Vec<char>,
    index: usize,
    pos: Pos,
}

impl Cursor {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.index + ahead).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek(0)?;
        self.index += 1;
        if c == '\n' {
            self.pos = Pos {
                line: self.pos.line + 1,
                col: 1,
            };
        } else {
            self.pos.col += 1;
        }
        Some(c)
    }

    /// Reads the rest of a string whose opening quote was just read at `start`.
    fn string(&mut self, start: Pos) -> Result<String, Fail> {
        let mut text = String::new();
        loop {
            let at = self.pos;
            match self.advance() {
                None | Some('\n') => return Err((start, "unterminated string".to_string())),
                Some('"') => return Ok(text),
                Some('\\') => {
                    let escaped = self.escape().ok_or((at, "bad escape".to_string()))?;
                    text.push(escaped);
                }
                Some(c) => text.push(c),
            }
        }
    }

    /// Reads what follows a backslash; `None` when it is not an escape.
    fn escape(&mut self) -> Option<char> {
        let c = match self.peek(0)? {
            c @ ('"' | '\\') => c,
            'x' => {
                let code = self.peek(1)?.to_digit(16)? * 16 + self.peek(2)?.to_digit(16)?;
                if !(32..=126).contains(&code) {
                    return None;
                }
                self.advance();
                self.advance();
                char::from_u32(code)?
            }
            _ => return None,
        };
        self.advance();
        Some(c)
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn tokenize(text: &str) -> Result<(Vec<Token>, Pos), Fail> {
    let mut cursor = Cursor {
        chars: text.chars().collect(),
        index: 0,
        pos: Pos { line: 1, col: 1 },
    };
    let mut tokens = vec![];
    while let Some(c) = cursor.peek(0) {
        let pos = cursor.pos;
        let kind = match c {
            ' ' | '\n' => {
                cursor.advance();
                continue;
            }
            '#' => {
                while cursor.peek(0).is_some_and(|c| c != '\n') {
                    cursor.advance();
                }
                continue;
            }
            '"' => {
                cursor.advance();
                Kind::Str(cursor.string(pos)?)
            }
            c if is_name_char(c) => {
                let mut name = String::new();
                while let Some(c) = cursor.peek(0).filter(|&c| is_name_char(c)) {
                    name.push(c);
                    cursor.advance();
                }
                Kind::Name(name)
            }
            '=' | '{' | '}' | '[' | ']' | ',' => {
                cursor.advance();
                Kind::Symbol(c)
            }
            _ => return Err((pos, "bad character".to_string())),
        };
        tokens.push(Token { kind, pos });
    }
    Ok((tokens, cursor.pos))
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    end: Pos,
    duplicate: Option<(Pos, String)>,
}

impl Parser {
    fn peek(&self) -> Option<&Kind> {
        self.tokens.get(self.index).map(|t| &t.kind)
    }

    fn pos(&self) -> Pos {
        self.tokens.get(self.index).map_or(self.end, |t| t.pos)
    }

    fn syntax_error<T>(&self) -> Result<T, Fail> {
        Err((self.pos(), "syntax error".to_string()))
    }

    fn expect(&mut self, symbol: char) -> Result<(), Fail> {
        if self.peek() != Some(&Kind::Symbol(symbol)) {
            return self.syntax_error();
        }
        self.index += 1;
        Ok(())
    }

    /// Entries up to the closing brace, or to the end of the file for the top table.
    fn entries(&mut self, top: bool) -> Result<BTreeMap<String, Value>, Fail> {
        let mut table = BTreeMap::new();
        loop {
            match self.peek() {
                Some(Kind::Name(name)) => {
                    let name = name.clone();
                    let pos = self.pos();
                    self.index += 1;
                    if table.contains_key(&name) && self.duplicate.is_none() {
                        self.duplicate = Some((pos, name.clone()));
                    }
                    self.expect('=')?;
                    let value = self.value()?;
                    table.insert(name, value);
                }
                Some(Kind::Symbol('}')) if !top => return Ok(table),
                None if top => return Ok(table),
                _ => return self.syntax_error(),
            }
        }
    }

    fn value(&mut self) -> Result<Value, Fail> {
        match self.peek() {
            Some(Kind::Str(text)) => {
                let text = text.clone();
                self.index += 1;
                Ok(Value::Str(text))
            }
            Some(Kind::Symbol('[')) => {
                self.index += 1;
                self.list()
            }
            Some(Kind::Symbol('{')) => {
                self.index += 1;
                let table = self.entries(false)?;
                self.expect('}')?;
                Ok(Value::Table(table))
            }
            _ => self.syntax_error(),
        }
    }

    fn list(&mut self) -> Result<Value, Fail> {
        let mut items = vec![];
        if self.peek() != Some(&Kind::Symbol(']')) {
            items.push(self.value()?);
            while self.peek() == Some(&Kind::Symbol(',')) {
                self.index += 1;
                items.push(self.value()?);
            }
        }
        self.expect(']')?;
        Ok(Value::List(items))
    }
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn print_value(path: &str, value: &Value, out: &mut String) {
    match value {
        Value::Str(text) => out.push_str(&format!("{path} = {}\n", quote(text))),
        Value::List(items) if items.is_empty() => out.push_str(&format!("{path} = []\n")),
        Value::List(items) => {
            for (i, item) in items.iter().enumerate() {
                print_value(&format!("{path}[{i}]"), item, out);
            }
        }
        Value::Table(table) if table.is_empty() => out.push_str(&format!("{path} = {{}}\n")),
        Value::Table(table) => print_entries(path, table, out),
    }
}

/// Prints the entries of a table; the top table has the empty path.
fn print_entries(path: &str, table: &BTreeMap<String, Value>, out: &mut String) {
    for (name, value) in table {
        let child = if path.is_empty() {
            name.clone()
        } else {
            format!("{path}.{name}")
        };
        print_value(&child, value, out);
    }
}

fn run(file: &str) -> Result<String, Fail> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let (tokens, end) = tokenize(&text)?;
    let mut parser = Parser {
        tokens,
        index: 0,
        end,
        duplicate: None,
    };
    let top = parser.entries(true)?;
    if let Some((pos, name)) = parser.duplicate {
        return Err((pos, format!("duplicate key {name}")));
    }
    let mut out = String::new();
    print_entries("", &top, &mut out);
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    match run(file) {
        Ok(out) => print!("{out}"),
        Err((pos, message)) => {
            println!("error: line {} col {}: {message}", pos.line, pos.col);
            process::exit(20);
        }
    }
}
