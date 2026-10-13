mod ast;
mod build;
mod check;
mod diagnostic;
mod interp;
mod json;
mod lexer;
mod llvm;
mod lower;
mod parser;

use diagnostic::Diagnostic;
use lower::Core;
use std::io::Write;
use std::process::ExitCode;
use std::{env, fs, io};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n       paka check <file> [--json]\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let args: Vec<_> = args.iter().map(|a| a.to_str().unwrap_or("\0")).collect();
    let (file, output) = match args[..] {
        ["run", file] => (file, None),
        ["build", file, "-o", output] => (file, Some(output)),
        ["check", file] => return check_file(file, false),
        ["check", file, "--json"] => return check_file(file, true),
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

fn read(file: &str) -> Result<Vec<u8>, Diagnostic> {
    fs::read(file).map_err(|e| Diagnostic::tool(file, "file-unreadable", e))
}

fn load(file: &str) -> Result<Core, Diagnostic> {
    front_end(file, &read(file)?)
}

/// `paka check`: the diagnostic `run` would print, with its fields as extra lines or as JSON.
fn check_file(file: &str, json: bool) -> ExitCode {
    let source = match read(file) {
        Ok(source) => source,
        Err(d) => return report_fields(&d, &[], json),
    };
    let result = parser::parse(file, &source).and_then(|p| check::check(file, &p).map(drop));
    result.map_or_else(|d| report_fields(&d, &source, json), |()| ExitCode::SUCCESS)
}

fn report_fields(d: &Diagnostic, source: &[u8], json: bool) -> ExitCode {
    report(
        &if json {
            d.json(source)
        } else {
            d.detailed(source)
        },
        1,
    )
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
