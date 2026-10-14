//! The file effects of M2-15 against real files: `paka run` copies one file to another and
//! says why when it cannot.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use std::fs::{self, File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const COPY: &str = "
type Job { dest: String = \"\" }

enum Msg { Start(src: String, dest: String), Loaded(text: String), Wrote, Failed(why: String) }

fn update(s: inout Job, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(src, dest) => {
      s.dest = dest
      [ReadFile(src, then: Loaded, or: Failed)]
    }
    Loaded(text) => [WriteFile(s.dest, text, then: Wrote, or: Failed)]
    Wrote => [Print(\"copied\"), Exit]
    Failed(why) => [Print(why), ExitWith(20)]
  }
}

app copy {
  state: Job
  start: Start
  update: update
  grants: [Console, ReadFiles, WriteFiles]
}
";

/// The bytes past the String limit of M2-06 section 1.4.
const TOO_BIG: u64 = 16_777_217;

/// Runs the copy tool in `dir` and returns its standard output and status.
fn copy(dir: &Scratch, src: &str, dest: &str) -> (String, i32) {
    let run = Command::new(PAKA)
        .args(["run", "main.paka", src, dest])
        .current_dir(dir.path(""))
        .output()
        .unwrap();
    assert!(
        run.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (
        String::from_utf8(run.stdout).unwrap(),
        run.status.code().unwrap(),
    )
}

fn workspace(name: &str) -> Scratch {
    let dir = Scratch::new(name);
    fs::write(dir.path("main.paka"), COPY).unwrap();
    fs::write(dir.path("in.txt"), "héllo\n").unwrap();
    fs::write(dir.path("empty.txt"), "").unwrap();
    fs::write(dir.path("bad.bin"), [b'a', 0xff, 0xfe]).unwrap();
    fs::create_dir(dir.path("folder")).unwrap();
    File::create(dir.path("big.txt"))
        .unwrap()
        .set_len(TOO_BIG)
        .unwrap();
    dir
}

#[test]
fn a_file_is_copied_replacing_what_was_there() {
    let dir = workspace("copied");
    fs::write(dir.path("out.txt"), "older and much longer text").unwrap();
    assert_eq!(copy(&dir, "in.txt", "out.txt"), ("copied\n".into(), 0));
    assert_eq!(fs::read(dir.path("out.txt")).unwrap(), "héllo\n".as_bytes());
    assert_eq!(copy(&dir, "empty.txt", "out.txt"), ("copied\n".into(), 0));
    assert_eq!(fs::read(dir.path("out.txt")).unwrap(), b"");
}

#[test]
fn each_way_a_read_fails_gives_its_reason() {
    let dir = workspace("read");
    let rows = [
        ("nothing.txt", "not found"),
        ("in.txt/below", "not found"),
        ("folder", "not a file"),
        ("/dev/null", "not a file"),
        ("bad.bin", "not text"),
        ("big.txt", "failed"),
        ("", "failed"),
    ];
    for (src, why) in rows {
        assert_eq!(
            copy(&dir, src, "out.txt"),
            (format!("{why}\n"), 20),
            "{src:?}"
        );
        assert!(!dir.path("out.txt").exists(), "{src:?}");
    }
}

#[test]
fn each_way_a_write_fails_gives_its_reason() {
    let dir = workspace("write");
    let rows = [
        ("folder/below/out.txt", "not found"),
        ("in.txt/below", "not found"),
        ("folder", "not a file"),
        ("/dev/full", "no space"),
        ("", "failed"),
    ];
    for (dest, why) in rows {
        assert_eq!(
            copy(&dir, "in.txt", dest),
            (format!("{why}\n"), 20),
            "{dest:?}"
        );
    }
}

#[test]
fn a_file_the_process_may_not_use_gives_no_permission() {
    let dir = workspace("permission");
    fs::set_permissions(dir.path("in.txt"), Permissions::from_mode(0o000)).unwrap();
    if File::open(dir.path("in.txt")).is_ok() {
        return; // a process that ignores permissions cannot be refused
    }
    assert_eq!(
        copy(&dir, "in.txt", "out.txt"),
        ("no permission\n".into(), 20)
    );
    fs::set_permissions(dir.path("in.txt"), Permissions::from_mode(0o644)).unwrap();
    fs::set_permissions(dir.path("folder"), Permissions::from_mode(0o555)).unwrap();
    let dest = "folder/out.txt";
    assert_eq!(copy(&dir, "in.txt", dest), ("no permission\n".into(), 20));
    fs::set_permissions(dir.path("folder"), Permissions::from_mode(0o755)).unwrap();
}
