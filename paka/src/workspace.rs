//! A program kept as definitions and rechecked after each edit with early cutoff (M4-12).
//!
//! An edit rechecks the edited definition when its body hash changed, the users of every name
//! that appeared or went, and the direct users of every definition whose interface hash
//! changed. The checker still runs over the whole program; only the rechecked definitions take
//! its results. One exception: lowering runs only on a program without errors and adds reads,
//! so when the program turns clean or broken every definition takes its fresh reads.
//!
//! The last version that checked clean stays released, to run, until a newer one checks clean;
//! the edits between are released together (M4-13).

use crate::diagnostic::Diagnostic;
use crate::index::Index;
use crate::parser::{parse_def, split};
use crate::source::Source;
use crate::{DefCheck, check_each};
use std::collections::{BTreeMap, BTreeSet};

const PATH: &str = "main.paka";

pub struct Workspace {
    defs: Vec<Stored>,
    index: Index,
    released: Option<String>,
}

struct Stored {
    /// The names it defines, its own first.
    names: Vec<String>,
    text: String,
    check: DefCheck,
    /// Its first line when `check` was made, to move a kept error with it.
    line: usize,
}

pub struct Recheck {
    pub rechecked: BTreeSet<String>,
    /// The rechecked definitions with an error.
    pub broken: Vec<Diagnostic>,
}

impl Workspace {
    pub fn new(text: &[u8]) -> Result<Workspace, Diagnostic> {
        let texts = split(PATH, text)?.into_iter();
        let texts: Vec<String> = texts
            .map(|(_, t)| String::from_utf8_lossy(t).into())
            .collect();
        let checks = check(&texts)?;
        let index = Index::build(&checks);
        let released = clean(&checks).then(|| program(texts.iter().map(String::as_str)));
        let lines = first_lines(&texts);
        let defs = (texts.into_iter().zip(checks).zip(lines))
            .map(|((text, check), line)| Stored {
                names: names(&text),
                text,
                check,
                line,
            })
            .collect();
        Ok(Workspace {
            defs,
            index,
            released,
        })
    }

    pub fn text(&self) -> String {
        program(self.defs.iter().map(|d| d.text.as_str()))
    }

