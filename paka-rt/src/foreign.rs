//! Foreign calls (docs/decisions/M7-02.md section 2), shared by both back ends: the caller's half
//! of a host, which starts the process and moves frames over its pipes, laid out as
//! `paka/src/foreign/host.c` describes; the hosts of a run; and the table of foreign effects
//! compiled code gives the run loop. Whatever the host does, the caller gets an answer or a
//! `Failure`, by the time limit at the latest: a thread reads the replies.

use crate::effects::{End, Reason};
use crate::ops::Float;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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

/// How long a call may take before its host is killed.
pub const LIMIT: Duration = Duration::from_secs(10);

/// Where the host of a function's declaration lies, if it has one.
pub type Finder = dyn Fn(&Function) -> Option<PathBuf> + Send + Sync;

/// A type that crosses to C (M7-02 section 1.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plain {
    Int,
    Float,
    Bool,
    String,
}

/// Each type's letter in the table text.
const LETTERS: [(Plain, char); 4] = [
    (Plain::Int, 'i'),
    (Plain::Float, 'f'),
    (Plain::Bool, 'b'),
    (Plain::String, 's'),
];

impl Plain {
    /// The pipe's kind of a result; a result is never a String.
    pub fn kind(self) -> Kind {
        match self {
            Plain::Int => Kind::Int,
            Plain::Float => Kind::Float,
            Plain::Bool => Kind::Bool,
            Plain::String => unreachable!("a String result does not cross; the checker refuses it"),
        }
    }

    /// The words a value takes in compiled code (M4-05 section 8).
    pub fn words(self) -> u64 {
        if self == Plain::String { 4 } else { 1 }
    }

    fn letter(self) -> char {
        LETTERS
            .iter()
            .find(|(p, _)| *p == self)
            .expect("every type has a letter")
            .1
    }

    fn of(letter: char) -> Option<Plain> {
        LETTERS.iter().find(|(_, l)| *l == letter).map(|(p, _)| *p)
    }
}

/// A foreign effect as compiled code's run loop performs it: its name, its declaration's name,
/// which is its grant, whether the app holds that grant, the declaration's index, and the
/// function's number among the declaration's and its types.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub grant: String,
    pub granted: bool,
    pub library: usize,
    pub number: usize,
    pub params: Vec<Plain>,
    pub result: Option<Plain>,
}

/// The table text of `functions`: a line each of name, grant, 1 when granted else 0, the
/// declaration's index, the number, the parameters' letters and the result's, `-` for none.
pub fn table(functions: &[Function]) -> String {
    let letters = |ps: &[Plain]| match ps {
        [] => "-".to_string(),
        _ => ps.iter().map(|p| p.letter()).collect(),
    };
    let line = |f: &Function| {
        let (params, result) = (letters(&f.params), letters(f.result.as_slice()));
        let granted = u8::from(f.granted);
        let (name, grant, library, number) = (&f.name, &f.grant, f.library, f.number);
        format!("{name} {grant} {granted} {library} {number} {params} {result}\n")
    };
    functions.iter().map(line).collect()
}

/// The functions of table text `text`, or `None` when a line is malformed.
pub fn functions(text: &str) -> Option<Vec<Function>> {
    let letters = |s: &str| match s {
        "-" => Some(vec![]),
        _ => s.chars().map(Plain::of).collect(),
    };
    let function = |line: &str| {
        let [name, grant, granted, library, number, params, result] =
            line.split(' ').collect::<Vec<_>>()[..]
        else {
            return None;
        };
        let result = letters(result)?;
        Some(Function {
            name: name.into(),
            grant: grant.into(),
            granted: match granted {
                "0" => false,
                "1" => true,
                _ => return None,
            },
            library: library.parse().ok()?,
            number: number.parse().ok()?,
            params: letters(params)?,
            result: match result[..] {
                [] => None,
                [Plain::String] => return None,
                [r] => Some(r),
                _ => return None,
            },
        })
    };
    text.lines().map(function).collect()
}

/// The end of app `app` when its foreign effect `name` lacks its grant, `grant`.
pub fn refused(app: &str, name: &str, grant: &str) -> End {
    let why = format!("effect {name} refused: grant {grant} is missing");
    (3, format!("{app}: {why}\n"))
}

/// The hosts of one run, by declaration, each with the key it was started under; a call under
/// another key starts a fresh host.
pub struct Hosts<K>(Vec<Option<(K, Host)>>);

impl<K> Default for Hosts<K> {
    fn default() -> Self {
        Hosts(vec![])
    }
}

impl<K: Clone + PartialEq> Hosts<K> {
    /// Calls function `number` of declaration `at` with `args`, its host started from `exe` under
    /// `key` when none runs under it: its result, of kind `result`, or the reason it failed. A host
    /// that failed is dropped, so the next call starts a fresh one.
    pub fn call(
        &mut self,
        (at, key): (usize, &K),
        exe: impl FnOnce() -> Option<PathBuf>,
        number: usize,
        args: &[Value],
        result: Option<Kind>,
    ) -> Result<Option<Value<'static>>, Reason> {
        if self.0.len() <= at {
            self.0.resize_with(at + 1, || None);
        }
        let slot = &mut self.0[at];
        if slot.as_ref().is_none_or(|(started, _)| started != key) {
            *slot = None;
            let host = exe().and_then(|exe| Host::start(&exe, LIMIT).ok());
            *slot = Some((key.clone(), host.ok_or("no host")?));
        }
        let (_, host) = slot.as_mut().expect("started");
        let answer = host.call(number as u32, args, result);
        if answer.is_err() {
            *slot = None;
        }
        answer.map_err(|f| f.reason())
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
