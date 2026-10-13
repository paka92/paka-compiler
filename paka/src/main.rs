#[allow(dead_code)] // the checker (M1-10) reads the tree
mod ast;
mod diagnostic;
#[allow(dead_code)] // the parser (M1-09) reads the token fields
mod lexer;
#[allow(dead_code)] // `paka run` calls the parser in the next commit
mod parser;

use diagnostic::Diagnostic;
use lexer::{Kind, Lexer};
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
        Ok(source) => match lex_all(file, &source) {
            Ok(()) => ExitCode::SUCCESS,
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

/// Stand-in until the parser (M1-09) pulls the tokens: a lexical error is the first problem
/// only when no syntax error comes before it.
fn lex_all(path: &str, source: &[u8]) -> Result<(), Diagnostic> {
    let mut lexer = Lexer::new(path, source);
    while lexer.next()?.kind != Kind::End {}
    Ok(())
}

/// Writes to standard error; a failed write is ignored.
fn report(text: &str, status: u8) -> ExitCode {
    let _ = io::stderr().write_all(text.as_bytes());
    ExitCode::from(status)
}
