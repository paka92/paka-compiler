//! Case runner: every folder under `tests/cases/<stage>/` against its hand-written expectations
//! (spec/m1.md section 11). The stage names the command: `check-text`, `check-json` and `examples`
//! run the command they are named for, every other stage runs `paka run`.
//! `run-interp` holds the run cases that only the interpreter runs until M4-40 (`native.rs` takes `run`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn read(case: &Path, name: &str) -> Option<Vec<u8>> {
    fs::read(case.join(name)).ok()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn check(case: &Path, mut command: Command) -> Result<(), String> {
    let (stdout, status, stderr, reject) = (
        read(case, "stdout"),
        read(case, "status"),
        read(case, "stderr"),
        read(case, "reject"),
    );
    if read(case, "main.paka").is_none() {
        return Err("no main.paka".into());
    }
    let out = command
        .current_dir(case)
        .output()
        .map_err(|e| e.to_string())?;
    let code = out.status.code().ok_or("ended by a signal")?;
    match (stdout, status, reject) {
        (Some(stdout), Some(status), None) => {
            let want_stderr = stderr.unwrap_or_default();
            if out.stdout != stdout {
                return Err(format!(
                    "stdout {:?}, want {:?}",
                    text(&out.stdout),
                    text(&stdout)
                ));
            }
            if out.stderr != want_stderr {
                return Err(format!(
                    "stderr {:?}, want {:?}",
                    text(&out.stderr),
                    text(&want_stderr)
                ));
            }
            if format!("{code}\n").as_bytes() != status {
                return Err(format!("status {code}, want {:?}", text(&status)));
            }
        }
        (None, None, Some(reject)) if stderr.is_none() => {
            let want = text(&reject);
            let want = want
                .strip_suffix('\n')
                .ok_or("reject lacks its line feed")?;
            let got = text(&out.stderr);
            let line = got.strip_suffix('\n').filter(|l| !l.contains('\n'));
            let rest = line.and_then(|l| l.strip_prefix(want));
            let message_follows = rest.is_some_and(|r| r.len() > 2 && r.starts_with(": "));
            if code != 1 || !out.stdout.is_empty() || !message_follows {
                return Err(format!("status {code}, stderr {got:?}, want {want}"));
            }
        }
        _ => return Err("expectation files are not exactly an accept or a reject case".into()),
    }
    Ok(())
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

fn args(stage: &Path) -> &'static [&'static str] {
    match stage.file_name().and_then(|n| n.to_str()) {
        Some("check-text") => &["check", "main.paka"],
        Some("check-json") => &["check", "main.paka", "--json"],
        Some("examples") => &["examples", "main.paka"],
        _ => &["run", "main.paka"],
    }
}

#[test]
fn all_cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases");
    let mut count = 0;
    let mut failures = vec![];
    for stage in subfolders(&root) {
        for case in subfolders(&stage) {
            count += 1;
            let mut command = Command::new(env!("CARGO_BIN_EXE_paka"));
            command.args(args(&stage));
            if let Err(e) = check(&case, command) {
                failures.push(format!("{}: {e}", case.display()));
            }
        }
    }
    assert!(count > 0, "no cases found");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

mod self_test {
    use super::*;

    type Files<'a> = Vec<(&'a str, &'a str)>;

    /// Makes a case folder holding `files`; the shell script stands in for `paka`.
    fn run(name: &str, files: &Files, script: &str) -> Result<(), String> {
        let case = std::env::temp_dir().join(format!("paka-runner-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&case);
        fs::create_dir_all(&case).unwrap();
        for (file, content) in files {
            fs::write(case.join(file), content).unwrap();
        }
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        let result = check(&case, command);
        fs::remove_dir_all(&case).unwrap();
        result
    }

    fn accept<'a>() -> Files<'a> {
        vec![("main.paka", ""), ("stdout", "hi\n"), ("status", "3\n")]
    }

    fn reject<'a>() -> Files<'a> {
        vec![("main.paka", ""), ("reject", "main.paka:2:5: error[x]\n")]
    }

    const SAY: &str = "echo hi; exit 3";
    const DIAGNOSE: &str = "echo 'main.paka:2:5: error[x]: words' >&2; exit 1";

    #[test]
    fn matching_cases_pass() {
        assert_eq!(run("accept", &accept(), SAY), Ok(()));
        assert_eq!(run("reject", &reject(), DIAGNOSE), Ok(()));
        let mut files = accept();
        files.push(("stderr", "e"));
        assert_eq!(
            run("stderr", &files, "echo hi; printf e >&2; exit 3"),
            Ok(())
        );
    }

    #[test]
    fn wrong_stdout_byte_fails() {
        assert!(run("stdout", &accept(), "echo ho; exit 3").is_err());
    }

    #[test]
    fn missing_final_newline_fails() {
        assert!(run("newline", &accept(), "printf hi; exit 3").is_err());
    }

    #[test]
    fn wrong_status_fails() {
        assert!(run("status", &accept(), "echo hi; exit 4").is_err());
    }

    #[test]
    fn wrong_standard_error_fails() {
        let extra = "echo hi; printf e >&2; exit 3";
        assert!(run("stderr-extra", &accept(), extra).is_err());
        let mut files = accept();
        files.push(("stderr", "e"));
        assert!(run("stderr-byte", &files, "echo hi; printf f >&2; exit 3").is_err());
    }

    #[test]
    fn wrong_line_column_or_code_fails() {
        for (name, script) in [
            ("line", DIAGNOSE.replace(":2:", ":3:")),
            ("column", DIAGNOSE.replace(":5:", ":6:")),
            ("code", DIAGNOSE.replace("[x]", "[y]")),
            ("status", DIAGNOSE.replace("exit 1", "exit 2")),
        ] {
            assert!(run(name, &reject(), &script).is_err(), "{name}");
        }
    }

    #[test]
    fn folder_without_expectation_fails() {
        assert!(run("none", &vec![("main.paka", "")], "exit 0").is_err());
    }

    #[test]
    fn folder_without_source_fails() {
        assert!(run("source", &accept()[1..].to_vec(), SAY).is_err());
    }
}
