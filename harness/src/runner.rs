//! Builds one submission and runs a task's cases against it, in Paka, Rust or TypeScript.
//! Programs run in temporary directories; the sandbox around them is a later step.

use crate::Error;
use crate::task::{Case, Task};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Compiler text sent back after a failed build is cut at this many bytes.
const BUILD_TEXT_CAP: usize = 8_000;

const TSC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ts/node_modules/.bin/tsc");
const TS_TYPES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ts/node_modules/@types");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Paka,
    Rust,
    TypeScript,
}

impl Lang {
    fn file(self) -> &'static str {
        match self {
            Lang::Paka => "main.paka",
            Lang::Rust => "main.rs",
            Lang::TypeScript => "main.ts",
        }
    }

    /// The command that builds the file in `dir`.
    fn build(self, runner: &Runner, dir: &Path) -> Command {
        let file = dir.join(self.file());
        let mut cmd;
        match self {
            Lang::Paka => {
                cmd = Command::new(&runner.paka);
                cmd.arg("check").arg(file);
            }
            Lang::Rust => {
                cmd = Command::new("rustc");
                cmd.args([
                    "--edition",
                    "2024",
                    "-C",
                    "debug-assertions=on",
                    "-C",
                    "overflow-checks=on",
                ])
                .arg("-o")
                .arg(dir.join("main"))
                .arg(file);
            }
            Lang::TypeScript => {
                cmd = Command::new(TSC);
                cmd.args([
                    "--strict", "--target", "es2022", "--module", "nodenext", "--types", "node",
                ])
                .args(["--typeRoots", TS_TYPES, "--noEmitOnError", "--skipLibCheck"])
                .arg("--rootDir")
                .arg(dir)
                .arg("--outDir")
                .arg(dir.join("out"))
                .arg(file);
            }
        }
        cmd
    }

    /// The command that runs the program built in `dir`, before the case arguments.
    fn run(self, runner: &Runner, dir: &Path) -> Command {
        match self {
            Lang::Paka => {
                let mut cmd = Command::new(&runner.paka);
                cmd.arg("run").arg(dir.join(self.file()));
                cmd
            }
            Lang::Rust => Command::new(dir.join("main")),
            Lang::TypeScript => {
                let mut cmd = Command::new("node");
                cmd.arg(dir.join("out/main.js"));
                cmd
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub build: Duration,
    pub case: Duration,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            build: Duration::from_secs(120),
            case: Duration::from_secs(10),
        }
    }
}

pub struct Runner {
    /// The `paka` binary.
    pub paka: PathBuf,
    pub limits: Limits,
}

/// How one case went. A case that fails in several ways gets the first of these in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Timeout,
    Stdout,
    Status,
    Files,
}

pub struct CaseResult {
    pub name: String,
    pub verdict: Verdict,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// None when the program was killed by the timeout or a signal.
    pub status: Option<i32>,
}

pub struct Report {
    /// The compiler's text when the build failed; no case is run then.
    pub build: Result<(), String>,
    pub visible: Vec<CaseResult>,
    pub hidden: Vec<CaseResult>,
}

impl Report {
    pub fn passed(&self) -> bool {
        let ok = |cases: &[CaseResult]| cases.iter().all(|c| c.verdict == Verdict::Pass);
        self.build.is_ok() && ok(&self.visible) && ok(&self.hidden)
    }
}

impl Runner {
    /// Builds `source` and runs every visible and hidden case of `task` against it.
    pub fn judge(&self, lang: Lang, source: &str, task: &Task) -> Result<Report, Error> {
        let build = Scratch::new()?;
        fs::write(build.0.join(lang.file()), source)?;
        let mut report = Report {
            build: self.build(lang, &build.0)?,
            visible: vec![],
            hidden: vec![],
        };
        if report.build.is_ok() {
            report.visible = self.run_all(lang, &build.0, &task.visible)?;
            report.hidden = self.run_all(lang, &build.0, &task.hidden)?;
        }
        Ok(report)
    }

    fn build(&self, lang: Lang, dir: &Path) -> Result<Result<(), String>, Error> {
        let done = exec(lang.build(self, dir).current_dir(dir), self.limits.build)?;
        if done.status == Some(0) {
            return Ok(Ok(()));
        }
        if done.timed_out {
            return Ok(Err(format!(
                "build timed out after {} seconds",
                self.limits.build.as_secs()
            )));
        }
        let text = [done.stdout, done.stderr].concat();
        let kept = &text[..text.len().min(BUILD_TEXT_CAP)];
        let note = if kept.len() < text.len() {
            format!("\n[cut after {BUILD_TEXT_CAP} bytes]")
        } else {
            String::new()
        };
        Ok(Err(String::from_utf8_lossy(kept).into_owned() + &note))
    }

    fn run_all(&self, lang: Lang, build: &Path, cases: &[Case]) -> Result<Vec<CaseResult>, Error> {
        cases
            .iter()
            .map(|case| self.run_case(lang, build, case))
            .collect()
    }

    /// The one place a submitted program runs: in a fresh directory holding the case's input
    /// files, with an empty environment but PATH, TZ=UTC and the C locale.
    fn run_case(&self, lang: Lang, build: &Path, case: &Case) -> Result<CaseResult, Error> {
        let work = Scratch::new()?;
        for (name, bytes) in &case.input {
            fs::write(work.0.join(name), bytes)?;
        }
        let mut cmd = lang.run(self, build);
        cmd.args(&case.args)
            .current_dir(&work.0)
            .env_clear()
            .envs([("TZ", "UTC"), ("LC_ALL", "C")]);
        cmd.envs(env::var_os("PATH").map(|path| ("PATH", path)));
        let run = exec(&mut cmd, self.limits.case)?;
        let files_match = case
            .output
            .iter()
            .all(|(name, bytes)| fs::read(work.0.join(name)).is_ok_and(|got| got == *bytes));
        let verdict = if run.timed_out {
            Verdict::Timeout
        } else if run.stdout != case.stdout {
            Verdict::Stdout
        } else if run.status != Some(case.status) {
            Verdict::Status
        } else if !files_match {
            Verdict::Files
        } else {
            Verdict::Pass
        };
        Ok(CaseResult {
            name: case.name.clone(),
            verdict,
            stdout: run.stdout,
            stderr: run.stderr,
            status: run.status,
        })
    }
}

struct Exec {
    status: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Runs the command with no input, to its end or until `limit`, when it is killed.
fn exec(cmd: &mut Command, limit: Duration) -> io::Result<Exec> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = drain(child.stdout.take().expect("piped"));
    let stderr = drain(child.stderr.take().expect("piped"));
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    Ok(Exec {
        status: status.and_then(|s| s.code()),
        timed_out: status.is_none(),
        stdout: stdout.join().expect("reader thread"),
        stderr: stderr.join().expect("reader thread"),
    })
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = vec![];
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// A new empty temporary directory, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> io::Result<Scratch> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("harness-{}-{n}", std::process::id()));
        fs::create_dir(&dir)?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
