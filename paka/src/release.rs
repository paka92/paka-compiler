//! Runs a release binary's function example in its own process (M7-11b, docs/decisions/M7-03.md
//! 1.1) and trusts only the exact pass: status 0, standard output `NAME pass` and nothing on
//! standard error.

use std::fmt;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long one example may run.
pub const LIMIT: Duration = Duration::from_secs(10);

/// How a run of one example ended.
#[derive(Debug, PartialEq)]
pub enum Ran {
    Pass,
    Fail,
    /// The binary's trap line.
    Trapped(String),
    TimedOut,
    /// Any other end, as the status and output seen.
    Other(String),
}

impl fmt::Display for Ran {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ran::Pass => f.write_str("pass"),
            Ran::Fail => f.write_str("fail"),
            Ran::Trapped(line) => write!(f, "trapped: {line}"),
            Ran::TimedOut => f.write_str("timed out"),
            Ran::Other(seen) => f.write_str(seen),
        }
    }
}

/// Runs example `name` of `exe`, killed past `LIMIT`.
pub fn run(exe: &Path, name: &str) -> io::Result<Ran> {
    let mut child = (Command::new(exe).args(["--paka-examples", name]))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let read = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = vec![];
            pipe.read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let out = read(Box::new(child.stdout.take().expect("piped")));
    let err = read(Box::new(child.stderr.take().expect("piped")));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > LIMIT {
            child.kill()?;
            child.wait()?;
            return Ok(Ran::TimedOut);
        }
        thread::sleep(Duration::from_millis(2));
    };
    let joined = |t: thread::JoinHandle<io::Result<Vec<u8>>>| t.join().expect("reader");
    let (out, err) = (joined(out)?, joined(err)?);
    let [out, err] = [out, err].map(|b| String::from_utf8_lossy(&b).into_owned());
    let trap = (err.strip_suffix('\n')).filter(|l| !l.contains('\n') && l.contains(": trap "));
    Ok(match (status.code(), out.as_str(), err.as_str(), trap) {
        (Some(0), o, "", _) if o == format!("{name} pass\n") => Ran::Pass,
        (Some(0), o, "", _) if o == format!("{name} fail\n") => Ran::Fail,
        (Some(3), "", _, Some(line)) => Ran::Trapped(line.into()),
        (code, o, e, _) => {
            let ended = code.map_or("killed by a signal".into(), |c| format!("status {c}"));
            Ran::Other(format!(
                "{ended}, standard output {o:?}, standard error {e:?}"
            ))
        }
    })
}
