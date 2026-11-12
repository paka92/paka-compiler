use crate::usage::Tokens;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// The usage total has reached a cap; no further call is made.
    CapReached {
        used: Tokens,
        caps: Tokens,
    },
    /// The process ended or closed its output before the turn was complete.
    Died,
    /// The stream reported a compaction: the run is void.
    Compacted,
    /// A line, a field or a file that does not have the expected shape.
    Malformed(String),
    WrongModel {
        expected: String,
        got: String,
    },
    EmptyReply,
    /// The input tokens of the first call differ from the expected count by more than the margin.
    FirstCall {
        expected: u64,
        got: u64,
    },
    /// The provider failed the turn; carries its message. The run is void.
    Provider(String),
    /// The reply is a refusal.
    Refused,
    /// The process reports a key source other than none: calls would be paid.
    ApiKey(String),
    /// The run already failed; nothing more is sent.
    Voided,
    /// The watchdog killed the process: a model call passed its time limit.
    TimedOut,
    /// A run was void three times (M3-03 section 1.1); the experiment waits for the owner.
    Paused(String),
    /// A counted batch may not start from this checkout (M3-18).
    Commit(String),
}

impl Error {
    /// The exit status of a command that stops on this error: 20 for the commit check.
    pub fn status(&self) -> u8 {
        if matches!(self, Error::Commit(_)) {
            20
        } else {
            1
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::CapReached { used, caps } => {
                write!(f, "usage cap reached: {used:?} of {caps:?}")
            }
            Error::Died => f.write_str("the claude process ended"),
            Error::Compacted => f.write_str("the conversation was compacted"),
            Error::Malformed(what) => write!(f, "unexpected shape: {what}"),
            Error::WrongModel { expected, got } => write!(f, "answered by {got}, not {expected}"),
            Error::EmptyReply => f.write_str("the reply is empty"),
            Error::FirstCall { expected, got } => {
                write!(f, "first call took {got} input tokens, expected {expected}")
            }
            Error::Provider(msg) => write!(f, "the turn failed: {msg}"),
            Error::Refused => f.write_str("the reply is a refusal"),
            Error::ApiKey(source) => write!(f, "the key source is {source:?}, not \"none\""),
            Error::Voided => f.write_str("the run already failed"),
            Error::TimedOut => f.write_str("the model call passed its time limit"),
            Error::Paused(run) => {
                write!(f, "run {run} was void three times; the experiment pauses")
            }
            Error::Commit(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}
