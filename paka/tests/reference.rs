//! The language reference pages stay within their size limits (TASKS.md M2-18 and M5-41).

#[allow(dead_code)]
mod common;

use common::tokens;

#[test]
fn reference_fits_in_8000_tokens() {
    let count = tokens(include_str!("../../spec/paka.md"));
    assert!(count <= 8000, "spec/paka.md is {count} tokens, limit 8000");
}

#[test]
fn screens_fit_in_2500_tokens() {
    let count = tokens(include_str!("../../spec/screens.md"));
    assert!(
        count <= 2500,
        "spec/screens.md is {count} tokens, limit 2500"
    );
}

#[test]
fn rule_rounds_up() {
    assert_eq!((tokens(""), tokens("abc"), tokens("abcd")), (0, 1, 2));
}
