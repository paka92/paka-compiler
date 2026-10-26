//! The server's side of the dev runner (docs/decisions/M4-04.md): one child process, sent the
//! runnable program when it changes, or only its changed files when its paths stay, and each run
//! after. A run that does not answer by its limit and
//! a little more, or a runner that dies, ends the child; the next run starts another. Look calls
//! go to the app the runner shows in a window (M5-38).

use super::visual::not_running;
use crate::serve::{Call, Failure, Reply};
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
/// How long a Look call may take.
const LOOK: Duration = Duration::from_secs(60);

pub struct Runner {
    exe: PathBuf,
    live: Option<Live>,
}

struct Live {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<String>,
    /// The generation of the program it holds, and its files.
    loaded: Option<u64>,
    held: Vec<(String, String)>,
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
            let file = |(path, text): &(String, String)| {
                Value::object([
                    ("path", path.as_str().into()),
                    ("text", text.as_str().into()),
                ])
            };
            let same = live.loaded.is_some() && paths(&live.held).eq(paths(files));
            let load = match same {
                true => {
                    let changed = (files.iter().zip(&live.held)).filter(|(a, b)| a != b);
                    Value::object([(
                        "change",
                        Value::Array(changed.map(|(f, _)| file(f)).collect()),
                    )])
                }
                false => Value::object([("files", Value::Array(files.iter().map(file).collect()))]),
            };
            if live.ask(&load, LOAD).is_err() {
                return Ok(self.end("crash"));
            }
            (live.loaded, live.held) = (Some(generation), files.to_vec());
        }
        let limit = Duration::from_millis(order.get("limit_ms").and_then(Value::as_u64).unwrap());
        match live.ask(order, limit + GRACE) {
            Ok(reply) => Ok(reply),
            Err(why) => Ok(self.end(why)),
        }
    }

    /// The Look `call` sent on to the runner's app; `not_running` with no runner, or when it
    /// does not answer.
    pub fn look(&mut self, call: &Call) -> Reply {
        let live = self.live.as_mut().ok_or_else(not_running)?;
        match live.ask(call.request(), LOOK) {
            Ok(reply) => relayed(reply),
            Err(_) => {
                self.end("");
                Err(not_running())
            }
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
            held: vec![],
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
        // One write: `Display` would send each piece of the request in its own system call.
        self.stdin
            .write_all(format!("{request}\n").as_bytes())
            .map_err(|_| "crash")?;
        self.stdin.flush().map_err(|_| "crash")?;
        match self.replies.recv_timeout(wait) {
            Ok(line) => parse(&line).map_err(|_| "crash"),
            Err(RecvTimeoutError::Timeout) => Err("time"),
            Err(RecvTimeoutError::Disconnected) => Err("crash"),
        }
    }
}

/// The runner's reply line to a Look call as this server's reply. Its fields and error kinds are
/// those of docs/decisions/M5-09.md section 2.
fn relayed(reply: Value) -> Reply {
    const FIELDS: [&str; 6] = ["tree", "running", "trap", "png", "width", "height"];
    const KINDS: [&str; 4] = ["not_running", "no_element", "no_focus", "bad_request"];
    let known = |names: &[&'static str], name: &str| names.iter().copied().find(|n| *n == name);
    if let Some(error) = reply.get("error") {
        let kind = error.get("kind").and_then(Value::as_str);
        let message = error.get("message").and_then(Value::as_str);
        let kind = kind.and_then(|k| known(&KINDS, k)).unwrap_or("not_running");
        return Err(Failure::new(kind, message.unwrap_or_default()));
    }
    let Value::Object(fields) = reply else {
        return Err(not_running());
    };
    let fields = fields
        .into_iter()
        .filter_map(|(k, v)| Some((known(&FIELDS, &k)?, v)));
    Ok(fields.collect())
}

fn paths(files: &[(String, String)]) -> impl Iterator<Item = &str> {
    files.iter().map(|(path, _)| path.as_str())
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.end("");
    }
}
