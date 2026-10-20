//! `paka-diff [ROOT]`: runs every run case under ROOT, `tests/conformance` by default, on the
//! interpreter and in dev mode, and lists each as agree, differ or unsupported, with whether the
//! dev run's ledger ended clean (M4-40). The status is 1 when a case differs.
//!
//! `paka-diff --fuzz FIRST COUNT KEEP` does the same for the generated programs of COUNT seeds
//! from FIRST (M4-41). Each that differs is shrunk while it keeps that kind of difference and
//! written to `KEEP/SEED.paka`. `PAKA_PLANT` plants a fault in the dev back end, for a self-test.
//!
//! Each run is a child, `paka-diff --one interp|dev NOTE LIMIT_MS ARGS...` in the staged case
//! folder: it runs `main.paka` as `paka run` does, stopped past the limit, and in dev mode writes
//! its ledger, or why the dev back end refused the program, to NOTE.

use paka::interp;
use paka::source::{Source, read};
use paka_installer::dev::Dev;
use paka_installer::diff::{Outcome, cases, difference, no_core_dumps, shrink, stage};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Host;
use paka_rt::{Fd1, abi};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

/// The heap a dev run may fill.
const HEAP: u64 = 1 << 30;
/// How long a run of a conformance case, and of a generated program, may take before it stops.
const LIMIT: Duration = Duration::from_secs(3);
const FUZZ_LIMIT: Duration = Duration::from_millis(250);

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let r = match args.first().and_then(|a| a.to_str()) {
        Some("--one") => return one(&args[1..]),
        Some("--fuzz") => {
            let number = |i: usize| args.get(i).and_then(|a| a.to_str()?.parse::<u64>().ok());
            let (Some(first), Some(count), Some(keep)) = (number(1), number(2), args.get(3)) else {
                eprintln!("usage: paka-diff --fuzz FIRST COUNT KEEP");
                return ExitCode::from(2);
            };
            fuzz(first..first + count, keep.as_ref())
        }
        _ => compare(
            &args
                .first()
                .map_or("tests/conformance".into(), PathBuf::from),
        ),
    };
    r.unwrap_or_else(|e| {
        eprintln!("paka-diff: {e}");
        ExitCode::from(2)
    })
}

fn compare(root: &Path) -> io::Result<ExitCode> {
    let scratch = scratch();
    let mut tally = Tally::default();
    for case in cases(root)? {
        let name = case
            .strip_prefix(root)
            .unwrap_or(&case)
            .display()
            .to_string();
        let flat = name.replace('/', "-");
        let dirs = ENGINES.map(|e| scratch.join(format!("{flat}-{e}")));
        dirs.iter().try_for_each(|d| stage(&case, d))?;
        tally.add(judge(&dirs, &name, LIMIT, &arguments(&case))?)?;
    }
    let _ = fs::remove_dir_all(&scratch);
    tally.end()
}

fn fuzz(seeds: Range<u64>, keep: &Path) -> io::Result<ExitCode> {
    let scratch = scratch();
    let dirs = ENGINES.map(|e| scratch.join(e));
    dirs.iter().try_for_each(fs::create_dir_all)?;
    let write = |text: &str| {
        dirs.iter()
            .try_for_each(|d| fs::write(d.join("main.paka"), text))
    };
    let mut tally = Tally::default();
    for seed in seeds {
        let text = paka_gen::program(seed);
        write(&text)?;
        let (verdict, line) = judge(&dirs, &format!("seed {seed}"), FUZZ_LIMIT, &[])?;
        let want = (verdict == 1).then(|| kind(&line));
        tally.add((verdict, line))?;
        let Some(want) = want else {
            continue;
        };
        let small = shrink(&text, |t| {
            let checks = paka::check("main.paka", &Source::file("main.paka", t.into())).is_ok();
            let judged = || write(t).and_then(|()| judge(&dirs, "", FUZZ_LIMIT, &[]));
            checks && judged().is_ok_and(|(v, line)| v == 1 && kind(&line) == want)
        });
        fs::create_dir_all(keep)?;
        let kept = keep.join(format!("{seed}.paka"));
        fs::write(&kept, &small)?;
        let lines = small.lines().count();
        writeln!(tally.out, "  kept {} ({lines} lines)", kept.display())?;
    }
    let _ = fs::remove_dir_all(&scratch);
    tally.end()
}

const ENGINES: [&str; 2] = ["interp", "dev"];

fn scratch() -> PathBuf {
    env::temp_dir().join(format!("paka-diff-{}", process::id()))
}

