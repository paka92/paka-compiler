//! Conformance runner: every case under `tests/conformance/<area>/` by the pass rules of
//! tests/conformance/README.md. `errors/` uses its `expect.json` format; every other area the
//! `stdout`, `status`, `stderr`, `reject` and `trap` files. Each case runs in a fresh copy of its
//! folder, so `files/`, `after/`, `absent` and `present` see only what the command did.

#[allow(dead_code)]
mod common;
#[path = "conformance/gate.rs"]
mod gate;

use common::{PAKA, Scratch, build_runtime};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/conformance");
const KNOWN: &[&str] = &[
    "main.paka",
    "stdout",
    "status",
    "stderr",
    "reject",
    "trap",
    "args",
    "command",
    "files",
    "after",
    "absent",
    "present",
    "expect.json",
];
/// The kinds that need a broken environment (README, errors/: "Not in the table").
const ENVIRONMENT: &[&str] = &[
    "file-unreadable",
    "clang-missing",
    "clang-failed",
    "runtime-missing",
];

/// Kinds of a migrate, which is no program file; paka/tests/migrate.rs tests them.
const MIGRATE: &[&str] = &[
    "bad-migrate-signature",
    "migrate-needs-reset",
    "effect-in-migrate",
    "old-value-use",
];

fn read(dir: &Path, name: &str) -> Option<Vec<u8>> {
    fs::read(dir.join(name)).ok()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    folders
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, target).unwrap();
        }
    }
}

/// The case's own files, then the contents of `files/`, in a fresh folder.
fn stage(case: &Path, run: &Path) {
    for entry in fs::read_dir(case).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            fs::copy(&path, run.join(path.file_name().unwrap())).unwrap();
        }
    }
    if case.join("files").is_dir() {
        copy_tree(&case.join("files"), run);
    }
}

/// `command` words, or `run main.paka` and the raw `args` lines.
fn command(case: &Path) -> Command {
    let mut command = Command::new(PAKA);
    match read(case, "command") {
        Some(words) => {
            command.args(text(&words).split_whitespace());
        }
        None => {
            command.args(["run", "main.paka"]);
            if let Some(args) = read(case, "args") {
                let mut lines: Vec<_> = args.split(|b| *b == b'\n').collect();
                lines.pop();
                command.args(lines.into_iter().map(OsStr::from_bytes));
            }
        }
    }
    command
}

fn want_eq(what: &str, got: &[u8], want: &[u8]) -> Result<(), String> {
    match got == want {
        true => Ok(()),
        false => Err(format!("{what} {:?}, want {:?}", text(got), text(want))),
    }
}

fn status_is(out: &Output, want: i32) -> Result<(), String> {
    match out.status.code() {
        Some(code) if code == want => Ok(()),
        code => Err(format!(
            "status {code:?}, want {want}; stderr {:?}",
            text(&out.stderr)
        )),
    }
}

fn app_name(source: &[u8]) -> String {
    let source = text(source);
    let line = source.lines().find_map(|l| l.strip_prefix("app "));
    line.and_then(|l| l.split_whitespace().next())
        .unwrap_or_default()
        .to_string()
}

