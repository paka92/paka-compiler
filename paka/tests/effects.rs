//! The file effects of M2-15 against real files: `paka run` copies one file to another and
//! says why when it cannot.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use std::fs::{self, File, Permissions};
use std::io::Write;
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

/// The String limit of M2-06 section 1.4, in bytes.
const LIMIT: u64 = 16_777_216;

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

/// A file of `len` bytes that starts with `head`, the rest zeros.
fn sparse(dir: &Scratch, name: &str, head: &[u8], len: u64) {
    let mut file = File::create(dir.path(name)).unwrap();
    file.write_all(head).unwrap();
    file.set_len(len).unwrap();
}

fn workspace(name: &str) -> Scratch {
    let dir = Scratch::new(name);
    fs::write(dir.path("main.paka"), COPY).unwrap();
    fs::write(dir.path("in.txt"), "héllo\n").unwrap();
    fs::write(dir.path("empty.txt"), "").unwrap();
    fs::write(dir.path("bad.bin"), [b'a', 0xff, 0xfe]).unwrap();
    fs::create_dir(dir.path("folder")).unwrap();
    let made = Command::new("mkfifo")
        .arg(dir.path("pipe"))
        .status()
        .unwrap();
    assert!(made.success());
    sparse(&dir, "limit.txt", b"", LIMIT);
    sparse(&dir, "big.txt", &[0xff], LIMIT + 1);
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
fn a_file_of_exactly_the_limit_is_read() {
    let dir = workspace("limit");
    assert_eq!(copy(&dir, "limit.txt", "out.txt"), ("copied\n".into(), 0));
    assert_eq!(fs::metadata(dir.path("out.txt")).unwrap().len(), LIMIT);
}

#[test]
fn each_way_a_read_fails_gives_its_reason() {
    let dir = workspace("read");
    let rows = [
        ("nothing.txt", "not found"),
        ("in.txt/below", "not found"),
        ("folder", "not a file"),
        ("/dev/null", "not a file"),
        ("pipe", "not a file"), // refused unopened: opening it would wait for a writer
        ("bad.bin", "not text"),
        ("big.txt", "failed"), // not text, but too big comes first
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
