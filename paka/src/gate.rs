//! `paka gate <tree> --base <tree>` (docs/decisions/M7-01.md): what keeps the merged tree from
//! merging into its base. The key must be the base's, the store must verify with it and follow
//! the base's, every record of every gated program and every owned file must match, and every
//! human-owned example must pass in the interpreter. None of the files it reads may be a link.
//! `paka release` gates one program the same way, with no base (docs/decisions/M7-03.md).

use crate::approvals::{Store, records_with};
use crate::diagnostic::Diagnostic;
use crate::foreign::{Read, read_regular};
use crate::pending::Item;
use crate::source::{Source, read};
use crate::{check_defs, interp};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const STORE: [&str; 3] = ["approvals", "approvals.sig", "approvals.pub"];
const OWNED: [&str; 2] = [".github/workflows", "TRUSTED"];

/// One item per refusal; none when `tree` may merge into `base`.
pub fn gate(tree: &Path, base: &Path) -> Vec<Item> {
    let refused = |why: &str| vec![Item::Refused(why.into())];
    let fixed = links(tree, STORE.iter().chain(&OWNED).map(|p| p.to_string()));
    if !fixed.is_empty() {
        return fixed;
    }
    let key = base.join("approvals.pub");
    let Ok(base_key) = fs::read(&key) else {
        return refused("approvals.pub: missing on the base");
    };
    if fs::read(tree.join("approvals.pub")).ok() != Some(base_key) {
        return refused("approvals.pub: changed; the key is changed only by hand");
    }
    let store = match Store::open(tree, &key) {
        Ok(store) => store,
        Err(why) => return refused(&why),
    };
    let linked = read_links(&store, tree, &store.programs());
    if !linked.is_empty() {
        return linked;
    }
    let mut items = vec![];
    if base.join("approvals").exists() {
        match Store::open(base, &key) {
            Ok(old) if !store.follows(&old) => items.push(Item::Refused(format!(
                "approvals: serial {} is not above the base's {}",
                store.serial, old.serial
            ))),
            Ok(_) => {}
            Err(why) => items.push(Item::Refused(format!("base: {why}"))),
        }
    }
    for program in store.programs() {
        let (source, found) = read(tree.join(program).as_os_str());
        items.extend(match found {
            Ok(()) => program_items(&store, program, &source, &read_regular),
            Err(d) => vec![unreadable(program, &source, d)],
        });
    }
    items.extend(file_items(&store, tree));
    items
}

/// The gate for `program` alone with no base: `source` is the program as read once, at a path
/// relative to `root`, the directory of `store`, which the caller verified; `read` gives its
/// foreign files.
pub fn program(
    store: &Store,
    root: &Path,
    program: &str,
    source: &Source,
    read: &Read,
) -> Vec<Item> {
    let fixed = links(root, STORE.iter().chain(&OWNED).map(|p| p.to_string()));
    if !fixed.is_empty() {
        return fixed;
    }
    let linked = read_links(store, root, &[program]);
    if !linked.is_empty() {
        return linked;
    }
    let mut items = match store.programs().contains(&program) {
        true => program_items(store, program, source, read),
        false => vec![],
    };
    items.extend(file_items(store, root));
    items
}

/// One refusal per link among the owned files, `programs` and their `.paka` files.
fn read_links(store: &Store, tree: &Path, programs: &[&str]) -> Vec<Item> {
    match store.owned_files(tree) {
        Ok(owned) => links(tree, owned.into_iter().chain(program_files(tree, programs))),
        Err(why) => vec![Item::Refused(why)],
    }
}

/// Each owned file that differs from its record.
fn file_items(store: &Store, tree: &Path) -> Vec<Item> {
    match store.file_changes(tree) {
        Ok(changes) => changes.into_iter().map(Item::Change).collect(),
        Err(why) => vec![Item::Refused(why)],
    }
}

fn unreadable(program: &str, source: &Source, d: Diagnostic) -> Item {
    Item::Refused(format!("program {program}: {}", source.locate(d).0))
}

/// The records of `program` its source no longer matches, then its human-owned examples that do
/// not pass.
fn program_items(store: &Store, program: &str, source: &Source, read: &Read) -> Vec<Item> {
    let checked = records_with(program, source, read).map(|now| {
        let changes = store.compare(program, now);
        (changes, check_defs(program, source))
    });
    let (changes, checked) = match checked {
        Ok(found) => found,
        Err(d) => return vec![unreadable(program, source, d)],
    };
    let mut items: Vec<_> = changes.into_iter().map(Item::Change).collect();
    for name in store.owned(program) {
        let result = match &checked {
            Err(d) => format!("not run: {}", source.locate(d.clone()).0),
            Ok(((core, ..), _)) => match interp::example(core, &name) {
                None | Some(Ok((true, _))) => continue,
                Some(Ok((false, entry))) => {
                    let first = entry.lines().next().unwrap_or_default();
                    first
                        .strip_prefix(&name)
                        .unwrap_or(first)
                        .trim_start()
                        .into()
                }
                Some(Err(why)) => format!("internal error: {why}"),
            },
        };
        let program = program.into();
        items.push(Item::Failing {
            program,
            name,
            result,
        });
    }
    items
}

/// Each program path, and the `.paka` files directly in a program directory.
fn program_files(tree: &Path, programs: &[&str]) -> Vec<String> {
    let mut files = vec![];
    for program in programs {
        files.push(program.to_string());
        let Ok(entries) = fs::read_dir(tree.join(program)) else {
            continue;
        };
        for name in entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
        {
            if name.ends_with(".paka") {
                files.push(format!("{program}/{name}"));
            }
        }
    }
    files
}

/// One refusal per symbolic link among `paths` and the directories on them, so the gate reads
/// only what the tree itself holds.
fn links(tree: &Path, paths: impl IntoIterator<Item = String>) -> Vec<Item> {
    let mut found = BTreeSet::new();
    for path in paths {
        let mut at = tree.to_path_buf();
        for part in Path::new(&path).components() {
            at.push(part);
            if fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink()) {
                found.insert(at.strip_prefix(tree).unwrap_or(&at).display().to_string());
                break;
            }
        }
    }
    let refuse = |path| Item::Refused(format!("{path}: a symbolic link; the gate follows none"));
    found.into_iter().map(refuse).collect()
}
