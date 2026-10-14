//! The commit check on temporary git repositories.

#[allow(dead_code)] // this test uses only the git helpers
mod common;

use common::{git, put, repo};
use harness::Error;
use harness::commit::check;

fn refused(dir: &std::path::Path) -> String {
    match check(dir) {
        Err(e @ Error::Commit(_)) => {
            assert_eq!(e.status(), 20);
            e.to_string()
        }
        other => panic!("expected the commit check to fail, got {other:?}"),
    }
}

#[test]
fn the_tag_on_head_passes() {
    let dir = repo("commit-pass");
    git(&dir, &["tag", "run"]);
    check(&dir).unwrap();
}

#[test]
fn no_tag_fails() {
    assert!(refused(&repo("commit-no-tag")).contains("run"));
}

#[test]
fn the_tag_on_another_commit_fails() {
    let dir = repo("commit-moved");
    git(&dir, &["tag", "run"]);
    put(&dir, "b", "2");
    git(&dir, &["add", "b"]);
    git(&dir, &["commit", "-q", "-m", "second"]);
    assert!(refused(&dir).contains("not the commit tagged"));
}

#[test]
fn a_dirty_tree_fails() {
    let dir = repo("commit-dirty");
    git(&dir, &["tag", "run"]);
    put(&dir, "a", "changed");
    assert!(refused(&dir).contains("not clean"));
    git(&dir, &["checkout", "a"]);
    put(&dir, "untracked", "x");
    assert!(refused(&dir).contains("not clean"));
}
