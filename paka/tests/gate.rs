//! `paka gate <tree> --base <tree>` (M7-07): each refusal docs/decisions/M7-01.md names, and a
//! merge that changes nothing approved passes. Keys are made without a passphrase in each test's
//! scratch directory.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use paka::approvals::{Record, Store, records};
use paka::source::Source;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PROGRAM: &str = "\
type Counter { count: Int = 0 }
enum Msg { Inc }

fn update(s: inout Counter, m: Msg) -> List<Effect<Msg>> {
  s.count = s.count + 1
  []
}

app counter {
  state: Counter
  start: Inc
  update: update
  grants: [Console]
}

example inc_once {
  var s = Counter {}
  _ = update(&s, Inc)
  s.count == 1
}

example starts_at_zero {
  Counter {}.count == 0
}

example agent_guess {
  Counter {}.count == 0
}
";

const WORKFLOW: &str = "on: pull_request_target\njobs: {}\n";

/// A base tree and a merged tree in one scratch directory, each holding PROGRAM, a workflow
/// file, the owner's key and the store that owns both examples but `agent_guess`, signed.
struct Trees {
    dir: Scratch,
    owner: PathBuf,
}

impl Trees {
    fn new(name: &str) -> Trees {
        let dir = Scratch::new(name);
        let owner = dir.path("owner");
        let made = Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
            .arg(&owner)
            .env_clear()
            .status()
            .unwrap();
        assert!(made.success());
        let trees = Trees { dir, owner };
        for tree in ["base", "merged"] {
            let path = trees.dir.path(tree);
            fs::create_dir_all(path.join(".github/workflows")).unwrap();
            fs::write(path.join(".github/workflows/gate.yml"), WORKFLOW).unwrap();
            fs::write(path.join("main.paka"), PROGRAM).unwrap();
            fs::copy(
                trees.owner.with_extension("pub"),
                path.join("approvals.pub"),
            )
            .unwrap();
            trees.sign(tree, &approved(1, PROGRAM));
        }
        trees
    }

    fn path(&self, file: &str) -> PathBuf {
        self.dir.path(file)
    }

    fn sign(&self, tree: &str, store: &Store) {
        store.save(&self.path(tree), &self.owner).unwrap();
    }

    /// Replaces `from` with `to` in the merged tree's `file`.
    fn edit(&self, file: &str, from: &str, to: &str) {
        let path = self.path("merged").join(file);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{from:?}");
        fs::write(path, text.replace(from, to)).unwrap();
    }

    /// The gate's standard output and exit status.
    fn gate(&self) -> (String, Option<i32>) {
        self.gate_with(&[])
    }

    fn gate_with(&self, flags: &[&str]) -> (String, Option<i32>) {
        let out = Command::new(PAKA)
            .arg("gate")
            .arg(self.path("merged"))
            .arg("--base")
            .arg(self.path("base"))
            .args(flags)
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        (text, out.status.code())
    }

    /// Asserts the gate refuses with exactly `lines`.
    fn refuses(&self, lines: &[&str]) {
        let mut expected: String = lines.iter().map(|l| format!("{l}\n")).collect();
        expected += "Revert the change, or ask the owner to run paka approve.\n";
        assert_eq!(self.gate(), (expected, Some(1)));
    }
}

/// The store with `serial` that holds `program`'s grants, the workflow file, and both examples
/// but `agent_guess`.
fn approved(serial: u64, program: &str) -> Store {
    let source = Source::file("main.paka", program.into());
    let mut records = records("main.paka", &source).unwrap();
    records.retain(|r| !matches!(r, Record::Example { name, .. } if name == "agent_guess"));
    records.push(Record::File {
        path: ".github/workflows/gate.yml".into(),
        text: WORKFLOW.into(),
    });
    Store { serial, records }
}

const PASSED: (&str, Option<i32>) = ("gate: passed\n", Some(0));

#[test]
fn an_unchanged_merge_passes() {
    let trees = Trees::new("gate_unchanged");
    assert_eq!(trees.gate(), (PASSED.0.into(), PASSED.1));
    // An agent-written example may change and fail.
    let guess = "example agent_guess {\n  Counter {}.count == ";
    trees.edit("main.paka", &format!("{guess}0"), &format!("{guess}9"));
    assert_eq!(trees.gate(), (PASSED.0.into(), PASSED.1));
}

