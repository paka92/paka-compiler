//! `paka gate <tree> --base <tree>` (docs/decisions/M7-01.md): what keeps the merged tree from
//! merging into its base. The key must be the base's, the store must verify with it and follow
//! the base's, every record of every gated program and every owned file must match, and every
//! human-owned example must pass in the interpreter. None of the files it reads may be a link.

use crate::approvals::Store;
use crate::pending::Item;
use crate::source::read;
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
    let read = match store.owned_files(tree) {
        Ok(owned) => owned.into_iter().chain(program_files(tree, &store)),
        Err(why) => return refused(&why),
    };
    let read = links(tree, read);
    if !read.is_empty() {
        return read;
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
        items.extend(program_items(&store, tree, program));
    }
    match store.file_changes(tree) {
        Ok(changes) => items.extend(changes.into_iter().map(Item::Change)),
        Err(why) => items.push(Item::Refused(why)),
    }
    items
}

/// The records of `program` its source no longer matches, then its human-owned examples that do
/// not pass.
fn program_items(store: &Store, tree: &Path, program: &str) -> Vec<Item> {
    let (source, found) = read(tree.join(program).as_os_str());
    let checked = found.and_then(|()| {
        let changes = store.changes(program, &source)?;
        Ok((changes, check_defs(program, &source)))
    });
    let (changes, checked) = match checked {
        Ok(found) => found,
        Err(d) => {
            let why = format!("program {program}: {}", source.locate(d).0);
            return vec![Item::Refused(why)];
        }
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

/// Each gated program path, and the `.paka` files directly in a program directory.
fn program_files(tree: &Path, store: &Store) -> Vec<String> {
    let mut files = vec![];
    for program in store.programs() {
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
