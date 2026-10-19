#![deny(clippy::float_arithmetic)]

mod build;
mod interp;
mod llvm;
#[cfg(test)]
mod vectors;

use core_form::Program;
use paka::diagnostic::Diagnostic;
use paka::serve::serve;
use paka::session::{self, Session};
use paka::{Checked, Hole, builtins, check, core_form, diagnostic, source};
use paka_rt::{Fd1, abi};
use source::{Source, read};
use std::ffi::OsStr;
use std::fs::{self, TryLockError};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;
use std::{env, io};

const USAGE: &str = "usage: paka run <path> [argument...]\n       paka build <path> -o <output>\n       paka check <path> [--json]\n       paka examples <path>\n       paka serve <path>\n";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let words: Vec<_> = args.iter().map(|a| a.to_str()).collect();
    let (file, output) = match words[..] {
        [Some("run"), _, ..] => (&args[1], None),
        [Some("build"), _, Some("-o"), _] => (&args[1], Some(&args[3])),
        [Some("check"), _] => return check_file(&args[1], false),
        [Some("check"), _, Some("--json")] => return check_file(&args[1], true),
        [Some("examples"), _] => return examples_file(&args[1]),
        [Some("serve"), _] => return serve_program(&args[1]),
        _ => return report(USAGE.as_bytes(), 2),
    };
    let path = &file.to_string_lossy();
    let (source, found) = read_and_check(file);
    let result = load(path, found).and_then(|(core, holes)| match output {
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
            for d in holes.into_iter().filter_map(|h| h.refusal) {
                reject(&source, d);
            }
            Ok(1)
        }
        Some(output) => build::build(path, &core, output).map(|()| 0),
    });
    match result {
        Ok(status) => ExitCode::from(status),
        Err(d) => reject(&source, d),
    }
}

/// The program at `file` and what checking it gives.
fn read_and_check(file: &OsStr) -> (Source, Checked) {
    let (source, read) = read(file);
    let found = read.and_then(|()| check(&file.to_string_lossy(), &source));
    (source, found)
}

/// The core form of a checked program that holds an app, and its holes.
fn load(path: &str, found: Checked) -> Result<(Program, Vec<Hole>), Diagnostic> {
    let (core, holes, _) = found?;
    match core.app {
        Some(_) => Ok((core, holes)),
        None => Err(Diagnostic::tool(path, "no-app", "the program has no app")),
    }
}

/// `paka examples`: the program needs no app, since each example starts with a fresh world.
fn examples_file(file: &OsStr) -> ExitCode {
    match read_and_check(file) {
        (_, Ok((core, ..))) => {
            let (mut out, mut err) = (io::stdout().lock(), io::stderr());
            ExitCode::from(interp::examples(&core, &mut out, &mut err))
        }
        (source, Err(d)) => reject(&source, d),
    }
}

/// `paka serve`: the tool interface on standard input and output (docs/decisions/M4-03.md). A
/// program that does not parse is not served, and neither is one another server holds.
fn serve_program(file: &OsStr) -> ExitCode {
    let (source, read) = read(file);
    let workspace = read.and_then(|()| session::workspace(&source.text));
    let workspace = match workspace {
        Ok(workspace) => workspace,
        Err(d) => return reject(&source, d),
    };
    let path = file.to_string_lossy();
    let lock = fs::File::open(file).map_err(TryLockError::Error);
    let lock = match lock.and_then(|f| f.try_lock().map(|()| f)) {
        Ok(lock) => lock,
        Err(e) => {
            let why = match e {
                TryLockError::WouldBlock => "another server holds this program".into(),
                TryLockError::Error(e) => e.to_string(),
            };
            return reject(&source, Diagnostic::tool(&path, "not-served", why));
        }
    };
    let mut session = Session::new(file, &source, workspace, lock);
    match serve(io::stdin().lock(), io::stdout().lock(), |call| {
        session.handle(call)
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(format!("paka serve: {e}\n").as_bytes(), 1),
    }
}

/// `paka check`: the diagnostic `run` would print, or a note per hole and a warning per unused
/// name in position order, with their fields as extra lines or as JSON (M2-06 section 1.5, M2-02
/// section 5).
fn check_file(file: &OsStr, json: bool) -> ExitCode {
    let (source, found) = read_and_check(file);
    let (shown, status) = match found {
        Ok((_, holes, unused)) => {
            let mut shown: Vec<_> = holes.into_iter().map(|h| h.note).chain(unused).collect();
            shown.sort_by_key(|d| d.position);
            (shown, 0)
        }
        Err(d) => (vec![d], 1),
    };
    for d in shown {
        let (d, path, text) = source.locate(d);
        if json {
            report(d.json(text).as_bytes(), status);
        } else {
            report_at(path, &d.detailed(text));
        }
    }
    ExitCode::from(status)
}

/// Reports `d` in the file it is in, as one line.
fn reject(source: &Source, d: Diagnostic) -> ExitCode {
    let (d, path, _) = source.locate(d);
    report_at(path, &format!("{d}\n"))
}

/// Reports a diagnostic, which starts with the shown path, with the path's own bytes there.
fn report_at(file: &OsStr, diagnostic: &str) -> ExitCode {
    let rest = &diagnostic[file.to_string_lossy().len()..];
    report(&[file.as_bytes(), rest.as_bytes()].concat(), 1)
}

/// Writes to standard error; a failed write is ignored.
fn report(text: &[u8], status: u8) -> ExitCode {
    let _ = io::stderr().write_all(text);
    ExitCode::from(status)
}
