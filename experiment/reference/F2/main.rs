use std::collections::BTreeMap;
use std::{env, fs, process};

struct Group {
    key: String,
    rests: Vec<String>,
}

fn field_number(arg: &str) -> Option<usize> {
    let digits = arg.trim_start_matches('0');
    if !arg.bytes().all(|b| b.is_ascii_digit()) || digits.len() != 1 {
        return None;
    }
    digits.parse().ok()
}

fn file_name(key: &str) -> String {
    let stem: String = key
        .chars()
        .map(|c| match c.to_ascii_lowercase() {
            c @ ('a'..='z' | '0'..='9') => c,
            _ => '_',
        })
        .collect();
    stem + ".txt"
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn run(input: &str, field: &str, index: &str) -> Result<String, String> {
    let f = field_number(field).ok_or("bad field")?;
    let text = fs::read_to_string(input).map_err(|_| format!("cannot read {input}"))?;

    // Keyed by file name, so iteration is in writing order.
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    let mut records = 0;
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let n = i + 1;
        let mut fields: Vec<&str> = line.split(',').collect();
        if fields.len() < f {
            return Err(format!("line {n}: no field {f}"));
        }
        let key = fields.remove(f - 1);
        if key.is_empty() {
            return Err(format!("line {n}: empty key"));
        }
        let name = file_name(key);
        let group = groups.entry(name.clone()).or_insert_with(|| Group {
            key: key.to_string(),
            rests: vec![],
        });
        if group.key != key {
            let earlier = &group.key;
            return Err(format!("line {n}: {key} clashes with {earlier} in {name}"));
        }
        group.rests.push(fields.join(","));
        records += 1;
    }

    let mut list = String::new();
    for (name, group) in &groups {
        let content: String = group.rests.iter().map(|r| format!("{r}\n")).collect();
        fs::write(name, content).map_err(|_| format!("cannot write {name}"))?;
        list += &format!("{name} {} {}\n", group.rests.len(), group.key);
    }
    fs::write(index, list).map_err(|_| format!("cannot write {index}"))?;
    let records = plural(records, "record");
    let files = plural(groups.len(), "file");
    Ok(format!("split {records} into {files}\n"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [input, field, index] = &args[..] else {
        panic!("usage: main INPUT FIELD INDEX");
    };
    match run(input, field, index) {
        Ok(summary) => print!("{summary}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
