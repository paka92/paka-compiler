//! Who owns each value a native back end holds (M4-33, docs/decisions/M4-05.md section 9): locals
//! and temporaries by scope, so every exit knows what to drop. It knows nothing of the machine, so
//! the dev and the release back end share it. A local read at a last use `last_use` finds moves
//! out there (M6-33): the back end makes its value a temporary and the local owns nothing until it
//! is written whole again. A local that moves in the arms of an `if` or `match` is held again at
//! the start of each arm, dropped there by an arm that never reads it, and at its end by one whose
//! read only borrowed it.

use crate::core_form::{Block, Expr, ExprKind, Local, Root, Ty};

mod last_use;

pub use last_use::{Node, visit_expr};

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
    /// The reads that move their local, and the branchings that move one in their arms.
    last_uses: last_use::Moves,
    /// Locals moved out and not written whole since: held, but owning nothing.
    moved: Vec<Local>,
}

impl<V> Default for Owners<V> {
    fn default() -> Owners<V> {
        Owners {
            scopes: vec![],
            last_uses: last_use::Moves::default(),
            moved: vec![],
        }
    }
}

impl<V: Copy + PartialEq> Owners<V> {
    /// The owners of a function whose body is `body`.
    pub fn new(body: &Block) -> Owners<V> {
        let last_uses = last_use::reads(body);
        Owners {
            last_uses,
            ..Owners::default()
        }
    }

    /// Starts arm `arm` of the `if` or `match` `e`: each local `e` moves in its arms owns again,
    /// as before the first arm, and those this arm never reads are taken, to drop.
    pub fn arm(&mut self, e: &Expr, arm: usize) -> Vec<(Ty, Owner<V>)> {
        let locals = self.branch_locals(e);
        self.moved.retain(|l| locals.iter().all(|(m, _)| m != l));
        let unread = locals.iter().filter(|(_, read)| !read[arm]);
        self.taken(unread.map(|&(l, _)| l).collect())
    }

    /// Ends an arm of `e`: each local `e` moves that the arm still holds, as when its read only
    /// borrowed, is taken, to drop, so after the arms it owns nothing on any path.
    pub fn arm_end(&mut self, e: &Expr) -> Vec<(Ty, Owner<V>)> {
        let locals = self.branch_locals(e);
        self.taken(locals.iter().map(|&(l, _)| l).collect())
    }

    fn branch_locals(&self, e: &Expr) -> Vec<(Local, Vec<bool>)> {
        let locals = self.last_uses.branches.get(&(e as *const Expr));
        locals.cloned().unwrap_or_default()
    }

    /// Those of `locals` still held, marked moved, to drop.
    fn taken(&mut self, mut locals: Vec<Local>) -> Vec<(Ty, Owner<V>)> {
        locals.retain(|&l| self.holds(l));
        let dropped = self.dropped(&self.scopes, None).into_iter();
        let dropped = dropped.filter(|&(_, o)| locals.iter().any(|&l| o == Owner::Local(l)));
        let dropped = dropped.collect();
        self.moved.extend(locals);
        dropped
    }

    pub fn enter(&mut self, is_loop: bool) {
        let held = vec![];
        self.scopes.push(Scope { is_loop, held });
    }

    /// Closes the innermost scope: what it still holds, to drop, last owned first.
    pub fn leave(&mut self) -> Vec<(Ty, Owner<V>)> {
        let scope = self.scopes.pop().expect("a scope is open");
        let dropped = self.dropped(std::slice::from_ref(&scope), None);
        let ended = |l: &Local| scope.held.iter().any(|&(_, o)| o == Owner::Local(*l));
        self.moved.retain(|l| !ended(l));
        dropped
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
        if self.moved.contains(&l) {
            return false;
        }
        let held = &mut self.innermost().held;
        let at = held.iter().rposition(|&(_, o)| o == Owner::Local(l));
        at.map(|i| held.remove(i)).is_some()
    }

    /// The local read `e` moves out, when `e` is a last use of a local that owns a value; the
    /// back end then makes that value a temporary.
    pub fn moves(&mut self, e: &Expr) -> Option<Local> {
        let l = whole_local(e).filter(|&l| self.holds(l))?;
        self.last_uses.reads.contains(&(e as *const Expr)).then(|| {
            self.moved.push(l);
            l
        })
    }

    /// Local `l` is written whole: false when it owns a value to drop first, true when it had
    /// moved out, and so owns the new value only.
    pub fn refill(&mut self, l: Local) -> bool {
        let at = self.moved.iter().position(|&m| m == l);
        at.map(|i| self.moved.swap_remove(i)).is_some()
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

    /// Whether any scope owns a value in local `l`.
    pub fn holds(&self, l: Local) -> bool {
        let mut held = self.scopes.iter().flat_map(|s| &s.held);
        !self.moved.contains(&l) && held.any(|&(_, o)| o == Owner::Local(l))
    }

    fn dropped(&self, scopes: &[Scope<V>], kept: Option<Local>) -> Vec<(Ty, Owner<V>)> {
        let held = scopes.iter().rev().flat_map(|s| s.held.iter().rev());
        let gone = |o: Owner<V>| match o {
            Owner::Local(l) => Some(l) == kept || self.moved.contains(&l),
            Owner::Temp(_) => false,
        };
        held.filter(|&&(_, o)| !gone(o)).copied().collect()
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

/// The local `e` reads whole, if it does.
pub fn whole_local(e: &Expr) -> Option<Local> {
    match &e.kind {
        ExprKind::Place(p) if p.steps.is_empty() => match p.root {
            Root::Local(l) => Some(l),
            Root::Value(_) => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
