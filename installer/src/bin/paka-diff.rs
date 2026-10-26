//! `paka-diff [ROOT]`: runs every run case under ROOT, `tests/conformance` by default, on the
//! interpreter and in dev mode, and lists each as agree, differ or unsupported, with whether the
//! dev run's ledger ended clean (M4-40); one that did not differs. The status is 1 when a case
//! differs.
//!
//! `paka-diff --release [ROOT]` adds release mode (M6-06): each case is also built with the `paka`
//! next to this binary and run, and must agree with the interpreter. A case with a hole must be
//! refused by the holes' names instead; a case is skipped while one of its tags (release.rs) is in
//! ROOT's `release-skip`. ROOT's `release-exempt` must list exactly the cases with a hole and the
//! example-status cases.
//!
//! `paka-diff --fuzz FIRST COUNT KEEP` does the same for the generated programs of COUNT seeds
//! from FIRST (M4-41). Each that differs is shrunk while it keeps that kind of difference and
//! written to `KEEP/SEED.paka`. `PAKA_PLANT` plants a fault in the dev back end, for a self-test.
//! `paka-diff --fuzz-release FIRST COUNT KEEP` also builds each program in release (M6-20) and
//! keeps each that differs as the run case `KEEP/SEED`, its expectations the interpreter's run.
//! Where the interpreter ends in a stack trap, release may finish or run on: that run is cut by
//! the limit and not compared, unless it died by a signal. `PAKA_PLANT` 9 to 18 plants a fault in
//! every release build, in the IR handed to clang (`RELEASE_PLANTS`, M6-21).
//!
//! `paka-diff --edits FIRST COUNT KEEP` runs the edit sequences of COUNT seeds from FIRST (M4-43).
//! After each edit that checks, the dev run has edited in every version before it, so it runs the
//! code incremental compiles left; its output must equal the interpreter's on the same version. A
//! sequence that differs is kept as `KEEP/SEED`, the dev run's folder: `main.paka` and the versions
//! before it in `edits/`. `PAKA_PLANT` 6 or 7 plants staleness in the compile cache.
//!
//! `paka-diff --ui [ROOT]` plays every UI example of the programs under ROOT, `tests` by default,
//! on both back ends (M5-26): each action's events, replayed in dev mode, must leave the tree, trap
//! or end the interpreter's action left. Each program is a child, `paka-diff --ui-one NAME` in its
//! folder. `PAKA_PLANT` 8 plants a view fault.
//!
//! Each run is a child, `paka-diff --one interp|dev NOTE LIMIT_MS ARGS...` in the staged case
//! folder: it runs `main.paka` as `paka run` does, stopped past the limit, and in dev mode writes
//! its ledger, or why the dev back end refused the program, to NOTE.