/// `<app>: trap <kind> at main.paka:LINE:COL` and a line feed, any LINE and COL.
fn trap_line(got: &str, app: &str, kind: &str) -> bool {
    let rest = got
        .strip_prefix(&format!("{app}: trap {kind} at main.paka:"))
        .and_then(|r| r.strip_suffix('\n'));
    let numbers: Vec<_> = rest.map(|r| r.split(':').collect()).unwrap_or_default();
    numbers.len() == 2
        && numbers
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Exactly one line, which `paka check` may follow with field lines (spec/paka.md §12).
fn reject(out: &Output, line: &[u8], check: bool) -> Result<(), String> {
    let want = text(line);
    let want = want
        .strip_suffix('\n')
        .ok_or("reject lacks its line feed")?;
    let got = text(&out.stderr);
    let mut lines = got.split_inclusive('\n');
    let first = lines.next().unwrap_or_default();
    let fields = lines.all(|l| check && field_line(l));
    let pass = fields
        && if want.starts_with("main.paka: ") {
            first.starts_with(want) && first.ends_with('\n')
        } else {
            let rest = first.strip_prefix(want).and_then(|r| r.strip_suffix('\n'));
            rest.is_some_and(|r| r.len() > 2 && r.starts_with(": "))
        };
    if out.status.code() != Some(1) || !out.stdout.is_empty() || !pass {
        return Err(format!(
            "status {:?}, stderr {got:?}, want {want}",
            out.status.code()
        ));
    }
    Ok(())
}

/// `  NAME: value` and a line feed (spec/paka.md §12).
fn field_line(line: &str) -> bool {
    let field = line.strip_prefix("  ").and_then(|l| l.strip_suffix('\n'));
    let Some((name, value)) = field.and_then(|f| f.split_once(": ")) else {
        return false;
    };
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase()) && !value.is_empty()
}

fn outcome(case: &Path, run: &Path, out: &Output) -> Result<(), String> {
    if let Some(line) = read(case, "reject") {
        let check = text(&read(case, "command").unwrap_or_default()).starts_with("check");
        return reject(out, &line, check);
    }
    let stdout = read(case, "stdout").ok_or("no stdout, reject or expect.json")?;
    let status = read(case, "status").ok_or("no status")?;
    want_eq("stdout", &out.stdout, &stdout)?;
    let code = text(&status);
    let code = code.strip_suffix('\n').and_then(|c| c.parse().ok());
    status_is(out, code.ok_or("status is not digits and a line feed")?)?;
    match read(case, "trap") {
        Some(kind) => {
            let app = app_name(&read(run, "main.paka").unwrap_or_default());
            let kind = text(&kind);
            let kind = kind.strip_suffix('\n').ok_or("trap lacks its line feed")?;
            match trap_line(&text(&out.stderr), &app, kind) {
                true if code == Some(3) => Ok(()),
                _ => Err(format!(
                    "stderr {:?}, want a {kind} trap",
                    text(&out.stderr)
                )),
            }
        }
        None => want_eq(
            "stderr",
            &out.stderr,
            &read(case, "stderr").unwrap_or_default(),
        ),
    }
}

fn lines(case: &Path, name: &str) -> Vec<String> {
    let bytes = read(case, name).unwrap_or_default();
    text(&bytes).lines().map(String::from).collect()
}

/// `after/`, `absent` and `present` against the folder the command ran in.
fn left_behind(case: &Path, run: &Path) -> Result<(), String> {
    let after = case.join("after");
    let mut stack = vec![after.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path.strip_prefix(&after).unwrap();
            let got = fs::read(run.join(relative)).map_err(|e| format!("{relative:?}: {e}"))?;
            want_eq(
                &format!("after/{}", relative.display()),
                &got,
                &fs::read(&path).unwrap(),
            )?;
        }
    }
    for path in lines(case, "absent") {
        if run.join(&path).symlink_metadata().is_ok() {
            return Err(format!("{path} exists"));
        }
    }
    for path in lines(case, "present") {
        if !run.join(&path).exists() {
            return Err(format!("{path} does not exist"));
        }
    }
    Ok(())
}

