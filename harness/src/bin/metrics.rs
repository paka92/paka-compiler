//! `metrics <runs-root> <rulings-file>`: prints the three metrics and the verdict
//! (see `harness::metrics` and the README).

use harness::{Error, metrics};
use std::path::Path;
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [root, rulings] = args.as_slice() else {
        eprintln!("usage: metrics <runs-root> <rulings-file>");
        return ExitCode::from(2);
    };
    let report = fs::read_to_string(rulings)
        .map_err(Error::from)
        .and_then(|text| metrics::report(Path::new(root), &text));
    match report {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("metrics: {e}");
            ExitCode::FAILURE
        }
    }
}
