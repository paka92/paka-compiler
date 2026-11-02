//! `paka release <path> -o <dir>` (docs/decisions/M7-03.md sections 1.3, 1.5 and 2): one read of
//! the program and its foreign files, the gate for that program on the read, one build, its
//! human-owned examples against the binary, and `<dir>` with the binary, a fresh host per granted
//! declaration and `RELEASE`, hashed again where it lands. A refusal leaves nothing at `<dir>`;
//! a release killed partway may leave an unchecked `<dir>`, and is run again.

use crate::build::{compile, runtime};
use crate::llvm::Checks;
use crate::{keep_examples, load, no_holes};
use paka::approvals::{self, Store};
use paka::core_form::Program;
use paka::diagnostic::Diagnostic;
use paka::hash::hex;
use paka::interp::{Logged, ui_log};
use paka::release::{Ran, in_release, replay, run};
use paka::sha256::sha256;
use paka::source::{Source, read};
use paka::{check, clang, foreign, gate};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, process};

/// The foreign files read once, by path: each one's bytes, or why there are none.
type Files = BTreeMap<PathBuf, Result<Vec<u8>, String>>;

/// Releases the program at `path` into `out`, the store's key checked against the public key
/// beside the private key `key`. Progress goes to standard output; a refusal is the text for
/// standard error.
pub fn release(path: &OsStr, out: &Path, key: &Path) -> Result<(), String> {
    if fs::symlink_metadata(out).is_ok() {
        return Err(exists(out));
    }
    let name = path.to_string_lossy();
    let compiler = compiler(&name)?;
    let shown = Path::new(path).display();
    let (root, program) = approvals::find(Path::new(path))
        .ok_or_else(|| format!("release: no approvals at or above {shown}\n"))?;
    let (store, approved) = verified(&root, key)?;
    let (source, found) = read(path);
    let checked = found.and_then(|()| load(&name, check(&name, &source), true));
    let (mut core, holes) = checked.map_err(|d| line(&source, d))?;
    let files = foreign_files(&core);
    let read = |p: &Path| {
        (files.get(p).cloned()).unwrap_or_else(|| Err(format!("{}: not read", p.display())))
    };
    gated(&store, &root, &program, &source, &read)?;
    let owned = owned(&store, &program)?;
    no_holes(holes).map_err(|ds| ds.into_iter().map(|d| line(&source, d)).collect::<String>())?;
    let logs: Vec<_> = (core.ui_examples.iter())
        .filter(|f| owned.contains(&f.name))
        .map(|f| (f.name.clone(), ui_log(&core, f)))
        .collect();
    let functions: BTreeSet<_> = (owned.into_iter())
        .filter(|n| core.examples.iter().any(|e| e.name == *n))
        .collect();
    keep_examples(&mut core, &functions, &name).map_err(|d| line(&source, d))?;
    let app = core.app.as_ref().expect("load keeps an app").name.clone();
    let mut command = format!("paka build {program} -o {app}");
    if !functions.is_empty() {
        command += " --examples";
        functions.iter().for_each(|n| command += &format!(" {n}"));
    }

    let made = staged(out)?;
    let dir = made.0.clone().expect("staged");
    let bin = dir.join(&app);
    compile(&name, &mut core, bin.as_os_str(), Checks::All).map_err(|d| line(&source, d))?;
    println!("build: {command}");
    let hosts = hosts(&core, &dir, &app, &read)?;
    let hashes = listed(&dir, &[vec![app.clone()], hosts.clone()].concat())?;
    let results = examples(&bin, &functions, logs)?;

    let mut text = format!("program    {program}\n");
    for (name, bytes) in sources(&source, &core, &files) {
        writeln!(text, "source     {name}  sha256:{}", hex(&sha256(bytes))).unwrap();
    }
    writeln!(text, "build      {command}").unwrap();
    writeln!(text, "binary     {app}  sha256:{}", hashes[&app]).unwrap();
    for host in &hosts {
        writeln!(text, "host       {host}  sha256:{}", hashes[host]).unwrap();
    }
    text += &compiler;
    let (serial, approved) = (store.serial, hex(&sha256(&approved)));
    writeln!(text, "approvals  serial {serial}  sha256:{approved}").unwrap();
    for (name, result) in &results {
        writeln!(text, "example    {name}  {result}").unwrap();
    }
    landed(made, out, &text, hashes.clone())?;
    println!("released: {}  {app} sha256 {}", out.display(), hashes[&app]);
    Ok(())
}

fn exists(out: &Path) -> String {
    format!("release: {} already exists\n", out.display())
}

/// The `compiler` and `runtime` lines of `RELEASE`: this paka and the runtime it links, hashed
/// before the build, and the pinned clang. `name` is the program path for diagnostics.
fn compiler(name: &str) -> Result<String, String> {
    let runtime = runtime(name).map_err(|d| format!("{d}\n"))?;
    let [paka, runtime] = [env::current_exe(), Ok(runtime)].map(|p| {
        p.and_then(fs::read)
            .map(|bytes| hex(&sha256(&bytes)))
            .map_err(|e| format!("release: the compiler: {e}\n"))
    });
    let (version, clang) = (env!("CARGO_PKG_VERSION"), clang::VERSION.trim());
    Ok(format!(
        "compiler   paka {version}  sha256:{}  clang {clang}\nruntime    libpaka_rt.a  sha256:{}\n",
        paka?, runtime?
    ))
}

