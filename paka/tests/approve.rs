//! `paka approve` (M7-06): the screens for a hand-written set of example and grant changes, two
//! of them differing only in one expected value, and the answers that sign a new store. The
//! command reads answers from a terminal only, so the walk-through runs it under `script`, which
//! gives it one. Keys are made without a passphrase in each test's scratch directory.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use paka::approvals::{Record, Store, records};
use paka::source::Source;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

// Caf\u{e9} \u{202e}reversed
example greets {
  \"caf\u{e9}\" == \"caf\u{e9}\"
}
";

/// PROGRAM as `main.paka` beside the owner's key as `approvals.pub` and a store, signed, that
/// holds its grants and owns `inc_once` and `starts_at_zero`; and the private key's path.
fn approved(name: &str) -> (Scratch, PathBuf) {
    let dir = Scratch::new(name);
    let key = dir.path("owner");
    let made = Command::new("/usr/bin/ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
        .arg(&key)
        .env_clear()
        .status()
        .unwrap();
    assert!(made.success());
    fs::copy(key.with_extension("pub"), dir.path("approvals.pub")).unwrap();
    fs::write(dir.path("main.paka"), PROGRAM).unwrap();
    let source = Source::file("main.paka", PROGRAM.into());
    let mut records = records("main.paka", &source).unwrap();
    records.retain(|r| !matches!(r, Record::Example { name, .. } if name == "greets"));
    let store = Store { serial: 4, records };
    store.save(&dir.path(""), &key).unwrap();
    (dir, key)
}

/// The two examples and the grant list each changed in one place.
fn edited() -> String {
    (PROGRAM.replace("s.count == 1", "s.count == 2"))
        .replace("Counter {}.count == 0", "Counter {}.count == 1")
        .replace("[Console]", "[Console, ReadFiles]")
}

/// `paka approve main.paka` with `flags` on a terminal, answering each item in turn: its output
/// with the terminal's carriage returns removed, and its exit status. Answers are sent once the
/// first item is shown, after the terminal's echo is off.
fn approve(dir: &Scratch, flags: &str, answers: &[&str]) -> (String, Option<i32>) {
    let command = format!("stty -echo; exec {PAKA} approve main.paka --key owner {flags}");
    let mut child = Command::new("/usr/bin/script")
        .args(["-qec", &command, "/dev/null"])
        .env_clear()
        .env("SHELL", "/bin/sh")
        .current_dir(dir.path(""))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut shown = vec![];
    let mut byte = [0];
    while !shown.ends_with(b"yes / no: ") && out.read(&mut byte).unwrap() == 1 {
        shown.push(byte[0]);
    }
    let mut stdin = child.stdin.take().unwrap();
    // The command may have ended before asking anything.
    let _ = stdin.write_all(format!("{}\n", answers.join("\n")).as_bytes());
    drop(stdin);
    out.read_to_end(&mut shown).unwrap();
    let status = child.wait().unwrap().code();
    (String::from_utf8(shown).unwrap().replace('\r', ""), status)
}

const SCREENS: &str = "\
[1/4] main.paka, app counter, grants, edited
  approved: [Console]   now: [Console, ReadFiles]
  + ReadFiles: may read any file you can read
Approve this item? yes / no: [2/4] main.paka, example inc_once, edited, now: fail
    example inc_once {
      var s = Counter {}
      _ = update(&s, Inc)
  -   s.count == 1
  +   s.count == 2
    }
  uses: Counter, Msg, update (not covered by this approval)
Approve this item? yes / no: [3/4] main.paka, example starts_at_zero, edited, now: fail
    example starts_at_zero {
  -   Counter {}.count == 0
  +   Counter {}.count == 1
    }
  uses: Counter (not covered by this approval)
Approve this item? yes / no: Approve this item? yes / no: [4/4] main.paka, example greets, adopt, now: pass
  + // Caf\\u{e9} \\u{202e}reversed
  + example greets {
  +   \"caf\\u{e9}\" == \"caf\\u{e9}\"
  + }
Approve this item? yes / no: ";

#[test]
fn each_change_is_shown_and_each_answer_is_kept() {
    let (dir, _) = approved("approve_walk");
    fs::write(dir.path("main.paka"), edited()).unwrap();
    let answers = ["yes", "no", "maybe", "yes", "yes"];
    let (out, status) = approve(&dir, "--adopt greets", &answers);
    let expected = format!("{SCREENS}approvals: serial 5, signed\n");
    assert_eq!((out.as_str(), status), (expected.as_str(), Some(0)));
    let store = Store::open(&dir.path(""), &dir.path("approvals.pub")).unwrap();
    assert_eq!(store.serial, 5);
    let source = Source::file("main.paka", edited().into_bytes());
    let pending = store.changes("main.paka", &source).unwrap();
    assert_eq!(pending.len(), 1, "only inc_once was answered no");
    assert!(matches!(&pending[0].now, Some(Record::Example { name, .. }) if name == "inc_once"));
    assert!(store.owned("main.paka").contains("greets"));
}

#[test]
fn dropping_an_example_gives_up_its_ownership() {
    let (dir, _) = approved("approve_drop");
    let (out, status) = approve(&dir, "--drop starts_at_zero", &["yes"]);
    let expected = "\
[1/1] main.paka, example starts_at_zero, drop ownership, now: pass
    example starts_at_zero {
      Counter {}.count == 0
    }
Approve this item? yes / no: approvals: serial 5, signed
";
    assert_eq!((out.as_str(), status), (expected, Some(0)));
    let store = Store::open(&dir.path(""), &dir.path("approvals.pub")).unwrap();
    assert_eq!(Vec::from_iter(store.owned("main.paka")), ["inc_once"]);
}

#[test]
fn nothing_is_signed_without_a_yes() {
    let (dir, _) = approved("approve_none");
    let before = fs::read(dir.path("approvals.sig")).unwrap();
    let nothing = "approvals: nothing awaits approval\n";
    assert_eq!(approve(&dir, "", &[]), (nothing.into(), Some(0)));
    fs::write(dir.path("main.paka"), edited()).unwrap();
    let (out, status) = approve(&dir, "", &["no", "no", "no"]);
    assert!(
        out.ends_with("approvals: nothing approved, nothing signed\n"),
        "{out}"
    );
    assert_eq!(status, Some(0));
    assert_eq!(fs::read(dir.path("approvals.sig")).unwrap(), before);
}

#[test]
fn answers_come_from_a_terminal_only() {
    let (dir, _) = approved("approve_no_terminal");
    fs::write(dir.path("main.paka"), edited()).unwrap();
    let out = Command::new(PAKA)
        .args(["approve", "main.paka", "--key", "owner"])
        .current_dir(dir.path(""))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let refused = "paka approve: answers are read from a terminal only\n";
    assert_eq!(String::from_utf8(out.stderr).unwrap(), refused);
}

#[test]
fn a_store_that_does_not_verify_is_not_approved_over() {
    let (dir, _) = approved("approve_unverified");
    let text = fs::read_to_string(dir.path("approvals")).unwrap();
    fs::write(dir.path("approvals"), text.replace("serial 4", "serial 9")).unwrap();
    let (out, status) = approve(&dir, "", &[]);
    assert_eq!(
        out,
        "paka approve: approvals: the signature does not verify\n"
    );
    assert_eq!(status, Some(1));
}
