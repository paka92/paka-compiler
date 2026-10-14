//! The model client: one `claude -p` process per run, one JSON line in per turn, the reply and its
//! token counts out. M3-01 section 1.7 fixes the flags and the conversation model.

use crate::Error;
use crate::json::quote;
use crate::stream::{Event, Outcome, parse_event};
use crate::usage::{Ledger, Tokens};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// A turn the provider failed is sent again this many times before the run gives up.
const RETRIES: u32 = 3;
/// How far the first call's input tokens may sit from the expected count (M3-01 section 1.7).
const FIRST_CALL_MARGIN: u64 = 50;

pub struct Config {
    /// The `claude` command.
    pub program: PathBuf,
    pub model: String,
    pub effort: String,
    pub system_prompt: String,
    /// The working directory of the process: a fresh empty directory made by the caller.
    pub dir: PathBuf,
    /// The input tokens the first call must show: system text, statement, reference and the fixed
    /// CLI text. `None` skips the guard, for the dry run that measures it.
    pub first_call_input: Option<u64>,
}

#[derive(Debug)]
pub struct Reply {
    pub text: String,
    /// Input is plain, cache-write and cache-read input together. Failed attempts are not in it.
    pub tokens: Tokens,
    pub model: String,
    /// 1 when the first attempt answered.
    pub attempts: u32,
}

enum Attempt {
    Reply {
        text: String,
        tokens: Tokens,
        model: String,
    },
    Failed(String),
}

pub struct Run {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    model: String,
    ledger: Ledger,
    first_call_input: Option<u64>,
}

impl Run {
    /// Starts the process, unless the ledger has reached a cap.
    pub fn start(config: &Config, ledger: &Ledger) -> Result<Run, Error> {
        ledger.check()?;
        let mut child = Command::new(&config.program)
            .args(args(config))
            .current_dir(&config.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout is piped");
        Ok(Run {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            model: config.model.clone(),
            ledger: ledger.clone(),
            first_call_input: config.first_call_input,
        })
    }

    /// Sends one user turn and reads the reply. A failed turn is sent again up to `RETRIES` times;
    /// only the attempt that answers is added to the ledger. Any error voids the run.
    pub fn send(&mut self, text: &str) -> Result<Reply, Error> {
        self.ledger.check()?;
        let guard = self.first_call_input.take();
        let line = user_line(text);
        let mut failure = String::new();
        for attempt in 1..=RETRIES + 1 {
            self.write(&line)?;
            match self.read_turn()? {
                Attempt::Failed(message) => failure = message,
                Attempt::Reply {
                    text,
                    tokens,
                    model,
                } => {
                    self.ledger.add(tokens)?;
                    self.accept(&text, tokens, &model, guard)?;
                    return Ok(Reply {
                        text,
                        tokens,
                        model,
                        attempts: attempt,
                    });
                }
            }
        }
        Err(Error::Provider(failure))
    }

    /// Ends the run: closes the process's input and waits for it to exit cleanly.
    pub fn close(mut self) -> Result<(), Error> {
        drop(self.stdin.take());
        match self.child.wait()?.success() {
            true => Ok(()),
            false => Err(Error::Died),
        }
    }

    /// The checks on an answered turn. Its tokens are already counted: they were spent.
    fn accept(
        &self,
        text: &str,
        tokens: Tokens,
        model: &str,
        guard: Option<u64>,
    ) -> Result<(), Error> {
        if model != self.model {
            return Err(Error::WrongModel {
                expected: self.model.clone(),
                got: model.into(),
            });
        }
        if text.is_empty() {
            return Err(Error::EmptyReply);
        }
        match guard {
            Some(expected) if tokens.input.abs_diff(expected) > FIRST_CALL_MARGIN => {
                Err(Error::FirstCall {
                    expected,
                    got: tokens.input,
                })
            }
            _ => Ok(()),
        }
    }

    fn write(&mut self, line: &str) -> Result<(), Error> {
        let stdin = self.stdin.as_mut().ok_or(Error::Died)?;
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| match e.kind() {
                ErrorKind::BrokenPipe => Error::Died,
                _ => e.into(),
            })
    }

    /// Reads lines up to the turn's `result`.
    fn read_turn(&mut self) -> Result<Attempt, Error> {
        let mut model = None;
        let mut line = String::new();
        loop {
            line.clear();
            if self.stdout.read_line(&mut line)? == 0 {
                return Err(Error::Died);
            }
            match parse_event(line.trim_end())? {
                Event::Assistant { model: m } => model = Some(m),
                Event::Compaction => return Err(Error::Compacted),
                Event::Ignored => {}
                Event::Result(Outcome::Failed(message)) => return Ok(Attempt::Failed(message)),
                Event::Result(Outcome::Reply { text, tokens }) => {
                    let model = model.ok_or_else(|| {
                        Error::Malformed("a reply without an assistant message".into())
                    })?;
                    return Ok(Attempt::Reply {
                        text,
                        tokens,
                        model,
                    });
                }
            }
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The flag line of M3-01 section 1.7. Never passed: `--bare`, `--fallback-model`,
/// `--max-budget-usd`, `--resume`, `--continue`.
fn args(config: &Config) -> [&str; 19] {
    [
        "-p",
        "--model",
        &config.model,
        "--effort",
        &config.effort,
        "--system-prompt",
        &config.system_prompt,
        "--tools",
        "",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--disable-slash-commands",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ]
}

fn user_line(text: &str) -> String {
    let text = quote(text);
    format!(
        "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":{text}}}]}}}}\n"
    )
}
