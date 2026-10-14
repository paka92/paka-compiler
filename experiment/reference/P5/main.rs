use std::collections::HashMap;
use std::process::exit;

const SPECIAL: [&str; 3] = ["if", "let", "define"];
const BUILTINS: [&str; 6] = ["+", "-", "*", "/", "<", "="];

enum Node {
    Atom(String, String),
    List(Vec<Node>, String),
}

impl Node {
    fn at(&self) -> &str {
        let (Node::Atom(_, at) | Node::List(_, at)) = self;
        at
    }

    // The name, when the node is a symbol that may be bound.
    fn name(&self) -> Option<&str> {
        match self {
            Node::Atom(t, _) if kind(t) == 's' && !SPECIAL.contains(&t.as_str()) => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone)]
enum Value {
    Int(i64),
    Bool(bool),
    Fn(String, usize),
}

// 'n' number, 'b' boolean, 's' symbol, 'x' bad atom.
fn kind(t: &str) -> char {
    let digits = t.strip_prefix('-').unwrap_or(t);
    let numeric = digits.starts_with(|c: char| c.is_ascii_digit());
    if numeric && digits.len() <= 7 && digits.bytes().all(|c| c.is_ascii_digit()) {
        'n'
    } else if t == "#t" || t == "#f" {
        'b'
    } else if numeric || t.starts_with('#') {
        'x'
    } else {
        's'
    }
}

fn fail(message: String) -> ! {
    println!("error: {message}");
    exit(20);
}

fn parse(text: &str) -> Vec<Node> {
    let mut open: Vec<(Vec<Node>, String)> = vec![(Vec::new(), String::new())];
    let cs: Vec<char> = text.chars().collect();
    let (mut i, mut line, mut col) = (0, 1, 1);
    while i < cs.len() {
        let at = format!("line {line} col {col}");
        let mut n = 1;
        match cs[i] {
            ';' => n = cs[i..].iter().take_while(|&&c| c != '\n').count(),
            '(' => open.push((Vec::new(), at)),
            ')' if open.len() == 1 => fail(format!("{at}: unexpected )")),
            ')' => {
                let (items, at) = open.pop().unwrap();
                open.last_mut().unwrap().0.push(Node::List(items, at));
            }
            ' ' | '\n' => {}
            _ => {
                n = cs[i..]
                    .iter()
                    .take_while(|c| !" \n();".contains(**c))
                    .count();
                let t: String = cs[i..i + n].iter().collect();
                if kind(&t) == 'x' {
                    fail(format!("{at}: bad atom"));
                }
                open.last_mut().unwrap().0.push(Node::Atom(t, at));
            }
        }
        (line, col) = if cs[i] == '\n' {
            (line + 1, 1)
        } else {
            (line, col + n)
        };
        i += n;
    }
    if let Some((_, at)) = open.get(1) {
        fail(format!("{at}: unclosed ("));
    }
    open.pop().unwrap().0
}

type Env = HashMap<String, Value>;

struct Lisp<'a> {
    funcs: Vec<(Vec<String>, &'a Node)>,
    globals: Env,
    depth: usize,
}

impl<'a> Lisp<'a> {
    fn eval(&mut self, node: &'a Node, env: &Env) -> Result<Value, String> {
        let items = match node {
            Node::Atom(t, at) => {
                return match kind(t) {
                    'n' => Ok(Value::Int(t.parse().unwrap())),
                    'b' => Ok(Value::Bool(t == "#t")),
                    _ if BUILTINS.contains(&t.as_str())
                        && !env.contains_key(t)
                        && !self.globals.contains_key(t) =>
                    {
                        Ok(Value::Fn(t.clone(), usize::MAX))
                    }
                    _ => env
                        .get(t)
                        .or(self.globals.get(t))
                        .cloned()
                        .ok_or(format!("{at}: unbound {t}")),
                };
            }
            Node::List(items, _) => items,
        };
        let err = |what: &str| Err(format!("{}: {what}", node.at()));
        if items.is_empty() {
            return err("empty call");
        }
        let head = if let Some(Node::Atom(t, _)) = items.first() {
            t.as_str()
        } else {
            ""
        };
        match head {
            "define" => return err("bad define"),
            "if" if items.len() != 4 => return err("bad if"),
            "if" => {
                return match self.eval(&items[1], env)? {
                    Value::Bool(c) => self.eval(&items[if c { 2 } else { 3 }], env),
                    _ => err("type error"),
                };
            }
            "let" => {
                let pairs = match &items[..] {
                    [_, Node::List(pairs, _), _] if pairs.len() % 2 == 0 => pairs,
                    _ => return err("bad let"),
                };
                let names: Vec<Option<&str>> = pairs.iter().step_by(2).map(Node::name).collect();
                if !distinct(&names) {
                    return err("bad let");
                }
                let mut inner = env.clone();
                for (name, expr) in names.iter().zip(pairs.iter().skip(1).step_by(2)) {
                    inner.insert(name.unwrap().to_string(), self.eval(expr, env)?);
                }
                return self.eval(&items[2], &inner);
            }
            _ => {}
        }
        let mut args = Vec::new();
        for item in items {
            args.push(self.eval(item, env)?);
        }
        let Value::Fn(name, f) = args.remove(0) else {
            return err("not a function");
        };
        if f == usize::MAX {
            return builtin(&name, &args).map_err(|what| format!("{}: {what}", node.at()));
        }
        let (params, body) = (self.funcs[f].0.clone(), self.funcs[f].1);
        if params.len() != args.len() {
            return err("wrong arity");
        }
        if self.depth == 50 {
            return err("too deep");
        }
        self.depth += 1;
        let v = self.eval(body, &params.into_iter().zip(args).collect());
        self.depth -= 1;
        v
    }

