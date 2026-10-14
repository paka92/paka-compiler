//! The model client: one `claude -p` process per run, one JSON line in per turn, the reply and its
//! token counts out. M3-01 section 1.7 fixes the flags and the conversation model.
//!
//! Any error voids the run (M3-03 reruns it) and no later turn is sent. Every token a turn is
//! known to have spent goes into the ledger, whether the turn was good or not; `Reply::tokens` is
//! the figure for the metrics.

use crate::Error;
use crate::json::quote;
use crate::stream::{Event, Outcome, parse_event};
use crate::usage::{Ledger, Tokens};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// How far the first call's input tokens may sit from the expected count (M3-01 section 1.7).
const FIRST_CALL_MARGIN: u64 = 50;
/// The only variables the process inherits, when set. No `ANTHROPIC_*` variable gets through.
const INHERITED_ENV: [&str; 4] = ["HOME", "PATH", "LANG", "USER"];

pub struct Config {
    /// The `claude` command.
    pub program: PathBuf,
    pub model: String,
    pub effort: String,
    pub system_prompt: String,
    /// The working directory of the process: a fresh empty directory made by the caller. The
    /// process's standard error is written to `stderr.log` in it.
    pub dir: PathBuf,
    /// The input tokens the first call must show: system text, statement, reference and the fixed
    /// CLI text. `None` skips the guard, for the dry run that measures it.
    pub first_call_input: Option<u64>,
}

#[derive(Debug)]
pub struct Reply {
    pub text: String,
    /// Input is plain, cache-write and cache-read input together.
    pub tokens: Tokens,
    /// The reply stopped at the output limit.
    pub cut_off: bool,
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
        let mut command = Command::new(&config.program);
        command
            .args([
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
            ])
            .current_dir(&config.dir)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(config.dir.join("stderr.log"))?);
        for key in INHERITED_ENV {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let mut child = command.spawn()?;
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

    /// Sends one user turn and reads the reply. After an error the process's input is closed and
    /// every later call returns `Voided`.
    pub fn send(&mut self, text: &str) -> Result<Reply, Error> {
        if self.stdin.is_none() {
            return Err(Error::Voided);
        }
        let reply = self.turn(text);
        if reply.is_err() {
            self.stdin = None;
        }
        reply
    }

    /// Ends the run: closes the process's input and waits for it to exit cleanly.
    pub fn close(mut self) -> Result<(), Error> {
        drop(self.stdin.take());
        match self.child.wait()?.success() {
            true => Ok(()),
            false => Err(Error::Died),
        }
    }

    fn turn(&mut self, text: &str) -> Result<Reply, Error> {
        self.ledger.check()?;
        let guard = self.first_call_input.take();
        let line = user_line(text);
        let stdin = self.stdin.as_mut().ok_or(Error::Voided)?;
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| match e.kind() {
                ErrorKind::BrokenPipe => Error::Died,
                _ => e.into(),
            })?;
        self.read_turn(guard)
    }

    /// Reads lines up to the turn's `result`. A model or compaction problem is kept until then, so
    /// that the usage the result reports is still counted.
    fn read_turn(&mut self, guard: Option<u64>) -> Result<Reply, Error> {
        let mut answered = false;
        let mut void = None;
        let mut line = String::new();
        loop {
            line.clear();
            if self.stdout.read_line(&mut line)? == 0 {
                return Err(Error::Died);
            }
            match parse_event(line.trim_end())? {
                Event::Init {
                    api_key_source: Some(source),
                } if source != "none" => return Err(Error::ApiKey(source)),
                Event::Init { .. } | Event::Ignored => {}
                Event::Assistant { model } => {
                    answered = true;
                    if model != self.model {
                        void.get_or_insert(Error::WrongModel {
                            expected: self.model.clone(),
                            got: model,
                        });
                    }
                }
                Event::Compaction => {
                    void.get_or_insert(Error::Compacted);
                }
                Event::Result(outcome) => return self.finish(outcome, answered, void, guard),
            }
        }
    }

    fn finish(
        &self,
        outcome: Outcome,
        answered: bool,
        void: Option<Error>,
        guard: Option<u64>,
    ) -> Result<Reply, Error> {
        let (tokens, verdict) = match outcome {
            Outcome::Failed { message, tokens } => (tokens, Err(Error::Provider(message))),
            Outcome::Reply {
                text,
                tokens,
                stop_reason,
            } => (tokens, judge(text, tokens, stop_reason, answered, guard)),
        };
        self.ledger.add(tokens)?;
        void.map_or(verdict, Err)
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn judge(
    text: String,
    tokens: Tokens,
    stop_reason: Option<String>,
    answered: bool,
    guard: Option<u64>,
) -> Result<Reply, Error> {
    if !answered {
        return Err(Error::Malformed(
            "a reply without an assistant message".into(),
        ));
    }
    if stop_reason.as_deref() == Some("refusal") {
        return Err(Error::Refused);
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
        _ => Ok(Reply {
            text,
            tokens,
            cut_off: stop_reason.as_deref() == Some("max_tokens"),
        }),
    }
}

fn user_line(text: &str) -> String {
    let text = quote(text);
    format!(
        "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":{text}}}]}}}}\n"
    )
}