/// The kind of difference a `differ` line names: the first word of its reason.
fn kind(line: &str) -> String {
    let reason = line.split_once(": ").map_or("", |(_, r)| r);
    reason.split(' ').next().unwrap_or("").into()
}

/// The verdict on the program staged in `dirs`, 0 agree, 1 differ or 2 unsupported, and its line.
fn judge(
    dirs: &[PathBuf; 2],
    name: &str,
    limit: Duration,
    args: &[OsString],
) -> io::Result<(usize, String)> {
    let start = Instant::now();
    let (reference, _) = run(&dirs[0], ENGINES[0], limit, args)?;
    // Compiled code outruns the interpreter, so a dev run far slower than it has hung.
    let slow = (start.elapsed() * 4).max(Duration::from_millis(20));
    // How far a stopped run got depends on speed, not meaning.
    if reference.status == Some(4) && reference.stderr == b"stopped: time\n" {
        return Ok((
            2,
            format!("unsupported {name}: the interpreter ran past the limit"),
        ));
    }
    let (dev, note) = run(&dirs[1], ENGINES[1], limit.min(slow), args)?;
    Ok(
        match (
            note.strip_prefix("unsupported: "),
            difference(&reference, &dev),
        ) {
            (Some(why), _) => (2, format!("unsupported {name}: {why}")),
            (None, None) => (0, format!("agree {name}; {}", ledger(&note))),
            (None, Some(why)) => (1, format!("differ {name}: {why}; {}", ledger(&note))),
        },
    )
}

/// Runs `main.paka` in `dir` on `engine` in a child, stopped past `limit`: its outcome and note.
fn run(
    dir: &Path,
    engine: &str,
    limit: Duration,
    args: &[OsString],
) -> io::Result<(Outcome, String)> {
    let note = dir.with_extension("note");
    let _ = fs::remove_file(&note);
    let output = (Command::new(env::current_exe()?).current_dir(dir))
        .args([OsString::from("--one"), engine.into(), note.clone().into()])
        .arg(limit.as_millis().to_string())
        .args(args)
        .output()?;
    let note = fs::read_to_string(&note).unwrap_or_default();
    Ok((output.into(), note))
}

/// The verdicts so far, each listed as it comes.
struct Tally {
    out: io::StdoutLock<'static>,
    counts: [usize; 3],
}

impl Default for Tally {
    fn default() -> Tally {
        Tally {
            out: io::stdout().lock(),
            counts: [0; 3],
        }
    }
}

impl Tally {
    fn add(&mut self, (verdict, line): (usize, String)) -> io::Result<()> {
        self.counts[verdict] += 1;
        writeln!(self.out, "{line}")
    }

    /// The summary line; the status is 1 when a program differs.
    fn end(mut self) -> io::Result<ExitCode> {
        let [agree, differ, unsupported] = self.counts;
        writeln!(
            self.out,
            "{agree} agree, {differ} differ, {unsupported} unsupported"
        )?;
        Ok(ExitCode::from(u8::from(differ > 0)))
    }
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
    let ms = args[2].to_str().and_then(|n| n.parse().ok());
    let (engine, note, args) = (args[0].to_str(), &args[1], &args[3..]);
    let limit = Duration::from_millis(ms.expect("a limit in milliseconds"));
    let (source, found) = read("main.paka".as_ref());
    let checked = found.and_then(|()| paka::check("main.paka", &source));
    let Ok((program, ..)) = checked else {
        return ExitCode::from(1);
    };
    abi::ignore_signals();
    no_core_dumps();
    let status = match engine {
        Some("interp") => {
            // A shrunk program may loop for good; the interpreter has no interrupt.
            thread::spawn(move || {
                thread::sleep(limit);
                let _ = io::stderr().write_all(b"stopped: time\n");
                process::exit(4)
            });
            interp::run(&program, args, &mut Fd1::default(), &mut io::stderr())
        }
        _ => {
            let mut dev = Dev::default();
            let plant = env::var("PAKA_PLANT").ok().and_then(|n| n.parse().ok());
            dev.emitter.plant = plant.unwrap_or(0);
            let edited = dev.edit(&source).and_then(|()| match dev.program.app {
                Some(_) => Ok(()),
                None => Err("the program has no app".into()),
            });
            if let Err(why) = edited {
                let _ = fs::write(note, format!("unsupported: {why}"));
                return ExitCode::from(1);
            }
            let ran = dev.run(args, &mut Host(Fd1::default()), HEAP, limit);
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
