use std::collections::HashMap;
use std::{env, fs, process};

/// An error with the column it is reported at.
type Fail = (usize, String);

#[derive(Clone)]
enum Kind {
    Number(i64),
    Name(String),
    Symbol(char),
}

#[derive(Clone)]
struct Token {
    kind: Kind,
    col: usize,
}

enum Expr {
    Number(i64),
    Variable(String, usize),
    Negate(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>, usize),
}

fn is_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn tokenize(line: &str) -> Result<Vec<Token>, Fail> {
    let bytes = line.as_bytes();
    if let Some(i) = bytes.iter().position(|&c| {
        !(c == b' ' || c.is_ascii_alphanumeric() || c == b'_' || b"+-*/%()=".contains(&c))
    }) {
        return Err((i + 1, "bad character".to_string()));
    }
    let mut tokens = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let c = bytes[i];
        let kind = if c == b' ' {
            i += 1;
            continue;
        } else if c.is_ascii_digit() {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            Kind::Number(line[start..i].parse().expect("number fits"))
        } else if is_name_start(c) {
            while i < bytes.len() && (is_name_start(bytes[i]) || bytes[i].is_ascii_digit()) {
                i += 1;
            }
            Kind::Name(line[start..i].to_string())
        } else {
            i += 1;
            Kind::Symbol(c as char)
        };
        tokens.push(Token {
            kind,
            col: start + 1,
        });
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    end_col: usize,
}

impl Parser {
    fn peek_symbol(&self) -> Option<char> {
        match self.tokens.get(self.pos)?.kind {
            Kind::Symbol(c) => Some(c),
            _ => None,
        }
    }

    fn syntax_error<T>(&self) -> Result<T, Fail> {
        let col = self.tokens.get(self.pos).map_or(self.end_col, |t| t.col);
        Err((col, "syntax error".to_string()))
    }

    fn line(&mut self) -> Result<(Option<String>, Expr), Fail> {
        let assigned = match (self.tokens.first(), self.tokens.get(1)) {
            (
                Some(Token {
                    kind: Kind::Name(name),
                    ..
                }),
                Some(Token {
                    kind: Kind::Symbol('='),
                    ..
                }),
            ) => Some(name.clone()),
            _ => None,
        };
        if assigned.is_some() {
            self.pos = 2;
        }
        let expr = self.expression()?;
        if self.pos < self.tokens.len() {
            return self.syntax_error();
        }
        Ok((assigned, expr))
    }

    fn expression(&mut self) -> Result<Expr, Fail> {
        self.binary("+-", Self::term)
    }

    fn term(&mut self) -> Result<Expr, Fail> {
        self.binary("*/%", Self::unary)
    }

    /// Operands joined from the left by any of the operators.
    fn binary(
        &mut self,
        operators: &str,
        operand: fn(&mut Self) -> Result<Expr, Fail>,
    ) -> Result<Expr, Fail> {
        let mut left = operand(self)?;
        while let Some(op) = self.peek_symbol().filter(|c| operators.contains(*c)) {
            let col = self.tokens[self.pos].col;
            self.pos += 1;
            let right = operand(self)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right), col);
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Fail> {
        if self.peek_symbol() == Some('-') {
            self.pos += 1;
            return Ok(Expr::Negate(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, Fail> {
        let Some(token) = self.tokens.get(self.pos).cloned() else {
            return self.syntax_error();
        };
        match token.kind {
            Kind::Number(n) => {
                self.pos += 1;
                Ok(Expr::Number(n))
            }
            Kind::Name(name) => {
                self.pos += 1;
                Ok(Expr::Variable(name, token.col))
            }
            Kind::Symbol('(') => {
                self.pos += 1;
                let inner = self.expression()?;
                if self.peek_symbol() != Some(')') {
                    return self.syntax_error();
                }
                self.pos += 1;
                Ok(inner)
            }
            Kind::Symbol(_) => self.syntax_error(),
        }
    }
}

fn eval(expr: &Expr, vars: &HashMap<String, i64>) -> Result<i64, Fail> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name, col) => vars
            .get(name)
            .copied()
            .ok_or((*col, format!("unknown name {name}"))),
        Expr::Negate(inner) => Ok(-eval(inner, vars)?),
        Expr::Binary(op, left, right, col) => {
            let (a, b) = (eval(left, vars)?, eval(right, vars)?);
            match op {
                '+' => Ok(a + b),
                '-' => Ok(a - b),
                '*' => Ok(a * b),
                _ if b == 0 => Err((*col, "division by zero".to_string())),
                '/' => Ok(a / b),
                _ => Ok(a % b),
            }
        }
    }
}

/// Runs one line; an expression line gives its value.
fn run_line(line: &str, vars: &mut HashMap<String, i64>) -> Result<Option<i64>, Fail> {
    let tokens = tokenize(line)?;
    if tokens.is_empty() {
        return Ok(None);
    }
    let mut parser = Parser {
        tokens,
        pos: 0,
        end_col: line.len() + 1,
    };
    let (assigned, expr) = parser.line()?;
    let value = eval(&expr, vars)?;
    match assigned {
        Some(name) => {
            vars.insert(name, value);
            Ok(None)
        }
        None => Ok(Some(value)),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut vars = HashMap::new();
    let mut status = 0;
    for (i, line) in text.lines().enumerate() {
        match run_line(line, &mut vars) {
            Ok(Some(value)) => println!("{value}"),
            Ok(None) => {}
            Err((col, message)) => {
                println!("error: line {} col {col}: {message}", i + 1);
                status = 20;
                break;
            }
        }
    }
    process::exit(status);
}
