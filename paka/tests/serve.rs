//! `paka serve` (M4-16, M4-17): the hand-written transcripts in `tests/serve`, the lock that keeps
//! one server per program, and a program that does not parse.
//!
//! A transcript `x.txt` is served on a copy of `x/before`, or of the wc sample without it: on its
//! `main.paka` when there is one, else on the directory. The files after it match `x/after`.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const WC: &str = "tests/cases/run-interp/sample_wc_count";

fn server(dir: &Path) -> std::process::Child {
    let program = if dir.join("main.paka").exists() {
        "main.paka"
    } else {
        "."
    };
    Command::new(PAKA)
        .args(["serve", program])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn transcripts_get_their_replies() {
    let dir = Path::new(ROOT).join("tests/serve");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "txt") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let sent: Vec<u8> = (text.lines())
            .filter_map(|l| l.strip_prefix("> ").or((l == ">").then_some("")))
            .flat_map(|l| [sent_bytes(l), vec![b'\n']].concat())
            .collect();
        let want: Vec<_> = text.lines().filter_map(|l| l.strip_prefix("< ")).collect();
        let stem = path.with_extension("");
        let before = Some(stem.join("before")).filter(|b| b.exists());
        let scratch = Scratch::new(&format!("serve-{}", stem.file_name().unwrap().display()));
        let program = scratch.path("");
        copy(&before.unwrap_or(Path::new(ROOT).join(WC)), &program);
        let mut child = server(&program);
        child.stdin.take().unwrap().write_all(&sent).unwrap();
        let out = child.wait_with_output().unwrap();
        let got = String::from_utf8(out.stdout).unwrap();
        assert_eq!(got.lines().collect::<Vec<_>>(), want, "{}", path.display());
        assert_eq!(out.status.code(), Some(0));
        if stem.join("after").exists() {
            assert_eq!(
                files(&program),
                files(&stem.join("after")),
                "{}",
                path.display()
            );
        }
        seen += 1;
    }
    assert!(seen >= 4);
}

fn copy(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
    }
}

/// The `.paka` files in `dir` and their text.
fn files(dir: &Path) -> Vec<(String, String)> {
    let mut files: Vec<_> = (fs::read_dir(dir).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "paka"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into();
            (name, fs::read_to_string(&p).unwrap())
        })
        .collect();
    files.sort();
    files
}

/// A sent line's bytes; `\xff` stands for that byte, which is not UTF-8.
fn sent_bytes(line: &str) -> Vec<u8> {
    let parts: Vec<_> = line.split(r"\xff").map(str::as_bytes).collect();
    parts.join(&0xff)
}

#[test]
fn a_second_server_on_one_program_is_refused() {
    let wc = Scratch::new("serve-lock");
    let program = fs::read(Path::new(ROOT).join(WC).join("main.paka")).unwrap();
    fs::write(wc.path("main.paka"), program).unwrap();
    let wc = wc.path("");
    let mut first = server(&wc);
    // A reply means the first server holds the lock, which moves to the file the edit wrote.
    let mut stdin = first.stdin.take().unwrap();
    let add =
        r#"{"v":1,"id":1,"call":"edit","name":"g","base":null,"text":"fn g() -> Int {\n  0\n}"}"#;
    writeln!(stdin, "{add}").unwrap();
    let mut line = String::new();
    let mut stdout = std::io::BufReader::new(first.stdout.take().unwrap());
    std::io::BufRead::read_line(&mut stdout, &mut line).unwrap();
    assert!(line.contains(r#""ok":true"#), "{line}");

    let second = server(&wc).wait_with_output().unwrap();
    assert_eq!(second.status.code(), Some(1));
    let err = String::from_utf8(second.stderr).unwrap();
    assert_eq!(
        err,
        "main.paka: error[not-served]: another server holds this program\n"
    );
    drop(stdin);
    assert!(first.wait().unwrap().success());
}

#[test]
fn a_program_that_does_not_parse_is_not_served() {
    let dir = Scratch::new("serve-parse");
    fs::write(dir.path("main.paka"), "fn f( {\n}\n").unwrap();
    let out = server(&dir.path("")).wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.starts_with("main.paka:1:"), "{err}");
}
