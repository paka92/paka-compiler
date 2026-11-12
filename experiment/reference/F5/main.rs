use std::{env, fs, process};

struct Chapter {
    title: String,
    lines: Vec<String>,
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|b| b == b' ')
}

/// The title of a heading line, or None for any other line.
fn heading(line: &str) -> Option<&str> {
    match line {
        "#" => Some(""),
        _ => line.strip_prefix("# ").map(|t| t.trim_matches(' ')),
    }
}

fn slug(title: &str) -> String {
    let mut s = String::new();
    for c in title.to_ascii_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s: String = s.chars().take(20).collect();
    let s = s.trim_end_matches('-');
    if s.is_empty() {
        "chapter".into()
    } else {
        s.into()
    }
}

fn chapters(text: &str) -> Result<Vec<Chapter>, String> {
    let mut found: Vec<Chapter> = vec![];
    for (i, line) in text.lines().enumerate() {
        let at = |why| format!("error: line {}: {why}", i + 1);
        match (heading(line), found.last_mut()) {
            (Some(""), _) => return Err(at("empty title")),
            (Some(title), _) => found.push(Chapter {
                title: title.into(),
                lines: vec![],
            }),
            (None, Some(c)) => c.lines.push(line.into()),
            (None, None) if is_blank(line) => {}
            (None, None) => return Err(at("text before first chapter")),
        }
    }
    if found.is_empty() {
        return Err("error: no chapters".into());
    }
    for c in &mut found {
        while c.lines.last().is_some_and(|l| is_blank(l)) {
            c.lines.pop();
        }
        let lead = c.lines.iter().take_while(|l| is_blank(l)).count();
        c.lines.drain(..lead);
    }
    Ok(found)
}

fn bases(found: &[Chapter]) -> Vec<String> {
    let mut taken: Vec<String> = vec![];
    for c in found {
        let s = slug(&c.title);
        let mut base = s.clone();
        let mut n = 2;
        while taken.contains(&base) {
            base = format!("{s}-{n}");
            n += 1;
        }
        taken.push(base);
    }
    taken
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn write(name: &str, text: &str) -> Result<(), String> {
    fs::write(name, text).map_err(|_| format!("error: cannot write {name}"))
}

fn run(book: &str, toc_name: &str) -> Result<String, String> {
    let text = fs::read_to_string(book).map_err(|_| format!("error: cannot read {book}"))?;
    let found = chapters(&text)?;
    let mut toc = String::new();
    for (k, (c, base)) in found.iter().zip(bases(&found)).enumerate() {
        let name = format!("{base}.txt");
        let body: String = c.lines.iter().map(|l| format!("{l}\n")).collect();
        write(&name, &format!("# {}\n{body}", c.title))?;
        let count = plural(c.lines.len(), "line");
        toc += &format!("{}. {} -> {name} ({count})\n", k + 1, c.title);
    }
    write(toc_name, &toc)?;
    Ok(format!(
        "split {book} into {}",
        plural(found.len(), "chapter")
    ))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [book, toc] = &args[..] else {
        panic!("usage: main BOOK CONTENTS");
    };
    match run(book, toc) {
        Ok(line) => println!("{line}"),
        Err(line) => {
            println!("{line}");
            process::exit(20);
        }
    }
}
