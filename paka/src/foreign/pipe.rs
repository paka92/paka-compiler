//! The caller's half of a host: it starts the process and moves frames over its pipes, laid out
//! as `host.c` describes. Whatever the host does, the caller gets an answer or a `Failure`, by
//! the time limit at the latest: a thread reads the replies.

use paka_rt::ops::Float;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

/// An argument or a result; a result is never a String.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value<'a> {
    Int(i64),
    Float(Float),
    Bool(bool),
    String(&'a str),
}

/// The type of a result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Int,
    Float,
    Bool,
}

#[derive(Debug, PartialEq)]
pub enum Failure {
    /// The pipe broke: the host exited or was killed.
    Died,
    /// No reply came within the time limit.
    TimedOut,
    /// The reply is not an answer of the expected kind, or the host refused the request.
    BadReply,
}

impl Failure {
    /// The reason a program is given (docs/decisions/M7-02.md section 2).
    pub fn reason(&self) -> &'static str {
        match self {
            Failure::Died => "host died",
            Failure::TimedOut => "timed out",
            Failure::BadReply => "bad reply",
        }
    }
}

/// A running host, killed when dropped.
pub struct Host {
    child: Child,
    input: ChildStdin,
    replies: Receiver<Result<Vec<u8>, Failure>>,
    limit: Duration,
}

impl Host {
    /// Starts the host at `exe`, each call answered within `limit`; its standard error and
    /// directory are the caller's.
    pub fn start(exe: &Path, limit: Duration) -> std::io::Result<Host> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let input = child.stdin.take().expect("piped");
        let output = child.stdout.take().expect("piped");
        let (send, replies) = mpsc::channel();
        thread::spawn(move || read_replies(output, &send));
        Ok(Host {
            child,
            input,
            replies,
            limit,
        })
    }

    /// Calls function `n` with `args`; its result is of kind `result`.
    pub fn call(
        &mut self,
        n: u32,
        args: &[Value],
        result: Option<Kind>,
    ) -> Result<Option<Value<'static>>, Failure> {
        let mut request = n.to_le_bytes().to_vec();
        for arg in args {
            match *arg {
                Value::Int(i) => request.extend(i.to_le_bytes()),
                Value::Float(f) => request.extend(f.bits().to_le_bytes()),
                Value::Bool(b) => request.push(b.into()),
                Value::String(s) => {
                    request.extend((s.len() as u64).to_le_bytes());
                    request.extend(s.as_bytes());
                }
            }
        }
        let reply = self.exchange(&request)?;
        let word = || Some(u64::from_le_bytes(reply.get(1..)?.try_into().ok()?));
        let answer = match (reply.first(), result) {
            (Some(0), None) if reply.len() == 1 => Some(None),
            (Some(0), Some(Kind::Int)) => word().map(|w| Some(Value::Int(w as i64))),
            (Some(0), Some(Kind::Float)) => word().map(|w| Some(Value::Float(Float::from_bits(w)))),
            (Some(0), Some(Kind::Bool)) => match reply[1..] {
                [b @ (0 | 1)] => Some(Some(Value::Bool(b == 1))),
                _ => None,
            },
            _ => None,
        };
        answer.ok_or(Failure::BadReply)
    }

    /// Sends one request frame holding `request` and answers the reply frame's bytes.
    pub fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, Failure> {
        let mut frame = (request.len() as u64).to_le_bytes().to_vec();
        frame.extend(request);
        self.input.write_all(&frame).map_err(|_| Failure::Died)?;
        match self.replies.recv_timeout(self.limit) {
            Ok(reply) => reply,
            Err(RecvTimeoutError::Timeout) => Err(Failure::TimedOut),
            Err(RecvTimeoutError::Disconnected) => Err(Failure::Died),
        }
    }
}

/// Sends each reply frame's bytes until the pipe breaks or a frame is too long to be a reply.
fn read_replies(mut output: ChildStdout, replies: &Sender<Result<Vec<u8>, Failure>>) {
    let mut head = [0; 8];
    while output.read_exact(&mut head).is_ok() {
        let len = u64::from_le_bytes(head);
        // The longest reply is a status and 8 bytes.
        if len > 9 {
            let _ = replies.send(Err(Failure::BadReply));
            return;
        }
        let mut reply = vec![0; len as usize];
        if output.read_exact(&mut reply).is_err() || replies.send(Ok(reply)).is_err() {
            return;
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
