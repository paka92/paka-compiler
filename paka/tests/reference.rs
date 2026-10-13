//! The language reference stays within its size limit (TASKS.md M2-18).

/// The one counting rule: a token is three bytes, rounded up.
fn tokens(text: &str) -> usize {
    text.len().div_ceil(3)
}

#[test]
fn reference_fits_in_8000_tokens() {
    let reference = include_str!("../../spec/paka.md");
    let count = tokens(reference);
    assert!(count <= 8000, "spec/paka.md is {count} tokens, limit 8000");
}

#[test]
fn rule_rounds_up() {
    assert_eq!((tokens(""), tokens("abc"), tokens("abcd")), (0, 1, 2));
}
