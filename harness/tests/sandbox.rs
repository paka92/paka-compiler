//! Programs that try to break out of the sandbox, in each language that can express the attempt.
//! A program that is refused prints `blocked`; each test also checks the outside effect itself.

mod common;

use common::{put, runner, scratch};
use harness::runner::{CaseResult, Lang, OUTPUT_CAP, Report, Runner, Verdict};
use harness::task::Task;
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::time::{Duration, Instant};

const SECRET: &str = "SECRET_7f3a";

/// A task whose visible and hidden case both run with `args` and expect `stdout` and status 0.
/// The hidden case holds `in/secret.ts`; its path in the task folder is `secret(root)`.
fn task(root: &Path, args: &[String], stdout: &str) -> Task {
    put(root, "statement.md", "Try to leave the sandbox.\n");
    for case in ["cases/visible", "hidden/hidden"] {
        if !args.is_empty() {
            put(root, &format!("{case}/args"), &(args.join("\n") + "\n"));
        }
        put(root, &format!("{case}/stdout"), stdout);
    }
    put(root, "hidden/hidden/in/secret.ts", &format!("{SECRET}\n"));
    Task::load(root).unwrap()
}

fn secret(root: &Path) -> String {
    root.join("hidden/hidden/in/secret.ts")
        .display()
        .to_string()
}

/// Judges a program that must build; gives its visible and hidden case results.
fn judge(runner: &Runner, lang: Lang, source: &str, task: &Task) -> Vec<CaseResult> {
    let report = runner.judge(lang, source, task).unwrap();
    assert_eq!(report.build, Ok(()), "{lang:?}");
    let Report {
        visible, hidden, ..
    } = report;
    visible.into_iter().chain(hidden).collect()
}

fn verdicts(cases: &[CaseResult]) -> Vec<Verdict> {
    cases.iter().map(|c| c.verdict).collect()
}

#[test]
fn network_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let rust = r#"fn main() {
        let addr = std::env::args().nth(1).unwrap();
        match std::net::TcpStream::connect(addr) {
            Ok(_) => println!("connected"),
            Err(_) => println!("blocked"),
        }
    }"#;
    let ts = r#"import { connect } from "node:net";
    const [host, port] = process.argv[2]!.split(":");
    connect(Number(port), host!)
      .on("connect", () => { console.log("connected"); process.exit(0); })
      .on("error", () => console.log("blocked"));"#;
    let root = scratch("network");
    let task = task(&root, &[addr], "blocked\n");
    let runner = runner();
    for (lang, source) in [(Lang::Rust, rust), (Lang::TypeScript, ts)] {
        let got = verdicts(&judge(&runner, lang, source, &task));
        assert_eq!(got, [Verdict::Pass; 2], "{lang:?}");
    }
    listener.set_nonblocking(true).unwrap();
    assert!(listener.accept().is_err(), "a program reached the host");
    // Paka has no network effect: asking for one does not build.
    let paka = "enum Msg { Start }\nfn update(m: Msg) -> List<Effect<Msg>> { [Exit] }\n\
                app net { start: Start\n update: update\n grants: [Network] }\n";
    let report = runner.judge(Lang::Paka, paka, &task).unwrap();
    assert!(report.build.unwrap_err().contains("unknown-grant"));
}