/// One `errors/` case: `expect.json` against `paka check --json`, or the first line of `run` or
/// `build` for a kind with `via`.
fn error_case(run: &Path, expect: &[u8]) -> Result<(), String> {
    let Json::Object(want) = Json::parse(&text(expect))? else {
        return Err("expect.json is not an object".into());
    };
    let kind = match want.get("kind") {
        Some(Json::String(kind)) => kind.clone(),
        _ => return Err("expect.json has no kind".into()),
    };
    let args: &[&str] = match want.get("via") {
        Some(Json::String(v)) if v == "run" => &["run", "main.paka"],
        Some(Json::String(v)) if v == "build" => {
            build_runtime();
            &["build", "main.paka", "-o", "out"]
        }
        Some(v) => return Err(format!("via {v:?}")),
        None => &[],
    };
    if !args.is_empty() {
        let out = paka(run, args);
        status_is(&out, 1)?;
        let prefix = via_prefix(run, &want, &kind)?;
        let got = text(&out.stderr);
        let first = got.lines().next().unwrap_or_default();
        return match first.starts_with(&prefix) {
            true => Ok(()),
            false => Err(format!("first stderr line {first:?}, want {prefix:?}")),
        };
    }
    let out = paka(run, &["check", "main.paka", "--json"]);
    status_is(&out, 1)?;
    want_eq("stdout", &out.stdout, b"")?;
    let Json::Object(got) = Json::parse(&text(&out.stderr))? else {
        return Err("stderr is not one JSON object".into());
    };
    for (key, value) in &want {
        if got.get(key) != Some(value) {
            return Err(format!("{key}: {:?}, want {value:?}", got.get(key)));
        }
    }
    Ok(())
}

/// The start of a `via` case's first line: positioned when `expect.json` gives `line`, `column`
/// and `text`, and then the source must hold `text` there.
fn via_prefix(run: &Path, want: &BTreeMap<String, Json>, kind: &str) -> Result<String, String> {
    let at = ["line", "column", "text"].map(|k| want.get(k));
    let [
        Some(Json::Number(line)),
        Some(Json::Number(col)),
        Some(Json::String(t)),
    ] = at
    else {
        return match at {
            [None, None, None] => Ok(format!("main.paka: error[{kind}]: ")),
            _ => Err("via takes line, column and text together or none of them".into()),
        };
    };
    let (line, col) = (*line as usize, *col as usize);
    let source = text(&read(run, "main.paka").unwrap_or_default());
    let source_line = source.lines().nth(line - 1).unwrap_or_default().as_bytes();
    if !source_line
        .get(col - 1..)
        .unwrap_or_default()
        .starts_with(t.as_bytes())
    {
        return Err(format!("main.paka has no {t:?} at {line}:{col}"));
    }
    Ok(format!("main.paka:{line}:{col}: error[{kind}]: "))
}

fn paka(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new(PAKA).args(args).current_dir(dir).output();
    out.unwrap()
}

fn run_case(case: &Path) -> Result<(), String> {
    for entry in fs::read_dir(case).unwrap() {
        let name = entry.unwrap().file_name();
        if !KNOWN.contains(&name.to_str().unwrap_or_default()) {
            return Err(format!("unknown file {name:?}"));
        }
    }
    let name = case.strip_prefix(ROOT).unwrap().display().to_string();
    let scratch = Scratch::new(&format!("conformance-{}", name.replace('/', "-")));
    let run = scratch.path("");
    stage(case, &run);
    if read(case, "main.paka").is_none() {
        return Err("no main.paka".into());
    }
    if let Some(expect) = read(case, "expect.json") {
        return error_case(&run, &expect);
    }
    if text(&read(case, "command").unwrap_or_default()).starts_with("build") {
        build_runtime();
    }
    let out = command(case).current_dir(&run).output();
    let out = out.map_err(|e| e.to_string())?;
    outcome(case, &run, &out)?;
    left_behind(case, &run)
}

fn cases() -> Vec<PathBuf> {
    let areas = subfolders(Path::new(ROOT));
    areas.iter().flat_map(|a| subfolders(a)).collect()
}

/// Every case with its outcome, run once for all tests.
fn results() -> &'static [(PathBuf, Result<(), String>)] {
    static RESULTS: OnceLock<Vec<(PathBuf, Result<(), String>)>> = OnceLock::new();
    RESULTS.get_or_init(|| {
        let run = |case: PathBuf| (run_case(&case), case);
        cases().into_iter().map(run).map(|(r, c)| (c, r)).collect()
    })
}

#[test]
fn every_case_passes() {
    let results = results();
    assert!(results.len() > 900, "only {} cases found", results.len());
    let failures: Vec<_> = results
        .iter()
        .filter_map(|(case, result)| result.as_ref().err().map(|e| (case, e)))
        .map(|(case, e)| format!("{}: {e}", case.strip_prefix(ROOT).unwrap().display()))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases fail:\n{}",
        failures.len(),
        results.len(),
        failures.join("\n")
    );
}

