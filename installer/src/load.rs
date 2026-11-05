//! Installs a rebuilt program's new code together (M4-37): code already installed under its key is
//! reused, and the caller swaps the whole group at once.

use super::install;
use std::collections::{BTreeMap, BTreeSet};
use std::io;

/// The address of each key's installed code.
pub struct Loader<K> {
    installed: BTreeMap<K, usize>,
}

impl<K> Default for Loader<K> {
    fn default() -> Self {
        Loader {
            installed: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy> Loader<K> {
    /// The (slot, address) group pointing slot `i` at the code of `keys[i]`, after installing in
    /// one mapping the code of each key not installed before. A failure installs nothing.
    pub fn group<'c>(
        &mut self,
        keys: &[K],
        code: impl Fn(&K) -> &'c [u8],
    ) -> io::Result<Vec<(usize, usize)>> {
        let mut seen = BTreeSet::new();
        let new: Vec<K> = (keys.iter().copied())
            .filter(|k| !self.installed.contains_key(k) && seen.insert(*k))
            .collect();
        let bytes: Vec<&[u8]> = new.iter().map(&code).collect();
        let at = install(&bytes)?;
        (self.installed).extend(new.into_iter().zip(at.iter().map(|c| c.address)));
        Ok(keys.iter().map(|k| self.installed[k]).enumerate().collect())
    }

    /// How many distinct pieces of code are installed.
    pub fn installed(&self) -> usize {
        self.installed.len()
    }
}
