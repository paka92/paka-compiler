use std::fs::File;
use std::io::pipe;
use std::process::{Command, Output, Stdio};

const USAGE: &str = "usage: paka run <file>\n       paka build <file> -o <output>\n       paka check <file> [--json]\n";

fn paka(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_paka"))
        .args(args)
        .output()
        .unwrap()
}

fn assert_usage(args: &[&str]) {
    let out = paka(args);
    assert_eq!(out.status.code(), Some(2), "args {args:?}");
    assert!(out.stdout.is_empty(), "args {args:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), USAGE, "args {args:?}");
}

#[test]
fn bad_arguments_print_usage_and_exit_2() {
    assert_usage(&[]);
    assert_usage(&["run"]);
    assert_usage(&["run", "a.paka", "b.paka"]);
    assert_usage(&["build", "a.paka"]);
    assert_usage(&["build", "a.paka", "-o"]);
    assert_usage(&["build", "a.paka", "out", "-o"]);
    assert_usage(&["build", "a.paka", "-o", "out", "extra"]);
    assert_usage(&["fmt", "a.paka"]);
    assert_usage(&["check", "a.paka", "b.paka"]);
}

fn assert_unreadable(args: &[&str], path: &str) {
    let out = paka(args);
    assert_eq!(out.status.code(), Some(1), "args {args:?}");
    assert!(out.stdout.is_empty(), "args {args:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    let prefix = format!("{path}: error[file-unreadable]: ");
    let rest = stderr
        .strip_prefix(&prefix)
        .unwrap_or_else(|| panic!("{stderr:?}"));
    let message = rest
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("{stderr:?}"));
    assert!(!message.is_empty() && !message.contains('\n'), "{stderr:?}");
}

#[test]
fn missing_file_is_one_line_and_exit_1() {
    assert_unreadable(&["run", "no/such/file.paka"], "no/such/file.paka");
    let args = ["build", "no/such/file.paka", "-o", "out"];
    assert_unreadable(&args, "no/such/file.paka");
}

#[test]
fn directory_is_unreadable() {
    let dir = env!("CARGO_MANIFEST_DIR");
    assert_unreadable(&["run", dir], dir);
}

#[test]
fn lexical_reject_is_one_diagnostic_and_exit_1() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/cases/lex/unterminated_string"
    );
    for args in [
        vec!["run", "main.paka"],
        vec!["build", "main.paka", "-o", "unused"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_paka"))
            .args(&args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "args {args:?}");
        assert!(out.stdout.is_empty(), "args {args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        let rest = stderr.strip_prefix("main.paka:4:10: error[unterminated-string]: ");
        let message = rest.and_then(|r| r.strip_suffix('\n'));
        assert!(
            message.is_some_and(|m| !m.is_empty() && !m.contains('\n')),
            "{stderr:?}"
        );
    }
}

const LIGHT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");

#[test]
fn run_of_light_prints_the_line_and_exits_0() {
    let out = paka(&["run", LIGHT]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, b"let there be light\n");
    assert!(out.stderr.is_empty());
}

/// Runs `paka run` on light.paka with the given standard output.
fn run_light_into(stdout: Stdio) -> Output {
    Command::new(env!("CARGO_BIN_EXE_paka"))
        .args(["run", LIGHT])
        .stdout(stdout)
        .output()
        .unwrap()
}

#[test]
fn run_without_the_console_grant_is_refused() {
    let source = std::fs::read_to_string(LIGHT).unwrap();
    let file = std::env::temp_dir().join(format!("paka-no-console-{}.paka", std::process::id()));
    std::fs::write(&file, source.replace("[Console]", "[]")).unwrap();
    let out = paka(&["run", file.to_str().unwrap()]);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    let want = "light: effect Print refused: grant Console is missing\n";
    assert_eq!(String::from_utf8_lossy(&out.stderr), want);
}

/// The light program with its text replaced by an interpolation, which the back end cannot build.
fn interpolated_light(name: &str) -> std::path::PathBuf {
    let source = std::fs::read_to_string(LIGHT).unwrap();
    let file = std::env::temp_dir().join(format!("paka-{name}-{}.paka", std::process::id()));
    std::fs::write(&file, source.replace("light\")", "{1}\")")).unwrap();
    file
}

#[test]
fn run_of_an_interpolated_program_prints_it() {
    let file = interpolated_light("interpolated-run");
    let out = paka(&["run", file.to_str().unwrap()]);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "let there be 1\n");
    assert!(out.stderr.is_empty());
}

