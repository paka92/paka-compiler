//! `batch <runs-root> <reps> <task>...`: runs every task in all three languages, reps 1 to
//! `<reps>`, with the model and settings of `experiment/protocol.md`. Runs that already ended are
//! skipped, so the same command resumes. The token ledger is `<runs-root>/usage.json`. Status 20
//! when the checkout is not the clean `run` commit and a counted task is asked for.

use harness::Error;
use harness::runner::{Lang, Runner};
use harness::trial::{self, Setup, Spec};
use harness::usage::{DEFAULT_CAPS, Ledger, Tokens};
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
    let [root, reps, tasks @ ..] = args else {
        return Err(Error::Malformed(
            "usage: batch <runs-root> <reps> <task>...".into(),
        ));
    };
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
        repo: Some(repo.into()),
    };
    let specs: Vec<Spec> = tasks
        .iter()
        .flat_map(|task| LANGS.map(|lang| (task, lang)))
        .flat_map(|(task, lang)| {
            (1..=reps).map(move |rep| Spec {
                task: repo.join("experiment/tasks").join(task),
                lang,
                rep,
                first_call_input: None,
            })
        })
        .collect();
    trial::run_batch(&setup, &specs)
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