fn expectations() -> Vec<(PathBuf, BTreeMap<String, Json>)> {
    let errors = subfolders(&Path::new(ROOT).join("errors"));
    let parse = |dir: &PathBuf| match Json::parse(&text(&read(dir, "expect.json").unwrap())) {
        Ok(Json::Object(map)) => (dir.clone(), map),
        _ => panic!("{}: expect.json is not an object", dir.display()),
    };
    errors.iter().map(parse).collect()
}

/// The codes of `KINDS` in the compiler's source.
fn kinds() -> Vec<String> {
    let source = include_str!("../src/diagnostic.rs");
    let table = source.split("const KINDS").nth(1).unwrap();
    let table = table.split("\n];").next().unwrap();
    let names = table.lines().filter_map(|l| l.trim().strip_prefix("(\""));
    names
        .map(|l| l.split('"').next().unwrap().to_string())
        .collect()
}

#[test]
fn every_error_kind_has_a_case() {
    let kinds = kinds();
    assert!(kinds.len() > 50, "only {} kinds read", kinds.len());
    let covered: BTreeSet<_> = expectations()
        .into_iter()
        .map(|(_, e)| match e.get("kind") {
            Some(Json::String(kind)) => kind.clone(),
            _ => panic!("a case has no kind"),
        })
        .collect();
    let missing: Vec<_> = kinds
        .iter()
        .filter(|k| {
            !covered.contains(*k)
                && !ENVIRONMENT.contains(&k.as_str())
                && !MIGRATE.contains(&k.as_str())
        })
        .collect();
    let unknown: Vec<_> = covered.iter().filter(|k| !kinds.contains(k)).collect();
    assert!(missing.is_empty(), "kinds without a case: {missing:?}");
    assert!(unknown.is_empty(), "cases of no kind: {unknown:?}");
}

#[test]
fn every_expected_fix_checks() {
    let mut fixes = 0;
    for (dir, expect) in expectations() {
        let Some(Json::Object(fix)) = expect.get("fix") else {
            continue;
        };
        let at = |key: &str| match fix.get(key) {
            Some(Json::Number(n)) => *n as usize,
            _ => panic!("{}: fix has no {key}", dir.display()),
        };
        let Some(Json::String(replacement)) = fix.get("replacement") else {
            panic!("{}: fix has no replacement", dir.display());
        };
        let mut source = read(&dir, "main.paka").unwrap();
        source.splice(at("start")..at("end"), replacement.bytes());
        let scratch = Scratch::new(&format!("fix-{}", dir.file_name().unwrap().display()));
        fs::write(scratch.path("main.paka"), source).unwrap();
        let out = paka(&scratch.path(""), &["check", "main.paka"]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}: {}",
            dir.display(),
            text(&out.stderr)
        );
        fixes += 1;
    }
    assert!(fixes > 0, "no case holds a fix");
}

