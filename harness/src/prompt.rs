//! What a run sends the model (M3-01 section 2): the system text, the first message with the
//! statement and the visible cases, and the feedback after each failed submission (section 1.3).
//! Nothing here reads a hidden case's text; of hidden cases the feedback gives only counts.

use crate::json::quote;
use crate::runner::{CaseResult, Lang, Report, Verdict};
use crate::task::{Case, Task};
use std::borrow::Cow;
use std::fmt::Write;

/// Submissions per run: the first try and at most seven edits (M3-01 section 1.2).
pub const SUBMISSIONS: u32 = 8;
/// A failing visible case's standard output and standard error are each cut here.
const SHOWN_CAP: usize = 2_000;
/// The compiler's text after a failed build is cut here.
const BUILD_TEXT_CAP: usize = 8_000;

/// The system text; Paka runs carry the language reference after it.
pub fn system(lang: Lang, reference: &str) -> String {
    let build = match lang {
        Lang::Paka => "paka check main.paka",
        Lang::Rust => {
            "rustc --edition 2024 -C debug-assertions=on -C overflow-checks=on -o main main.rs"
        }
        Lang::TypeScript => {
            "tsc --strict --noEmitOnError --target es2022 --module commonjs --types node \
             --outDir out main.ts"
        }
    };
    let name = lang.name();
    let mut text = format!(
        "You write one program in {name}. Reply with the whole file {} in exactly one\n\
         code block. Any other text is allowed; only the code block is used. A reply with no\n\
         code block or with several counts as a failed submission.\n\
         The harness builds it with: {build}\n\
         It runs the program on cases. Input comes as files named in the arguments.\n\
         Output is standard output, the exit status and, where the task says so, written files.\n\
         There is no standard input.\n\
         After each submission you get the build result or the case results.\n\
         You have {SUBMISSIONS} submissions.\n\
         Use only what {name} itself ships with: no outside packages.\n",
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
        Err(compiler) => {
            let (kept, cut) = cut(compiler.as_bytes(), BUILD_TEXT_CAP);
            let note = if cut {
                format!("\n[cut after {BUILD_TEXT_CAP} bytes]")
            } else {
                String::new()
            };
            format!("build: failed\n{}{note}\n", kept.trim_end())
        }
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
    if case.args.is_empty() {
        return "(none)".into();
    }
    let quoted: Vec<String> = case.args.iter().map(|a| quote(a)).collect();
    quoted.join(" ")
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
        if got.stderr.is_empty() {
            "(empty)".into()
        } else {
            shown(&got.stderr)
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
    let (kept, cut) = cut(bytes, SHOWN_CAP);
    let text = quote(&kept);
    if cut {
        format!("{text} [cut after {SHOWN_CAP} bytes]")
    } else {
        text
    }
}

/// The first `cap` bytes as text, and whether anything was cut.
fn cut(bytes: &[u8], cap: usize) -> (Cow<'_, str>, bool) {
    let kept = String::from_utf8_lossy(&bytes[..bytes.len().min(cap)]);
    (kept, bytes.len() > cap)
}

/// How many hidden cases pass, and each failure counted under the first way that applies:
/// over time or memory, wrong exit status, wrong output, wrong written file. Only a timeout is
/// known to be over time; a program stopped by the memory cap shows as a wrong exit status.
fn hidden(task: &Task, report: &Report) -> String {
    const WAYS: [&str; 4] = [
        "over time or memory",
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
