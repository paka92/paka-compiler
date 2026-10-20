//! Compiled headless apps (M4-36): the run loop drives the handler through its slots and hands
//! each effect to the performer the interpreter shares, so output, status and refusals match.

mod common;

use common::Dev;
use paka::core_form::Type;
use paka_rt::abi::dev::app::{App, Stopped, serve};
use paka_rt::abi::dev::{Context, on_runner};
use paka_rt::effects::{End, Host, Reason, World};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

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
    let p = &dev.program;
    let a = p.app.as_ref().expect("an app");
    let update = &p.fns[a.update];
    let (msg, effects) = (update.params.last().unwrap().ty, update.result.unwrap());
    let Type::Enum(e) = p.types[msg] else {
        unreachable!("the message is a user enum");
    };
    let fields = &p.enums[e].variants[a.start].fields;
    let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
    let typed = |t: usize| (dev.records[t], dev.layouts.get(t).size as u64);
    let state = paka::emit::state_slot(p).map(|def| {
        let (record, size) = typed(update.params[0].ty);
        (def, record, size)
    });
    let app = App {
        name: &a.name,
        grants: &a.grants,
        update: a.update,
        state,
        start: a.start as u64,
        message: typed(msg),
        effects: dev.records[effects],
    };
    let mut world = In(dir.to_path_buf(), Host(vec![]));
    let start = match paka_rt::effects::arguments(&a.name, &names, args) {
        Ok(start) => start,
        Err(end) => return (Ok(end), String::new(), (0, 0)),
    };
    let (end, ledger) = on_runner(HEAP, |ctx| {
        // SAFETY: `ctx` is a context that lives through the session.
        let session = unsafe { dev.slots.session((&raw mut *ctx).cast()) }.unwrap();
        let mut call = |ctx: &mut Context, id: usize, args: &[u64]| {
            // SAFETY: `Dev::edit` installed every definition as M4-05 section 5 compiles it, and the
            // loop passes each the argument words its signature takes.
            unsafe { session.call(id, (&raw mut *ctx).cast(), args) }.unwrap()
        };
        let end = serve(ctx, &app, &start, &mut call, &mut world);
        (end, (ctx.heap.leaks().len(), ctx.heap.faults().len()))
    });
    (end, String::from_utf8(world.1.0).unwrap(), ledger)
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
    let (source, args) = case(&dir).unwrap();
    let mut dev = Dev::default();
    dev.edit(&source).unwrap();
    let ran = run(&dev, &dir, &args);
    assert_eq!(ran, (Ok((0, String::new())), "11 words\n".into(), (0, 0)));
}

/// Every run case whose app compiles ends as the interpreter's hand-written expectations say: the
/// same output and status, and the same line on standard error unless it traps, whose report
/// waits on M4-40.
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
            let Some((source, args)) = case(&dir) else {
                continue;
            };
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            let mut dev = Dev::default();
            if let Err(why) = dev.edit(&source) {
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