use paka::interp;
use paka::source::{Source, read};
use paka_installer::dev::Dev;
use paka_installer::diff::{
    Outcome, cases, difference, difference_in, no_core_dumps, shrink, stage, ui_examples,
};
use paka_installer::release;
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::Host;
use paka_rt::{Fd1, abi};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// The heap a dev run may fill.
const HEAP: u64 = 1 << 30;
/// How long a run of a conformance case, and of a generated program, may take before it stops.
const LIMIT: Duration = Duration::from_secs(3);
const FUZZ_LIMIT: Duration = Duration::from_millis(250);
const CONFORMANCE: &str = "tests/conformance";

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let r = match args.first().and_then(|a| a.to_str()) {
        Some("--one") => return one(&args[1..]),
        Some("--ui-one") => return ui_one(&args[1].to_string_lossy()),
        Some("--ui") => ui(&args.get(1).map_or("tests".into(), PathBuf::from)),
        Some("--release") => {
            compare_release(&args.get(1).map_or(CONFORMANCE.into(), PathBuf::from))
        }
        Some(mode @ ("--fuzz" | "--fuzz-release" | "--edits")) => {
            let number = |i: usize| args.get(i).and_then(|a| a.to_str()?.parse::<u64>().ok());
            let (Some(first), Some(count), Some(keep)) = (number(1), number(2), args.get(3)) else {
                eprintln!("usage: paka-diff {mode} FIRST COUNT KEEP");
                return ExitCode::from(2);
            };
            let seeds = first..first + count;
            match mode {
                "--edits" => edits(seeds, keep.as_ref()),
                _ => fuzz(seeds, keep.as_ref(), mode == "--fuzz-release"),
            }
        }
        _ => compare(&args.first().map_or(CONFORMANCE.into(), PathBuf::from)),
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

fn compare_release(root: &Path) -> io::Result<ExitCode> {
    let file = |name| fs::read_to_string(root.join(name));
    let invalid = |why| io::Error::new(io::ErrorKind::InvalidData, why);
    let skip = release::skipped(&file("release-skip")?).map_err(invalid)?;
    let [holes, examples] = release::exempt(&file("release-exempt")?).map_err(invalid)?;
    let named = |dir: &Path| dir.strip_prefix(root).unwrap_or(dir).display().to_string();
    let command = |dir: &Path| fs::read_to_string(dir.join("command")).unwrap_or_default();
    let found: BTreeSet<_> = (programs(root)?.iter())
        .filter(|dir| command(dir).starts_with("examples "))
        .map(|dir| named(dir))
        .collect();
    if found != examples {
        return Err(invalid(format!(
            "release-exempt lists examples {examples:?}, not {found:?}"
        )));
    }
    let paka = env::current_exe()?.with_file_name("paka");
    let scratch = scratch();
    let (mut tally, mut skipped, mut seen) = (Tally::default(), 0, BTreeSet::new());
    for case in cases(root)? {
        let name = named(&case);
        let flat = name.replace('/', "-");
        let dirs = ENGINES.map(|e| scratch.join(format!("{flat}-{e}")));
        dirs.iter().try_for_each(|d| stage(&case, d))?;
        let args = arguments(&case);
        let ((mut verdict, mut line), reference) = judged(&dirs, &name, LIMIT, &args)?;
        let (source, found) = read(&dirs[0].join("main.paka").into_os_string());
        let Ok((core, ..)) = found.and_then(|()| paka::check("main.paka", &source)) else {
            tally.add((1, format!("differ {name}: the program does not check")))?;
            continue;
        };
        let names: Vec<_> = (core.sites.iter())
            .filter_map(|s| s.hole.as_deref())
            .collect();
        let result = if !names.is_empty() {
            seen.insert(name.clone());
            refused(&paka, &dirs[1], &names, holes.contains(&name))?
        } else if release::tags(&core).is_disjoint(&skip) {
            let built = scratch.join(&flat);
            stage(&case, &built)?;
            ran(&paka, &built, &reference, &args)?
        } else {
            skipped += 1;
            Ok("skipped".into())
        };
        match result {
            Ok(note) => line += &format!("; release {note}"),
            Err(why) => (verdict, line) = (1, format!("differ {name}: release {why}")),
        }
        tally.add((verdict, line))?;
    }
    let _ = fs::remove_dir_all(&scratch);
    if seen != holes {
        return Err(invalid(format!(
            "release-exempt lists holes {holes:?}, not {seen:?}"
        )));
    }
    writeln!(tally.out, "{skipped} skipped in release")?;
    tally.end()
}

/// Whether `paka build` refuses the program staged in `dir`, which has holes `names`, naming each
/// one; `listed` says the exemption list names the case.
fn refused(
    paka: &Path,
    dir: &Path,
    names: &[&str],
    listed: bool,
) -> io::Result<Result<String, String>> {
    if !listed {
        return Ok(Err(
            "has a hole but is not a `hole` line of release-exempt".into()
        ));
    }
    let out = Outcome::from(build(paka, dir)?);
    let text = String::from_utf8_lossy(&out.stderr);
    let named = names.iter().all(|n| text.contains(&format!("??{n}")));
    Ok(
        match out.status == Some(1) && text.contains("[hole-in-build]") && named {
            true => Ok("refused by the holes' names".into()),
            false => Err(format!(
                "build did not refuse the holes: {:?}, {text:?}",
                out.status
            )),
        },
    )
}

/// Builds the program staged in `dir`, runs it stopped past the limit, and compares its run with
/// the interpreter's, `reference`.
fn ran(
    paka: &Path,
    dir: &Path,
    reference: &Outcome,
    args: &[OsString],
) -> io::Result<Result<String, String>> {
    let out = build(paka, dir)?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).into_owned();
        let stale = match why.contains("undefined") && why.contains("paka_rt1_") {
            true => " (the runtime looks stale: rebuild paka-rt in release)",
            false => "",
        };
        return Ok(Err(format!("build failed{stale}: {why:?}")));
    }
    let release = native(&dir.with_extension("prog"), dir, args, LIMIT)?;
    Ok(match difference_in(reference, &release, "release") {
        Some(why) => Err(why),
        None => Ok("agree".into()),
    })
}

