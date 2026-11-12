//! Seeded edit sequences over a generated program (M4-43). Each edit changes what the program does
//! or where its code and data sit, so code compiled before it goes stale: a literal, a record
//! field or a variant put first, a function added, deleted or given a parameter, two definitions
//! swapped. An edit builds on the last version that checks, as an author rewrites a broken one.

use crate::Rng;

pub const KINDS: [&str; 7] = [
    "literal",
    "layout",
    "variant",
    "add",
    "delete",
    "swap",
    "signature",
];

/// One version of the program: the edit that made it, "start" for the first, and its text.
pub struct Version {
    pub kind: &'static str,
    pub text: String,
    pub checks: bool,
}

/// The program of `seed` and the `steps` edits after it; `checks` says whether a text checks.
pub fn versions(seed: u64, steps: usize, checks: impl Fn(&str) -> bool) -> Vec<Version> {
    let mut rng = Rng(!seed);
    let text = crate::program(seed);
    let mut base = defs(&text);
    let mut out = vec![Version {
        kind: "start",
        checks: checks(&text),
        text,
    }];
    while out.len() <= steps {
        let kind = *rng.pick(&KINDS);
        let mut next = base.clone();
        if !edit(&mut rng, &mut next, kind, out.len()) {
            continue;
        }
        let text = next.join("\n\n") + "\n";
        let ok = checks(&text);
        if ok {
            base = next;
        }
        out.push(Version {
            kind,
            text,
            checks: ok,
        });
    }
    out
}

/// The top-level definitions of `text`; each starts at a line with no indent.
fn defs(text: &str) -> Vec<String> {
    let mut defs: Vec<String> = vec![];
    for line in text.lines().filter(|l| !l.is_empty()) {
        match defs.last_mut() {
            Some(d) if line.starts_with([' ', '}']) => {
                d.push('\n');
                d.push_str(line);
            }
            _ => defs.push(line.into()),
        }
    }
    defs
}

/// Applies an edit of `kind` to `defs`, numbered `n`; false when none fits.
fn edit(rng: &mut Rng, defs: &mut Vec<String>, kind: &str, n: usize) -> bool {
    let of = |prefix: &str| -> Vec<usize> {
        (0..defs.len())
            .filter(|&i| defs[i].starts_with(prefix))
            .collect()
    };
    match kind {
        "literal" => {
            let fns = of("fn ");
            let Some(&i) = fns.get(rng.below(fns.len().max(1))) else {
                return false;
            };
            let words = literals(&defs[i]);
            let Some(&(at, len)) = words.get(rng.below(words.len().max(1))) else {
                return false;
            };
            let new = match &defs[i][at..at + len] {
                "true" => "false",
                "false" => "true",
                _ => rng.pick(&["0", "1", "2", "3", "7", "100"]),
            };
            defs[i].replace_range(at..at + len, new);
        }
        "layout" | "variant" => {
            let prefix = if kind == "layout" { "type " } else { "enum " };
            let found = of(prefix);
            let Some(&i) = found.get(rng.below(found.len().max(1))) else {
                return false;
            };
            let first = match kind {
                "layout" => format!("{{ z{n}: Int = {n}, "),
                _ => format!("{{ Z{n}, "),
            };
            defs[i] = defs[i].replacen("{ ", &first, 1);
        }
        "add" => {
            let at = rng.below(defs.len() + 1);
            defs.insert(at, format!("fn g{n}() -> Int {{\n  {n}\n}}"));
        }
        "delete" => {
            let added = of("fn g");
            let Some(&i) = added.get(rng.below(added.len().max(1))) else {
                return false;
            };
            defs.remove(i);
        }
        "swap" => {
            let (i, j) = (rng.below(defs.len()), rng.below(defs.len()));
            defs.swap(i, j);
        }
        _ => {
            let fns: Vec<usize> = (of("fn ").into_iter())
                .filter(|&i| !defs[i].starts_with("fn update("))
                .collect();
            let Some(&i) = fns.get(rng.below(fns.len().max(1))) else {
                return false;
            };
            let name = defs[i][3..]
                .split('(')
                .next()
                .unwrap_or_default()
                .to_string();
            for d in defs.iter_mut() {
                *d = with_parameter(d, &name, n);
            }
        }
    }
    true
}

/// `d` with an Int parameter put first in `name`'s header and `n` first in each call of it.
fn with_parameter(d: &str, name: &str, n: usize) -> String {
    let call = format!("{name}(");
    let mut out = String::with_capacity(d.len());
    let mut rest = d;
    while let Some(at) = rest.find(&call) {
        let before = rest[..at].chars().next_back();
        let end = at + call.len();
        out.push_str(&rest[..end]);
        rest = &rest[end..];
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let header = out.ends_with(&format!("fn {call}"));
        out.push_str(&if header {
            format!("q{n}: Int")
        } else {
            n.to_string()
        });
        if !rest.starts_with(')') {
            out.push_str(", ");
        }
    }
    out + rest
}

/// The Int literals and Bool words of a body, outside strings, as (offset, length).
fn literals(d: &str) -> Vec<(usize, usize)> {
    let b = d.as_bytes();
    let body = d.find('\n').unwrap_or(d.len());
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let (mut found, mut i) = (vec![], body);
    while i < b.len() {
        if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += 1 + usize::from(b[i] == b'\\');
            }
            i += 1;
            continue;
        }
        if !word(b[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && word(b[i]) {
            i += 1;
        }
        let w = &d[start..i];
        let dotted = b[start - 1] == b'.' || b.get(i) == Some(&b'.');
        let int = w.bytes().all(|c| c.is_ascii_digit()) && !dotted;
        if int || w == "true" || w == "false" {
            found.push((start, i - start));
        }
    }
    found
}
