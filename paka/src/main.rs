#![deny(clippy::float_arithmetic)]

mod body;
mod build;
mod core_form;
mod interp;
mod llvm;
mod lower;
mod resolve;
mod sig;
#[cfg(test)]
mod vectors;

use core_form::Program;
use paka::ast::Def;
use paka::diagnostic::Diagnostic;
use paka::{ast, builtins, diagnostic, parser};
use paka_rt::{Fd1, abi};
use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::{env, fs, io};

const USAGE: &str = "usage: paka run <file> [argument...]\n       paka build <file> -o <output>\n       paka check <file> [--json]\n       paka examples <file>\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let words: Vec<_> = args.iter().map(|a| a.to_str()).collect();
    let (file, output) = match words[..] {
        [Some("run"), _, ..] => (&args[1], None),
        [Some("build"), _, Some("-o"), _] => (&args[1], Some(&args[3])),
        [Some("check"), _] => return check_file(&args[1], false),
        [Some("check"), _, Some("--json")] => return check_file(&args[1], true),
        [Some("examples"), _] => return examples_file(&args[1]),
        _ => return report(USAGE.as_bytes(), 2),
    };
    let path = &file.to_string_lossy();
    let result = load(file, path).and_then(|(core, holes)| match output {
        None => {
            abi::ignore_signals();
            Ok(interp::run(
                &core,
                &args[2..],
                &mut Fd1::default(),
                &mut io::stderr(),
            ))
        }
        Some(_) if holes.iter().any(|h| h.refusal.is_some()) => {
            for d in holes.iter().filter_map(|h| h.refusal.as_ref()) {
                report_at(file, &format!("{d}\n"));
            }
            Ok(1)
        }
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

/// The core form of a checked program that holds an app, and its holes.
fn load(file: &OsStr, path: &str) -> Result<(Program, Vec<Hole>), Diagnostic> {
    let (core, holes) = check(path, &read(file, path)?)?;
    match core.app {
        Some(_) => Ok((core, holes)),
        None => Err(Diagnostic::tool(path, "no-app", "the program has no app")),
    }
}

/// `paka examples`: the program needs no app, since each example starts with a fresh world.
fn examples_file(file: &OsStr) -> ExitCode {
    let path = &file.to_string_lossy();
    match read(file, path).and_then(|source| check(path, &source)) {
        Ok((core, _)) => {
            let (mut out, mut err) = (io::stdout().lock(), io::stderr());
            ExitCode::from(interp::examples(&core, &mut out, &mut err))
        }
        Err(d) => report_at(file, &format!("{d}\n")),
    }
}

/// `paka check`: the diagnostic `run` would print, or a note per hole, with their fields as extra
/// lines or as JSON (M2-06 section 1.5).
fn check_file(file: &OsStr, json: bool) -> ExitCode {
    let path = &file.to_string_lossy();
    let (found, source) = match read(file, path) {
        Err(d) => (Err(d), vec![]),
        Ok(source) => (check(path, &source), source),
    };
    let (shown, status) = match found {
        Ok((_, holes)) => (holes.into_iter().map(|h| h.note).collect(), 0),
        Err(d) => (vec![d], 1),
    };
    for d in &shown {
        if json {
            report(d.json(&source).as_bytes(), status);
        } else {
            report_at(file, &d.detailed(&source));
        }
    }
    ExitCode::from(status)
}

/// A hole in a checked program: `paka check` notes it, `paka build` refuses it unless an example
/// holds it (M2-06 section 1.5).
struct Hole {
    note: Diagnostic,
    refusal: Option<Diagnostic>,
}

/// Parses and checks a program, then lowers it to the core form.
fn check(path: &str, source: &[u8]) -> Result<(Program, Vec<Hole>), Diagnostic> {
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
    let core = lower::lower(path, source, &program, &resolved, &table, &bodies);
    let holes = holes(path, &program, &table, &bodies, &core);
    Ok((core, holes))
}

/// The holes of a checked program, in position order.
fn holes(
    path: &str,
    program: &ast::Program,
    table: &sig::Table,
    bodies: &body::Bodies,
    core: &Program,
) -> Vec<Hole> {
    let mut sites: Vec<_> = (core.sites.iter())
        .filter_map(|s| Some(((s.line, s.col), s.hole.as_deref()?)))
        .collect();
    sites.sort();
    sites.dedup();
    let hole = |(at, name): (ast::Pos, &str)| {
        let def = program.defs.iter().rposition(|d| d.name().at <= at);
        let def = def.expect("a hole stands in a definition");
        let len = name.len() + 2;
        let takes =
            (bodies.types[def].get(&(at, len))).map_or("no value".into(), |&t| table.types.show(t));
        let note = Diagnostic::new(path, at, len, "hole", format!("??{name} takes {takes}"));
        let finish = format!("a release build has no holes; finish ??{name}");
        let refusal = Diagnostic::new(path, at, len, "hole-in-build", finish);
        Hole {
            note: note.in_definition(program.defs[def].name().text),
            refusal: (!matches!(program.defs[def], Def::Example { .. })).then_some(refusal),
        }
    };
    sites.into_iter().map(hole).collect()
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
