//! Who owns each value a native back end holds (M4-33, docs/decisions/M4-05.md section 9): locals
//! and temporaries by scope, so every exit knows what to drop. It knows nothing of the machine, so
//! the dev and the release back end share it.

use crate::core_form::{Local, Ty};

/// An owner: a local, or a temporary by its value `V` in the back end's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Owner<V> {
    Local(Local),
    Temp(V),
}

struct Scope<V> {
    is_loop: bool,
    held: Vec<(Ty, Owner<V>)>,
}

/// The scopes open at a point of a function, innermost last.
pub struct Owners<V> {
    scopes: Vec<Scope<V>>,
}

impl<V> Default for Owners<V> {
    fn default() -> Owners<V> {
        Owners { scopes: vec![] }
    }
}

impl<V: Copy + PartialEq> Owners<V> {
    pub fn enter(&mut self, is_loop: bool) {
        let held = vec![];
        self.scopes.push(Scope { is_loop, held });
    }

    /// Closes the innermost scope: what it still holds, to drop, last owned first.
    pub fn leave(&mut self) -> Vec<(Ty, Owner<V>)> {
        let scope = self.scopes.pop().expect("a scope is open");
        scope.held.into_iter().rev().collect()
    }

    pub fn own(&mut self, ty: Ty, owner: Owner<V>) {
        self.innermost().held.push((ty, owner));
    }

    /// Takes temporary `v` from its owner, innermost scope first; false when none owns it.
    pub fn disown(&mut self, v: V) -> bool {
        self.take(Owner::Temp(v))
    }

    /// Takes local `l` from the innermost scope; false when that scope does not own it.
    pub fn disown_local(&mut self, l: Local) -> bool {
        let held = &mut self.innermost().held;
        let at = held.iter().rposition(|&(_, o)| o == Owner::Local(l));
        at.map(|i| held.remove(i)).is_some()
    }

    /// Where the innermost scope's held list ends, to free a statement's temporaries after it.
    pub fn mark(&mut self) -> usize {
        self.innermost().held.len()
    }

    /// The temporaries owned since `mark` in the innermost scope, taken, last owned first.
    pub fn temps_since(&mut self, mark: usize) -> Vec<(Ty, Owner<V>)> {
        let held = &mut self.innermost().held;
        let mut temps = vec![];
        let mut i = held.len();
        while i > mark {
            i -= 1;
            if matches!(held[i].1, Owner::Temp(_)) {
                temps.push(held.remove(i));
            }
        }
        temps
    }

    /// What a `break` or `continue` drops: every scope through the innermost loop's body.
    pub fn to_loop(&self) -> Vec<(Ty, Owner<V>)> {
        let at = self.scopes.iter().rposition(|s| s.is_loop);
        let from = at.expect("a loop holds it");
        self.dropped(&self.scopes[from..], None)
    }

    /// What leaving the function drops: everything but `kept`, the local it returns.
    pub fn to_fn(&self, kept: Option<Local>) -> Vec<(Ty, Owner<V>)> {
        self.dropped(&self.scopes, kept)
    }

    /// Whether any scope owns local `l`.
    pub fn holds(&self, l: Local) -> bool {
        let mut held = self.scopes.iter().flat_map(|s| &s.held);
        held.any(|&(_, o)| o == Owner::Local(l))
    }

    fn dropped(&self, scopes: &[Scope<V>], kept: Option<Local>) -> Vec<(Ty, Owner<V>)> {
        let held = scopes.iter().rev().flat_map(|s| s.held.iter().rev());
        let kept = kept.map(Owner::Local);
        held.filter(|&&(_, o)| Some(o) != kept).copied().collect()
    }

    fn take(&mut self, owner: Owner<V>) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(i) = scope.held.iter().rposition(|&(_, o)| o == owner) {
                scope.held.remove(i);
                return true;
            }
        }
        false
    }

    fn innermost(&mut self) -> &mut Scope<V> {
        self.scopes.last_mut().expect("a scope is open")
    }
}

#[cfg(test)]
mod tests;
