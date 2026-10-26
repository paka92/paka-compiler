use std::fs::File;
use std::io::pipe;
use std::process::{Command, Output, Stdio};

const USAGE: &str = "usage: paka run <path> [argument...]\n       paka build <path> -o <output>\n       paka check <path> [--json]\n       paka examples <path>\n       paka serve <path>\n       paka approve <path> [--adopt <name>] [--drop <name>] [--key <file>]\n       paka gate <tree> --base <tree>\n";

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
fn arguments_after_the_file_are_not_files() {
    assert_unreadable(&["run", "no/such/file.paka", "b.paka"], "no/such/file.paka");
}

/// A folder under the temporary directory holding `files`, each a relative path and its text.
fn program_dir(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("paka-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

const HELLO: &str = "enum Msg { Start }\nfn update(m: Msg) -> List<Effect<Msg>> {\n  \
    [Print(greeting()), Exit]\n}\napp hi {\n  start: Start\n  update: update\n  grants: [Console]\n}\n";

#[test]
fn directory_is_one_program_of_the_paka_files_directly_in_it() {
    let dir = program_dir(
        "dir-run",
        &[
            ("a.paka", "fn greeting() -> String {\n  \"hello\"\n}\n"),
            ("b.paka", HELLO),
            ("notes.txt", "not paka"),
            ("sub/c.paka", "not paka"),
            ("d.paka/e.paka", "not paka"),
        ],
    );
    let out = paka(&["run", dir.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(stderr_of(&out), "");
    assert_eq!(
        (out.status.code(), &out.stdout[..]),
        (Some(0), &b"hello\n"[..])
    );
}

#[test]
fn directory_diagnostic_names_its_file_and_counts_in_it() {
    // Byte order reads `Z.paka` first, so the later `f` is the one in `a.paka`.
    let files = [
        ("a.paka", "\nfn f() -> Int { 2 }\n"),
        ("Z.paka", "fn f() -> Int { 1 }\n"),
    ];
    let dir = program_dir("dir-duplicate", &files);
    let path = format!("{}/", dir.display());
    let (text, json) = (paka(&["check", &path]), paka(&["check", &path, "--json"]));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((text.status.code(), json.status.code()), (Some(1), Some(1)));
    let want = format!("{path}a.paka:2:4: error[duplicate-name]: ");
    assert!(stderr_of(&text).starts_with(&want), "{}", stderr_of(&text));
    let want = format!("\"file\":\"{path}a.paka\",\"start\":4,\"end\":5,\"line\":2,\"column\":4,");
    assert!(stderr_of(&json).contains(&want), "{}", stderr_of(&json));
}

#[test]
fn directory_duplicate_local_counts_its_line_in_the_file() {
    let files = [
        ("a.paka", "\n\n\nfn f() -> Int { 1 }\n"),
        ("b.paka", "fn g(x: Int) -> Int {\n  let x = 1\n  x\n}\n"),
    ];
    let dir = program_dir("dir-local", &files);
    let out = paka(&["check", dir.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    let want = format!("{}/b.paka:2:7: error[duplicate-local]: ", dir.display());
    assert!(stderr_of(&out).starts_with(&want), "{}", stderr_of(&out));
    assert!(
        stderr_of(&out).contains("x is already bound at line 1"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn directory_paka_entry_that_cannot_be_read_is_unreadable() {
    let dir = program_dir("dir-dangling", &[("a.paka", HELLO)]);
    std::os::unix::fs::symlink(dir.join("gone"), dir.join("b.paka")).unwrap();
    let path = dir.to_str().unwrap();
    assert_unreadable(&["run", path], &format!("{path}/b.paka"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn directory_that_cannot_be_searched_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = program_dir("dir-unsearchable", &[("a.paka", HELLO)]);
    let mode = |m| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(m)).unwrap();
    mode(0o644);
    // As root the mode does not stop a search, so there is nothing to test.
    let searchable = std::fs::metadata(dir.join("a.paka")).is_ok();
    let path = dir.to_str().unwrap();
    if !searchable {
        assert_unreadable(&["run", path], &format!("{path}/a.paka"));
    }
    mode(0o755);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn directory_file_name_is_written_as_its_bytes() {
    use std::os::unix::ffi::OsStrExt;
    let dir = program_dir("dir-bytes", &[]);
    let name = std::ffi::OsStr::from_bytes(b"\xff.paka");
    std::fs::write(dir.join(name), "fn f() -> Int { 1 }\nfn f() -> Int { 2 }\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_paka"))
        .args(["check".as_ref(), dir.as_os_str()])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let want = [
        dir.as_os_str().as_bytes(),
        b"/\xff.paka:2:4: error[duplicate-name]: ",
    ]
    .concat();
    assert!(out.stderr.starts_with(&want), "{:?}", out.stderr);
}

#[test]
fn directory_without_paka_files_has_no_app() {
    let dir = program_dir("dir-empty", &[("notes.txt", "not paka")]);
    let out = paka(&["run", dir.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    let want = format!("{}: error[no-app]: ", dir.display());
    assert!(stderr_of(&out).starts_with(&want), "{}", stderr_of(&out));
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
    let source = std::fs::read_to_string(LIGHT).unwrap();
    let file = std::env::temp_dir().join(format!("paka-map-build-{}.paka", std::process::id()));
    let map = "let ages = [\"ana\": 3]\n  [Print";
    std::fs::write(&file, source.replace("[Print", map)).unwrap();
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
fn deep_expressions_do_not_lower_the_call_depth_limit() {
    let helper = format!(
        "fn down(n: Int) -> Int {{\n  if n == 0 {{ 0 }} else {{ {}down(n - 1){} }}\n}}\n",
        "(1 + ".repeat(10),
        ")".repeat(10)
    );
    for (n, status) in [(9998, 0), (9999, 3)] {
        let out = run_program("depth-limit", &helper, &format!("  let r = down({n})"));
        assert_eq!(out.status.code(), Some(status), "down({n})");
        assert_eq!(out.stderr.is_empty(), status == 0, "down({n})");
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
