#![deny(clippy::float_arithmetic)]

mod ast;
mod body;
mod build;
mod builtins;
mod core_form;
mod diagnostic;
mod interp;
mod json;
mod lexer;
mod llvm;
mod lower;
mod parser;
mod resolve;
mod sig;
#[cfg(test)]
mod vectors;

use ast::Def;
use core_form::Program;
use diagnostic::Diagnostic;
use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::{env, fs, io};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n       paka check <file> [--json]\n       paka examples <file>\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let words: Vec<_> = args.iter().map(|a| a.to_str()).collect();
    let (file, output) = match words[..] {
        [Some("run"), _] => (&args[1], None),
        [Some("build"), _, Some("-o"), _] => (&args[1], Some(&args[3])),
        [Some("check"), _] => return check_file(&args[1], false),
        [Some("check"), _, Some("--json")] => return check_file(&args[1], true),
        [Some("examples"), _] => return examples_file(&args[1]),
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

/// The core form of a checked program that holds an app.
fn load(file: &OsStr, path: &str) -> Result<Program, Diagnostic> {
    let core = check(path, &read(file, path)?)?;
    match core.app {
        Some(_) => Ok(core),
        None => Err(Diagnostic::tool(path, "no-app", "the program has no app")),
    }
}

/// `paka examples`: the program needs no app, since each example starts with a fresh world.
fn examples_file(file: &OsStr) -> ExitCode {
    let path = &file.to_string_lossy();
    match read(file, path).and_then(|source| check(path, &source)) {
        Ok(core) => {
            let (mut out, mut err) = (io::stdout().lock(), io::stderr());
            ExitCode::from(interp::examples(&core, &mut out, &mut err))
        }
        Err(d) => report_at(file, &format!("{d}\n")),
    }
}

/// `paka check`: the diagnostic `run` would print, with its fields as extra lines or as JSON.
fn check_file(file: &OsStr, json: bool) -> ExitCode {
    let path = &file.to_string_lossy();
    let (d, source) = match read(file, path) {
        Err(d) => (d, vec![]),
        Ok(source) => match check(path, &source) {
            Ok(_) => return ExitCode::SUCCESS,
            Err(d) => (d, source),
        },
    };
    if json {
        report(d.json(&source).as_bytes(), 1)
    } else {
        report_at(file, &d.detailed(&source))
    }
}

/// Parses and checks a program, then lowers it to the core form.
fn check(path: &str, source: &[u8]) -> Result<Program, Diagnostic> {
    let program = parser::parse(path, source)?;
    let resolved = resolve::resolve(path, &program)?;
    let mut table = sig::Table::build(path, &program, &resolved);
    if let Some(d) = second_app(path, &program) {
        return Err(d);
    }
    let bodies = body::check(path, &program, &resolved, &mut table);
    if let Some(d) = bodies.defs.iter().flatten().next() {
        return Err(d.clone());
    }
    Ok(lower::lower(
        path, source, &program, &resolved, &table, &bodies,
    ))
}

fn second_app(path: &str, program: &ast::Program) -> Option<Diagnostic> {
    let mut apps = program.defs.iter().filter_map(|d| match d {
        Def::App { name, .. } => Some(name),
        _ => None,
    });
    let second = apps.nth(1)?;
    let message = "a program holds one app";
    let d = Diagnostic::new(path, second.at, second.text.len(), "multiple-apps", message);
    Some(d.in_definition(second.text))
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
