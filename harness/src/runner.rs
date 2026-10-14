//! Builds one submission and runs a task's cases against it, in Paka, Rust or TypeScript.
//! Every build and every case runs in the sandbox of `Runner::sandbox`.

use crate::Error;
use crate::task::{Case, Task};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Compiler text sent back after a failed build is cut at this many bytes.
const BUILD_TEXT_CAP: usize = 8_000;

/// Standard output and standard error are each cut at this many bytes; reaching it kills the program.
pub const OUTPUT_CAP: usize = 1 << 20;
/// Memory per process of a case (M3-01) and of a build, which links with many threads.
const CASE_MEMORY: u64 = 1 << 30;
const BUILD_MEMORY: u64 = 4 << 30;
/// Largest file a build or a case may write.
const FILE_CAP: u64 = 64 << 20;
/// Processes and threads inside one sandbox, so a fork bomb stops.
const TASK_CAP: u32 = 256;

const BWRAP: &str = "/usr/bin/bwrap";
const PRLIMIT: &str = "/usr/bin/prlimit";
const TS_MODULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ts/node_modules");
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
                cmd = Command::new(runner.rust.join("bin/rustc"));
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
                    "--strict", "--target", "es2022", "--module", "commonjs", "--types", "node",
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
                let mut cmd = Command::new(&runner.node);
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
    /// Absolute path of `node`, found once on PATH: cases run without a PATH.
    pub node: PathBuf,
    /// The sysroot of the pinned Rust toolchain; `rustc` runs from it, not through rustup.
    pub rust: PathBuf,
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
    /// None when the harness killed the program: timeout or output past the cap; a signal shows
    /// as 128 plus its number.
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
    pub fn new(paka: PathBuf) -> io::Result<Runner> {
        let path = env::var_os("PATH").unwrap_or_default();
        let node = env::split_paths(&path)
            .map(|dir| dir.join("node"))
            .find(|node| node.is_file())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "node is not on PATH"))?;
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()?;
        if !sysroot.status.success() {
            return Err(io::Error::other("rustc --print sysroot failed"));
        }
        let rust = String::from_utf8_lossy(&sysroot.stdout).trim().into();
        Ok(Runner {
            paka,
            node,
            rust,
            limits: Limits::default(),
        })
    }

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

    /// Builds in `dir`, which also takes the compilers' temporary files.
    fn build(&self, lang: Lang, dir: &Path) -> Result<Result<(), String>, Error> {
        let mut cmd = lang.build(self, dir);
        cmd.current_dir(dir)
            .envs([("PATH", "/usr/bin".as_ref()), ("TMPDIR", dir.as_os_str())]);
        let mut cmd = self.sandbox(&cmd, dir, None, BUILD_MEMORY);
        let done = exec(&mut cmd, self.limits.build)?;
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
    /// files, with an empty environment but TZ=UTC and the C locale, seeing its build read-only.
    fn run_case(&self, lang: Lang, build: &Path, case: &Case) -> Result<CaseResult, Error> {
        let work = Scratch::new()?;
        for (name, bytes) in &case.input {
            fs::write(work.0.join(name), bytes)?;
        }
        let mut cmd = lang.run(self, build);
        cmd.args(&case.args)
            .current_dir(&work.0)
            .envs([("TZ", "UTC"), ("LC_ALL", "C")]);
        let mut cmd = self.sandbox(&cmd, &work.0, Some(build), CASE_MEMORY);
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

    /// `cmd` inside bubblewrap: no network, its own process tree that dies with the sandbox, and
    /// a read-only view of /usr, the toolchains and `read`. `write` is the only writable
    /// directory; the task folder and every other path are absent. prlimit caps memory per
    /// process, file size and processes, and turns off core dumps; `env -i` leaves only the
    /// variables set on `cmd`.
    fn sandbox(&self, cmd: &Command, write: &Path, read: Option<&Path>, memory: u64) -> Command {
        let mut sb = Command::new(BWRAP);
        sb.args(["--unshare-all", "--die-with-parent", "--new-session"])
            .args(["--ro-bind", "/usr", "/usr", "--symlink", "usr/lib", "/lib"])
            .args(["--symlink", "usr/lib64", "/lib64", "--proc", "/proc"])
            .args(["--dev", "/dev", "--remount-ro", "/dev"]);
        for path in ["/etc/alternatives", TS_MODULES] {
            sb.args(["--ro-bind-try", path, path]);
        }
        let tools = [&self.rust, &self.paka, &self.node].map(PathBuf::as_path);
        for path in tools.into_iter().chain(read) {
            sb.arg("--ro-bind").arg(path).arg(path);
        }
        sb.arg("--bind").arg(write).arg(write);
        sb.args(["--remount-ro", "/"]);
        if let Some(dir) = cmd.get_current_dir() {
            sb.arg("--chdir").arg(dir);
        }
        sb.args(["--", PRLIMIT])
            .arg(format!("--data={memory}"))
            .arg(format!("--fsize={FILE_CAP}"))
            .arg(format!("--nproc={TASK_CAP}"))
            .args(["--core=0", "--", "/usr/bin/env", "-i"]);
        for (key, value) in cmd.get_envs() {
            if let Some(value) = value {
                sb.arg([key, value].join("=".as_ref()));
            }
        }
        sb.arg(cmd.get_program()).args(cmd.get_args());
        sb
    }
}

struct Exec {
    /// None when killed by the harness.
    status: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Runs the command with no input, to its end, until `limit` or until an output reaches
/// `OUTPUT_CAP`; then it is killed. Killing the sandbox kills every process inside it.
fn exec(cmd: &mut Command, limit: Duration) -> io::Result<Exec> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let full = Arc::new(AtomicBool::new(false));
    let stdout = drain(child.stdout.take().expect("piped"), full.clone());
    let stderr = drain(child.stderr.take().expect("piped"), full.clone());
    let deadline = Instant::now() + limit;
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        timed_out = Instant::now() >= deadline;
        if timed_out || full.load(Ordering::Relaxed) {
            child.kill()?;
            child.wait()?;
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    Ok(Exec {
        status,
        timed_out,
        stdout: stdout.join().expect("reader thread"),
        stderr: stderr.join().expect("reader thread"),
    })
}

/// Reads the pipe up to `OUTPUT_CAP` bytes; reaching it sets `full`.
fn drain(pipe: impl Read + Send + 'static, full: Arc<AtomicBool>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = vec![];
        let _ = pipe.take(OUTPUT_CAP as u64).read_to_end(&mut bytes);
        if bytes.len() == OUTPUT_CAP {
            full.store(true, Ordering::Relaxed);
        }
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
