use std::collections::HashSet;
use std::{env, fs, process};

#[derive(Default)]
struct Merge {
    chain: Vec<String>,
    done: HashSet<String>,
    lines: Vec<String>,
}

impl Merge {
    fn file(&mut self, file: &str, text: &str) -> Result<(), String> {
        self.chain.push(file.to_string());
        for (i, line) in text.lines().enumerate() {
            if !line.starts_with("#include") {
                self.lines.push(line.to_string());
                continue;
            }
            let at = format!("{file} line {}", i + 1);
            let Some(name) = line.strip_prefix("#include ").filter(|n| is_name(n)) else {
                return Err(format!("{at}: bad include"));
            };
            if self.chain.iter().any(|c| c == name) {
                let chain = self.chain.join(" -> ");
                return Err(format!("{at}: include cycle: {chain} -> {name}"));
            }
            if self.done.contains(name) {
                continue;
            }
            let Ok(sub) = fs::read_to_string(name) else {
                return Err(format!("{at}: cannot read {name}"));
            };
            self.file(name, &sub)?;
        }
        self.chain.pop();
        self.done.insert(file.to_string());
        Ok(())
    }
}

fn is_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphanumeric())
        && s.len() <= 30
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn run(root: &str, out: &str) -> Result<String, String> {
    let text = fs::read_to_string(root).map_err(|_| format!("cannot read {root}"))?;
    let mut merge = Merge::default();
    merge.file(root, &text)?;
    let content: String = merge.lines.iter().map(|l| format!("{l}\n")).collect();
    fs::write(out, content).map_err(|_| format!("cannot write {out}"))?;
    let files = plural(merge.done.len(), "file");
    let lines = plural(merge.lines.len(), "line");
    Ok(format!("read {files}, wrote {lines} to {out}\n"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [root, out] = &args[..] else {
        panic!("usage: main ROOT OUT");
    };
    match run(root, out) {
        Ok(summary) => print!("{summary}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