/// `paka build main.paka` in `dir`, written beside it.
fn build(paka: &Path, dir: &Path) -> io::Result<process::Output> {
    let mut command = Command::new(paka);
    command.current_dir(dir).args([
        "build".as_ref(),
        "main.paka".as_ref(),
        "-o".as_ref(),
        dir.with_extension("prog").as_os_str(),
    ]);
    if let Some(plant) = release_plant() {
        command.env("PATH", planted_clang(plant)?);
    }
    command.output()
}

/// The release fault `PAKA_PLANT` 9 to 18 names (M6-21): a sed script on the IR handed to clang.
fn release_plant() -> Option<usize> {
    let n: usize = env::var("PAKA_PLANT").ok()?.parse().ok()?;
    (9..9 + RELEASE_PLANTS.len()).contains(&n).then_some(n)
}

const RELEASE_PLANTS: [&str; 10] = [
    // 9: `>` as `>=`.
    "s/icmp sgt/icmp sge/g",
    // 10: Float arithmetic folded as if no value were NaN or infinite.
    r"s/= f\(add\|sub\|mul\|div\) double/= f\1 fast double/",
    // 11: the bounds check of a List index left out.
    "s/= icmp uge i64 .*/= icmp ne i64 0, 0/",
    // 12: the overflow check of checked arithmetic left out.
    "s/= extractvalue { i64, i1 } .*, 1$/= icmp ne i64 0, 0/",
    // 13: the stack check of each function small enough to inline left out.
    "/^define .* noinline {/,/^}/!s/br i1 %deep,/br i1 false,/",
    // 14: sort's Float instance built as its Int instance.
    "/^define internal i32 @s[0-9]/,/^}/{s/fcmp olt double/icmp slt i64/;s/double/i64/g}",
    // 15: each owned value dropped twice.
    "/call void @paka_rt1_drop(/p",
    // 16: a String-keyed Map row of word values laid out as an Int-keyed one.
    "/@paka_rt1_map(/s/i64 -9223372036854775800,/i64 8,/",
    // 17: the count check of a shift left out.
    r"s/= icmp ugt i64 \(.*\), 63$/= icmp ne i64 0, 0/",
    // 18: the range check of `to_int` left out.
    "s/= fcmp o[gl][et] double .*, 0x[C4]3E0000000000000$/= icmp eq i64 0, 0/",
];

/// A `PATH` whose first `clang` applies release plant `n` to each IR file it is given, then runs
/// the next `clang` on `PATH`.
fn planted_clang(n: usize) -> io::Result<OsString> {
    let dir = scratch().join(format!("plant{n}"));
    let clang = dir.join("clang");
    if !clang.exists() {
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("plant.sed"), RELEASE_PLANTS[n - 9])?;
        let script = "#!/bin/sh\n\
            for a; do case $a in *.ll) sed -i -f \"${0%/*}/plant.sed\" \"$a\";; esac; done\n\
            PATH=${PATH#*:} exec clang \"$@\"\n";
        fs::write(&clang, script)?;
        fs::set_permissions(&clang, fs::Permissions::from_mode(0o755))?;
    }
    let mut path = dir.into_os_string();
    path.push(":");
    path.push(env::var_os("PATH").unwrap_or_default());
    Ok(path)
}

/// Runs `exe` in `dir` with no display, killed past `limit`, its output in files beside `dir`.
fn native(exe: &Path, dir: &Path, args: &[OsString], limit: Duration) -> io::Result<Outcome> {
    let [stdout, stderr] = ["stdout", "stderr"].map(|e| dir.with_extension(e));
    let mut child = (Command::new(exe).current_dir(dir).args(args))
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout)?)
        .stderr(fs::File::create(&stderr)?)
        .spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() > limit {
            child.kill()?;
            child.wait()?;
            break None;
        }
        thread::sleep(Duration::from_millis(2));
    };
    let Some(status) = status else {
        return Ok(Outcome {
            stdout: fs::read(&stdout)?,
            stderr: b"stopped: time\n".to_vec(),
            status: Some(4),
        });
    };
    Ok(process::Output {
        status,
        stdout: fs::read(&stdout)?,
        stderr: fs::read(&stderr)?,
    }
    .into())
}

