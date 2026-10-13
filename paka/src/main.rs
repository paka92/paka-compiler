#![deny(clippy::float_arithmetic)]

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
#[cfg(test)]
mod vectors;

use diagnostic::Diagnostic;
use lower::Core;
use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::{env, fs, io};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n       paka check <file> [--json]\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let words: Vec<_> = args.iter().map(|a| a.to_str()).collect();
    let (file, output) = match words[..] {
        [Some("run"), _] => (&args[1], None),
        [Some("build"), _, Some("-o"), _] => (&args[1], Some(&args[3])),
        [Some("check"), _] => return check_file(&args[1], false),
        [Some("check"), _, Some("--json")] => return check_file(&args[1], true),
        _ => return report(USAGE.as_bytes(), 2),
    };
    let path = &file.to_string_lossy();
    let result = load(file, path).and_then(|core| match output {
        None => Ok(interp::run(
            &core,
            &mut io::stdout().lock(),
            &mut io::stderr(),
        )),
        Some(output) => build::build(path, &core, output).map(|()| 0),
    });
    match result {
        Ok(status) => ExitCode::from(status),
        Err(d) => report_at(file, &format!("{d}\n")),
    }
}

/// `path` is `file` as diagnostics show it: bytes that are not UTF-8 replaced.
fn read(file: &OsStr, path: &str) -> Result<Vec<u8>, Diagnostic> {
    fs::read(file).map_err(|e| Diagnostic::tool(path, "file-unreadable", e))
}

fn load(file: &OsStr, path: &str) -> Result<Core, Diagnostic> {
    front_end(path, &read(file, path)?)
}

/// `paka check`: the diagnostic `run` would print, with its fields as extra lines or as JSON.
fn check_file(file: &OsStr, json: bool) -> ExitCode {
    let path = &file.to_string_lossy();
    let (d, source) = match read(file, path) {
        Err(d) => (d, vec![]),
        Ok(source) => {
            let result =
                parser::parse(path, &source).and_then(|p| check::check(path, &p).map(drop));
            match result {
                Ok(()) => return ExitCode::SUCCESS,
                Err(d) => (d, source),
            }
        }
    };
    if json {
        report(d.json(&source).as_bytes(), 1)
    } else {
        report_at(file, &d.detailed(&source))
    }
}

fn front_end(path: &str, source: &[u8]) -> Result<Core, Diagnostic> {
    let program = parser::parse(path, source)?;
    let checked = check::check(path, &program)?;
    Ok(lower::lower(&program, &checked))
}

/// Reports a diagnostic, which starts with the shown path, with the file argument's own bytes there.
fn report_at(file: &OsStr, diagnostic: &str) -> ExitCode {
    let rest = &diagnostic[file.to_string_lossy().len()..];
    report(&[file.as_bytes(), rest.as_bytes()].concat(), 1)
}

/// Writes to standard error; a failed write is ignored.
fn report(text: &[u8], status: u8) -> ExitCode {
    let _ = io::stderr().write_all(text);
    ExitCode::from(status)
}
