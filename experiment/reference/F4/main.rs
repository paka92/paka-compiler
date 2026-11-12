use std::{env, fs, process};

/// The map: (OLD in lower case, NEW as written), in map order.
type Map = Vec<(String, String)>;

fn is_key(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn read(name: &str) -> Result<String, String> {
    fs::read_to_string(name).map_err(|_| format!("error: cannot read {name}"))
}

fn parse_map(text: &str) -> Result<Map, String> {
    let mut map: Map = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        let at = |why: String| format!("error: map line {}: {why}", i + 1);
        match words[..] {
            [] => continue,
            [old, new] if is_key(old) && is_key(new) => {
                if map.iter().any(|(o, _)| *o == old.to_ascii_lowercase()) {
                    return Err(at(format!("duplicate {old}")));
                }
                map.push((old.to_ascii_lowercase(), new.to_string()));
            }
            _ => return Err(at("bad entry".into())),
        }
    }
    if map.is_empty() {
        return Err("error: empty map".into());
    }
    Ok(map)
}

/// The page with its links rewritten, and the number rewritten.
fn rewrite(name: &str, text: &str, map: &Map) -> Result<(String, usize), String> {
    let mut out = String::new();
    let mut count = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let mut rest = line;
        while let Some(start) = rest.find("[[") {
            let col = line.len() - rest.len() + start + 1;
            let at = |why| format!("error: {name} line {} col {col}: {why}", i + 1);
            out += &rest[..start];
            let after = &rest[start + 2..];
            let end = after.find("]]").ok_or_else(|| at("unclosed link"))?;
            let inside = &after[..end];
            let (target, label) = match inside.split_once('|') {
                Some((t, l)) => (t, Some(l)),
                None => (inside, None),
            };
            if !is_key(target) || label.is_some_and(|l| l.contains('|')) {
                return Err(at("bad link"));
            }
            let lower = target.to_ascii_lowercase();
            let target = match map.iter().find(|(old, _)| *old == lower) {
                Some((_, new)) => {
                    count += 1;
                    new
                }
                None => target,
            };
            out += &format!("[[{target}");
            if let Some(label) = label {
                out += &format!("|{label}");
            }
            out += "]]";
            rest = &after[end + 2..];
        }
        out += rest;
    }
    Ok((out, count))
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn run(map_name: &str, pages: &[String]) -> Result<String, String> {
    let map = parse_map(&read(map_name)?)?;
    let texts = pages
        .iter()
        .map(|p| read(p))
        .collect::<Result<Vec<_>, _>>()?;
    let mut results = vec![];
    for (name, text) in pages.iter().zip(&texts) {
        results.push(rewrite(name, text, &map)?);
    }
    let mut report = String::new();
    let (mut links, mut written) = (0, 0);
    for (name, (text, count)) in pages.iter().zip(&results) {
        if *count > 0 {
            fs::write(name, text).map_err(|_| format!("error: cannot write {name}"))?;
            written += 1;
        }
        links += count;
        report += &format!("{name}: {}\n", plural(*count, "link"));
    }
    let (l, p) = (plural(links, "link"), plural(written, "page"));
    Ok(report + &format!("rewrote {l} in {p}\n"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [map, list] = &args[..] else {
        panic!("usage: main MAP LIST");
    };
    let list = fs::read_to_string(list).expect("list is readable");
    let pages: Vec<String> = list.lines().map(String::from).collect();
    match run(map, &pages) {
        Ok(out) => print!("{out}"),
        Err(line) => {
            println!("{line}");
            process::exit(20);
        }
    }
}
