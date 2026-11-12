//! `metrics <runs-root> <rulings-file> [reference-tokens]`: prints the three metrics and the
//! verdict (see `harness::metrics` and the README). Run folders must belong to the counted or
//! reserve tasks of `experiment/protocol.md`, or of the file `PAKA_PROTOCOL` names. Status 20
//! when the run set is incomplete.

use harness::{Error, metrics};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

const PROTOCOL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../experiment/protocol.md");

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
    let protocol = env::var_os("PAKA_PROTOCOL").map_or(PathBuf::from(PROTOCOL), PathBuf::from);
    metrics::check_tasks(Path::new(root), &fs::read_to_string(protocol)?)?;
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
            ExitCode::from(e.status())
        }
    }
}
