//! `paka-dev-replay <path>`: the visual app at `path` compiled by the dev back end and run with no
//! window on the clicks of standard input, as a release binary runs under `--paka-replay` (M7-03),
//! so dev and release frames can be timed alike (M6-31). Built with paka's `measure` feature, it
//! times them as a `measure` runtime does.

use paka_installer::dev::Dev;
use paka_rt::abi::measure;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [path] = &args[..] else {
        eprintln!("usage: paka-dev-replay <path>");
        return ExitCode::from(2);
    };
    let (source, read) = paka::source::read(path);
    let mut dev = Dev::default();
    if let Err(e) = read
        .map_err(|d| d.to_string())
        .and_then(|()| dev.edit(&source))
    {
        eprint!("{e}");
        return ExitCode::from(1);
    }
    let status = measure::around(|| {
        let lines = io::stdin().lock().lines().map(Result::unwrap_or_default);
        let lines: Vec<_> = lines.collect();
        let (mut out, mut err) = (io::stdout(), io::stderr());
        let (status, line) = dev.replay(lines.into_iter(), &mut out, &mut err);
        let _ = out.flush();
        let _ = err.write_all(line.as_bytes());
        status.into()
    });
    ExitCode::from(status as u8)
}