#[test]
fn build_of_a_program_beyond_the_m1_subset_is_unsupported_form() {
    let file = interpolated_light("interpolated-build");
    let output = file.with_extension("out");
    let out = paka(&[
        "build",
        file.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ]);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(!output.exists());
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("error[unsupported-form]"), "{text}");
}

#[test]
fn full_disk_gives_status_3_and_its_line() {
    let full = File::options().write(true).open("/dev/full").unwrap();
    let out = run_light_into(full.into());
    assert_eq!(out.status.code(), Some(3));
    let want = "light: effect Print failed: no space left on device\n";
    assert_eq!(String::from_utf8_lossy(&out.stderr), want);
}

#[test]
fn closed_pipe_gives_status_3_and_its_line() {
    let (reader, writer) = pipe().unwrap();
    drop(reader);
    let out = run_light_into(writer.into());
    assert_eq!(out.status.code(), Some(3), "ended by a signal?");
    let want = "light: effect Print failed: broken pipe\n";
    assert_eq!(String::from_utf8_lossy(&out.stderr), want);
}

/// Runs `paka run` on a program whose `update` body is `body`, then `[Exit]`; `helper` goes before it.
fn run_program(name: &str, helper: &str, body: &str) -> Output {
    let source = format!(
        "enum Msg {{ Start }}\n\n{helper}\n\
         fn update(m: Msg) -> List<Effect<Msg>> {{\n{body}\n  [Exit]\n}}\n\n\
         app t {{\n  start:  Start\n  update: update\n  grants: [Console]\n}}\n"
    );
    let file = std::env::temp_dir().join(format!("paka-{name}-{}.paka", std::process::id()));
    std::fs::write(&file, source).unwrap();
    let out = paka(&["run", file.to_str().unwrap()]);
    std::fs::remove_file(&file).unwrap();
    out
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn an_unimplemented_built_in_is_one_internal_error_line() {
    let out = run_program("len", "", "  let n = len([1, 2])");
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    assert_eq!(
        stderr_of(&out),
        "t: internal error: built-in len is not implemented yet\n"
    );
}

#[test]
fn deep_expression_nesting_traps_stack_without_a_signal() {
    let not = format!(
        "fn down(n: Int) -> Bool {{\n  {}down(n + 1)\n}}\n",
        "!".repeat(64)
    );
    let sum = format!(
        "fn down(n: Int) -> Int {{\n  {}down(n + 1){}\n}}\n",
        "(1 + ".repeat(60),
        ")".repeat(60)
    );
    for (name, helper) in [("nested-not", not), ("nested-sum", sum)] {
        let out = run_program(name, &helper, "  let r = down(0)");
        assert_eq!(out.status.code(), Some(3), "{name}");
        assert!(out.stdout.is_empty(), "{name}");
        let stderr = stderr_of(&out);
        let (head, tail) = stderr.split_once(&format!("paka-{name}-")).unwrap();
        assert_eq!(head, "t: trap stack at ", "{name}");
        assert_eq!(tail.matches('\n').count(), 1, "{name}: {stderr:?}");
    }
}

#[test]
fn to_int_shows_its_operand_in_the_value_text_form() {
    let cases = [
        (
            "1000000000000000000000000000000.0",
            "1000000000000000000000000000000.0",
        ),
        ("9223372036854775807.0", "9223372036854776000.0"),
    ];
    for (operand, shown) in cases {
        let out = run_program("to-int", "", &format!("  let n = to_int({operand})"));
        assert_eq!(out.status.code(), Some(3), "{operand}");
        let stderr = stderr_of(&out);
        let lines: Vec<_> = stderr.lines().collect();
        assert_eq!(lines[1], format!("  to_int({shown})"), "{operand}");
    }
}
