//! The case runner on hand-written tasks: a known-good and known-bad program per language, and the
//! task loader on a good and several malformed folders.

use harness::Error;
use harness::runner::{Lang, Limits, Runner, Verdict};
use harness::task::Task;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PAKA_GOOD: &str = r#"
type Tally { words: Int = 0 }
enum Msg { Start(path: String), Loaded(text: String), Failed(why: String) }
fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path) => [ReadFile(path, then: Loaded, or: Failed)]
    Loaded(text) => {
      s.words = len(words(text))
      [Print("{s.words} words"), Exit]
    }
    Failed(why) => [Print("cannot read file: {why}"), ExitWith(20)]
  }
}
app wc {
  state: Tally
  start: Start
  update: update
  grants: [ReadFiles, Console]
}
"#;

const RUST_GOOD: &str = r#"
fn main() {
    let path = std::env::args().nth(1).unwrap();
    match std::fs::read_to_string(&path) {
        Ok(text) => println!("{} words", text.split_whitespace().count()),
        Err(_) => {
            println!("cannot read file: not found");
            std::process::exit(20);
        }
    }
}
"#;

const TS_GOOD: &str = r#"
import { readFileSync } from "node:fs";
try {
  const text: string = readFileSync(process.argv[2]!, "utf8");
  console.log(`${text.split(/\s+/).filter((w) => w !== "").length} words`);
} catch {
  console.log("cannot read file: not found");
  process.exit(20);
}
"#;

/// One language's programs for the word task, each wrong in one way.
struct Programs {
    lang: Lang,
    good: String,
    wrong_output: String,
    wrong_status: String,
    broken: String,
    endless: String,
}

fn programs() -> [Programs; 3] {
    [
        Programs {
            lang: Lang::Paka,
            good: PAKA_GOOD.into(),
            wrong_output: PAKA_GOOD.replace("{s.words} words", "{s.words}"),
            wrong_status: PAKA_GOOD.replace("ExitWith(20)", "ExitWith(21)"),
            broken: PAKA_GOOD.replace("Exit]", "Quit]"),
            endless: PAKA_GOOD.replace(
                "Effect<Msg>> {",
                "Effect<Msg>> {\n  var n = 0\n  while true { n += 1 }",
            ),
        },
        Programs {
            lang: Lang::Rust,
            good: RUST_GOOD.into(),
            wrong_output: RUST_GOOD.replace("{} words", "{}"),
            wrong_status: RUST_GOOD.replace("exit(20)", "exit(21)"),
            broken: RUST_GOOD.replace("count()", "count"),
            endless: RUST_GOOD.replace("fn main() {", "fn main() {\n    loop {}"),
        },
        Programs {
            lang: Lang::TypeScript,
            good: TS_GOOD.into(),
            wrong_output: TS_GOOD.replace("} words", "}"),
            wrong_status: TS_GOOD.replace("exit(20)", "exit(21)"),
            broken: TS_GOOD.replace("string = read", "number = read"),
            endless: TS_GOOD.replace("try {", "while (true) {}\ntry {"),
        },
    ]
}

