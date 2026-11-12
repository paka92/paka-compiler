use std::collections::BTreeMap;
use std::{env, fs, process};

fn parse_count(s: &str) -> Option<usize> {
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok().filter(|n| (1..=1000).contains(n))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file, count] = &args[..] else {
        panic!("usage: main FILE N");
    };
    let Some(count) = parse_count(count) else {
        println!("error: bad count");
        process::exit(20);
    };
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let words = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty());
    for word in words {
        *counts.entry(word.to_ascii_lowercase()).or_default() += 1;
    }
    if counts.is_empty() {
        println!("no words");
    }
    let mut ranked: Vec<(&String, &usize)> = counts.iter().collect();
    ranked.sort_by_key(|&(_, n)| std::cmp::Reverse(*n));
    for (word, n) in ranked.into_iter().take(count) {
        println!("{n} {word}");
    }
}
