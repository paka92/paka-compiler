//! Example results by closure hash, mode and compiler version (M7-09). Each result is one file,
//! named by its key, in a directory per compiler version in the user's cache directory; opening
//! the cache removes the other versions' directories. A result holds the example's status and
//! entry only, never a label or approval state.
//!
//! A pass entry names no position. Any other entry names positions, so it is also keyed by where
//! the definitions it reaches stand, and is not kept when one of them has no known place.

use crate::DefCheck;
use crate::hash::{Hash, closure, hex};
use crate::index::Index;
use crate::parser::split;
use crate::sha256::sha256;
use crate::source::Source;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use std::{env, fs, process};

/// `paka examples` runs examples in the interpreter only.
const MODE: &str = "interpreter";

/// Whether an example passed, and its entry.
pub type Entry = (bool, String);

/// The results at hand, and how many examples ran instead.
#[derive(Default)]
pub struct Cache {
    store: Option<Store>,
    pub runs: usize,
}

struct Store {
    /// The directory of this compiler version.
    dir: PathBuf,
    bodies: BTreeMap<String, Hash>,
    index: Index,
    /// Each definition's file, line and text from the start of its first line.
    placed: BTreeMap<String, Vec<u8>>,
}

impl Cache {
    /// The cache under the user's cache directory, for this executable; none without either.
    pub fn user(defs: &[DefCheck], source: &Source) -> Cache {
        match (user_dir(), stamp()) {
            (Some(dir), Some(stamp)) => Cache::open(dir, &stamp, defs, source),
            _ => Cache::default(),
        }
    }

    /// The cache in `dir` for the compiler `stamp` names, over a checked program. Results of any
    /// other version in `dir` are removed.
    pub fn open(dir: PathBuf, stamp: &str, defs: &[DefCheck], source: &Source) -> Cache {
        let version = hex(&sha256(stamp.as_bytes()));
        prune(&dir, &version);
        let store = Store {
            dir: dir.join(version),
            bodies: defs.iter().map(|d| (d.name.clone(), d.body)).collect(),
            index: Index::build(defs),
            placed: placed(source),
        };
        Cache {
            store: Some(store),
            runs: 0,
        }
    }

    /// The result of example `name`: the held one, else what `run` gives, which is then kept.
    pub fn result(
        &mut self,
        name: &str,
        run: impl FnOnce() -> Result<Entry, String>,
    ) -> Result<Entry, String> {
        let keys = self.store.as_ref().map(|s| (s, s.keys(name)));
        if let Some(entry) = keys.as_ref().and_then(|(s, k)| s.get(k)) {
            return Ok(entry);
        }
        self.runs += 1;
        let entry = run()?;
        if let Some((store, keys)) = keys {
            store.put(&keys, &entry);
        }
        Ok(entry)
    }
}

/// An example's key, and where the definitions it reaches stand when they all have a place.
type Keys = (Hash, Option<Hash>);

impl Store {
    fn keys(&self, name: &str) -> Keys {
        let mut names = self.index.reached(name, |n| self.bodies.contains_key(n));
        names.insert(name.into());
        let closure = hex(&closure(&names, &self.bodies));
        let key = format!("{MODE}\n{closure}");
        (sha256(key.as_bytes()), self.places(&names))
    }

    fn places(&self, names: &BTreeSet<String>) -> Option<Hash> {
        let mut text = vec![];
        for name in names {
            let placed = self.placed.get(name)?;
            text.extend(format!("{}:", placed.len()).bytes());
            text.extend(placed);
        }
        Some(sha256(&text))
    }

    /// A pass is held under its key alone; any other entry under its key and places.
    fn get(&self, keys: &Keys) -> Option<Entry> {
        let read = |passed| Some((passed, fs::read_to_string(self.file(keys, passed)?).ok()?));
        read(true).or_else(|| read(false))
    }

    /// Keeps `entry`; a failure only loses it. A temporary file renamed into place means a reader
    /// sees a whole entry or none.
    fn put(&self, keys: &Keys, (passed, entry): &Entry) {
        let Some(file) = self.file(keys, *passed) else {
            return;
        };
        let temporary = file.with_extension(process::id().to_string());
        let _ = fs::create_dir_all(&self.dir)
            .and_then(|()| fs::write(&temporary, entry))
            .and_then(|()| fs::rename(&temporary, file));
        let _ = fs::remove_file(temporary);
    }

    fn file(&self, (key, places): &Keys, passed: bool) -> Option<PathBuf> {
        let name = match passed {
            true => hex(key),
            false => format!("{}-{}", hex(key), hex(&(*places)?)),
        };
        Some(self.dir.join(name))
    }
}

/// Each definition's file name, line, and text from the start of the line it starts on, so
/// any change to a position inside it changes what it maps to.
fn placed(source: &Source) -> BTreeMap<String, Vec<u8>> {
    let mut placed = BTreeMap::new();
    for file in &source.files {
        let text = &source.text[file.bytes.clone()];
        let (mut at, mut line, mut line_start) = (0, 1, 0);
        for (name, def) in split(&file.name, text).unwrap_or_default() {
            let start = def.as_ptr() as usize - text.as_ptr() as usize;
            let newlines = text[at..start]
                .iter()
                .enumerate()
                .filter(|(_, b)| **b == b'\n');
            for (i, _) in newlines {
                line += 1;
                line_start = at + i + 1;
            }
            at = start;
            let mut key = format!("{}:{}:{line}:", file.name.len(), file.name).into_bytes();
            key.extend(&text[line_start..start + def.len()]);
            placed.insert(name.to_string(), key);
        }
    }
    placed
}

/// Removes every version directory in `dir` but `keep`. Only names a version directory can have
/// are touched; a failure leaves the directory for the next run.
fn prune(dir: &Path, keep: &str) {
    let version = |n: &str| n.len() == 64 && n.bytes().all(|b| b.is_ascii_hexdigit());
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if version(&name) && name != keep {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// `$XDG_CACHE_HOME/paka/examples`, else `$HOME/.cache/paka/examples`.
fn user_dir() -> Option<PathBuf> {
    let absolute = |v: Option<_>| v.map(PathBuf::from).filter(|p: &PathBuf| p.is_absolute());
    let base = absolute(env::var_os("XDG_CACHE_HOME"))
        .or_else(|| absolute(env::var_os("HOME")).map(|h| h.join(".cache")))?;
    Some(base.join(Path::new("paka/examples")))
}

/// The compiler version: the package version and this executable's size and change time, so a
/// rebuilt compiler never reads its predecessor's results.
fn stamp() -> Option<String> {
    let meta = fs::metadata(env::current_exe().ok()?).ok()?;
    let at = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let version = env!("CARGO_PKG_VERSION");
    Some(format!("paka {version} {} {}", meta.len(), at.as_nanos()))
}
