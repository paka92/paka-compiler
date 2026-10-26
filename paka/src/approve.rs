//! The screens of `paka approve` (docs/decisions/M7-01.md section 1.7): one item at a time, old
//! and new, in words only the toolchain writes, every character outside printable ASCII escaped.

use crate::approvals::{Change, Record, Store, records};
use crate::core_form::Program;
use crate::diagnostic::Diagnostic;
use crate::source::Source;
use crate::{DefCheck, Lowered, check_defs, interp};
use std::collections::BTreeSet;
use std::path::Path;

/// A change awaiting the owner, and its screen without the `[i/n]` in front.
pub struct Item {
    pub change: Change,
    pub screen: String,
}

/// The items awaiting the owner for `program`, a path relative to the store at `root`: its
/// changed records, then each example to `adopt` and to `drop`, then the owned files. The
/// program must check, since each example screen shows its status.
pub fn items(
    store: &Store,
    root: &Path,
    program: &str,
    source: &Source,
    adopt: &[String],
    drop: &[String],
) -> Result<Vec<Item>, Diagnostic> {
    let checked = check_defs(program, source)?;
    let refused = |why: String| Diagnostic::tool(program, "not-approvable", why);
    let mut changes: Vec<_> = (store.changes(program, source)?.into_iter())
        .map(|c| (word(&c), c))
        .collect();
    let records = records(program, source)?;
    for name in adopt {
        let found = records.iter().find(|r| example_named(r, name));
        let found = found.ok_or_else(|| refused(format!("no example {name} to adopt")))?;
        if store.owned(program).contains(name) {
            return Err(refused(format!("example {name} is human-owned already")));
        }
        let change = Change {
            approved: None,
            now: Some(found.clone()),
        };
        changes.push(("adopt", change));
    }
    for name in drop {
        let found =
            (store.records.iter()).find(|r| example_named(r, name) && r.program() == Some(program));
        let found = found.ok_or_else(|| refused(format!("example {name} is not human-owned")))?;
        let change = Change {
            approved: Some(found.clone()),
            now: None,
        };
        changes.push((DROP, change));
    }
    for change in store.file_changes(root).map_err(refused)? {
        changes.push((word(&change), change));
    }
    Ok((changes.into_iter())
        .map(|(word, change)| Item {
            screen: escaped(&screen(program, word, &change, &checked)),
            change,
        })
        .collect())
}

const DROP: &str = "drop ownership";

/// The screen of `change`, shown with `word`, in the checked `program`.
fn screen(
    program: &str,
    word: &str,
    change: &Change,
    checked: &(Lowered, Vec<DefCheck>),
) -> String {
    let ((core, ..), defs) = checked;
    match change.approved.as_ref().or(change.now.as_ref()) {
        Some(Record::Grants { app, .. }) => {
            let grants = |r: &Option<Record>| match r {
                Some(Record::Grants { grants, .. }) => Some(grants.clone()),
                _ => None,
            };
            let head = format!("{program}, app {app}, grants, {word}");
            grants_screen(&head, grants(&change.approved), grants(&change.now))
        }
        Some(Record::Example { name, .. }) => {
            let head = format!(
                "{program}, example {name}, {word}, now: {}",
                status(core, name)
            );
            let mut screen = text_screen(&head, change, word == DROP);
            let uses = uses(defs, name);
            if change.now.is_some() && !uses.is_empty() {
                screen += &format!("  uses: {uses} (not covered by this approval)\n");
            }
            screen
        }
        Some(Record::File { path, .. }) => {
            text_screen(&format!("file {path}, {word}"), change, false)
        }
        None => String::new(),
    }
}

/// The status word of example `name` run now: `pass`, `fail`, `trapped`, `unfinished` or `gone`.
fn status(core: &Program, name: &str) -> String {
    match interp::example(core, name) {
        Some(Ok((_, entry))) => entry.split([' ', '\n']).nth(1).unwrap_or_default().into(),
        Some(Err(_)) => "internal error".into(),
        None => "gone".into(),
    }
}

