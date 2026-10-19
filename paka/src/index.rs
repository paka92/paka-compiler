//! Who uses what (M4-10): for every name a definition read, the definitions that read it,
//! including names that name nothing yet.

use crate::DefCheck;
use crate::resolve::near;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, PartialEq)]
pub struct Index {
    users: BTreeMap<String, BTreeSet<String>>,
    reads: BTreeMap<String, BTreeSet<String>>,
}

impl Index {
    pub fn build(defs: &[DefCheck]) -> Index {
        let mut index = Index::default();
        defs.iter()
            .for_each(|d| index.set(&d.name, d.reads.clone()));
        index
    }

    /// The definitions that read `name`.
    pub fn users(&self, name: &str) -> impl Iterator<Item = &str> {
        self.users
            .get(name)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    /// Replaces what `user` reads, as after it is rechecked.
    pub fn set(&mut self, user: &str, reads: BTreeSet<String>) {
        self.remove(user);
        for name in &reads {
            let users = self.users.entry(name.clone()).or_default();
            users.insert(user.into());
        }
        self.reads.insert(user.into(), reads);
    }

    pub fn remove(&mut self, user: &str) {
        for name in self.reads.remove(user).into_iter().flatten() {
            let users = self.users.get_mut(&name).expect("every read has its users");
            users.remove(user);
            if users.is_empty() {
                self.users.remove(&name);
            }
        }
    }

    /// The users to recheck when `names` start to name something: those that read one, and
    /// those of a name that names nothing and could now be suggested one of them.
    pub fn waiting(&self, names: &[&str]) -> BTreeSet<String> {
        let dangling = (self.users.keys()).filter(|r| !self.reads.contains_key(*r));
        let near = dangling.filter(|r| names.iter().any(|n| near(r, n).is_some()));
        let read = names.iter().copied().chain(near.map(String::as_str));
        read.flat_map(|n| self.users(n)).map(Into::into).collect()
    }
}
