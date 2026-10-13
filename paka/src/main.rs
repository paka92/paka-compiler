mod ast;
mod check;
mod diagnostic;
#[allow(dead_code)] // wired into `paka run` in the next commit
mod interp;
mod lexer;
#[allow(dead_code)] // read by the interpreter (M1-13)
mod lower;
mod parser;

use diagnostic::Diagnostic;
use lower::Core;
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
        Ok(source) => match front_end(file, &source) {
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

fn front_end(path: &str, source: &[u8]) -> Result<Core, Diagnostic> {
    let program = parser::parse(path, source)?;
    let checked = check::check(path, &program)?;
    Ok(lower::lower(&program, &checked))
}

/// Writes to standard error; a failed write is ignored.
fn report(text: &str, status: u8) -> ExitCode {
    let _ = io::stderr().write_all(text.as_bytes());
    ExitCode::from(status)
}