    /// The text of the last version without errors.
    pub fn released(&self) -> Option<&str> {
        self.released.as_deref()
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Each definition's names, its own first, and its text.
    pub fn defs(&self) -> impl Iterator<Item = (&[String], &str)> {
        (self.defs.iter()).map(|d| (d.names.as_slice(), d.text.as_str()))
    }

    /// The definition named `name`: its text, and its result with an error placed from the text's
    /// first line.
    pub fn def(&self, name: &str) -> Option<(&str, DefCheck)> {
        let d = self.defs.iter().find(|d| d.names[0] == name)?;
        Some((&d.text, moved(&d.check, d.line, 1)))
    }

    /// Each definition's result, an error placed where the definition now stands.
    pub fn checks(&self) -> Vec<DefCheck> {
        let lines = first_lines(&self.defs.iter().map(|d| &d.text).collect::<Vec<_>>());
        (self.defs.iter().zip(lines))
            .map(|(d, line)| moved(&d.check, d.line, line))
            .collect()
    }

    /// Replaces the definition named `name` by `new`, adds `new` when none has that name, or
    /// deletes it when `new` is `None`. A program-wide error leaves the workspace as it was.
    pub fn edit(&mut self, name: &str, new: Option<&str>) -> Result<Recheck, Diagnostic> {
        let at = self.defs.iter().position(|d| d.names[0] == name);
        let new_names = match new {
            Some(text) => parse_def(PATH, text.as_bytes())?.names(),
            None => vec![],
        };
        let edited = new_names.first().copied();
        let mut texts: Vec<_> = self.defs.iter().map(|d| d.text.as_str()).collect();
        match (at, new) {
            (Some(i), Some(text)) => texts[i] = text,
            (Some(i), None) => {
                texts.remove(i);
            }
            (None, Some(text)) => texts.push(text),
            (None, None) => {}
        }
        let fresh = check(&texts)?;
        let old_names: BTreeSet<&str> = at.map_or(vec![], |i| self.names(i)).into_iter().collect();
        let new_names: BTreeSet<&str> = new_names.into_iter().collect();
        let rechecked = self.rechecked(&fresh, &old_names, &new_names, edited);
        let texts: Vec<String> = texts.into_iter().map(String::from).collect();
        let was_clean = self.defs.iter().all(|d| d.check.error.is_none());
        let is_clean = clean(&fresh);
        self.update(texts, fresh, &rechecked, was_clean != is_clean);
        if is_clean {
            self.released = Some(self.text());
        }
        let broken = (self.checks().into_iter())
            .filter(|c| rechecked.contains(&c.name))
            .filter_map(|c| c.error)
            .collect();
        Ok(Recheck { rechecked, broken })
    }

    fn names(&self, i: usize) -> Vec<&str> {
        self.defs[i].names.iter().map(String::as_str).collect()
    }

    fn rechecked(
        &self,
        fresh: &[DefCheck],
        old_names: &BTreeSet<&str>,
        new_names: &BTreeSet<&str>,
        edited: Option<&str>,
    ) -> BTreeSet<String> {
        let old: BTreeMap<_, _> = (self.defs.iter())
            .map(|d| (d.check.name.as_str(), &d.check))
            .collect();
        let gone: Vec<_> = old_names.difference(new_names).copied().collect();
        let appeared: Vec<_> = new_names.difference(old_names).copied().collect();
        let mut wake = self.index.waiting(&appeared);
        wake.extend(
            gone.iter()
                .flat_map(|n| self.index.users(n))
                .map(String::from),
        );
        for def in fresh {
            let was = old.get(def.name.as_str());
            if was.is_some_and(|w| w.interface != def.interface) {
                wake.extend(self.index.users(&def.name).map(String::from));
            }
            if Some(def.name.as_str()) == edited && was.is_none_or(|w| w.body != def.body) {
                wake.insert(def.name.clone());
            }
        }
        let present: BTreeSet<_> = fresh.iter().map(|d| d.name.as_str()).collect();
        wake.retain(|n| present.contains(n.as_str()));
        wake
    }

    /// Takes the fresh results of the rechecked and the reworded definitions, and the fresh
    /// hashes of all; the fresh reads of all when `relowered`.
    fn update(
        &mut self,
        texts: Vec<String>,
        fresh: Vec<DefCheck>,
        rechecked: &BTreeSet<String>,
        relowered: bool,
    ) {
        let lines = first_lines(&texts);
        let mut kept: BTreeMap<_, _> = (std::mem::take(&mut self.defs).into_iter())
            .map(|d| (d.check.name.clone(), d))
            .collect();
        for name in kept.keys().filter(|n| !fresh.iter().any(|f| &f.name == *n)) {
            self.index.remove(name);
        }
        for ((text, check), line) in texts.into_iter().zip(fresh).zip(lines) {
            let old = kept.remove(&check.name);
            let old = old.filter(|o| o.text == text && !rechecked.contains(&check.name));
            let stored = match old {
                Some(old) if !relowered => {
                    let check = DefCheck {
                        body: check.body,
                        interface: check.interface,
                        ..old.check
                    };
                    Stored { check, ..old }
                }
                Some(old) => {
                    self.index.set(&check.name, check.reads.clone());
                    let check = DefCheck {
                        error: old.check.error,
                        ..check
                    };
                    Stored { check, ..old }
                }
                None => {
                    self.index.set(&check.name, check.reads.clone());
                    Stored {
                        names: names(&text),
                        text,
                        check,
                        line,
                    }
                }
            };
            self.defs.push(stored);
        }
    }
}

fn check(texts: &[impl AsRef<str>]) -> Result<Vec<DefCheck>, Diagnostic> {
    let text = program(texts.iter().map(AsRef::as_ref));
    check_each(PATH, &Source::file(PATH, text.into_bytes()))
}

fn clean(checks: &[DefCheck]) -> bool {
    checks.iter().all(|c| c.error.is_none())
}

fn program<'t>(texts: impl Iterator<Item = &'t str>) -> String {
    texts.collect::<Vec<_>>().join("\n\n") + "\n"
}

/// Each definition's first line in the program of `texts`.
fn first_lines(texts: &[impl AsRef<str>]) -> Vec<usize> {
    let mut line = 1;
    let mut lines = vec![];
    for text in texts {
        lines.push(line);
        line += text.as_ref().lines().count() + 1;
    }
    lines
}

fn names(text: &str) -> Vec<String> {
    let def = parse_def(PATH, text.as_bytes()).expect("a stored definition parses");
    def.names().into_iter().map(String::from).collect()
}

/// `check` of a definition that moved from line `from` to line `to`.
fn moved(check: &DefCheck, from: usize, to: usize) -> DefCheck {
    let mut check = check.clone();
    if let Some(d) = &mut check.error {
        d.position = d.position.map(|(line, col)| (line - from + to, col));
    }
    check
}