/// Just enough JSON for `expect.json` and the `check --json` object.
#[derive(Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// One value and nothing but white space after it.
    fn parse(source: &str) -> Result<Json, String> {
        let mut chars = source.chars().peekable();
        let value = Json::value(&mut chars)?;
        match chars.all(char::is_whitespace) {
            true => Ok(value),
            false => Err(format!("more than one JSON value in {source:?}")),
        }
    }

    fn value(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Json, String> {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let word = |chars: &mut std::iter::Peekable<std::str::Chars>, w: &str, v| {
            w.chars().all(|c| chars.next() == Some(c)).then_some(v)
        };
        let value = match chars.peek().copied() {
            Some('n') => word(chars, "null", Json::Null),
            Some('t') => word(chars, "true", Json::Bool(true)),
            Some('f') => word(chars, "false", Json::Bool(false)),
            Some('"') => Json::string(chars).map(Json::String),
            Some('[') => {
                chars.next();
                let mut items = vec![];
                while let Some(item) = Json::item(chars, ']')? {
                    items.push(item);
                }
                Some(Json::Array(items))
            }
            Some('{') => {
                chars.next();
                let mut map = BTreeMap::new();
                while let Some(Json::String(key)) = Json::item(chars, '}')? {
                    while chars.next_if(|c| c.is_whitespace() || *c == ':').is_some() {}
                    map.insert(key, Json::value(chars)?);
                }
                Some(Json::Object(map))
            }
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let mut number = String::new();
                while let Some(c) = chars.next_if(|c| "+-.eE".contains(*c) || c.is_ascii_digit()) {
                    number.push(c);
                }
                number.parse().ok().map(Json::Number)
            }
            _ => None,
        };
        value.ok_or_else(|| "bad JSON".to_string())
    }

    /// The next item of an array or object, or `None` at its closing `close`.
    fn item(
        chars: &mut std::iter::Peekable<std::str::Chars>,
        close: char,
    ) -> Result<Option<Json>, String> {
        while chars.next_if(|c| c.is_whitespace() || *c == ',').is_some() {}
        match chars.next_if_eq(&close) {
            Some(_) => Ok(None),
            None => Json::value(chars).map(Some),
        }
    }

    fn string(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
        chars.next();
        let mut out = String::new();
        let mut high = None;
        loop {
            let c = match chars.next()? {
                '"' => return Some(out),
                '\\' => match chars.next()? {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'u' => {
                        let hex: String = chars.take(4).collect();
                        let unit = u32::from_str_radix(&hex, 16).ok()?;
                        if (0xD800..0xDC00).contains(&unit) {
                            high = Some(unit);
                            continue;
                        }
                        let code = match high.take() {
                            Some(h) => 0x10000 + ((h - 0xD800) << 10) + (unit - 0xDC00),
                            None => unit,
                        };
                        char::from_u32(code)?
                    }
                    c => c,
                },
                c => c,
            };
            out.push(c);
        }
    }
}

mod self_test {
    use super::*;

    #[test]
    fn json_reads_nested_values_and_escapes() {
        let got = Json::parse(r#" {"a": [1, -2.5e1, null], "b": {"c": "x\né😀\"" }, "d": true} "#);
        let Ok(Json::Object(map)) = got else {
            panic!("{got:?}")
        };
        assert_eq!(
            map["a"],
            Json::Array(vec![Json::Number(1.0), Json::Number(-25.0), Json::Null])
        );
        let Json::Object(b) = &map["b"] else { panic!() };
        assert_eq!(b["c"], Json::String("x\né😀\"".into()));
        assert_eq!(map["d"], Json::Bool(true));
        assert!(Json::parse("{} {}").is_err());
        assert!(Json::parse("nul").is_err());
    }

    #[test]
    fn field_lines_name_a_field() {
        assert!(field_line("  text: \"&xs\"\n"));
        assert!(!field_line("  text:\n"));
        assert!(!field_line("   text: x\n"));
        assert!(!field_line("  Text: x\n"));
        assert!(!field_line("  text: x"));
        assert!(!field_line("main.paka: error[no-app]: x\n"));
    }

    #[test]
    fn trap_line_takes_any_position_only() {
        assert!(trap_line(
            "app: trap stack at main.paka:3:14\n",
            "app",
            "stack"
        ));
        assert!(!trap_line(
            "app: trap stack at main.paka:3:14\nx\n",
            "app",
            "stack"
        ));
        assert!(!trap_line(
            "app: trap memory at main.paka:3:14\n",
            "app",
            "stack"
        ));
        assert!(!trap_line(
            "other: trap stack at main.paka:3:14\n",
            "app",
            "stack"
        ));
        assert!(!trap_line(
            "app: trap stack at main.paka:3:\n",
            "app",
            "stack"
        ));
    }

    #[test]
    fn kinds_are_read_from_the_source() {
        let kinds = kinds();
        assert_eq!(kinds.first().map(String::as_str), Some("file-unreadable"));
        assert!(kinds.iter().any(|k| k == "wildcard-on-enum"));
    }
}