/// Programs that print the three files named by their arguments, or `blocked` for each they
/// cannot read.
const READ_RUST: &str = r#"fn main() {
    for path in std::env::args().skip(1) {
        match std::fs::read_to_string(path) {
            Ok(text) => print!("{text}"),
            Err(_) => println!("blocked"),
        }
    }
}"#;
const READ_TS: &str = r#"import { readFileSync } from "node:fs";
for (const path of process.argv.slice(2)) {
  try { process.stdout.write(readFileSync(path, "utf8")); } catch { console.log("blocked"); }
}"#;
const READ_PAKA: &str = r#"
type Seen { left: Int = 3 }
enum Msg { Start(a: String, b: String, c: String), Loaded(text: String), Failed(why: String) }
fn update(s: inout Seen, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(a, b, c) => [read(a), read(b), read(c)]
    Loaded(text) => answer(&s, text)
    Failed(_) => answer(&s, "blocked")
  }
}
fn read(path: String) -> Effect<Msg> {
  ReadFile(path, then: Loaded, or: Failed)
}
fn answer(s: inout Seen, text: String) -> List<Effect<Msg>> {
  s.left -= 1
  if s.left == 0 { [Print(text), Exit] } else { [Print(text)] }
}
app peek {
  state: Seen
  start: Start
  update: update
  grants: [ReadFiles, Console]
}
"#;
const READERS: [(Lang, &str); 3] = [
    (Lang::Rust, READ_RUST),
    (Lang::TypeScript, READ_TS),
    (Lang::Paka, READ_PAKA),
];

#[test]
fn hidden_cases_and_the_harness_are_unreadable() {
    let root = scratch("read");
    let args = [
        secret(&root),
        root.join("statement.md").display().to_string(),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/runner.rs").into(),
    ];
    let task = task(&root, &args, "blocked\nblocked\nblocked\n");
    let runner = runner();
    for (lang, source) in READERS {
        let got = verdicts(&judge(&runner, lang, source, &task));
        assert_eq!(got, [Verdict::Pass; 2], "{lang:?}");
    }
}

/// The sandbox's first process and the program itself show only sandbox paths and the variables
/// set for the build or case, at run time in every language and at build time in Rust.
#[test]
fn proc_shows_no_harness_environment_or_host_path() {
    assert!(
        env::var_os("CARGO_PKG_NAME").is_some(),
        "cargo sets the marker"
    );
    let root = scratch("proc");
    let args = ["/proc/1/environ", "/proc/1/cmdline", "/proc/self/environ"].map(String::from);
    let task = task(&root, &args, "");
    let runner = runner();
    for (lang, source) in READERS {
        for case in judge(&runner, lang, source, &task) {
            assert_hidden(&runner, &format!("{lang:?} run"), &case.stdout, "LC_ALL=C");
        }
    }
    let rust = r#"fn main() {
        let shown = [
            &include_bytes!("/proc/1/environ")[..],
            include_bytes!("/proc/1/cmdline"),
            include_bytes!("/proc/self/environ"),
        ];
        for bytes in shown {
            std::io::Write::write_all(&mut std::io::stdout(), bytes).unwrap();
        }
    }"#;
    let case = &judge(&runner, Lang::Rust, rust, &task)[0];
    assert_hidden(&runner, "Rust build", &case.stdout, "TMPDIR=/build");
}

/// `shown` holds `expected`, so the files were read, and nothing of the harness.
fn assert_hidden(runner: &Runner, what: &str, shown: &[u8], expected: &str) {
    let text = String::from_utf8_lossy(shown).replace('\0', " ");
    assert!(text.contains(expected), "{what} read nothing: {text}");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let host = [repo, &env::temp_dir(), &runner.rust].map(|p| p.display().to_string());
    for leak in ["CARGO_", "HOME="]
        .into_iter()
        .chain(host.iter().map(String::as_str))
    {
        assert!(!text.contains(leak), "{what} shows {leak}: {text}");
    }
}

#[test]
fn hidden_cases_are_unreadable_at_build() {
    let root = scratch("read-build");
    let task = task(&root, &[], "");
    let rust = format!(
        "const S: &str = include_str!({:?});\nfn main() {{ print!(\"{{S}}\"); }}\n",
        secret(&root)
    );
    let ts = format!(
        "/// <reference path={:?} />\nconsole.log(1);\n",
        secret(&root)
    );
    let runner = runner();
    for (lang, source) in [(Lang::Rust, rust), (Lang::TypeScript, ts)] {
        let text = runner
            .judge(lang, &source, &task)
            .unwrap()
            .build
            .unwrap_err();
        assert!(
            !text.contains(SECRET),
            "{lang:?} read the hidden case: {text}"
        );
    }
}

