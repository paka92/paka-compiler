use std::{env, fs, process};

fn fail(message: String) -> ! {
    println!("error: {message}");
    process::exit(20);
}

fn read(name: &str) -> String {
    fs::read_to_string(name).unwrap_or_else(|_| fail(format!("cannot read {name}")))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [first, second, output] = &args[..] else {
        panic!("usage: main FIRST SECOND OUTPUT");
    };
    let (first_text, second_text) = (read(first), read(second));
    let lines: Vec<&str> = first_text.lines().chain(second_text.lines()).collect();
    let width = lines.len().to_string().len();

    let mut content = String::new();
    for (i, text) in lines.iter().enumerate() {
        let gap = if text.is_empty() { "" } else { " " };
        content += &format!("{:>width$} |{gap}{text}\n", i + 1);
    }
    if fs::write(output, content).is_err() {
        fail(format!("cannot write {output}"));
    }
    let count = lines.len();
    let noun = if count == 1 { "line" } else { "lines" };
    println!("wrote {count} {noun} to {output}");
}
