//! A run's raw log: one JSON object per line, appended and synced line by line, so a kill loses
//! at most the line being written. The last line says how the run ended: `end` for a finished
//! run, `void` for a void one; a log with neither belongs to a run cut short.

use crate::json::{self, Value, quote};
use std::fmt::Display;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// One log line under construction.
pub struct Line(String);

impl Line {
    pub fn new(event: &str) -> Line {
        Line(format!("{{\"event\":{}", quote(event)))
    }

    /// A number, a boolean or `null`: written as it displays.
    pub fn raw(mut self, key: &str, value: impl Display) -> Line {
        self.0 += &format!(",{}:{value}", quote(key));
        self
    }

    pub fn text(self, key: &str, value: &str) -> Line {
        self.raw(key, quote(value))
    }

    /// Milliseconds since the Unix epoch, under the key `at`.
    pub fn now(self) -> Line {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        self.raw("at", ms)
    }

    /// The finished object, as JSON text without the line feed.
    pub fn json(self) -> String {
        self.0 + "}"
    }
}

pub struct Log(File);

impl Log {
    /// A new log; one already at `path` is an error.
    pub fn create(path: &Path) -> io::Result<Log> {
        OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(path)
            .map(Log)
    }

    pub fn write(&mut self, line: Line) -> io::Result<()> {
        self.0.write_all((line.json() + "\n").as_bytes())?;
        self.0.sync_data()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Ended,
    Void,
    /// Cut short: no closing line, or no log at all.
    Cut,
}

/// How the run logged at `path` ended.
pub fn state(path: &Path) -> io::Result<State> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(State::Cut),
        Err(e) => return Err(e),
    };
    // A line without its line feed was cut by the kill.
    let last = text.strip_suffix('\n').and_then(|t| t.lines().last());
    let event = last.and_then(|l| json::parse(l).ok());
    Ok(
        match event
            .as_ref()
            .and_then(|e| e.get("event"))
            .and_then(Value::as_str)
        {
            Some("end") => State::Ended,
            Some("void") => State::Void,
            _ => State::Cut,
        },
    )
}

/// Closes the log of a run cut short with a `void` line, after ending any half-written line.
pub fn close_cut(path: &Path, reason: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    let text = std::fs::read(path)?;
    let feed = if text.is_empty() || text.ends_with(b"\n") {
        ""
    } else {
        "\n"
    };
    let line = Line::new("void").text("reason", reason).now().json();
    file.write_all(format!("{feed}{line}\n").as_bytes())?;
    file.sync_data()
}
