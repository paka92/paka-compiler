use std::{env, fs, process};

/// A character of the joined text, with its place in the file as read.
#[derive(Clone, Copy)]
struct Ch {
    c: u8,
    line: usize,
    col: usize,
}

fn error(message: &str, at: Ch) -> String {
    format!("line {} col {}: {message}", at.line, at.col)
}

fn join(text: &str) -> Vec<Ch> {
    let bytes = text.as_bytes();
    let (mut line, mut col) = (1, 1);
    let mut joined = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && bytes.get(i + 1) == Some(&b'\n') {
            (line, col, i) = (line + 1, 1, i + 2);
            continue;
        }
        joined.push(Ch { c, line, col });
        if c == b'\n' {
            (line, col) = (line + 1, 1);
        } else {
            col += 1;
        }
        i += 1;
    }
    joined
}

fn strip(text: &[Ch]) -> Result<String, String> {
    let at = |i: usize| text.get(i).map(|ch| ch.c);
    let mut out = vec![];
    let mut i = 0;
    while i < text.len() {
        match (text[i].c, at(i + 1)) {
            (b'/', Some(b'/')) => {
                while at(i).is_some_and(|c| c != b'\n') {
                    i += 1;
                }
            }
            (b'/', Some(b'*')) => {
                let start = text[i];
                let mut depth = 1;
                i += 2;
                while depth > 0 {
                    match (at(i), at(i + 1)) {
                        (None, _) => return Err(error("unterminated comment", start)),
                        (Some(b'/'), Some(b'*')) => (depth, i) = (depth + 1, i + 2),
                        (Some(b'*'), Some(b'/')) => (depth, i) = (depth - 1, i + 2),
                        (Some(b'\n'), _) => {
                            out.push(b'\n');
                            i += 1;
                        }
                        _ => i += 1,
                    }
                }
            }
            (quote @ (b'"' | b'\''), _) => {
                let start = text[i];
                out.push(quote);
                i += 1;
                loop {
                    match (at(i), at(i + 1)) {
                        (None | Some(b'\n'), _) | (Some(b'\\'), None | Some(b'\n')) => {
                            return Err(error("unterminated string", start));
                        }
                        (Some(b'\\'), Some(next)) => {
                            out.extend([b'\\', next]);
                            i += 2;
                        }
                        (Some(c), _) => {
                            out.push(c);
                            i += 1;
                            if c == quote {
                                break;
                            }
                        }
                    }
                }
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(String::from_utf8(out).unwrap())
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let joined = join(&text);
    let stripped = strip(&joined)?;
    let mut lines: Vec<&str> = stripped.split('\n').collect();
    if joined.last().is_none_or(|ch| ch.c == b'\n') {
        lines.pop();
    }
    Ok(lines
        .iter()
        .map(|l| format!("{}\n", l.trim_end_matches(' ')))
        .collect())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    match run(file) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
