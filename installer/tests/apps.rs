//! Compiled headless apps (M4-36): the run loop drives the handler through its slots and hands
//! each effect to the performer the interpreter shares, so output, status and refusals match.

mod common;

use common::{Dev, source};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::effects::{End, Host, Reason, World};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A heap no case fills.
const HEAP: u64 = 1 << 30;

/// The host, its output kept and its paths relative to a case's folder.
struct In(PathBuf, Host<Vec<u8>>);

impl In {
    fn path(&self, path: &str) -> String {
        self.0.join(path).to_string_lossy().into_owned()
    }
}

impl World for In {
    fn print(&mut self, text: &str) -> Result<(), Reason> {
        self.1.print(text)
    }

    fn read(&mut self, path: &str) -> Result<String, Reason> {
        let path = self.path(path);
        self.1.read(&path)
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), Reason> {
        let path = self.path(path);
        self.1.write(&path, text)
    }
}

/// The run's end, or why it stopped, and its output; the ledger's leaks and faults.
type Ran = (Result<End, Stopped>, String, (usize, usize));

/// Runs the app of `dev` with `args`, its files in `dir`.
fn run(dev: &Dev, dir: &Path, args: &[OsString]) -> Ran {
    let mut world = In(dir.to_path_buf(), Host(vec![]));
    let ran = dev.run(args, &mut world, HEAP, Duration::MAX);
    (ran.end, String::from_utf8(world.1.0).unwrap(), ran.ledger)
}

fn case(dir: &Path) -> Option<(String, Vec<OsString>)> {
    let source = fs::read_to_string(dir.join("main.paka")).ok()?;
    fs::read(dir.join("status")).ok()?;
    let args = fs::read_to_string(dir.join("args")).unwrap_or_default();
    Some((source, args.lines().map(OsString::from).collect()))
}

fn root(stage: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/cases")
        .join(stage)
}

#[test]
fn the_word_count_sample_prints_its_count() {
    let dir = root("run-interp/sample_wc_count");
    let (text, args) = case(&dir).unwrap();
    let mut dev = Dev::default();
    dev.edit(&source(&text)).unwrap();
    let ran = run(&dev, &dir, &args);
    assert_eq!(ran, (Ok((0, String::new())), "11 words\n".into(), (0, 0)));
}

/// Every run case whose app compiles ends as the interpreter's hand-written expectations say: the
/// same output and status, and the same line on standard error unless it traps; `paka-diff`
/// compares trap reports (M4-40).
#[test]
fn compiled_apps_end_as_the_interpreter_does() {
    let mut compiled = vec![];
    let mut refused = vec![];
    for stage in ["run", "run-interp"] {
        let mut dirs: Vec<_> = (fs::read_dir(root(stage)).unwrap())
            .map(|d| d.unwrap().path())
            .collect();
        dirs.sort();
        for dir in dirs {
            let Some((text, args)) = case(&dir) else {
                continue;
            };
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            let mut dev = Dev::default();
            if let Err(why) = dev.edit(&source(&text)) {
                refused.push(format!("{name}: {why}"));
                continue;
            }
            if dev.program.app.is_none() {
                continue;
            }
            let read = |file| fs::read_to_string(dir.join(file)).unwrap_or_default();
            let (end, stdout, ledger) = run(&dev, &dir, &args);
            assert_eq!(stdout, read("stdout"), "{name} stdout");
            let status = match &end {
                Ok((status, line)) => {
                    assert_eq!(*line, read("stderr"), "{name} stderr");
                    assert_eq!(ledger, (0, 0), "{name} leaks or faults");
                    *status
                }
                Err(Stopped::Trap) => 3,
                Err(Stopped::Time) => panic!("{name} stopped"),
            };
            assert_eq!(format!("{status}\n"), read("status"), "{name} status");
            compiled.push(name);
        }
    }
    println!("compiled: {compiled:?}\nrefused:\n{}", refused.join("\n"));
    let each = [
        "sample_wc_count",
        "print_refused_after_read",
        "read_refused",
    ];
    for name in each {
        assert!(compiled.iter().any(|c| c == name), "{name} compiles");
    }
}
