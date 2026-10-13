#[allow(dead_code)] // the checker (M1-10) reads the tree
mod ast;
mod diagnostic;
mod lexer;
mod parser;

use diagnostic::Diagnostic;
use std::io::Write;
use std::process::ExitCode;
use std::{env, fs, io};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let args: Vec<_> = args.iter().map(|a| a.to_str().unwrap_or("\0")).collect();
    let file = match args[..] {
        ["run", file] | ["build", file, "-o", _] => file,
        _ => return report(USAGE, 2),
    };
    match fs::read(file) {
        Ok(source) => match parser::parse(file, &source) {
            Ok(_) => ExitCode::SUCCESS,
            Err(d) => report(&format!("{d}\n"), 1),
        },
        Err(e) => {
            let d = Diagnostic {
                path: file.into(),
                position: None,
                code: "file-unreadable",
                message: e.to_string(),
            };
            report(&format!("{d}\n"), 1)
        }
    }
}

/// Writes to standard error; a failed write is ignored.
fn report(text: &str, status: u8) -> ExitCode {
    let _ = io::stderr().write_all(text.as_bytes());
    ExitCode::from(status)
}
