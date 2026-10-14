//! Token counts and the usage cap: a running total kept in a file, across runs and processes.

use crate::Error;
use crate::json::{self, Value};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
}

/// The cap on each counter. Calls stop when either total has reached its cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub input: u64,
    pub output: u64,
}

impl Default for Caps {
    /// The caps of M3-02 section 1.8.
    fn default() -> Caps {
        Caps {
            input: 16_000_000,
            output: 9_500_000,
        }
    }
}

/// The running total, kept as `{"input":N,"output":M}`. A file that is missing is a total of zero;
/// one that cannot be read exactly is an error.
#[derive(Debug, Clone)]
pub struct Ledger {
    path: PathBuf,
    caps: Caps,
}

impl Ledger {
    pub fn new(path: impl Into<PathBuf>, caps: Caps) -> Ledger {
        Ledger {
            path: path.into(),
            caps,
        }
    }

    pub fn used(&self) -> Result<Tokens, Error> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Tokens::default()),
            Err(e) => return Err(e.into()),
        };
        self.read(&text).ok_or_else(|| {
            Error::Malformed(format!(
                "{}: expected {{\"input\":N,\"output\":M}}",
                self.path.display()
            ))
        })
    }

    fn read(&self, text: &str) -> Option<Tokens> {
        let total = json::parse(text).ok()?;
        let count = |key| total.get(key).and_then(Value::as_u64);
        Some(Tokens {
            input: count("input")?,
            output: count("output")?,
        })
    }

    /// Refuses when either total has reached its cap.
    pub fn check(&self) -> Result<(), Error> {
        let used = self.used()?;
        if used.input >= self.caps.input || used.output >= self.caps.output {
            return Err(Error::CapReached {
                used,
                caps: self.caps,
            });
        }
        Ok(())
    }

    /// Adds to the total; the file is replaced whole, so a kill leaves the old or the new total.
    pub fn add(&self, tokens: Tokens) -> Result<(), Error> {
        let used = self.used()?;
        let sum = used
            .input
            .checked_add(tokens.input)
            .zip(used.output.checked_add(tokens.output));
        let (input, output) =
            sum.ok_or_else(|| Error::Malformed("usage total overflows".into()))?;
        let staged = self.path.with_extension("tmp");
        fs::write(
            &staged,
            format!("{{\"input\":{input},\"output\":{output}}}\n"),
        )?;
        fs::rename(staged, &self.path)?;
        Ok(())
    }
}
