//! `paka gate <tree> --base <tree>` (docs/decisions/M7-01.md): what keeps the merged tree from
//! merging into its base. The key must be the base's, the store must verify with it and follow
//! the base's, every record of every gated program and every owned file must match, and every
//! human-owned example must pass in the interpreter.

use crate::approvals::{Change, Record, Store};
use crate::source::read;
use crate::{check_defs, interp};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// One line per refusal; none when `tree` may merge into `base`.
pub fn gate(tree: &Path, base: &Path) -> Vec<String> {
    let key = base.join("approvals.pub");
    let Ok(base_key) = fs::read(&key) else {
        return vec!["approvals.pub: missing on the base".into()];
    };
    if fs::read(tree.join("approvals.pub")).ok() != Some(base_key) {
        return vec!["approvals.pub: changed; the key is changed only by hand".into()];
    }
    let store = match Store::open(tree, &key) {
        Ok(store) => store,
        Err(why) => return vec![why],
    };
    let mut refused = vec![];
    if base.join("approvals").exists() {
        match Store::open(base, &key) {
            Ok(old) if !store.follows(&old) => refused.push(format!(
                "approvals: serial {} is not above the base's {}",
                store.serial, old.serial
            )),
            Ok(_) => {}
            Err(why) => refused.push(format!("base: {why}")),
        }
    }
    for program in store.programs() {
        refused.extend(program_refusals(&store, tree, program));
    }
    match store.file_changes(tree) {
        Ok(changes) => refused.extend(changes.iter().map(line)),
        Err(why) => refused.push(why),
    }
    refused
}

/// The records of `program` its source no longer matches, then its human-owned examples that do
/// not pass.
fn program_refusals(store: &Store, tree: &Path, program: &str) -> Vec<String> {
    let (source, found) = read(tree.join(program).as_os_str());
    let checked = found.and_then(|()| {
        let changes = store.changes(program, &source)?;
        Ok((changes, check_defs(program, &source)))
    });
    let (changes, checked) = match checked {
        Ok(found) => found,
        Err(d) => return vec![format!("program {program}: {}", source.locate(d).0)],
    };
    let mut refused: Vec<_> = changes.iter().map(line).collect();
    for name in store.owned(program) {
        let failed = match &checked {
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
        refused.push(format!("example {program} {name}: {failed}"));
    }
    refused
}

/// A change as one line: kind, program and name, then what differs.
fn line(change: &Change) -> String {
    let record = change.approved.as_ref().or(change.now.as_ref());
    let what = match (&change.approved, &change.now) {
        (Some(Record::Grants { grants: a, .. }), Some(Record::Grants { grants: b, .. })) => {
            format!("approved {}, found {}", list(a), list(b))
        }
        (None, Some(Record::Grants { grants, .. })) => {
            format!("never approved, found {}", list(grants))
        }
        (None, _) => "never approved".into(),
        (Some(_), Some(_)) => "edited".into(),
        (Some(_), None) => "renamed or deleted".into(),
    };
    match record {
        Some(Record::Grants { program, app, .. }) => format!("grants {program} {app}: {what}"),
        Some(Record::Example { program, name, .. }) => format!("example {program} {name}: {what}"),
        Some(Record::File { path, .. }) => format!("file {path}: {what}"),
        None => what,
    }
}

fn list(grants: &BTreeSet<String>) -> String {
    let names: Vec<_> = grants.iter().map(String::as_str).collect();
    format!("[{}]", names.join(", "))
}