fn fuzz(seeds: Range<u64>, keep: &Path, release: bool) -> io::Result<ExitCode> {
    no_core_dumps();
    let scratch = scratch();
    let dirs = ENGINES.map(|e| scratch.join(e));
    let built = scratch.join("release");
    let all = [&dirs[0], &dirs[1], &built];
    all.iter().try_for_each(fs::create_dir_all)?;
    let write = |text: &str| {
        all.iter()
            .try_for_each(|d| fs::write(d.join("main.paka"), text))
    };
    let judge = |name: &str| match release {
        true => judged_release(&dirs, &built, name),
        false => judged(&dirs, name, FUZZ_LIMIT, &[]).map(|(verdict, _)| verdict),
    };
    let mut tally = Tally::default();
    for seed in seeds {
        let text = paka_gen::program(seed);
        write(&text)?;
        let (verdict, line) = judge(&format!("seed {seed}"))?;
        let want = (verdict == 1).then(|| kind(&line));
        tally.add((verdict, line))?;
        let Some(want) = want else {
            continue;
        };
        let small = shrink(&text, |t| {
            let checks = paka::check("main.paka", &Source::file("main.paka", t.into())).is_ok();
            let judged = || write(t).and_then(|()| judge(""));
            checks && judged().is_ok_and(|(v, line)| v == 1 && kind(&line) == want)
        });
        fs::create_dir_all(keep)?;
        let kept = match release {
            true => keep_case(&keep.join(seed.to_string()), &small, &dirs[0])?,
            false => keep.join(format!("{seed}.paka")),
        };
        fs::write(&kept, &small)?;
        let lines = small.lines().count();
        writeln!(tally.out, "  kept {} ({lines} lines)", kept.display())?;
    }
    let _ = fs::remove_dir_all(&scratch);
    tally.end()
}

/// `judged`'s verdict on the generated program staged in `dirs`, with the copy in `built` also
/// built and run in release, which must agree with the interpreter.
fn judged_release(dirs: &[PathBuf; 2], built: &Path, name: &str) -> io::Result<(usize, String)> {
    let ((verdict, line), reference) = judged(dirs, name, FUZZ_LIMIT, &[])?;
    if stopped(&reference) {
        return Ok((verdict, line));
    }
    let (release, note) = released(built, &reference)?;
    if release == 1 {
        return Ok((1, format!("differ {name}: release {note}")));
    }
    let verdict = if verdict == 1 {
        1
    } else {
        verdict.min(release)
    };
    let rest = line.split_once(' ').map_or("", |(_, r)| r);
    let word = ["agree", "differ", "unsupported"][verdict];
    Ok((verdict, format!("{word} {rest}; release {note}")))
}

/// The release verdict on the program staged in `built`, against the interpreter's run.
fn released(built: &Path, reference: &Outcome) -> io::Result<(usize, String)> {
    let out = build(&env::current_exe()?.with_file_name("paka"), built)?;
    let why = String::from_utf8_lossy(&out.stderr);
    let refused = ["[unsupported-form]", "[hole-in-build]"]
        .iter()
        .any(|r| why.contains(r));
    if !out.status.success() {
        return Ok(match refused {
            true => (
                2,
                format!("unsupported: {}", why.lines().next().unwrap_or("")),
            ),
            false => (1, format!("build failed: {why:?}")),
        });
    }
    let run = native(&built.with_extension("prog"), built, &[], FUZZ_LIMIT)?;
    let trap = String::from_utf8_lossy(&reference.stderr);
    let stack = trap
        .lines()
        .next()
        .is_some_and(|l| l.contains(" trap stack at "));
    Ok(match difference_in(reference, &run, "release") {
        // Release may finish or run on where the interpreter's stack ran out, not crash (M6-01).
        _ if reference.status == Some(3) && stack && run.status.is_some() => {
            (2, "not compared: a stack trap".into())
        }
        Some(why) => (1, why),
        None => (0, "agree".into()),
    })
}