type Breaker = fn(&Path);

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("harness-runner-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A case folder: arguments, input file, expected output and status.
fn put_case(dir: &Path, name: &str, args: &str, input: Option<&str>, stdout: &str, status: &str) {
    put(dir, &format!("{name}/args"), args);
    put(dir, &format!("{name}/stdout"), stdout);
    put(dir, &format!("{name}/status"), status);
    if let Some(text) = input {
        put(dir, &format!("{name}/in/in.txt"), text);
    }
}

/// The word task: visible cases `missing` and `two`, hidden case `empty`.
fn word_task(name: &str) -> (PathBuf, Task) {
    let root = scratch(name);
    put(
        &root,
        "statement.md",
        "Count the words in the file named by the argument.\n",
    );
    let visible = root.join("cases");
    put_case(
        &visible,
        "two",
        "in.txt\n",
        Some("hello brave\nworld\n"),
        "3 words\n",
        "0\n",
    );
    put_case(
        &visible,
        "missing",
        "none.txt\n",
        None,
        "cannot read file: not found\n",
        "20\n",
    );
    put_case(
        &root.join("hidden"),
        "empty",
        "in.txt\n",
        Some(""),
        "0 words\n",
        "0\n",
    );
    let task = Task::load(&root).unwrap();
    (root, task)
}

fn runner() -> Runner {
    // Cargo builds the paka binary beside the test binary's `deps` folder.
    let exe = std::env::current_exe().unwrap();
    let paka = exe.parent().unwrap().parent().unwrap().join("paka");
    assert!(
        paka.exists(),
        "build the paka binary first: {}",
        paka.display()
    );
    let limits = Limits {
        build: Duration::from_secs(120),
        case: Duration::from_secs(5),
    };
    Runner {
        limits,
        ..Runner::new(paka).unwrap()
    }
}

/// The verdicts of a program that builds: visible cases in name order, then hidden ones.
fn verdicts(runner: &Runner, lang: Lang, source: &str, task: &Task) -> Vec<Verdict> {
    let report = runner.judge(lang, source, task).unwrap();
    assert_eq!(report.build, Ok(()), "{lang:?}");
    report
        .visible
        .iter()
        .chain(&report.hidden)
        .map(|c| c.verdict)
        .collect()
}

#[test]
fn known_good_program_passes_in_every_language() {
    let (_root, task) = word_task("good");
    let runner = runner();
    for p in programs() {
        assert_eq!(
            verdicts(&runner, p.lang, &p.good, &task),
            [Verdict::Pass; 3],
            "{:?}",
            p.lang
        );
        assert!(runner.judge(p.lang, &p.good, &task).unwrap().passed());
    }
}

#[test]
fn wrong_output_and_wrong_status_are_told_apart() {
    let (_root, task) = word_task("wrong");
    let runner = runner();
    for p in programs() {
        let got = verdicts(&runner, p.lang, &p.wrong_output, &task);
        assert_eq!(
            got,
            [Verdict::Pass, Verdict::Stdout, Verdict::Stdout],
            "{:?}",
            p.lang
        );
        let got = verdicts(&runner, p.lang, &p.wrong_status, &task);
        assert_eq!(
            got,
            [Verdict::Status, Verdict::Pass, Verdict::Pass],
            "{:?}",
            p.lang
        );
        assert!(
            !runner
                .judge(p.lang, &p.wrong_output, &task)
                .unwrap()
                .passed()
        );
    }
}

#[test]
fn build_error_gives_the_compiler_text_and_runs_no_case() {
    let (_root, task) = word_task("broken");
    let runner = runner();
    for p in programs() {
        let report = runner.judge(p.lang, &p.broken, &task).unwrap();
        let text = report.build.clone().expect_err(&format!("{:?}", p.lang));
        assert!(!text.is_empty(), "{:?}", p.lang);
        assert!(report.visible.is_empty() && report.hidden.is_empty() && !report.passed());
    }
}

#[test]
fn endless_program_times_out() {
    let (_root, task) = word_task("endless");
    let mut runner = runner();
    runner.limits.case = Duration::from_millis(500);
    for p in programs() {
        let got = verdicts(&runner, p.lang, &p.endless, &task);
        assert_eq!(got, [Verdict::Timeout; 3], "{:?}", p.lang);
    }
}

#[test]
fn written_files_are_compared_byte_for_byte() {
    let root = scratch("files");
    put(&root, "statement.md", "Copy the input to out.txt.\n");
    let cases = root.join("cases");
    put_case(&cases, "copy", "in.txt\n", Some("a\nb\n"), "", "0\n");
    put(&cases, "copy/out/out.txt", "a\nb\n");
    let hidden = root.join("hidden");
    put_case(&hidden, "wrong", "in.txt\n", Some("x\n"), "", "0\n");
    put(&hidden, "wrong/out/out.txt", "y\n");
    let task = Task::load(&root).unwrap();
    let source = r#"fn main() {
        let path = std::env::args().nth(1).unwrap();
        std::fs::copy(path, "out.txt").unwrap();
    }"#;
    assert_eq!(
        verdicts(&runner(), Lang::Rust, source, &task),
        [Verdict::Pass, Verdict::Files]
    );
}

#[test]
fn loader_reads_the_folder_format() {
    let (_root, task) = word_task("load");
    assert_eq!(
        task.statement,
        "Count the words in the file named by the argument.\n"
    );
    let names: Vec<_> = task.visible.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["missing", "two"]);
    assert_eq!(task.hidden.len(), 1);
    let missing = &task.visible[0];
    assert_eq!(missing.args, ["none.txt"]);
    assert_eq!(missing.status, 20);
    assert_eq!(missing.stdout, b"cannot read file: not found\n");
}

#[test]
fn loader_rejects_malformed_folders() {
    let breakers: [(&str, Breaker); 7] = [
        ("no statement", |r| {
            fs::remove_file(r.join("statement.md")).unwrap()
        }),
        ("empty statement", |r| put(r, "statement.md", " \n")),
        ("no hidden cases", |r| {
            fs::remove_dir_all(r.join("hidden")).unwrap()
        }),
        ("hidden has no case", |r| {
            fs::remove_dir_all(r.join("hidden/empty")).unwrap()
        }),
        ("case without stdout", |r| {
            fs::remove_file(r.join("cases/two/stdout")).unwrap()
        }),
        ("status not a number", |r| {
            put(r, "cases/two/status", "ok\n")
        }),
        ("stray file in a case", |r| put(r, "cases/two/notes", "x")),
    ];
    for (i, (what, break_it)) in breakers.into_iter().enumerate() {
        let (root, _) = word_task(&format!("bad{i}"));
        break_it(&root);
        assert!(
            matches!(Task::load(&root), Err(Error::Malformed(_))),
            "{what}"
        );
    }
}

#[test]
fn case_environment_is_the_time_zone_and_locale_only() {
    let root = scratch("env");
    put(&root, "statement.md", "Print the environment.\n");
    put_case(
        &root.join("cases"),
        "env",
        "",
        None,
        "LC_ALL=C\nTZ=UTC\n",
        "0\n",
    );
    put_case(
        &root.join("hidden"),
        "env",
        "",
        None,
        "LC_ALL=C\nTZ=UTC\n",
        "0\n",
    );
    let task = Task::load(&root).unwrap();
    let source = r#"fn main() {
        let mut vars: Vec<_> = std::env::vars().map(|(k, v)| format!("{k}={v}")).collect();
        vars.sort();
        for v in vars { println!("{v}"); }
    }"#;
    let got = verdicts(&runner(), Lang::Rust, source, &task);
    assert_eq!(got, [Verdict::Pass; 2]);
}
