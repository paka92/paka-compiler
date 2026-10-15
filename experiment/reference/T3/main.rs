use std::{env, fs, process};

/// Derived ordering: a number chunk comes before a text chunk.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    Number(u64),
    Text(String),
}

fn fail(message: &str) -> ! {
    println!("error: {message}");
    process::exit(20);
}

/// The chunks of a line, or `None` when a number chunk has more than 15 digits.
fn chunks(line: &str) -> Option<Vec<Chunk>> {
    let mut chunks = vec![];
    let mut rest = line;
    while let Some(first) = rest.chars().next() {
        let is_digit = first.is_ascii_digit();
        let end = rest
            .find(|c: char| c.is_ascii_digit() != is_digit)
            .unwrap_or(rest.len());
        let (chunk, after) = rest.split_at(end);
        chunks.push(match is_digit {
            true if chunk.len() > 15 => return None,
            true => Chunk::Number(chunk.parse().unwrap()),
            false => Chunk::Text(chunk.to_ascii_lowercase()),
        });
        rest = after;
    }
    Some(chunks)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file, order] = &args[..] else {
        panic!("usage: main FILE up|down");
    };
    if order != "up" && order != "down" {
        fail("bad order");
    }
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut keyed = vec![];
    for (i, line) in text.lines().enumerate() {
        match chunks(line) {
            Some(key) => keyed.push((key, line)),
            None => fail(&format!("line {}: number too long", i + 1)),
        }
    }
    keyed.sort_by(|a, b| match order.as_str() {
        "up" => a.0.cmp(&b.0),
        _ => b.0.cmp(&a.0),
    });
    for (_, line) in keyed {
        println!("{line}");
    }
}