#[test]
fn a_changed_human_owned_example_is_refused() {
    let trees = Trees::new("gate_changed_example");
    trees.edit("main.paka", "s.count == 1", "s.count >= 1");
    trees.refuses(&["example main.paka inc_once: edited"]);
}

#[test]
fn a_deleted_human_owned_example_is_refused() {
    let trees = Trees::new("gate_deleted_example");
    trees.edit(
        "main.paka",
        "example starts_at_zero {\n  Counter {}.count == 0\n}\n\n",
        "",
    );
    trees.refuses(&["example main.paka starts_at_zero: renamed or deleted"]);
}

#[test]
fn a_renamed_human_owned_example_is_refused() {
    let trees = Trees::new("gate_renamed_example");
    trees.edit(
        "main.paka",
        "example starts_at_zero",
        "example begins_at_zero",
    );
    trees.refuses(&["example main.paka starts_at_zero: renamed or deleted"]);
}

#[test]
fn a_changed_grant_list_is_refused() {
    let trees = Trees::new("gate_changed_grants");
    trees.edit("main.paka", "[Console]", "[Console, WriteFiles]");
    trees.refuses(&["grants main.paka counter: approved [Console], found [Console, WriteFiles]"]);
    trees.edit("main.paka", "[Console, WriteFiles]", "[]");
    trees.refuses(&["grants main.paka counter: approved [Console], found []"]);
}

#[test]
fn a_new_app_in_a_gated_program_is_refused() {
    let trees = Trees::new("gate_new_app");
    trees.edit("main.paka", "app counter", "app tally");
    trees.refuses(&[
        "grants main.paka counter: renamed or deleted",
        "grants main.paka tally: never approved, found [Console]",
    ]);
    // A program the store has no record of is not gated.
    fs::write(trees.path("merged/other.paka"), PROGRAM).unwrap();
    trees.edit("main.paka", "app tally", "app counter");
    assert_eq!(trees.gate(), (PASSED.0.into(), PASSED.1));
}

#[test]
fn an_app_added_to_a_program_gated_by_examples_alone_is_refused() {
    let trees = Trees::new("gate_first_app");
    let examples_only = PROGRAM.replace(
        "app counter {\n  state: Counter\n  start: Inc\n  update: update\n  grants: [Console]\n}\n\n",
        "",
    );
    let mut store = approved(2, &examples_only);
    store
        .records
        .retain(|r| !matches!(r, Record::Grants { .. }));
    for tree in ["base", "merged"] {
        fs::write(trees.path(tree).join("main.paka"), &examples_only).unwrap();
        trees.sign(tree, &store);
    }
    assert_eq!(trees.gate(), (PASSED.0.into(), PASSED.1));
    fs::write(trees.path("merged/main.paka"), PROGRAM).unwrap();
    trees.refuses(&["grants main.paka counter: never approved, found [Console]"]);
}

#[test]
fn a_missing_or_tampered_store_record_is_refused() {
    let trees = Trees::new("gate_tampered");
    let store = trees.path("merged/approvals");
    let text = fs::read_to_string(&store).unwrap();
    let not_verified = "approvals: the signature does not verify";
    fs::write(&store, text.replace("[Console]", "[Console, ReadFiles]")).unwrap();
    trees.refuses(&[not_verified]);
    let at = text.find("example main.paka starts_at_zero").unwrap();
    let end = at + text[at..].find("\n  }\n").unwrap() + 5;
    fs::write(&store, [&text[..at], &text[end..]].concat()).unwrap();
    trees.refuses(&[not_verified]);
    fs::remove_file(&store).unwrap();
    let (out, status) = trees.gate();
    assert!(out.starts_with("approvals: "), "{out}");
    assert_eq!(status, Some(1));
}

#[test]
fn failing_and_unfinished_human_owned_examples_are_refused() {
    let trees = Trees::new("gate_failing");
    trees.edit("main.paka", "s.count + 1", "s.count + 2");
    trees.refuses(&["example main.paka inc_once: fail at main.paka:19:11"]);
    trees.edit("main.paka", "s.count + 2", "??step");
    trees.refuses(&["example main.paka inc_once: unfinished at main.paka:5:13"]);
    trees.edit("main.paka", "??step", "s.count / 0");
    trees.refuses(&["example main.paka inc_once: trapped at main.paka:5:21"]);
}

