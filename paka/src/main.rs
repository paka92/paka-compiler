mod ast;
mod build;
mod check;
mod diagnostic;
mod interp;
mod lexer;
mod llvm;
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
    let (file, output) = match args[..] {
        ["run", file] => (file, None),
        ["build", file, "-o", output] => (file, Some(output)),
        _ => return report(USAGE, 2),
    };
    let result = load(file).and_then(|core| match output {
        None => Ok(interp::run(
            &core,
            &mut io::stdout().lock(),
            &mut io::stderr(),
        )),
        Some(output) => build::build(file, &core, output).map(|()| 0),
    });
    match result {
        Ok(status) => ExitCode::from(status),
        Err(d) => report(&format!("{d}\n"), 1),
    }
}

fn load(file: &str) -> Result<Core, Diagnostic> {
    let source = fs::read(file).map_err(|e| Diagnostic::tool(file, "file-unreadable", e))?;
    front_end(file, &source)
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
