//! The server's side of the dev runner (docs/decisions/M4-04.md): one child process, sent the
//! runnable program when it changes and each run after. A run that does not answer by its limit and
//! a little more, or a runner that dies, ends the child; the next run starts another.

use crate::serve::Failure;
use paka_json::{Value, parse};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// How long the runner may take to install a program.
const LOAD: Duration = Duration::from_secs(60);
/// How long past its limit a run's answer may take.
const GRACE: Duration = Duration::from_millis(100);

pub struct Runner {
    exe: PathBuf,
    live: Option<Live>,
}

struct Live {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<String>,
    /// The generation of the program it holds.
    loaded: Option<u64>,
}

impl Runner {
    /// A runner started from `exe` when first needed.
    pub fn new(exe: PathBuf) -> Runner {
        Runner { exe, live: None }
    }

    /// The run `order` asks for, on `files` of program generation `generation`.
    pub fn run(
        &mut self,
        (generation, files): (u64, &[(String, String)]),
        order: &Value,
    ) -> Result<Value, Failure> {
        let live = match &mut self.live {
            Some(live) => live,
            None => self.live.insert(self.start()?),
        };
        if live.loaded != Some(generation) {
            let files = files.iter().map(|(path, text)| {
                Value::object([
                    ("path", path.as_str().into()),
                    ("text", text.as_str().into()),
                ])
            });
            let load = Value::object([("files", Value::Array(files.collect()))]);
            if live.ask(&load, LOAD).is_err() {
                return Ok(self.end("crash"));
            }
            live.loaded = Some(generation);
        }
        let limit = Duration::from_millis(order.get("limit_ms").and_then(Value::as_u64).unwrap());
        match live.ask(order, limit + GRACE) {
            Ok(reply) => Ok(reply),
            Err(why) => Ok(self.end(why)),
        }
    }

    fn start(&self) -> Result<Live, Failure> {
        let spawned = (Command::new(&self.exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped()))
        .spawn();
        let mut child = spawned.map_err(|e| {
            let message = format!("cannot start {}: {e}", self.exe.display());
            Failure::new("io", message)
        })?;
        let (stdin, stdout) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        let (send, replies) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Live {
            child,
            stdin,
            replies,
            loaded: None,
        })
    }

    /// Ends the runner, and the run it was on, stopped for `why`.
    fn end(&mut self, why: &str) -> Value {
        if let Some(mut live) = self.live.take() {
            let _ = live.child.kill();
            let _ = live.child.wait();
        }
        let run = [("stdout", "".into()), ("stderr", "".into())];
        let rest = [("status", Value::Null), ("trap", Value::Null)];
        Value::object(run.into_iter().chain(rest).chain([("stopped", why.into())]))
    }
}

impl Live {
    /// The runner's answer to `request` within `wait`; else why it stopped.
    fn ask(&mut self, request: &Value, wait: Duration) -> Result<Value, &'static str> {
        writeln!(self.stdin, "{request}").map_err(|_| "crash")?;
        self.stdin.flush().map_err(|_| "crash")?;
        match self.replies.recv_timeout(wait) {
            Ok(line) => parse(&line).map_err(|_| "crash"),
            Err(RecvTimeoutError::Timeout) => Err("time"),
            Err(RecvTimeoutError::Disconnected) => Err("crash"),
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.end("");
    }
}