/// Writes the run case `dir` of program `text`, expecting what its run on the interpreter staged
/// in `interp` leaves; the path of its program.
fn keep_case(dir: &Path, text: &str, interp: &Path) -> io::Result<PathBuf> {
    fs::write(interp.join("main.paka"), text)?;
    let (out, _) = run(interp, ENGINES[0], FUZZ_LIMIT, &[])?;
    let status = out
        .status
        .ok_or_else(|| io::Error::other("the interpreter was killed"))?;
    fs::create_dir_all(dir)?;
    fs::write(dir.join("stdout"), &out.stdout)?;
    fs::write(dir.join("status"), format!("{status}\n"))?;
    if !out.stderr.is_empty() {
        fs::write(dir.join("stderr"), &out.stderr)?;
    }
    Ok(dir.join("main.paka"))
}

fn ui(root: &Path) -> io::Result<ExitCode> {
    let mut tally = Tally::default();
    for dir in programs(root)? {
        let (source, found) = read(&dir.join("main.paka").into_os_string());
        let checked = found.and_then(|()| paka::check("main.paka", &source));
        if !checked.is_ok_and(|(p, ..)| !p.ui_examples.is_empty()) {
            continue;
        }
        let name = match dir.strip_prefix(root) {
            Ok(rel) if rel != Path::new("") => rel.display().to_string(),
            _ => dir.display().to_string(),
        };
        let out = (Command::new(env::current_exe()?).current_dir(&dir))
            .args(["--ui-one", &name])
            .output()?;
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let verdict = ["agree", "differ"].iter().position(|v| line.starts_with(v));
            tally.add((verdict.unwrap_or(2), line.into()))?;
        }
        if !matches!(out.status.code(), Some(0 | 1)) {
            let err = String::from_utf8_lossy(&out.stderr);
            tally.add((
                1,
                format!("differ {name}: ended with {:?}: {err}", out.status),
            ))?;
        }
    }
    tally.end()
}

/// The folders under `root` that hold a `main.paka`, in order.
fn programs(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = vec![];
    if root.join("main.paka").is_file() {
        found.push(root.into());
    }
    let mut dirs: Vec<_> =
        (fs::read_dir(root)?.map(|e| e.map(|e| e.path()))).collect::<io::Result<_>>()?;
    dirs.sort();
    for dir in dirs.into_iter().filter(|d| d.is_dir()) {
        found.extend(programs(&dir)?);
    }
    Ok(found)
}

/// Plays the UI examples of `main.paka` here on both back ends and lists each as agree or
/// differ, or the program as unsupported; the status is 1 when one differs.
fn ui_one(name: &str) -> ExitCode {
    let (source, found) = read("main.paka".as_ref());
    let Ok((core, ..)) = found.and_then(|()| paka::check("main.paka", &source)) else {
        return ExitCode::from(2);
    };
    abi::ignore_signals();
    no_core_dumps();
    let mut dev = Dev::default();
    dev.emitter.plant = env::var("PAKA_PLANT").map_or(0, |n| n.parse().unwrap_or(0));
    if let Err(why) = dev.edit(&source) {
        println!("unsupported {name}: {why}");
        return ExitCode::SUCCESS;
    }
    let mut differ = false;
    for (example, verdict) in ui_examples(&core, &dev, HEAP, LIMIT) {
        match verdict {
            Ok(actions) => println!("agree {name} {example}; {actions} actions"),
            Err(why) => {
                differ = true;
                println!("differ {name} {example}: {why}");
            }
        }
    }
    ExitCode::from(u8::from(differ))
}

/// The edits after each generated program.
const STEPS: usize = 8;

