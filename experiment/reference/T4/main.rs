use std::collections::{HashMap, HashSet};
use std::{env, fs, process};

enum Piece {
    Text(String),
    Ref(String),
    Len(String),
    Default(String, Vec<Piece>),
}

/// A syntax error: the column of its `$` (or of the bad name character) and the message.
type Bad = (usize, &'static str);

fn is_name_start(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_ascii_lowercase())
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
}

/// Reads a name starting at `i`, which holds a lower-case letter.
fn name(cs: &[char], i: &mut usize) -> String {
    let start = *i;
    while *i < cs.len() && is_name_char(cs[*i]) {
        *i += 1;
    }
    cs[start..*i].iter().collect()
}

/// Reads pieces from `i`; `outer` is the column of the outermost open `${`, inside a default.
/// Column of `cs[k]` is `base + k`.
fn pieces(
    cs: &[char],
    i: &mut usize,
    base: usize,
    outer: Option<usize>,
) -> Result<Vec<Piece>, Bad> {
    let mut out = vec![];
    loop {
        let Some(&c) = cs.get(*i) else {
            return match outer {
                Some(col) => Err((col, "unclosed brace")),
                None => Ok(out),
            };
        };
        if c == '}' && outer.is_some() {
            return Ok(out);
        }
        let col = base + *i;
        *i += 1;
        if c != '$' {
            out.push(Piece::Text(c.to_string()));
            continue;
        }
        match cs.get(*i) {
            Some('$') => {
                *i += 1;
                out.push(Piece::Text("$".into()));
            }
            Some('{') => {
                *i += 1;
                out.push(reference(cs, i, base, outer.unwrap_or(col), col)?);
            }
            c if is_name_start(c) => out.push(Piece::Ref(name(cs, i))),
            _ => return Err((col, "bad dollar")),
        }
    }
}

/// Reads a reference after its `${`; `col` is the column of its `$`.
fn reference(
    cs: &[char],
    i: &mut usize,
    base: usize,
    outer: usize,
    col: usize,
) -> Result<Piece, Bad> {
    let unclosed = Err((outer, "unclosed brace"));
    let bad = Err((col, "bad reference"));
    let hash = cs.get(*i) == Some(&'#');
    if hash {
        *i += 1;
    }
    match cs.get(*i) {
        None => return unclosed,
        c if !is_name_start(c) => return bad,
        _ => {}
    }
    let n = name(cs, i);
    match (cs.get(*i), cs.get(*i + 1)) {
        (None, _) => unclosed,
        (Some('}'), _) => {
            *i += 1;
            Ok(if hash { Piece::Len(n) } else { Piece::Ref(n) })
        }
        (Some(':'), None) if !hash => unclosed,
        (Some(':'), Some('-')) if !hash => {
            *i += 2;
            let d = pieces(cs, i, base, Some(outer))?;
            *i += 1;
            Ok(Piece::Default(n, d))
        }
        _ => bad,
    }
}

fn definition(line: &str) -> Result<(String, Vec<Piece>), Bad> {
    let cs: Vec<char> = line.chars().collect();
    if !is_name_start(cs.first()) {
        return Err((1, "bad name"));
    }
    let mut i = 0;
    let n = name(&cs, &mut i);
    if cs.get(i) != Some(&'=') {
        return Err((i + 1, "bad name"));
    }
    i += 1;
    let ps = pieces(&cs, &mut i, 1, None)?;
    Ok((n, ps))
}

struct Expander {
    known: HashSet<String>,
    defs: HashMap<String, Vec<Piece>>,
    done: HashMap<String, String>,
    chain: Vec<String>,
}

impl Expander {
    fn expand(&mut self, n: &str) -> Result<String, String> {
        if let Some(v) = self.done.get(n) {
            return Ok(v.clone());
        }
        if let Some(at) = self.chain.iter().position(|c| c == n) {
            let mut names = self.chain[at..].to_vec();
            names.push(n.to_string());
            return Err(format!("cycle {}", names.join(" -> ")));
        }
        let ps = self
            .defs
            .remove(n)
            .expect("a defined name not yet expanded");
        self.chain.push(n.to_string());
        let v = self.pieces(&ps)?;
        self.chain.pop();
        self.done.insert(n.to_string(), v.clone());
        Ok(v)
    }

    fn lookup(&mut self, n: &str) -> Result<String, String> {
        if !self.known.contains(n) {
            return Err(format!("undefined {n} in {}", self.chain.last().unwrap()));
        }
        self.expand(n)
    }

    fn pieces(&mut self, ps: &[Piece]) -> Result<String, String> {
        let mut out = String::new();
        for p in ps {
            match p {
                Piece::Text(t) => out += t,
                Piece::Ref(n) => out += &self.lookup(n)?,
                Piece::Len(n) => out += &self.lookup(n)?.chars().count().to_string(),
                Piece::Default(n, d) => {
                    let v = match self.known.contains(n) {
                        true => self.expand(n)?,
                        false => String::new(),
                    };
                    out += &if v.is_empty() { self.pieces(d)? } else { v };
                }
            }
        }
        Ok(out)
    }
}

fn run(text: &str) -> Result<Vec<String>, String> {
    let mut order = vec![];
    let mut defs = HashMap::new();
    for (k, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (n, ps) =
            definition(line).map_err(|(col, msg)| format!("line {} col {col}: {msg}", k + 1))?;
        if !defs.contains_key(&n) {
            order.push(n.clone());
        }
        defs.insert(n, ps);
    }
    let mut ex = Expander {
        known: defs.keys().cloned().collect(),
        defs,
        done: HashMap::new(),
        chain: vec![],
    };
    order
        .iter()
        .map(|n| Ok(format!("{n}={}", ex.expand(n)?)))
        .collect()
}

fn main() {
    let path = env::args().nth(1).expect("usage: main FILE");
    let text = fs::read_to_string(path).expect("input file is readable");
    match run(&text) {
        Ok(out) => out.iter().for_each(|l| println!("{l}")),
        Err(why) => {
            println!("error: {why}");
            process::exit(20);
        }
    }
}