#[test]
fn nothing_is_written_outside_the_case_directory() {
    let rust = r#"fn main() {
        for path in std::env::args().skip(1) {
            if std::fs::write(path, "x").is_err() {
                println!("blocked");
            }
        }
    }"#;
    let ts = r#"import { writeFileSync } from "node:fs";
    for (const path of process.argv.slice(2)) {
      try { writeFileSync(path, "x"); } catch { console.log("blocked"); }
    }"#;
    let paka = r#"
type Seen { left: Int = 2 }
enum Msg { Start(a: String, b: String), Wrote, Failed(why: String) }
fn update(s: inout Seen, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(a, b) => [write(a), write(b)]
    Wrote => answer(&s, "written")
    Failed(_) => answer(&s, "blocked")
  }
}
fn write(path: String) -> Effect<Msg> {
  WriteFile(path, "x", then: Wrote, or: Failed)
}
fn answer(s: inout Seen, text: String) -> List<Effect<Msg>> {
  s.left -= 1
  if s.left == 0 { [Print(text), Exit] } else { [Print(text)] }
}
app scribble {
  state: Seen
  start: Start
  update: update
  grants: [WriteFiles, Console]
}
"#;
    let root = scratch("write");
    let outside = root.join("outside.txt");
    // The case directory sits in the temporary directory; this name is its sibling.
    let sibling = format!("escape-{}.txt", std::process::id());
    let args = [outside.display().to_string(), format!("../{sibling}")];
    let task = task(&root, &args, "blocked\nblocked\n");
    let runner = runner();
    for (lang, source) in [
        (Lang::Rust, rust),
        (Lang::TypeScript, ts),
        (Lang::Paka, paka),
    ] {
        let got = verdicts(&judge(&runner, lang, source, &task));
        assert_eq!(got, [Verdict::Pass; 2], "{lang:?}");
    }
    assert!(!outside.exists());
    assert!(!std::env::temp_dir().join(sibling).exists());
}

#[test]
fn memory_past_one_gigabyte_stops_the_program() {
    let rust = r#"fn main() {
        let mut kept: Vec<Vec<u8>> = vec![];
        for i in 1.. {
            kept.push(vec![1; 100 << 20]);
            if i == 6 { println!("600 MB"); }
        }
    }"#;
    let ts = r#"const kept: number[][] = [];
    for (let i = 1; ; i++) {
      kept.push(new Array(12_500_000).fill(i));
      if (i === 6) console.log("600 MB");
    }"#;
    // Paka prints only when the handler ends, so nothing is printed.
    let paka = r#"
enum Msg { Start }
fn update(m: Msg) -> List<Effect<Msg>> {
  let block = repeat("x", 8000000)
  var kept: List<String> = []
  for n in 0..<1000 {
    append(&kept, block + "{n}")
  }
  [Print("{len(kept)}"), Exit]
}
app hog {
  start: Start
  update: update
  grants: [Console]
}
"#;
    let mut runner = runner();
    runner.limits.case = Duration::from_secs(10);
    for (lang, source, stdout) in [
        (Lang::Rust, rust, "600 MB\n"),
        (Lang::TypeScript, ts, "600 MB\n"),
        (Lang::Paka, paka, ""),
    ] {
        let root = scratch(&format!("memory-{lang:?}"));
        let cases = judge(&runner, lang, source, &task(&root, &[], stdout));
        assert_eq!(verdicts(&cases), [Verdict::Status; 2], "{lang:?}");
        // Out of memory aborts: 128 + SIGABRT.
        assert!(cases.iter().all(|c| c.status == Some(134)), "{lang:?}");
    }
}

#[test]
fn endless_output_is_cut() {
    let rust = r#"fn main() {
        let line = "x".repeat(99);
        loop { println!("{line}"); }
    }"#;
    let ts = r#"import { writeSync } from "node:fs";
    const line = "x".repeat(99) + "\n";
    for (;;) writeSync(1, line);"#;
    let paka = r#"
