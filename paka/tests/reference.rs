//! The language reference stays within its size limit (TASKS.md M2-18; raised by M5-41).

#[allow(dead_code)]
mod common;

use common::tokens;

#[test]
fn reference_fits_in_10500_tokens() {
    let reference = include_str!("../../spec/paka.md");
    let count = tokens(reference);
    assert!(
        count <= 10_500,
        "spec/paka.md is {count} tokens, limit 10500"
    );
}

#[test]
fn rule_rounds_up() {
    assert_eq!((tokens(""), tokens("abc"), tokens("abcd")), (0, 1, 2));
}