    fn define(&mut self, items: &'a [Node], at: &str) -> Result<(), String> {
        let value = match items {
            [_, Node::List(sig, _), body] if !sig.is_empty() => {
                let names: Vec<Option<&str>> = sig.iter().map(Node::name).collect();
                if names[0].is_none() || !distinct(&names[1..]) {
                    return Err(format!("{at}: bad define"));
                }
                let params = names[1..].iter().map(|p| p.unwrap().to_string()).collect();
                self.funcs.push((params, body));
                Value::Fn(names[0].unwrap().to_string(), self.funcs.len() - 1)
            }
            [_, Node::Atom(..), expr] if items[1].name().is_some() => {
                self.eval(expr, &Env::new())?
            }
            _ => return Err(format!("{at}: bad define")),
        };
        let name = match &items[1] {
            Node::List(sig, _) => sig[0].name(),
            atom => atom.name(),
        };
        self.globals.insert(name.unwrap().to_string(), value);
        Ok(())
    }
}

// Every name is a name, and no two are the same.
fn distinct(names: &[Option<&str>]) -> bool {
    names
        .iter()
        .enumerate()
        .all(|(i, n)| n.is_some() && !names[..i].contains(n))
}

fn builtin(name: &str, args: &[Value]) -> Result<Value, &'static str> {
    if args.len() != 2 {
        return Err("wrong arity");
    }
    let (a, b) = match (name, &args[0], &args[1]) {
        ("=", Value::Bool(a), Value::Bool(b)) => return Ok(Value::Bool(a == b)),
        (_, Value::Int(a), Value::Int(b)) => (*a, *b),
        _ => return Err("type error"),
    };
    let r = match name {
        "=" => return Ok(Value::Bool(a == b)),
        "<" => return Ok(Value::Bool(a < b)),
        "/" if b == 0 => return Err("division by zero"),
        "/" => a / b,
        "+" => a + b,
        "-" => a - b,
        _ => a * b,
    };
    if r.abs() > 9_999_999 {
        Err("overflow")
    } else {
        Ok(Value::Int(r))
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::Int(n) => n.to_string(),
        Value::Bool(b) => (if *b { "#t" } else { "#f" }).to_string(),
        Value::Fn(name, _) => format!("<fn {name}>"),
    }
}

fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let forms = parse(&text);
    let mut lisp = Lisp {
        funcs: Vec::new(),
        globals: Env::new(),
        depth: 0,
    };
    for form in &forms {
        let result = match form {
            Node::List(items, at) if matches!(items.first(), Some(Node::Atom(t, _)) if t == "define") => {
                lisp.define(items, at)
            }
            _ => lisp
                .eval(form, &Env::new())
                .map(|v| println!("{}", show(&v))),
        };
        if let Err(message) = result {
            fail(message);
        }
    }
}
