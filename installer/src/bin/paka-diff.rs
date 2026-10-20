//! `paka-diff [ROOT]`: runs every run case under ROOT, `tests/conformance` by default, on the
//! interpreter and in dev mode, and lists each as agree, differ or unsupported, with whether the
//! dev run's ledger ended clean (M4-40). The status is 1 when a case differs.
//!
//! Each run is a child, `paka-diff --one interp|dev NOTE ARGS...` in the staged case folder: it
//! runs `main.paka` as `paka run` does, and in dev mode writes its ledger, or why the dev back end
//! refused the program, to NOTE.

use paka::interp;
use paka::source::read;
use paka_installer::dev::Dev;
use paka_installer::diff::{Outcome, cases, difference, stage};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Host;
use paka_rt::{Fd1, abi};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

/// The heap a dev run may fill.
const HEAP: u64 = 1 << 30;
/// How long a dev run may take before its interrupt.
const LIMIT: Duration = Duration::from_secs(10);

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match args.first().and_then(|a| a.to_str()) {
        Some("--one") => one(&args[1..]),
        _ => {
            let root = args
                .first()
                .map_or("tests/conformance".into(), PathBuf::from);
            compare(&root).unwrap_or_else(|e| {
                eprintln!("paka-diff: {e}");
                ExitCode::from(2)
            })
        }
    }
}

fn compare(root: &Path) -> io::Result<ExitCode> {
    let exe = env::current_exe()?;
    let scratch = env::temp_dir().join(format!("paka-diff-{}", std::process::id()));
    let mut out = io::stdout().lock();
    let mut counts = [0; 3];
    for case in cases(root)? {
        let name = case
            .strip_prefix(root)
            .unwrap_or(&case)
            .display()
            .to_string();
        let run = |engine: &str| -> io::Result<(Outcome, String)> {
            let dir = scratch.join(format!("{}-{engine}", name.replace('/', "-")));
            stage(&case, &dir)?;
            let note = dir.with_extension("note");
            let output = (Command::new(&exe).current_dir(&dir))
                .args([OsString::from("--one"), engine.into(), note.clone().into()])
                .args(arguments(&case))
                .output()?;
            let note = fs::read_to_string(&note).unwrap_or_default();
            Ok((output.into(), note))
        };
        let ((reference, _), (dev, note)) = (run("interp")?, run("dev")?);
        let line = match (
            note.strip_prefix("unsupported: "),
            difference(&reference, &dev),
        ) {
            (Some(why), _) => (2, format!("unsupported {name}: {why}")),
            (None, None) => (0, format!("agree {name}; {}", ledger(&note))),
            (None, Some(why)) => (1, format!("differ {name}: {why}; {}", ledger(&note))),
        };
        counts[line.0] += 1;
        writeln!(out, "{}", line.1)?;
    }
    let _ = fs::remove_dir_all(&scratch);
    let [agree, differ, unsupported] = counts;
    writeln!(
        out,
        "{agree} agree, {differ} differ, {unsupported} unsupported"
    )?;
    Ok(ExitCode::from(u8::from(differ > 0)))
}

/// The case's arguments: one per line of its `args` file.
fn arguments(case: &Path) -> Vec<OsString> {
    let bytes = fs::read(case.join("args")).unwrap_or_default();
    let mut lines: Vec<_> = bytes.split(|b| *b == b'\n').map(<[u8]>::to_vec).collect();
    lines.pop();
    lines.into_iter().map(OsString::from_vec).collect()
}

fn ledger(note: &str) -> String {
    match note.strip_prefix("ledger ").and_then(|n| n.split_once(' ')) {
        Some(("0", "0")) => "ledger clean".into(),
        Some((leaks, faults)) => format!("ledger {leaks} leaks, {faults} bad frees"),
        None => "no ledger".into(),
    }
}

/// One run of `main.paka` here, on the engine `args[0]`, with the program's arguments after the
/// note's path.
fn one(args: &[OsString]) -> ExitCode {
    let (engine, note, args) = (args[0].to_str(), &args[1], &args[2..]);
    let (source, found) = read("main.paka".as_ref());
    let checked = found.and_then(|()| paka::check("main.paka", &source));
    let Ok((program, ..)) = checked else {
        return ExitCode::from(1);
    };
    abi::ignore_signals();
    let status = match engine {
        Some("interp") => interp::run(&program, args, &mut Fd1::default(), &mut io::stderr()),
        _ => {
            let mut dev = Dev::default();
            if let Err(why) = dev.edit(&source) {
                let _ = fs::write(note, format!("unsupported: {why}"));
                return ExitCode::from(1);
            }
            let ran = dev.run(args, &mut Host(Fd1::default()), HEAP, LIMIT);
            let (leaks, faults) = ran.ledger;
            let _ = fs::write(note, format!("ledger {leaks} {faults}"));
            let (status, line) = match ran.end {
                Ok(end) => end,
                Err(Stopped::Trap) => (3, dev.report(&ran.trap)),
                Err(Stopped::Time) => (4, "stopped: time\n".into()),
            };
            let _ = io::stderr().write_all(line.as_bytes());
            status
        }
    };
    ExitCode::from(status)
}