fn edits(seeds: Range<u64>, keep: &Path) -> io::Result<ExitCode> {
    let scratch = scratch();
    let dirs = ENGINES.map(|e| scratch.join(e));
    dirs.iter().try_for_each(fs::create_dir_all)?;
    let mut tally = Tally::default();
    let mut kinds: BTreeMap<&str, (u64, u64)> = BTreeMap::new();
    let before = dirs[1].join("edits");
    for seed in seeds {
        let _ = fs::remove_dir_all(&before);
        fs::create_dir(&before)?;
        let versions = paka_gen::edit::versions(seed, STEPS, |t| {
            paka::check("main.paka", &Source::file("main.paka", t.into())).is_ok()
        });
        for (step, v) in versions.iter().enumerate() {
            let counts = kinds.entry(v.kind).or_default();
            counts.0 += 1;
            counts.1 += u64::from(v.checks);
            if v.checks {
                dirs.iter()
                    .try_for_each(|d| fs::write(d.join("main.paka"), &v.text))?;
                let name = format!("seed {seed} step {step} {}", v.kind);
                let (verdict, line) = judge(&dirs, &name, FUZZ_LIMIT, &[])?;
                tally.add((verdict, line))?;
                if verdict == 1 {
                    let kept = keep.join(seed.to_string());
                    fs::create_dir_all(kept.join("edits"))?;
                    fs::write(kept.join("main.paka"), &v.text)?;
                    for (k, v) in versions[..step].iter().enumerate() {
                        fs::write(kept.join(format!("edits/{k}.paka")), &v.text)?;
                    }
                    writeln!(tally.out, "  kept {}", kept.display())?;
                    break;
                }
            }
            fs::write(before.join(format!("{step}.paka")), &v.text)?;
        }
    }
    let _ = fs::remove_dir_all(&scratch);
    for (kind, (made, accepted)) in kinds {
        writeln!(tally.out, "{kind:>9}: {made} made, {accepted} accepted")?;
    }
    tally.end()
}

const ENGINES: [&str; 2] = ["interp", "dev"];

fn scratch() -> PathBuf {
    env::temp_dir().join(format!("paka-diff-{}", process::id()))
}

/// The kind of difference a `differ` line names: the first word of its reason, two for release.
fn kind(line: &str) -> String {
    let reason = line.split_once(": ").map_or("", |(_, r)| r);
    let words = 1 + usize::from(reason.starts_with("release "));
    reason.split(' ').take(words).collect::<Vec<_>>().join(" ")
}

/// Whether a run was stopped past its limit.
fn stopped(o: &Outcome) -> bool {
    o.status == Some(4) && o.stderr == b"stopped: time\n"
}

/// The verdict on the program staged in `dirs`, 0 agree, 1 differ or 2 unsupported, and its line.
fn judge(
    dirs: &[PathBuf; 2],
    name: &str,
    limit: Duration,
    args: &[OsString],
) -> io::Result<(usize, String)> {
    judged(dirs, name, limit, args).map(|(verdict, _)| verdict)
}

/// The verdict `judge` gives and the interpreter's run.
fn judged(
    dirs: &[PathBuf; 2],
    name: &str,
    limit: Duration,
    args: &[OsString],
) -> io::Result<((usize, String), Outcome)> {
    let start = Instant::now();
    let (reference, _) = run(&dirs[0], ENGINES[0], limit, args)?;
    // Compiled code outruns the interpreter, so a dev run far slower than it has hung.
    let slow = (start.elapsed() * 4).max(Duration::from_millis(20));
    // How far a stopped run got depends on speed, not meaning.
    if stopped(&reference) {
        let why = format!("unsupported {name}: the interpreter ran past the limit");
        return Ok(((2, why), reference));
    }
    let (dev, note) = run(&dirs[1], ENGINES[1], limit.min(slow), args)?;
    let verdict = match (
        note.strip_prefix("unsupported: "),
        difference(&reference, &dev),
    ) {
        (Some(why), _) => (2, format!("unsupported {name}: {why}")),
        // A leak or a bad free is a fault even when the output agrees.
        (None, None) if ledger(&note) != "ledger clean" => {
            (1, format!("differ {name}: {}", ledger(&note)))
        }
        (None, None) => (0, format!("agree {name}; {}", ledger(&note))),
        (None, Some(why)) => (1, format!("differ {name}: {why}; {}", ledger(&note))),
    };
    Ok((verdict, reference))
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
            // A step of an edit sequence: the versions before it, `edits/0.paka` on, go in first.
            for k in 0.. {
                let Ok(text) = fs::read(format!("edits/{k}.paka")) else {
                    break;
                };
                let _ = dev.edit(&Source::file("main.paka", text));
            }
            let edited = dev.edit(&source).and_then(|()| match dev.program.app {
                Some(_) => Ok(()),
                None => Err("the program has no app".into()),
            });
            if let Err(why) = edited {
                let _ = fs::write(note, format!("unsupported: {why}"));
                return ExitCode::from(1);
            }
            let ran = dev.run(args, &mut Host(Fd1::default()), HEAP, limit);
            let (mut leaks, faults) = ran.ledger;
            // A trap's unwind drops nothing; the run's end frees what was live (M4-05 section 9).
            if matches!(ran.end, Err(Stopped::Trap)) {
                leaks = 0;
            }
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
