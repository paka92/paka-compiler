//! Drop lists written by hand: each exit drops what is open at it, last owned first.

use super::{Owner, Owners};

use Owner::{Local as L, Temp as T};

/// Types are not looked at; every owner here is of type 0.
fn names(dropped: Vec<(usize, Owner<u32>)>) -> Vec<Owner<u32>> {
    dropped.into_iter().map(|(_, o)| o).collect()
}

fn owners() -> Owners<u32> {
    let mut o = Owners::default();
    o.enter(false);
    o.own(0, L(0));
    o
}

#[test]
fn a_scope_drops_its_owners_last_first() {
    let mut o = owners();
    o.enter(false);
    o.own(0, L(1));
    o.own(0, T(7));
    assert_eq!(names(o.leave()), [T(7), L(1)]);
    assert_eq!(names(o.leave()), [L(0)]);
}

#[test]
fn a_moved_temporary_is_not_dropped() {
    let mut o = owners();
    o.own(0, T(1));
    o.own(0, T(2));
    assert!(o.disown(1));
    assert!(!o.disown(1));
    assert_eq!(names(o.leave()), [T(2), L(0)]);
}

#[test]
fn a_statement_frees_its_temporaries_and_keeps_its_locals() {
    let mut o = owners();
    let mark = o.mark();
    o.own(0, T(1));
    o.own(0, L(1));
    o.own(0, T(2));
    assert_eq!(names(o.temps_since(mark)), [T(2), T(1)]);
    assert_eq!(names(o.leave()), [L(1), L(0)]);
}

#[test]
fn a_break_drops_through_the_loop_body_only() {
    let mut o = owners();
    o.enter(true);
    o.own(0, L(1));
    o.enter(false);
    o.own(0, T(3));
    assert_eq!(names(o.to_loop()), [T(3), L(1)]);
    // Nothing is taken: the code after the exit still closes the scopes.
    assert_eq!(names(o.leave()), [T(3)]);
}

#[test]
fn a_return_drops_everything_but_the_local_it_returns() {
    let mut o = owners();
    o.enter(true);
    o.own(0, L(1));
    o.own(0, T(4));
    assert_eq!(names(o.to_fn(Some(1))), [T(4), L(0)]);
    assert_eq!(names(o.to_fn(None)), [T(4), L(1), L(0)]);
}

#[test]
fn a_block_moves_out_only_its_own_local() {
    let mut o = owners();
    o.enter(false);
    o.own(0, L(1));
    assert!(!o.disown_local(0));
    assert!(o.disown_local(1));
    assert_eq!(names(o.leave()), []);
}