/// The store in `root` and the bytes it was read from, once its `approvals.pub` is the public
/// key beside `key` and signs it.
fn verified(root: &Path, key: &Path) -> Result<(Store, Vec<u8>), String> {
    let mut public = key.as_os_str().to_owned();
    public.push(".pub");
    let public = PathBuf::from(public);
    let stored = fs::read_to_string(root.join("approvals.pub"))
        .map_err(|e| format!("key: approvals.pub: {e}\n"))?;
    let mine =
        fs::read_to_string(&public).map_err(|e| format!("key: {}: {e}\n", public.display()))?;
    if key_fields(&stored).is_none() || key_fields(&stored) != key_fields(&mine) {
        return Err(format!(
            "key: approvals.pub differs from {}\n",
            public.display()
        ));
    }
    println!("key: approvals.pub matches {}", public.display());
    let text = fs::read(root.join("approvals")).map_err(|e| format!("approvals: {e}\n"))?;
    let store = Store::verify(root, &text, &stored);
    Ok((store.map_err(|why| format!("{why}\n"))?, text))
}

/// A public key's type and base64 fields, its comment left out.
fn key_fields(text: &str) -> Option<(&str, &str)> {
    let mut fields = text.lines().next()?.split_whitespace();
    Some((fields.next()?, fields.next()?))
}

/// Nothing pending for `program` in the gate, on the files as read once.
fn gated(
    store: &Store,
    root: &Path,
    program: &str,
    source: &Source,
    read: &foreign::Read,
) -> Result<(), String> {
    let items = gate::program(store, root, program, source, read);
    if !items.is_empty() {
        let lines: String = items.iter().map(|i| i.line() + "\n").collect();
        return Err(lines + "Revert the change, or ask the owner to run paka approve.\n");
    }
    println!(
        "gate: approvals serial {}, signature good, nothing pending",
        store.serial
    );
    Ok(())
}

/// The human-owned examples of `program`, which must have records and at least one.
fn owned(store: &Store, program: &str) -> Result<BTreeSet<String>, String> {
    if !store.programs().contains(&program) {
        return Err(format!("release: {program} has no approval records\n"));
    }
    let owned = store.owned(program);
    if owned.is_empty() {
        return Err(format!("release: {program} has no human-owned example\n"));
    }
    Ok(owned)
}

/// The `source:` and `header:` files of every declaration in `core`, each read once.
fn foreign_files(core: &Program) -> Files {
    let mut files = Files::new();
    for library in &core.libraries {
        for (_, file) in library.files.iter().filter(|(key, _)| key != "library") {
            let path = library.dir.join(file);
            files
                .entry(path)
                .or_insert_with_key(|p| foreign::read_regular(p));
        }
    }
    files
}

/// Each file read, by its name in the program's directory, and its bytes: the `.paka` files,
/// then the foreign files that could be read.
fn sources<'a>(source: &'a Source, core: &Program, files: &'a Files) -> Vec<(String, &'a [u8])> {
    let mut all: Vec<_> = (source.files.iter())
        .map(|f| (f.name.clone(), &source.text[f.bytes.clone()]))
        .collect();
    let mut seen = vec![];
    for library in &core.libraries {
        for (_, file) in &library.files {
            let path = library.dir.join(file);
            if let Some(Ok(bytes)) = files.get(&path).filter(|_| !seen.contains(&path)) {
                all.push((file.clone(), bytes.as_slice()));
                seen.push(path);
            }
        }
    }
    all
}

/// The directory a release is built in beside `out`, `.<dir>.release-<pid>`, once those of runs
/// no longer alive are removed.
fn staged(out: &Path) -> Result<Made, String> {
    let base = out
        .file_name()
        .ok_or_else(|| format!("release: {} names no directory\n", out.display()))?;
    let parent = out.parent().unwrap_or(Path::new(""));
    let prefix = format!(".{}.release-", base.to_string_lossy());
    let entries = fs::read_dir(parent.join("."))
        .into_iter()
        .flatten()
        .flatten();
    // Without /proc every run would look dead.
    let alive =
        |pid: &str| !Path::new("/proc/self").exists() || Path::new("/proc").join(pid).exists();
    for name in entries.filter_map(|e| e.file_name().into_string().ok()) {
        let pid = name
            .strip_prefix(&prefix)
            .filter(|p| p.parse::<u32>().is_ok());
        if pid.is_some_and(|pid| !alive(pid)) {
            let _ = fs::remove_dir_all(parent.join(&name));
        }
    }
    let dir = parent.join(format!("{prefix}{}", process::id()));
    fs::create_dir(&dir).map_err(|e| format!("release: {}: {e}\n", dir.display()))?;
    Ok(Made(Some(dir)))
}

