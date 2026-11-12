//! Builds one submission and runs a task's cases against it, in Paka, Rust or TypeScript.
//! Every build and every case runs in the sandbox of `Runner::sandbox`.

use crate::Error;
use crate::task::{Case, Task};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
/// bwrap's fixed options; `cc` links through /etc/alternatives.
const SANDBOX: &str = "--unshare-all --die-with-parent --new-session --ro-bind /usr /usr \
    --symlink usr/lib /lib --symlink usr/lib64 /lib64 --proc /proc --dev /dev --remount-ro /dev \
    --ro-bind-try /etc/alternatives /etc/alternatives";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Paka,
    Rust,
    TypeScript,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::Paka => "Paka",
            Lang::Rust => "Rust",
            Lang::TypeScript => "TypeScript",
        }
    }

    pub fn file(self) -> &'static str {
        match self {
            Lang::Paka => "main.paka",
            Lang::Rust => "main.rs",
            Lang::TypeScript => "main.ts",
        }
    }

    /// The command line that builds the file, with the paths the sandbox shows.
    fn build(self) -> &'static [&'static str] {
        match self {
            Lang::Paka => &["/tools/bin/paka", "check", "/build/main.paka"],
            Lang::Rust => &[
                "/tools/rust/bin/rustc",
                "--edition",
                "2024",
                "-C",
                "debug-assertions=on",
                "-C",
                "overflow-checks=on",
                "-o",
                "/build/main",
                "/build/main.rs",
            ],
            Lang::TypeScript => &[
                "/tools/ts/node_modules/.bin/tsc",
                "--strict",
                "--target",
                "es2022",
                "--module",
                "commonjs",
                "--types",
                "node",
                "--typeRoots",
                "/tools/ts/node_modules/@types",
                "--noEmitOnError",
                "--skipLibCheck",
                "--rootDir",
                "/build",
                "--outDir",
                "/build/out",
                "/build/main.ts",
            ],
        }
    }

    /// The command line that runs the built program, before the case arguments.
    fn run(self) -> &'static [&'static str] {
        match self {
            Lang::Paka => &["/tools/bin/paka", "run", "/build/main.paka"],
            Lang::Rust => &["/build/main"],
            Lang::TypeScript => &["/tools/bin/node", "/build/out/main.js"],
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
    /// The compiler's full text when the build failed; no case is run then.
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
        let env = ["PATH=/tools/bin:/usr/bin", "TMPDIR=/build"];
        let mut cmd = confined(BUILD_MEMORY, &env);
        cmd.args(lang.build());
        let done = exec(&mut cmd, &self.sandbox(dir, None), self.limits.build)?;
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
        Ok(Err(String::from_utf8_lossy(&text).into_owned()))
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
        let mut cmd = confined(CASE_MEMORY, &["TZ=UTC", "LC_ALL=C"]);
        cmd.args(lang.run()).args(&case.args);
        let run = exec(
            &mut cmd,
            &self.sandbox(build, Some(&work.0)),
            self.limits.case,
        )?;
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

    /// bwrap's options, NUL-separated: no network, its own process tree that dies with the
    /// sandbox, and a read-only view of /usr and the toolchains under /tools. The build directory
    /// shows as /build, writable while building; a case also gets `work` as /work, the only
    /// writable directory then. The task folder and every other host path are absent.
    fn sandbox(&self, build: &Path, work: Option<&Path>) -> Vec<u8> {
        let mut opts: Vec<&OsStr> = SANDBOX.split(' ').map(OsStr::new).collect();
        let mut binds: Vec<(&str, &OsStr, &str)> = vec![
            (
                "--ro-bind-try",
                TS_MODULES.as_ref(),
                "/tools/ts/node_modules",
            ),
            ("--ro-bind", self.rust.as_ref(), "/tools/rust"),
            ("--ro-bind", self.paka.as_ref(), "/tools/bin/paka"),
            ("--ro-bind", self.node.as_ref(), "/tools/bin/node"),
        ];
        let cwd = match work {
            None => {
                binds.push(("--bind", build.as_ref(), "/build"));
                "/build"
            }
            Some(work) => {
                binds.push(("--ro-bind", build.as_ref(), "/build"));
                binds.push(("--bind", work.as_ref(), "/work"));
                "/work"
            }
        };
        for (how, host, inside) in binds {
            opts.extend([OsStr::new(how), host, OsStr::new(inside)]);
        }
        opts.extend(["--remount-ro", "/", "--chdir", cwd].map(OsStr::new));
        let mut bytes = vec![];
        for opt in opts {
            bytes.extend(opt.as_bytes());
            bytes.push(0);
        }
        bytes
    }
}

/// bwrap reading its options from standard input, so they stay out of the sandbox's
/// /proc/1/cmdline; then prlimit's caps on memory per process, file size and processes with core
/// dumps off; then `env -i` with only `env`. The program's command line goes after.
fn confined(memory: u64, env: &[&str]) -> Command {
    let mut cmd = Command::new(BWRAP);
    cmd.env_clear()
        .args(["--args", "0", "--", PRLIMIT])
        .arg(format!("--data={memory}"))
        .arg(format!("--fsize={FILE_CAP}"))
        .arg(format!("--nproc={TASK_CAP}"))
        .args(["--core=0", "--", "/usr/bin/env", "-i"])
        .args(env);
    cmd
}

struct Exec {
    /// None when killed by the harness.
    status: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Runs the command with `input` as its whole standard input, to its end, until `limit` or until
/// an output goes past `OUTPUT_CAP`; then it is killed. Killing the sandbox kills every process
/// inside it.
fn exec(cmd: &mut Command, input: &[u8], limit: Duration) -> io::Result<Exec> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("piped").write_all(input)?;
    let full = Arc::new(AtomicBool::new(false));
    let stdout = drain(child.stdout.take().expect("piped"), full.clone());
    let stderr = drain(child.stderr.take().expect("piped"), full.clone());
    let deadline = Instant::now() + limit;
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        if !full.load(Ordering::Relaxed) {
            if Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            timed_out = true;
        }
        child.kill()?;
        child.wait()?;
        break None;
    };
    Ok(Exec {
        status,
        timed_out,
        stdout: stdout.join().expect("reader thread"),
        stderr: stderr.join().expect("reader thread"),
    })
}

/// Reads the pipe up to `OUTPUT_CAP` bytes; one byte more sets `full`.
fn drain(pipe: impl Read + Send + 'static, full: Arc<AtomicBool>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = vec![];
        let _ = pipe.take(OUTPUT_CAP as u64 + 1).read_to_end(&mut bytes);
        if bytes.len() > OUTPUT_CAP {
            bytes.truncate(OUTPUT_CAP);
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
