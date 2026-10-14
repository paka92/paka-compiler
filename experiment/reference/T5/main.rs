use std::cmp::Ordering;
use std::{env, fs, process};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Folded,
    Numeric,
}

struct Key {
    field: usize,
    start: usize,
    kind: Kind,
    down: bool,
}

/// A number from 1 to 99 without a leading zero, read from the front of `s`.
fn small_number(s: &mut &[u8]) -> Option<usize> {
    let digits = s.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 || digits > 2 || s[0] == b'0' {
        return None;
    }
    let n = std::str::from_utf8(&s[..digits]).ok()?.parse().ok()?;
    *s = &s[digits..];
    Some(n)
}

fn parse_key(text: &str) -> Option<Key> {
    let mut s = text.as_bytes();
    let field = small_number(&mut s)?;
    let mut start = 1;
    if let Some(rest) = s.strip_prefix(b".") {
        s = rest;
        start = small_number(&mut s)?;
    }
    let kind = match s.first() {
        Some(b'n') => Kind::Numeric,
        Some(b'i') => Kind::Folded,
        _ => Kind::Text,
    };
    if kind != Kind::Text {
        s = &s[1..];
    }
    let down = s == b"-";
    (s.is_empty() || down).then_some(Key {
        field,
        start,
        kind,
        down,
    })
}

/// The value of a number text, in millionths, or `None` when the text is not a number.
fn number(text: &str) -> Option<i64> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let fraction_ok = !body.contains('.') || (1..=6).contains(&fraction.len());
    if !(1..=9).contains(&whole.len())
        || !all_digits(whole)
        || !all_digits(fraction)
        || !fraction_ok
    {
        return None;
    }
    let value =
        whole.parse::<i64>().ok()? * 1_000_000 + format!("{fraction:0<6}").parse::<i64>().ok()?;
    Some(if negative { -value } else { value })
}

fn key_text<'a>(line: &'a str, key: &Key) -> &'a str {
    let field = line.split(',').nth(key.field - 1).unwrap_or("");
    field.get(key.start - 1..).unwrap_or("")
}

fn compare(a: &str, b: &str, key: &Key) -> Ordering {
    let (x, y) = (key_text(a, key), key_text(b, key));
    let order = match key.kind {
        Kind::Text => x.cmp(y),
        Kind::Folded => x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase()),
        Kind::Numeric => number(x).cmp(&number(y)),
    };
    if key.down { order.reverse() } else { order }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file, list] = &args[..] else {
        panic!("usage: main FILE KEYS");
    };
    let mut keys = vec![];
    for (i, text) in list.split(',').enumerate() {
        match parse_key(text) {
            Some(key) => keys.push(key),
            None => {
                println!("error: bad key {}", i + 1);
                process::exit(20);
            }
        }
    }
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_by(|a, b| {
        keys.iter()
            .map(|k| compare(a, b, k))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal)
    });
    for line in lines {
        println!("{line}");
    }
}