enum Msg { Start }
fn update(m: Msg) -> List<Effect<Msg>> {
  let line = repeat("x", 1000000)
  var effects: List<Effect<Msg>> = list_of(Print(line), count: 100)
  append(&effects, Exit)
  effects
}
app flood {
  start: Start
  update: update
  grants: [Console]
}
"#;
    let root = scratch("output");
    let task = task(&root, &[], "");
    let mut runner = runner();
    runner.limits.case = Duration::from_secs(3);
    for (lang, source) in [
        (Lang::Rust, rust),
        (Lang::TypeScript, ts),
        (Lang::Paka, paka),
    ] {
        let cases = judge(&runner, lang, source, &task);
        assert_eq!(verdicts(&cases), [Verdict::Stdout; 2], "{lang:?}");
        assert!(
            cases.iter().all(|c| c.stdout.len() == OUTPUT_CAP),
            "{lang:?}"
        );
    }
}

#[test]
fn output_of_exactly_the_cap_passes() {
    let rust = r#"fn main() {
        let cap: usize = std::env::args().nth(1).unwrap().parse().unwrap();
        print!("{}", "x".repeat(cap));
    }"#;
    let root = scratch("output-cap");
    let task = task(&root, &[OUTPUT_CAP.to_string()], &"x".repeat(OUTPUT_CAP));
    let cases = judge(&runner(), Lang::Rust, rust, &task);
    assert_eq!(verdicts(&cases), [Verdict::Pass; 2]);
}

#[test]
fn endless_file_stops_at_the_file_cap() {
    let rust = r#"use std::io::Write;
    fn main() {
        let mut file = std::fs::File::create("big.txt").unwrap();
        loop { file.write_all(&[b'x'; 1 << 20]).unwrap(); }
    }"#;
    let root = scratch("file");
    let task = task(&root, &[], "");
    let cases = judge(&runner(), Lang::Rust, rust, &task);
    assert_eq!(verdicts(&cases), [Verdict::Status; 2]);
    // 128 + SIGXFSZ.
    assert!(cases.iter().all(|c| c.status == Some(153)));
}

#[test]
fn timeout_kills_children_that_try_to_outlive_it() {
    let rust = r#"use std::os::unix::process::CommandExt;
    fn main() {
        let args: Vec<String> = std::env::args().collect();
        if args[1] == "child" {
            loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
        }
        std::process::Command::new("/proc/self/exe")
            .args(["child", &args[2]])
            .process_group(0)
            .spawn()
            .unwrap();
        println!("spawned");
        loop {}
    }"#;
    let ts = r#"import { spawn } from "node:child_process";
    spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)", process.argv[3]!], {
      detached: true,
      stdio: "ignore",
    }).unref();
    console.log("spawned");
    for (;;) {}"#;
    let mut runner = runner();
    runner.limits.case = Duration::from_secs(1);
    for (lang, source) in [(Lang::Rust, rust), (Lang::TypeScript, ts)] {
        let marker = format!("orphan-{}-{lang:?}", std::process::id());
        let root = scratch(&format!("fork-{lang:?}"));
        let args = ["parent".into(), marker.clone()];
        let cases = judge(&runner, lang, source, &task(&root, &args, "spawned\n"));
        assert_eq!(verdicts(&cases), [Verdict::Timeout; 2], "{lang:?}");
        assert!(cases.iter().all(|c| c.stdout == b"spawned\n"), "{lang:?}");
        assert!(gone(&marker), "{lang:?} left a process behind");
    }
}

/// Whether no process names `marker` on its command line within a second.
fn gone(marker: &str) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        if !processes().iter().any(|cmdline| cmdline.contains(marker)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn processes() -> Vec<String> {
    let entries = fs::read_dir("/proc").unwrap().flatten();
    let pids = entries.map(|e| e.path()).filter(|p| is_pid(p));
    pids.filter_map(|p| fs::read(p.join("cmdline")).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .collect()
}

fn is_pid(path: &Path) -> bool {
    let name = path.file_name().unwrap().to_string_lossy();
    name.chars().all(|c| c.is_ascii_digit())
}
