//! What a run sends the model (M3-01 section 2): the system text, the first message with the
//! statement and the visible cases, and the feedback after each failed submission (section 1.3).
//! Nothing here reads a hidden case's text; of hidden cases the feedback gives only counts.

use crate::json::quote;
use crate::runner::{CaseResult, Lang, Report, Verdict};
use crate::task::{Case, Task};
use std::fmt::Write;

/// Submissions per run: the first try and at most seven edits (M3-01 section 1.2).
pub const SUBMISSIONS: u32 = 8;
/// A failing visible case's standard output and standard error are each cut here.
const SHOWN_CAP: usize = 2_000;

/// The system text; Paka runs carry the language reference after it.
pub fn system(lang: Lang, reference: &str) -> String {
    let (build, rule) = match lang {
        Lang::Paka => (
            "paka check main.paka",
            "the built-ins only; the app may list any grant the reference names",
        ),
        Lang::Rust => (
            "rustc --edition 2024 -C debug-assertions=on -C overflow-checks=on -o main main.rs",
            "the standard library only",
        ),
        Lang::TypeScript => (
            "tsc --strict --noEmitOnError --target es2022 --module commonjs --types node \
             --outDir out main.ts",
            "Node's built-in modules only",
        ),
    };
    let mut text = format!(
        "You write one program in {}. Reply with the whole file {} in one\n\
         code block. Any other text is allowed; only the code block is used.\n\
         The harness builds it with: {build}\n\
         It runs the program on cases. Input comes as files named in the arguments.\n\
         Output is standard output, the exit status and, where the task says so, written files.\n\
         There is no standard input.\n\
         After each submission you get the build result or the case results.\n\
         You have {SUBMISSIONS} submissions.\n\
         Use {rule}.\n",
        lang.name(),
        lang.file(),
    );
    if lang == Lang::Paka {
        text += "The language reference follows.\n\n";
        text += reference;
    }
    text
}

/// The first user message: the statement with the visible cases appended.
pub fn first(task: &Task) -> String {
    let mut text = format!("{}\n\n## Visible cases\n", task.statement.trim_end());
    for (n, case) in task.visible.iter().enumerate() {
        let _ = write!(text, "\n### Case {}\n\nArguments: {}\n", n + 1, args(case));
        for (name, bytes) in &case.input {
            text += &block(&format!("File `{name}`"), bytes);
        }
        text += &block("Expected standard output", &case.stdout);
        for (name, bytes) in &case.output {
            text += &block(&format!("Expected written file `{name}`"), bytes);
        }
        let _ = writeln!(text, "\nExpected exit status: {}", case.status);
    }
    text
}

/// The feedback when a reply holds no usable program.
pub fn no_program(why: &str, n: u32) -> String {
    format!("no program: {why}\nsubmission {n} of {SUBMISSIONS}\n")
}

/// The feedback after submission `n` failed: the compiler's text, or each failing visible case in
/// full and the hidden cases counted.
pub fn feedback(task: &Task, report: &Report, n: u32) -> String {
    let mut text = match &report.build {
        Err(compiler) => format!("build: failed\n{}\n", compiler.trim_end()),
        Ok(()) => "build: ok\n".into(),
    };
    if report.build.is_ok() {
        for (k, (case, got)) in task.visible.iter().zip(&report.visible).enumerate() {
            if got.verdict != Verdict::Pass {
                text += &failing(k + 1, case, got);
            }
        }
        text += &hidden(task, report);
    }
    let _ = writeln!(text, "submission {n} of {SUBMISSIONS}");
    text
}

/// A labelled code block, noting what a code block cannot show.
fn block(label: &str, bytes: &[u8]) -> String {
    let body = String::from_utf8_lossy(bytes);
    let (note, end) = if body.is_empty() {
        (" (empty)", "")
    } else if body.ends_with('\n') {
        ("", "")
    } else {
        (" (no line feed at the end)", "\n")
    };
    format!("\n{label}{note}:\n```\n{body}{end}```\n")
}

fn args(case: &Case) -> String {
    match case.args.is_empty() {
        true => "(none)".into(),
        false => case
            .args
            .iter()
            .map(|a| quote(a))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn failing(n: usize, case: &Case, got: &CaseResult) -> String {
    let status = match (got.verdict, got.status) {
        (Verdict::Timeout, _) => "timed out".into(),
        (_, None) => "killed".into(),
        (_, Some(status)) => status.to_string(),
    };
    let mut text = format!(
        "visible case {n}: FAIL\n  args: {}\n  expected stdout: {}   status {}\n  \
         got stdout:      {}   status {status}\n  stderr: {}\n",
        args(case),
        quote(&String::from_utf8_lossy(&case.stdout)),
        case.status,
        shown(&got.stdout),
        match got.stderr.is_empty() {
            true => "(empty)".into(),
            false => shown(&got.stderr),
        },
    );
    if got.verdict == Verdict::Files {
        let names: Vec<&str> = case.output.iter().map(|(name, _)| name.as_str()).collect();
        let _ = writeln!(
            text,
            "  written files: missing or different ({})",
            names.join(", ")
        );
    }
    text
}

/// Program output as a quoted string, cut after `SHOWN_CAP` bytes with a note.
fn shown(bytes: &[u8]) -> String {
    let kept = &bytes[..bytes.len().min(SHOWN_CAP)];
    let text = quote(&String::from_utf8_lossy(kept));
    match kept.len() < bytes.len() {
        true => format!("{text} [cut after {SHOWN_CAP} bytes]"),
        false => text,
    }
}

/// How many hidden cases pass, and each failure counted under the first way that applies:
/// over time, wrong exit status, wrong output, wrong written file.
fn hidden(task: &Task, report: &Report) -> String {
    const WAYS: [&str; 4] = [
        "over time",
        "wrong exit status",
        "wrong output",
        "wrong written file",
    ];
    let mut counts = [0; 4];
    for (case, got) in task.hidden.iter().zip(&report.hidden) {
        let way = match got.verdict {
            Verdict::Pass => continue,
            Verdict::Timeout => 0,
            _ if got.status != Some(case.status) => 1,
            _ if got.stdout != case.stdout => 2,
            _ => 3,
        };
        counts[way] += 1;
    }
    let failed: u32 = counts.iter().sum();
    let total = report.hidden.len();
    let mut text = format!("hidden cases: {} of {total} pass", total - failed as usize);
    let ways: Vec<String> = WAYS
        .iter()
        .zip(counts)
        .filter(|(_, count)| *count > 0)
        .map(|(way, count)| format!("{count} {way}"))
        .collect();
    if !ways.is_empty() {
        let _ = write!(text, " ({})", ways.join(", "));
    }
    text + "\n"
}
