//! The approval store (M7-04) and the labels `paka examples` takes from it (M7-05). Keys are made
//! without a passphrase in each test's scratch directory.

#[allow(dead_code)]
mod common;

use common::{EXAMPLE_CACHE, PAKA, Scratch};
use paka::approvals::{Foreign, Record, Store, records};
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

// Counting from zero.
example inc_once {
  var s = Counter {}
  _ = update(&s, Inc)
  s.count == 1
}

example starts_at_zero {
  Counter {}.count == 0
}
";

/// A new key pair named `name` in `dir`: the private key's path.
fn key(dir: &Scratch, name: &str) -> PathBuf {
    let path = dir.path(name);
    let made = Command::new("/usr/bin/ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
        .arg(&path)
        .env_clear()
        .status()
        .unwrap();
    assert!(made.success());
    path
}

/// A scratch directory holding PROGRAM as `main.paka`, its owner's key as `approvals.pub`, and
/// the store of `records` with `serial`, signed by that key.
fn approved(name: &str, serial: u64, records: Vec<Record>) -> (Scratch, Store) {
    let dir = Scratch::new(name);
    fs::write(dir.path("main.paka"), PROGRAM).unwrap();
    let owner = key(&dir, "owner");
    fs::copy(owner.with_extension("pub"), dir.path("approvals.pub")).unwrap();
    let store = Store { serial, records };
    store.save(&dir.path(""), &owner).unwrap();
    (dir, store)
}

fn open(dir: &Scratch) -> Result<Store, String> {
    Store::open(&dir.path(""), &dir.path("approvals.pub"))
}

fn source(text: &str) -> Source {
    Source::file("main.paka", text.into())
}

fn current() -> Vec<Record> {
    records("main.paka", &source(PROGRAM)).unwrap()
}

#[test]
fn a_recorded_approval_verifies() {
    let (dir, store) = approved("verifies", 1, current());
    let opened = open(&dir).unwrap();
    assert_eq!(opened, store);
    assert_eq!(opened.changes("main.paka", &source(PROGRAM)).unwrap(), []);
    let example = r#"// Counting from zero.
example inc_once {
  var s = Counter {}
  _ = update(&s, Inc)
  s.count == 1
}"#;
    let wanted = Record::Example {
        program: "main.paka".into(),
        name: "inc_once".into(),
        text: example.into(),
    };
    assert!(store.records.contains(&wanted), "{:?}", store.records);
}

#[test]
fn an_edited_example_is_detected() {
    let (dir, _) = approved("edited_example", 1, current());
    let edited = PROGRAM.replace("s.count == 1", "s.count == 2");
    let store = open(&dir).unwrap();
    let changes = store.changes("main.paka", &source(&edited)).unwrap();
    assert_eq!(changes.len(), 1);
    let approved = &changes[0].approved;
    assert!(matches!(approved, Some(Record::Example { name, .. }) if name == "inc_once"));
    let now = changes[0].now.as_ref();
    assert!(matches!(now, Some(Record::Example { text, .. }) if text.contains("s.count == 2")));
}

#[test]
fn a_deleted_example_is_detected() {
    let (dir, _) = approved("deleted_example", 1, current());
    let deleted = PROGRAM.replace("example starts_at_zero", "example renamed");
    let store = open(&dir).unwrap();
    let changes = store.changes("main.paka", &source(&deleted)).unwrap();
    assert_eq!(changes.len(), 1);
    assert!(
        matches!(&changes[0].approved, Some(Record::Example { name, .. }) if name == "starts_at_zero")
    );
    assert_eq!(changes[0].now, None);
}

#[test]
fn an_edited_grant_list_is_detected() {
    let (dir, _) = approved("edited_grants", 1, current());
    let store = open(&dir).unwrap();
    let edited = PROGRAM.replace("[Console]", "[Console, ReadFiles]");
    let changes = store.changes("main.paka", &source(&edited)).unwrap();
    assert_eq!(changes.len(), 1);
    let found = match &changes[0].now {
        Some(Record::Grants { grants, .. }) => grants.iter().map(String::as_str).collect(),
        _ => vec![],
    };
    assert_eq!(found, ["Console", "ReadFiles"]);
    let reordered = PROGRAM.replace("[Console]", "[Console, ReadFiles]");
    let reordered = reordered.replace("[Console, ReadFiles]", "[ReadFiles, Console]");
    assert_eq!(
        store
            .changes("main.paka", &source(&reordered))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_tampered_record_is_refused() {
    let (dir, _) = approved("tampered", 1, current());
    let text = fs::read_to_string(dir.path("approvals")).unwrap();
    let tampered = text.replace("[Console]", "[Console, WriteFiles]");
    assert_ne!(text, tampered);
    fs::write(dir.path("approvals"), tampered).unwrap();
    let refused = open(&dir).unwrap_err();
    assert_eq!(refused, "approvals: the signature does not verify");
}

#[test]
fn a_store_signed_by_another_key_is_refused() {
    let (dir, store) = approved("other_key", 1, current());
    let other = key(&dir, "other");
    store.save(&dir.path(""), &other).unwrap();
    assert!(open(&dir).is_err());
    fs::remove_file(dir.path("approvals.sig")).unwrap();
    assert!(open(&dir).is_err(), "a store with no signature verifies");
}

#[test]
fn an_older_signed_store_put_back_is_refused() {
    let (dir, old) = approved("older", 7, current());
    let owner = dir.path("owner");
    let mut records = current();
    records.pop();
    let new = Store { serial: 8, records };
    new.save(&dir.path(""), &owner).unwrap();
    assert!(open(&dir).unwrap().follows(&old));
    old.save(&dir.path(""), &owner).unwrap();
    let put_back = open(&dir).unwrap();
    assert!(
        !put_back.follows(&new),
        "an older store replaced a newer one"
    );
    let same_serial = Store {
        serial: 8,
        records: vec![],
    };
    assert!(!same_serial.follows(&new) && new.follows(&new));
}

#[test]
fn every_record_kind_reads_back() {
    let store = Store {
        serial: 3,
        records: vec![
            Record::Grants {
                program: "examples/sum".into(),
                app: "sum".into(),
                grants: ["Console".into(), "Sum".into()].into(),
                foreign: vec![Foreign {
                    text: "foreign Sum {\n  source: \"sum.c\"\n}".into(),
                    files: vec![("sum.c".into(), "ab".repeat(32))],
                }],
            },
            Record::Grants {
                program: "examples/none".into(),
                app: "none".into(),
                grants: [].into(),
                foreign: vec![],
            },
            Record::File {
                path: ".github/workflows/gate.yml".into(),
                text: "on: push\n\n  indented\n".into(),
            },
        ],
    };
    let text = store.text();
    let expected = "\
serial 3
grants examples/sum sum [Console, Sum]
  foreign 3
    foreign Sum {
      source: \"sum.c\"
    }
  sha256 sum.c abababababababababababababababababababababababababababababababab
grants examples/none none []
file .github/workflows/gate.yml 4
  on: push

    indented

";
    assert_eq!(text, expected);
    assert_eq!(Store::parse(&text).unwrap(), store);
    for broken in ["serial x\n", "serial 1\nexample a b 2\n  one\n", "serial 1"] {
        assert!(Store::parse(broken).is_err(), "{broken:?}");
    }
}

/// `paka examples` on the program in `dir`: standard output and error.
fn examples(dir: &Scratch) -> (String, String) {
    let out = Command::new(PAKA)
        .args(["examples", "main.paka"])
        .env("XDG_CACHE_HOME", EXAMPLE_CACHE)
        .current_dir(dir.path(""))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = |b| String::from_utf8(b).unwrap();
    (text(out.stdout), text(out.stderr))
}

fn owning(program: &str, name: &str) -> Vec<Record> {
    let owned = |r: &Record| matches!(r, Record::Example { name: n, .. } if n == name);
    (current().into_iter().filter(owned))
        .map(|r| match r {
            Record::Example { name, text, .. } => Record::Example {
                program: program.into(),
                name,
                text,
            },
            r => r,
        })
        .collect()
}

const AGENT_WRITTEN: &str = "\
inc_once agent-written pass
starts_at_zero agent-written pass
";

#[test]
fn the_listing_labels_from_the_store() {
    let (dir, _) = approved("labels", 1, owning("main.paka", "inc_once"));
    let expected = "\
inc_once human-owned pass
starts_at_zero agent-written pass
";
    assert_eq!(examples(&dir), (expected.into(), "".into()));
    // An edited example still reads as owned: the label is that a record exists.
    let edited = PROGRAM.replace("// Counting from zero.\n", "");
    fs::write(dir.path("main.paka"), edited).unwrap();
    assert_eq!(examples(&dir).0, expected);
}

#[test]
fn an_unapproved_example_never_reads_as_human_owned() {
    let none = Scratch::new("no_store");
    fs::write(none.path("main.paka"), PROGRAM).unwrap();
    assert_eq!(examples(&none), (AGENT_WRITTEN.into(), "".into()));

    let (other, _) = approved("other_program", 1, owning("other.paka", "inc_once"));
    assert_eq!(examples(&other), (AGENT_WRITTEN.into(), "".into()));

    let (unsigned, _) = approved("unsigned", 1, vec![]);
    let text = Store {
        serial: 2,
        records: owning("main.paka", "inc_once"),
    };
    fs::write(unsigned.path("approvals"), text.text()).unwrap();
    let refused =
        "approvals: the signature does not verify; every example reads as agent-written\n";
    assert_eq!(examples(&unsigned), (AGENT_WRITTEN.into(), refused.into()));

    let (stranger, store) = approved("stranger_key", 1, owning("main.paka", "inc_once"));
    store
        .save(&stranger.path(""), &key(&stranger, "other"))
        .unwrap();
    assert_eq!(examples(&stranger), (AGENT_WRITTEN.into(), refused.into()));
}