#[test]
fn an_older_signed_store_put_back_is_refused() {
    let trees = Trees::new("gate_older");
    trees.sign("base", &approved(5, PROGRAM));
    trees.refuses(&["approvals: serial 1 is not above the base's 5"]);
    let mut changed = approved(5, PROGRAM);
    changed.records.pop();
    trees.sign("merged", &changed);
    trees.refuses(&[
        "approvals: serial 5 is not above the base's 5",
        "file .github/workflows/gate.yml: never approved",
    ]);
    changed.serial = 6;
    trees.sign("merged", &changed);
    trees.refuses(&["file .github/workflows/gate.yml: never approved"]);
}

#[test]
fn a_changed_key_file_is_refused() {
    let trees = Trees::new("gate_key");
    let other = trees.path("other");
    let made = Command::new("/usr/bin/ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
        .arg(&other)
        .env_clear()
        .status()
        .unwrap();
    assert!(made.success());
    let key = trees.path("merged/approvals.pub");
    fs::copy(other.with_extension("pub"), &key).unwrap();
    approved(2, PROGRAM)
        .save(&trees.path("merged"), &other)
        .unwrap();
    trees.refuses(&["approvals.pub: changed; the key is changed only by hand"]);
    fs::remove_file(&key).unwrap();
    trees.refuses(&["approvals.pub: changed; the key is changed only by hand"]);
}

#[test]
fn owned_files_are_refused_when_changed_gone_or_unrecorded() {
    let trees = Trees::new("gate_files");
    let workflows = trees.path("merged/.github/workflows");
    trees.edit(".github/workflows/gate.yml", "jobs: {}", "jobs: {pass: {}}");
    trees.refuses(&["file .github/workflows/gate.yml: edited"]);
    fs::remove_file(workflows.join("gate.yml")).unwrap();
    trees.refuses(&["file .github/workflows/gate.yml: renamed or deleted"]);
    fs::write(workflows.join("gate.yml"), WORKFLOW).unwrap();
    fs::create_dir(workflows.join("more")).unwrap();
    fs::write(workflows.join("more/second.yml"), WORKFLOW).unwrap();
    trees.refuses(&["file .github/workflows/more/second.yml: never approved"]);
    fs::remove_dir_all(workflows.join("more")).unwrap();
    fs::write(trees.path("merged/TRUSTED"), "libc system\n").unwrap();
    trees.refuses(&["file TRUSTED: never approved"]);
}

#[test]
fn a_base_without_a_key_refuses_everything() {
    let trees = Trees::new("gate_no_base_key");
    fs::remove_file(trees.path("base/approvals.pub")).unwrap();
    trees.refuses(&["approvals.pub: missing on the base"]);
}

#[test]
fn json_lists_each_item_in_the_agreed_shape() {
    let trees = Trees::new("gate_json");
    let json = || trees.gate_with(&["--json"]);
    let passed = r#"{"pending":[],"blocks":[],"next":null}"#;
    assert_eq!(json(), (format!("{passed}\n"), Some(0)));
    trees.edit("main.paka", "[Console]", "[Console, ReadFiles]");
    trees.edit("main.paka", "s.count + 1", "s.count + 2");
    trees.edit(
        "main.paka",
        "example starts_at_zero",
        "example begins_at_zero",
    );
    let workflows = trees.path("merged/.github/workflows");
    fs::write(workflows.join("second.yml"), WORKFLOW).unwrap();
    let expected = concat!(
        r#"{"pending":["#,
        r#"{"kind":"grants","program":"main.paka","app":"counter","approved":["Console"],"found":["Console","ReadFiles"]},"#,
        r#"{"kind":"example","program":"main.paka","name":"starts_at_zero","change":"renamed or deleted"},"#,
        r#"{"kind":"example","program":"main.paka","name":"inc_once","result":"fail at main.paka:19:11"},"#,
        r#"{"kind":"file","path":".github/workflows/second.yml","change":"never approved"}],"#,
        r#""blocks":["merge","release"],"#,
        r#""next":"The owner runs: paka approve. Or revert the change."}"#,
        "\n",
    );
    assert_eq!(json(), (expected.into(), Some(1)));
    fs::remove_file(trees.path("base/approvals.pub")).unwrap();
    let expected = concat!(
        r#"{"pending":[{"kind":"refused","message":"approvals.pub: missing on the base"}],"#,
        r#""blocks":["merge","release"],"#,
        r#""next":"The owner runs: paka approve. Or revert the change."}"#,
        "\n",
    );
    assert_eq!(json(), (expected.into(), Some(1)));
}
