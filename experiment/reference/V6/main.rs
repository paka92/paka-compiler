use std::{env, fs, process};

fn fail(message: String) -> ! {
    println!("error: {message}");
    process::exit(20);
}

fn is_name(s: &str) -> bool {
    (1..=30).contains(&s.len())
        && s.starts_with(|c: char| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

fn files(n: usize) -> String {
    format!("{n} file{}", if n == 1 { "" } else { "s" })
}

/// A decimal number of at most `digits` digits with no leading zero unless it is 0.
fn number(s: &str, digits: usize) -> Option<usize> {
    let shaped = (1..=digits).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_digit())
        && (s.len() == 1 || !s.starts_with('0'));
    shaped.then(|| s.parse().unwrap())
}

/// The line starting at `pos` without its line feed, and where the next line starts.
fn line_at(text: &str, pos: usize) -> Option<(&str, usize)> {
    let len = text[pos..].find('\n')?;
    Some((&text[pos..pos + len], pos + len + 1))
}

fn parse_entry(line: &str) -> Option<(&str, usize)> {
    let (name, length) = line.strip_prefix("entry ")?.split_once(' ')?;
    Some((name, number(length, 5)?)).filter(|_| is_name(name))
}

/// The entries as (name, content), or the problem with the archive.
fn parse_archive(text: &str) -> Result<Vec<(&str, &str)>, String> {
    let problem = |pos: usize, reason: &str| {
        let line = 1 + text[..pos].matches('\n').count();
        format!("archive line {line}: {reason}")
    };
    let count = line_at(text, 0)
        .and_then(|(header, _)| number(header.strip_prefix("archive ")?, 3))
        .ok_or(problem(0, "bad header"))?;
    let mut pos = line_at(text, 0).unwrap().1;
    let mut entries: Vec<(&str, &str)> = vec![];
    for _ in 0..count {
        let (line, body) = line_at(text, pos).ok_or(problem(pos, "bad entry"))?;
        let (name, length) = parse_entry(line).ok_or(problem(pos, "bad entry"))?;
        if entries.iter().any(|(earlier, _)| *earlier == name) {
            return Err(problem(pos, &format!("duplicate {name}")));
        }
        let end = body + length;
        let content = text
            .get(body..end)
            .filter(|_| text[end..].starts_with('\n'));
        entries.push((name, content.ok_or(problem(pos, "bad length"))?));
        pos = end + 1;
    }
    if pos < text.len() {
        return Err(problem(pos, "trailing data"));
    }
    Ok(entries)
}

fn pack(list: &str, archive: &str) {
    let Ok(listed) = fs::read_to_string(list) else {
        fail(format!("cannot read {list}"));
    };
    let mut names: Vec<&str> = vec![];
    for (i, name) in listed.lines().enumerate() {
        if !is_name(name) {
            fail(format!("list line {}: bad name", i + 1));
        }
        if names.contains(&name) {
            fail(format!("list line {}: duplicate {name}", i + 1));
        }
        names.push(name);
    }
    let mut text = format!("archive {}\n", names.len());
    for name in &names {
        let Ok(content) = fs::read_to_string(name) else {
            fail(format!("cannot read {name}"));
        };
        text += &format!("entry {name} {}\n{content}\n", content.len());
    }
    if fs::write(archive, text).is_err() {
        fail(format!("cannot write {archive}"));
    }
    println!("packed {} into {archive}", files(names.len()));
}

fn unpack(archive: &str, which: &str) {
    if which != "*" && !is_name(which) {
        fail(format!("bad name {which}"));
    }
    let Ok(text) = fs::read_to_string(archive) else {
        fail(format!("cannot read {archive}"));
    };
    let mut entries = parse_archive(&text).unwrap_or_else(|message| fail(message));
    if which != "*" {
        entries.retain(|(name, _)| *name == which);
        if entries.is_empty() {
            fail(format!("no entry {which}"));
        }
    }
    for (name, content) in &entries {
        if fs::write(name, content).is_err() {
            fail(format!("cannot write {name}"));
        }
    }
    println!("unpacked {} from {archive}", files(entries.len()));
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [mode, first, second] = &args[..] else {
        panic!("usage: main pack LIST ARCHIVE | unpack ARCHIVE WHICH");
    };
    match mode.as_str() {
        "pack" => pack(first, second),
        "unpack" => unpack(first, second),
        _ => fail("bad mode".to_string()),
    }
}
