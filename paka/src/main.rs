#![deny(clippy::float_arithmetic)]

mod build;
mod llvm;
#[cfg(test)]
mod vectors;

use core_form::Program;
use paka::diagnostic::Diagnostic;
use paka::results::Cache;
use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::{Checked, Hole, approvals, approve, check, check_defs, core_form, diagnostic, gate};
use paka::{interp, pending, source};
use paka_json::Value;
use paka_rt::{Fd1, abi};
use source::{Source, read};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::{self, TryLockError};
use std::io::{IsTerminal, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, io};

const USAGE: &str = "usage: paka run <path> [argument...]\n       paka build <path> -o <output>\n       paka check <path> [--json]\n       paka examples <path>\n       paka serve <path>\n       paka approve <path> [--adopt <name>] [--drop <name>] [--key <file>]\n       paka gate <tree> --base <tree> [--json]\n";

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
        [Some("approve"), _, ..] => return approve_program(&args[1], &words[2..]),
        [Some("gate"), _, Some("--base"), _] => return gate_tree(&args[1], &args[3], false),
        [Some("gate"), _, Some("--base"), _, Some("--json")] => {
            return gate_tree(&args[1], &args[3], true);
        }
        _ => return report(USAGE.as_bytes(), 2),
    };
    let path = &file.to_string_lossy();
    let (source, found) = read_and_check(file);
    let result = load(path, found).and_then(|(mut core, holes)| match output {
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
        Some(output) => build::build(path, &mut core, output).map(|()| 0),
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

/// The core form of a checked program that holds a headless app, and its holes.
fn load(path: &str, found: Checked) -> Result<(Program, Vec<Hole>), Diagnostic> {
    let (core, holes, _) = found?;
    match &core.app {
        Some(app) if app.view.is_some() => {
            let why = "a visual app is not run or built from the command line yet";
            Err(Diagnostic::tool(path, "no-app", why))
        }
        Some(_) => Ok((core, holes)),
        None => Err(Diagnostic::tool(path, "no-app", "the program has no app")),
    }
}

/// `paka examples`: the program needs no app, since each example starts with a fresh world.
/// Results come from the user's example cache when it holds them (M7-09); labels from the
/// approval store, and a store that does not verify owns nothing (M7-05).
fn examples_file(file: &OsStr) -> ExitCode {
    let (source, read) = read(file);
    match read.and_then(|()| check_defs(&file.to_string_lossy(), &source)) {
        Ok(((core, ..), defs)) => {
            let mut cache = Cache::user(&defs, &source);
            let (mut out, mut err) = (io::stdout().lock(), io::stderr());
            let owned = approvals::owned(file.as_ref()).unwrap_or_else(|why| {
                let _ = writeln!(err, "{why}; every example reads as agent-written");
                BTreeSet::new()
            });
            let status = interp::examples(&core, &mut cache, &owned, &mut out, &mut err);
            ExitCode::from(status)
        }
        Err(d) => reject(&source, d),
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
    let runner = env::current_exe().map(|exe| exe.with_file_name("paka-runner"));
    let runner = Runner::new(runner.unwrap_or_else(|_| "paka-runner".into()));
    let mut session = Session::new(file, &source, workspace, lock, runner);
    match serve(io::stdin().lock(), io::stdout().lock(), |call| {
        session.handle(call)
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(format!("paka serve: {e}\n").as_bytes(), 1),
    }
}

/// `paka approve`: each item awaiting the owner on its own screen with its own yes or no, read
/// from a terminal only; then the store with a higher serial, signed (docs/decisions/M7-01.md).
fn approve_program(file: &OsStr, flags: &[Option<&str>]) -> ExitCode {
    let (mut adopt, mut drop, mut key) = (vec![], vec![], None);
    for pair in flags.chunks(2) {
        match pair {
            [Some("--adopt"), Some(name)] => adopt.push(name.to_string()),
            [Some("--drop"), Some(name)] => drop.push(name.to_string()),
            [Some("--key"), Some(path)] if key.is_none() => key = Some(PathBuf::from(path)),
            _ => return report(USAGE.as_bytes(), 2),
        }
    }
    if !io::stdin().is_terminal() {
        return report(b"paka approve: answers are read from a terminal only\n", 2);
    }
    let home = env::var_os("HOME").map(|h| Path::new(&h).join(".config/paka/approval_key"));
    let Some(key) = key.or(home) else {
        return report(b"paka approve: no HOME; name the key with --key\n", 2);
    };
    let (dir, program, mut store) = match approvals::to_approve(file.as_ref()) {
        Ok(found) => found,
        Err(why) => return report(format!("paka approve: {why}\n").as_bytes(), 1),
    };
    let (source, read) = read(file);
    let items = read.and_then(|()| approve::items(&store, &dir, &program, &source, &adopt, &drop));
    let items = match items {
        Ok(items) => items,
        Err(d) => return reject(&source, d),
    };
    if items.is_empty() {
        return report(b"approvals: nothing awaits approval\n", 0);
    }
    let count = items.len();
    let mut answers = io::stdin().lines();
    let mut approved = 0;
    for (at, item) in items.into_iter().enumerate() {
        print!("[{}/{count}] {}", at + 1, item.screen);
        match yes(&mut answers) {
            Some(true) => {
                store.apply(item.change);
                approved += 1;
            }
            Some(false) => {}
            None => return report(b"paka approve: no answer; nothing is signed\n", 1),
        }
    }
    if approved == 0 {
        return report(b"approvals: nothing approved, nothing signed\n", 0);
    }
    store.serial += 1;
    match store.save(&dir, &key) {
        Ok(()) => {
            println!("approvals: serial {}, signed", store.serial);
            ExitCode::SUCCESS
        }
        Err(why) => report(format!("paka approve: {why}\n").as_bytes(), 1),
    }
}

/// Asks until the answer is yes or no; none when the answers end.
fn yes(answers: &mut impl Iterator<Item = io::Result<String>>) -> Option<bool> {
    loop {
        print!("Approve this item? yes / no: ");
        let _ = io::stdout().flush();
        match answers.next()?.ok()?.trim() {
            "yes" => return Some(true),
            "no" => return Some(false),
            _ => {}
        }
    }
}

/// `paka gate`: one line per refusal, or the pending list as one JSON object; status 1 when
/// refused, else 0.
fn gate_tree(tree: &OsStr, base: &OsStr, json: bool) -> ExitCode {
    let items = gate::gate(tree.as_ref(), base.as_ref());
    let mut out = io::stdout().lock();
    if json {
        let _ = writeln!(out, "{}", Value::object(pending::fields(&items)));
    } else if items.is_empty() {
        let _ = writeln!(out, "gate: passed");
    } else {
        for item in &items {
            let _ = writeln!(out, "{}", item.line());
        }
        let _ = writeln!(
            out,
            "Revert the change, or ask the owner to run paka approve."
        );
    }
    ExitCode::from(u8::from(!items.is_empty()))
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
