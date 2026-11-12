use std::{env, fs, process};

fn parse_width(s: &str) -> Option<usize> {
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok().filter(|w| (1..=100).contains(w))
}

/// The words of a paragraph, with those longer than `width` cut into pieces.
fn pieces(paragraph: &[&str], width: usize) -> Vec<String> {
    let words = paragraph
        .iter()
        .flat_map(|l| l.split(' '))
        .filter(|w| !w.is_empty());
    words
        .flat_map(|w| {
            (0..w.len())
                .step_by(width)
                .map(|start| w[start..(start + width).min(w.len())].to_string())
        })
        .collect()
}

fn fill(words: &[String], width: usize) -> Vec<Vec<&str>> {
    let mut lines: Vec<Vec<&str>> = vec![];
    let mut length = 0;
    for word in words {
        match lines.last_mut() {
            Some(line) if length + 1 + word.len() <= width => {
                line.push(word);
                length += 1 + word.len();
            }
            _ => {
                lines.push(vec![word]);
                length = word.len();
            }
        }
    }
    lines
}

fn justify(words: &[&str], width: usize) -> String {
    if words.len() == 1 {
        return words[0].to_string();
    }
    let gaps = words.len() - 1;
    let spaces = width - words.iter().map(|w| w.len()).sum::<usize>();
    let mut line = String::new();
    for (i, word) in words.iter().enumerate() {
        line += word;
        if i < gaps {
            let extra = usize::from(i < spaces % gaps);
            line += &" ".repeat(spaces / gaps + extra);
        }
    }
    line
}

fn wrap(paragraph: &[&str], width: usize) -> Vec<String> {
    let words = pieces(paragraph, width);
    let lines = fill(&words, width);
    let last = lines.len() - 1;
    lines
        .iter()
        .enumerate()
        .map(|(i, words)| match i < last {
            true => justify(words, width),
            false => words.join(" "),
        })
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file, width] = &args[..] else {
        panic!("usage: main FILE W");
    };
    let Some(width) = parse_width(width) else {
        println!("error: bad width");
        process::exit(20);
    };
    let text = fs::read_to_string(file).expect("input file is readable");
    let lines: Vec<&str> = text.lines().collect();
    let paragraphs = lines
        .split(|l| l.trim_matches(' ').is_empty())
        .filter(|p| !p.is_empty());
    let wrapped: Vec<String> = paragraphs.map(|p| wrap(p, width).join("\n")).collect();
    if !wrapped.is_empty() {
        println!("{}", wrapped.join("\n\n"));
    }
}
