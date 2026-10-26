//! The caller's half of a host: it starts the process and moves frames over its pipes, laid out
//! as `host.c` describes. Whatever the host does, the caller gets an answer or a `Failure`.

use paka_rt::ops::Float;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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
    /// The reply is not an answer of the expected kind, or the host refused the request.
    BadReply,
}

/// A running host, killed when dropped.
pub struct Host {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
}

impl Host {
    /// Starts the host at `exe`; its standard error and directory are the caller's.
    pub fn start(exe: &Path) -> std::io::Result<Host> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let input = child.stdin.take().expect("piped");
        let output = child.stdout.take().expect("piped");
        Ok(Host {
            child,
            input,
            output,
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
        let mut head = [0; 8];
        self.output
            .read_exact(&mut head)
            .map_err(|_| Failure::Died)?;
        let len = u64::from_le_bytes(head);
        // The longest reply is a status and 8 bytes.
        if len > 9 {
            return Err(Failure::BadReply);
        }
        let mut reply = vec![0; len as usize];
        self.output
            .read_exact(&mut reply)
            .map_err(|_| Failure::Died)?;
        Ok(reply)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
