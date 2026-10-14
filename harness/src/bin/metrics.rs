//! `metrics <runs-root> <rulings-file> [reference-tokens]`: prints the three metrics and the
//! verdict (see `harness::metrics` and the README). Status 20 when the run set is incomplete.

use harness::{Error, metrics};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

fn run(args: &[String]) -> Result<metrics::Report, Error> {
    let [root, rulings, rest @ ..] = args else {
        return Err(Error::Malformed(
            "usage: metrics <runs-root> <rulings-file> [reference-tokens]".into(),
        ));
    };
    let reference = match rest {
        [] => None,
        [n] => Some(
            n.parse()
                .map_err(|_| Error::Malformed(format!("{n} is not a token count")))?,
        ),
        _ => return Err(Error::Malformed("too many arguments".into())),
    };
    metrics::report(Path::new(root), &fs::read_to_string(rulings)?, reference)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(report) => {
            print!("{}", report.text);
            if report.complete {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(20)
            }
        }
        Err(e) => {
            eprintln!("metrics: {e}");
            ExitCode::FAILURE
        }
    }
}
