//! `paka release <path> -o <dir>` (docs/decisions/M7-03.md sections 1.3, 1.5 and 2): one read of
//! the program and its foreign files, the gate for that program on the read, one build, its
//! human-owned examples against the binary, and `<dir>` with the binary, a fresh host per granted
//! declaration and `RELEASE`, hashed again where it lands. A refusal leaves nothing at `<dir>`.

use crate::build::compile;
use crate::llvm::Checks;
use crate::load;
use paka::approvals::{self, Store};
use paka::core_form::Program;
use paka::diagnostic::Diagnostic;
use paka::hash::hex;
use paka::interp::ui_log;
use paka::release::{Ran, in_release, replay, run};
use paka::sha256::sha256;
use paka::source::{Source, read};
use paka::{check, clang, foreign, gate};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

/// The foreign files read once, by path: each one's bytes, or why there are none.
type Files = BTreeMap<PathBuf, Result<Vec<u8>, String>>;

/// Releases the program at `path` into `out`, the store's key checked against the public key
/// beside the private key `key`. Progress goes to standard output; a refusal is the text for
/// standard error.
pub fn release(path: &OsStr, out: &Path, key: &Path) -> Result<(), String> {
    if fs::symlink_metadata(out).is_ok() {
        return Err(format!("release: {} already exists\n", out.display()));
    }
    let shown = Path::new(path).display();
    let (root, program) = approvals::find(Path::new(path))
        .ok_or_else(|| format!("release: no approvals at or above {shown}\n"))?;
    let (store, approved) = verified(&root, key)?;
    let name = path.to_string_lossy();
    let (source, found) = read(path);
    let checked = found.and_then(|()| load(&name, check(&name, &source), true));
    let (mut core, holes) = checked.map_err(|d| line(&source, d))?;
    let files = foreign_files(&core);
    let read = |p: &Path| {
        (files.get(p).cloned()).unwrap_or_else(|| Err(format!("{}: not read", p.display())))
    };
    let items = gate::program(&store, &root, &program, &source, &read);
    if !items.is_empty() {
        let lines: String = items.iter().map(|i| i.line() + "\n").collect();
        return Err(lines + "Revert the change, or ask the owner to run paka approve.\n");
    }
    let serial = store.serial;
    println!("gate: approvals serial {serial}, signature good, nothing pending");
    if !store.programs().contains(&program.as_str()) {
        return Err(format!("release: {program} has no approval records\n"));
    }
    let owned = store.owned(&program);
    if owned.is_empty() {
        return Err(format!("release: {program} has no human-owned example\n"));
    }
    let refusals: String = (holes.into_iter())
        .filter_map(|h| h.refusal.map(|d| line(&source, d)))
        .collect();
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let logs: Vec<_> = (core.ui_examples.iter())
        .filter(|f| owned.contains(&f.name))
        .map(|f| (f.name.clone(), ui_log(&core, f)))
        .collect();
    core.examples.retain(|e| owned.contains(&e.name));
    let functions: Vec<_> = (owned.iter())
        .filter(|n| core.examples.iter().any(|e| e.name == **n))
        .collect();
    let app = core.app.as_ref().expect("load keeps an app").name.clone();
    let mut command = format!("paka build {program} -o {app}");
    if !functions.is_empty() {
        command += " --examples";
        functions.iter().for_each(|n| command += &format!(" {n}"));
    }

    let base = out
        .file_name()
        .ok_or_else(|| format!("release: {} names no directory\n", out.display()))?;
    let parent = out.parent().unwrap_or(Path::new(""));
    let fresh = parent.join(format!(
        ".{}.release-{}",
        base.to_string_lossy(),
        process::id()
    ));
    fs::create_dir(&fresh).map_err(|e| format!("release: {}: {e}\n", fresh.display()))?;
    let mut made = Made(Some(fresh.clone()));
    let bin = fresh.join(&app);
    compile(&name, &mut core, bin.as_os_str(), Checks::All).map_err(|d| line(&source, d))?;
    println!("build: {command}");
    let hosts = hosts(&core, &fresh, &app, &read)?;
    let hashes = hashed(&fresh)?;

    let mut results = BTreeMap::new();
    let mut failed = String::new();
    for name in &functions {
        match run(&bin, name) {
            Ok(Ran::Pass) => _ = results.insert(name.to_string(), "pass".into()),
            Ok(ran) => writeln!(failed, "example {name}: {ran}").unwrap(),
            Err(e) => writeln!(failed, "example {name}: {e}").unwrap(),
        }
    }
    let mut agreed = vec![];
    for (name, log) in logs {
        let log = log.map_err(|why| format!("interpreter: {why}"));
        match log.and_then(|log| in_release(&log, |h| replay(&bin, h))) {
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

    let paka = env::current_exe()
        .and_then(fs::read)
        .map_err(|e| format!("release: the compiler: {e}\n"))?;
    let mut text = format!("program    {program}\n");
    for (name, bytes) in sources(&source, &core, &files) {
        writeln!(text, "source     {name}  sha256:{}", hex(&sha256(bytes))).unwrap();
    }
    writeln!(text, "build      {command}").unwrap();
    writeln!(text, "binary     {app}  sha256:{}", hashes[&app]).unwrap();
    for host in &hosts {
        writeln!(text, "host       {host}  sha256:{}", hashes[host]).unwrap();
    }
    let (version, clang) = (env!("CARGO_PKG_VERSION"), clang::VERSION.trim());
    let paka = hex(&sha256(&paka));
    writeln!(
        text,
        "compiler   paka {version}  sha256:{paka}  clang {clang}"
    )
    .unwrap();
    let approved = hex(&sha256(&approved));
    writeln!(text, "approvals  serial {serial}  sha256:{approved}").unwrap();
    for (name, result) in &results {
        writeln!(text, "example    {name}  {result}").unwrap();
    }
    fs::write(fresh.join("RELEASE"), &text).map_err(|e| format!("release: RELEASE: {e}\n"))?;
    fs::rename(&fresh, out).map_err(|e| format!("release: {}: {e}\n", out.display()))?;
    made.0 = Some(out.into());

    let mut expected = hashes.clone();
    expected.insert("RELEASE".into(), hex(&sha256(text.as_bytes())));
    let landed = hashed(out)?;
    let changed: String = (expected.keys().chain(landed.keys()))
        .filter(|n| expected.get(*n) != landed.get(*n))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|n| format!("artefact: {n} changed between the runs and the final path\n"))
        .collect();
    if !changed.is_empty() {
        return Err(changed);
    }
    made.0 = None;
    println!("released: {}  {app} sha256 {}", out.display(), hashes[&app]);
    Ok(())
}

/// The store in `root` and the bytes it was read from, once its `approvals.pub` is the public
/// key beside `key` and signs it.
fn verified(root: &Path, key: &Path) -> Result<(Store, Vec<u8>), String> {
    let mut public = key.as_os_str().to_owned();
    public.push(".pub");
    let public = PathBuf::from(public);
    let stored =
        fs::read(root.join("approvals.pub")).map_err(|e| format!("key: approvals.pub: {e}\n"))?;
    let mine = fs::read(&public).map_err(|e| format!("key: {}: {e}\n", public.display()))?;
    if stored != mine {
        return Err(format!(
            "key: approvals.pub differs from {}\n",
            public.display()
        ));
    }
    println!("key: approvals.pub matches {}", public.display());
    let text = fs::read(root.join("approvals")).map_err(|e| format!("approvals: {e}\n"))?;
    let store = Store::verify(root, &text, &String::from_utf8_lossy(&stored));
    Ok((store.map_err(|why| format!("{why}\n"))?, text))
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