/// The other definitions `name` reads.
fn uses(defs: &[DefCheck], name: &str) -> String {
    let def = defs.iter().find(|d| d.name == name);
    let reads = def.into_iter().flat_map(|d| &d.reads);
    let names: Vec<_> = reads
        .filter(|r| r.as_str() != name && defs.iter().any(|d| &d.name == *r))
        .map(String::as_str)
        .collect();
    names.join(", ")
}

fn example_named(record: &Record, name: &str) -> bool {
    matches!(record, Record::Example { name: n, .. } if n == name)
}

/// The word a change is shown with.
fn word(change: &Change) -> &'static str {
    match (&change.approved, &change.now) {
        (Some(_), Some(_)) => "edited",
        (Some(_), None) => "renamed or deleted",
        _ => "new",
    }
}

/// Both lists, then each grant added or removed with its fixed meaning.
fn grants_screen(
    head: &str,
    approved: Option<BTreeSet<String>>,
    now: Option<BTreeSet<String>>,
) -> String {
    let list = |g: &Option<BTreeSet<String>>| match g {
        Some(g) => format!(
            "[{}]",
            Vec::from_iter(g.iter().map(String::as_str)).join(", ")
        ),
        None => "none".into(),
    };
    let mut screen = format!(
        "{head}\n  approved: {}   now: {}\n",
        list(&approved),
        list(&now)
    );
    let (approved, now) = (approved.unwrap_or_default(), now.unwrap_or_default());
    for (mark, grants) in [
        ('+', now.difference(&approved)),
        ('-', approved.difference(&now)),
    ] {
        for grant in grants {
            screen += &format!("  {mark} {grant}: {}\n", meaning(grant));
        }
    }
    screen
}

fn meaning(grant: &str) -> &'static str {
    match grant {
        "Console" => "may print to the terminal",
        "ReadFiles" => "may read any file you can read",
        "WriteFiles" => "may create or replace any file you can write",
        _ => "has no fixed meaning: not a grant paka knows",
    }
}

/// The whole text, old and new, with each changed line marked; unmarked when `plain`.
fn text_screen(head: &str, change: &Change, plain: bool) -> String {
    let old = lines(&change.approved);
    let new = if plain {
        old.clone()
    } else {
        lines(&change.now)
    };
    let mut screen = format!("{head}\n");
    for (mark, line) in marked(&old, &new) {
        screen += &format!("  {mark} {line}\n");
    }
    screen
}

fn lines(record: &Option<Record>) -> Vec<&str> {
    match record {
        Some(Record::Example { text, .. } | Record::File { text, .. }) => {
            text.split('\n').collect()
        }
        _ => vec![],
    }
}

/// The lines of `old` and `new` in order, each marked `' '` when in both, `'-'` when only in
/// `old`, `'+'` when only in `new`, through their longest common run of lines.
fn marked<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(char, &'a str)> {
    let (n, m) = (old.len(), new.len());
    // longest[i][j]: the longest common run of old[i..] and new[j..].
    let mut longest = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            longest[i][j] = match old[i] == new[j] {
                true => longest[i + 1][j + 1] + 1,
                false => longest[i + 1][j].max(longest[i][j + 1]),
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, vec![]);
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            out.push((' ', old[i]));
            (i, j) = (i + 1, j + 1);
        } else if i < n && (j == m || longest[i + 1][j] >= longest[i][j + 1]) {
            out.push(('-', old[i]));
            i += 1;
        } else {
            out.push(('+', new[j]));
            j += 1;
        }
    }
    out
}

/// `text` with every character outside printable ASCII but the line feed as `\u{hex}`, so none
/// can hide a line, move the cursor or pass for another.
pub fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' | ' '..='~' => out.push(c),
            c => out += &format!("\\u{{{:x}}}", c as u32),
        }
    }
    out
}
