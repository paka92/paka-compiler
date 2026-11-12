use std::{env, fs, process};

/// One command: `kind` is `d`, `c` or `a`; `d` and `c` take lines `from` to `to`.
struct Command {
    line: usize,
    kind: u8,
    from: usize,
    to: usize,
    text: Vec<String>,
}

impl Command {
    /// The first and last place taken: gap g is 2g, line l is 2l - 1.
    fn places(&self) -> (usize, usize) {
        match self.kind {
            b'a' => (2 * self.from, 2 * self.from),
            _ => (2 * self.from - 1, 2 * self.to - 1),
        }
    }
}

fn number(s: &str) -> Option<usize> {
    let ok = s == "0" || (1..=4).contains(&s.len()) && !s.starts_with('0');
    (ok && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().unwrap())
}

fn text_line(line: &str) -> Option<String> {
    match line {
        ">" => Some(String::new()),
        _ => line.strip_prefix("> ").map(String::from),
    }
}

fn command(line: &str) -> Option<(u8, usize, usize)> {
    let (kind, range) = line.split_at_checked(2)?;
    let kind = match kind {
        "d " => b'd',
        "c " => b'c',
        "a " => b'a',
        _ => return None,
    };
    let (from, to) = match range.split_once('-') {
        Some((a, b)) if kind != b'a' => (number(a)?, number(b)?),
        Some(_) => return None,
        None => (number(range)?, number(range)?),
    };
    (kind == b'a' || from >= 1).then_some((kind, from, to))
}

fn parse_script(text: &str) -> Result<Vec<Command>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut commands: Vec<Command> = vec![];
    for (i, &line) in lines.iter().enumerate() {
        let at = |why: &str| format!("error: script line {}: {why}", i + 1);
        if let Some(text) = text_line(line) {
            match commands.last_mut() {
                Some(c) if c.kind != b'd' => c.text.push(text),
                _ => return Err(at("unexpected text")),
            }
            continue;
        }
        let (kind, from, to) = command(line).ok_or_else(|| at("bad line"))?;
        if to < from {
            return Err(at("backward range"));
        }
        let next_is_text = lines.get(i + 1).is_some_and(|l| text_line(l).is_some());
        if kind != b'd' && !next_is_text {
            return Err(at("missing text"));
        }
        commands.push(Command {
            line: i + 1,
            kind,
            from,
            to,
            text: vec![],
        });
    }
    Ok(commands)
}

/// The lines of the result, and whether its last line keeps a missing line feed.
fn apply(target: &str, commands: &[Command]) -> Result<(Vec<String>, bool), String> {
    let lines: Vec<&str> = target.lines().collect();
    let n = lines.len();
    let mut last_place = 0;
    for (k, c) in commands.iter().enumerate() {
        let at = |why: &str| format!("error: script line {}: {why}", c.line);
        if c.to > n {
            return Err(at("past end"));
        }
        let (first, last) = c.places();
        if k > 0 && first <= last_place {
            return Err(at("out of order"));
        }
        last_place = last;
    }
    let mut out: Vec<String> = vec![];
    let mut next = 1; // the next target line to copy
    let mut last_kept = !target.ends_with('\n') && n > 0;
    for c in commands {
        let upto = if c.kind == b'a' { c.from } else { c.from - 1 };
        out.extend(lines[next - 1..upto].iter().map(|l| l.to_string()));
        out.extend(c.text.iter().cloned());
        next = if c.kind == b'a' { upto + 1 } else { c.to + 1 };
        if c.to == n {
            last_kept = false;
        }
    }
    out.extend(lines[next - 1..].iter().map(|l| l.to_string()));
    Ok((out, last_kept))
}

fn read(name: &str) -> Result<String, String> {
    fs::read_to_string(name).map_err(|_| format!("error: cannot read {name}"))
}

fn run(target: &str, script: &str, output: &str) -> Result<String, String> {
    let commands = parse_script(&read(script)?)?;
    let (lines, last_kept) = apply(&read(target)?, &commands)?;
    let mut text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    if last_kept {
        text.pop();
    }
    fs::write(output, text).map_err(|_| format!("error: cannot write {output}"))?;
    let added: usize = commands.iter().map(|c| c.text.len()).sum();
    let gone: usize = commands
        .iter()
        .filter(|c| c.kind != b'a')
        .map(|c| c.to - c.from + 1)
        .sum();
    let k = lines.len();
    let word = if k == 1 { "line" } else { "lines" };
    Ok(format!("wrote {k} {word} to {output} (+{added} -{gone})"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [target, script, output] = &args[..] else {
        panic!("usage: main TARGET SCRIPT OUTPUT");
    };
    match run(target, script, output) {
        Ok(line) => println!("{line}"),
        Err(line) => {
            println!("{line}");
            process::exit(20);
        }
    }
}