/// Builds the host of each granted declaration of `core` from the files `read` gives, never from
/// the host cache, as `<app>.<name>` in `dir`: the names given.
fn hosts(
    core: &Program,
    dir: &Path,
    app: &str,
    read: &foreign::Read,
) -> Result<Vec<String>, String> {
    let granted = &core.app.as_ref().expect("load keeps an app").foreign;
    let mut names = vec![];
    for library in core.libraries.iter().filter(|l| granted.contains(&l.name)) {
        let (name, build) = (
            format!("{app}.{}", library.name),
            dir.join(format!(".host-{}", library.name)),
        );
        let built = foreign::build(library, &build, read)
            .and_then(|exe| fs::rename(exe, dir.join(&name)).map_err(|e| e.to_string()));
        let _ = fs::remove_dir_all(&build);
        built.map_err(|why| format!("host {}: {why}\n", library.name))?;
        names.push(name);
    }
    Ok(names)
}

/// The hashes of the files in `dir`, which holds only those `shipped`.
fn listed(dir: &Path, shipped: &[String]) -> Result<BTreeMap<String, String>, String> {
    let hashes = hashed(dir)?;
    match hashes.keys().find(|n| !shipped.contains(n)) {
        Some(name) => Err(format!("artefact: {name} is not a file RELEASE lists\n")),
        None => Ok(hashes),
    }
}

/// Runs each of `functions` on `bin` and replays each UI example's log: each result by name, or
/// a line per example that does not pass.
fn examples(
    bin: &Path,
    functions: &BTreeSet<String>,
    logs: Vec<(String, Result<Vec<Logged>, String>)>,
) -> Result<BTreeMap<String, String>, String> {
    let mut results = BTreeMap::new();
    let mut failed = String::new();
    for name in functions {
        match run(bin, name) {
            Ok(Ran::Pass) => _ = results.insert(name.clone(), "pass".into()),
            Ok(ran) => writeln!(failed, "example {name}: {ran}").unwrap(),
            Err(e) => writeln!(failed, "example {name}: {e}").unwrap(),
        }
    }
    let mut agreed = vec![];
    for (name, log) in logs {
        let log = log.map_err(|why| format!("interpreter: {why}"));
        match log.and_then(|log| in_release(&log, |h| replay(&mut Command::new(bin), h))) {
            Ok(n) => {
                agreed.push(format!("{name} {n} of {n} actions agree"));
                results.insert(name, format!("{n} of {n} actions agree"));
            }
            Err(why) => writeln!(failed, "example {name}: {why}").unwrap(),
        }
    }
    if !failed.is_empty() {
        return Err(failed);
    }
    let count = functions.len();
    println!("examples in binary: {count} of {count} pass");
    match agreed.is_empty() {
        true => println!("ui examples replayed: none"),
        false => println!("ui examples replayed: {}", agreed.join(", ")),
    }
    Ok(results)
}

/// Writes `RELEASE` into the staged directory of `made`, moves it to `out` and hashes it there
/// against `hashes`. std has no rename that refuses an existing target: an empty `out` made
/// between the last check and the rename is replaced.
fn landed(
    mut made: Made,
    out: &Path,
    text: &str,
    mut hashes: BTreeMap<String, String>,
) -> Result<(), String> {
    let dir = made.0.clone().expect("staged");
    fs::write(dir.join("RELEASE"), text).map_err(|e| format!("release: RELEASE: {e}\n"))?;
    if fs::symlink_metadata(out).is_ok() {
        return Err(exists(out));
    }
    fs::rename(&dir, out).map_err(|e| format!("release: {}: {e}\n", out.display()))?;
    made.0 = Some(out.into());
    hashes.insert("RELEASE".into(), hex(&sha256(text.as_bytes())));
    let landed = hashed(out)?;
    let changed: BTreeSet<_> = (hashes.keys().chain(landed.keys()))
        .filter(|n| hashes.get(*n) != landed.get(*n))
        .collect();
    if !changed.is_empty() {
        let line = |n| format!("artefact: {n} changed between the runs and the final path\n");
        return Err(changed.into_iter().map(line).collect());
    }
    made.0 = None;
    Ok(())
}

/// The SHA-256 of each file in `dir`, by name; every entry must be a regular file.
fn hashed(dir: &Path) -> Result<BTreeMap<String, String>, String> {
    let failed = |e: std::io::Error| format!("artefact: {}: {e}\n", dir.display());
    let mut hashes = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type().map_err(failed)?.is_file() {
            return Err(format!("artefact: {name} is not a regular file\n"));
        }
        let bytes = fs::read(entry.path()).map_err(failed)?;
        hashes.insert(name, hex(&sha256(&bytes)));
    }
    Ok(hashes)
}

/// `d` as one line, in the file it is in.
fn line(source: &Source, d: Diagnostic) -> String {
    format!("{}\n", source.locate(d).0)
}

/// A directory this run made, removed on drop unless taken.
struct Made(Option<PathBuf>);

impl Drop for Made {
    fn drop(&mut self) {
        if let Some(dir) = &self.0 {
            let _ = fs::remove_dir_all(dir);
        }
    }
}
