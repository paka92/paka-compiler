//! `paka gate <tree> --base <tree>` (docs/decisions/M7-01.md): what keeps the merged tree from
//! merging into its base. The key must be the base's, the store must verify with it and follow
//! the base's, every record of every gated program and every owned file must match, and every
//! human-owned example must pass in the interpreter.

use crate::approvals::Store;
use crate::pending::Item;
use crate::source::read;
use crate::{check_defs, interp};
use std::fs;
use std::path::Path;

/// One item per refusal; none when `tree` may merge into `base`.
pub fn gate(tree: &Path, base: &Path) -> Vec<Item> {
    let refused = |why: &str| vec![Item::Refused(why.into())];
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
