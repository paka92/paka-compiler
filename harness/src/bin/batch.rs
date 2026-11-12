//! `batch [--calibrate] <runs-root> <langs> <reps> <task>...`: runs every task in each language
//! of the comma-separated `<langs>` (`Paka,Rust,TypeScript`), reps 1 to `<reps>`, with the model
//! and settings of `experiment/protocol.md`. Runs that already ended are skipped, so the same
//! command resumes. The token ledger is `<runs-root>/usage.json`. Status 20 when the checkout is
//! not the clean `run` commit and a counted task is asked for. `--calibrate` is the calibration
//! pass before the `run` tag (M3-03.4): no commit check, and Paka is refused.

use harness::runner::{Lang, Runner};
use harness::trial::{self, Setup, Spec};
use harness::usage::{DEFAULT_CAPS, Ledger, Tokens};
use harness::{Error, json};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;
use std::{env, fs};

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const MODEL: &str = "claude-sonnet-5-5";
const EFFORT: &str = "medium";
const CALL_LIMIT: Duration = Duration::from_secs(600);
const LANGS: [Lang; 3] = [Lang::Paka, Lang::Rust, Lang::TypeScript];

fn run(args: &[String]) -> Result<Tokens, Error> {
    let (calibrate, args) = match args {
        [flag, rest @ ..] if flag == "--calibrate" => (true, rest),
        _ => (false, args),
    };
    let [root, langs, reps, tasks @ ..] = args else {
        return Err(Error::Malformed(
            "usage: batch [--calibrate] <runs-root> <langs> <reps> <task>...".into(),
        ));
    };
    let langs = langs
        .split(',')
        .map(|name| {
            LANGS
                .into_iter()
                .find(|l| l.name() == name)
                .ok_or_else(|| Error::Malformed(format!("{name} is not a language")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if calibrate && langs.contains(&Lang::Paka) {
        return Err(Error::Malformed(
            "calibration runs Rust and TypeScript only".into(),
        ));
    }
    let reps: u32 = reps
        .parse()
        .map_err(|_| Error::Malformed(format!("{reps} is not a rep count")))?;
    let repo = Path::new(REPO);
    // The paka binary sits beside this one.
    let paka = env::current_exe()?.with_file_name("paka");
    let root = PathBuf::from(root);
    let setup = Setup {
        claude: "claude".into(),
        model: MODEL.into(),
        effort: EFFORT.into(),
        reference: fs::read_to_string(repo.join("spec/paka.md"))?,
        runner: Runner::new(paka)?,
        ledger: Ledger::new(root.join("usage.json"), DEFAULT_CAPS),
        call_limit: CALL_LIMIT,
        root,
        repo: (!calibrate).then(|| repo.into()),
    };
    let mut used = Tokens::default();
    for task in tasks {
        for &lang in &langs {
            for rep in 1..=reps {
                let spec = Spec {
                    task: repo.join("experiment/tasks").join(task),
                    lang,
                    rep,
                    first_call_input: None,
                };
                let spec = Spec {
                    first_call_input: first_call_input(&setup.root, &spec)?,
                    ..spec
                };
                let t = trial::run_batch(&setup, &[spec])?;
                used.input += t.input;
                used.output += t.output;
            }
        }
    }
    Ok(used)
}

/// The guard's expected first-call input: the first call of an earlier run of the same task and
/// language. Every run of a pair sends the same first call, so any growth is outside text.
fn first_call_input(root: &Path, spec: &Spec) -> Result<Option<u64>, Error> {
    let id = spec.id();
    let pair = &id[..id.rfind('-').unwrap_or(id.len()) + 1];
    let Ok(runs) = fs::read_dir(root) else {
        return Ok(None);
    };
    for run in runs {
        let run = run?.path();
        let name = run.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with(pair) || name == id {
            continue;
        }
        for attempt in fs::read_dir(&run)? {
            let Ok(text) = fs::read_to_string(attempt?.path().join("log")) else {
                continue;
            };
            let first = text.lines().filter_map(|l| json::parse(l).ok()).find(|v| {
                v.get("event").and_then(|e| e.as_str()) == Some("reply")
                    && v.get("n").and_then(|n| n.as_u64()) == Some(1)
            });
            if let Some(input) = first.and_then(|v| v.get("input")?.as_u64()) {
                return Ok(Some(input));
            }
        }
    }
    Ok(None)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(t) => {
            println!(
                "batch used {} input and {} output tokens",
                t.input, t.output
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("batch: {e}");
            ExitCode::from(e.status())
        }
    }
}
