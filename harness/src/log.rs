//! A run's raw log: one JSON object per line, appended and synced line by line, so a kill loses
//! at most the line being written. The last line says how the run ended: `end` for a finished
//! run, `void` for a void one; a log with neither belongs to a run cut short.

use crate::json::{self, Value, quote};
use std::fmt::Display;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// One log line under construction: keys and values written as JSON.
pub struct Line(Vec<(&'static str, String)>);

impl Line {
    pub fn new(event: &str) -> Line {
        Line(vec![("event", quote(event))])
    }

    /// A value already written as JSON: a number, a boolean, `null`, an array or an object.
    pub fn raw(mut self, key: &'static str, value: impl Display) -> Line {
        self.0.push((key, value.to_string()));
        self
    }

    pub fn text(self, key: &'static str, value: &str) -> Line {
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
        json::object(self.0)
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
    /// `fault` is false for a run stopped by the usage cap, which does not use up an attempt.
    Void {
        fault: bool,
    },
    /// Cut short: no closing line, a torn last line, or no log at all.
    Cut,
}

/// How the run logged at `path` ended.
pub fn state(path: &Path) -> io::Result<State> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(State::Cut),
        Err(e) => return Err(e),
    };
    let Some(last) = last_event(&bytes) else {
        return Ok(State::Cut);
    };
    Ok(match last.get("event").and_then(Value::as_str) {
        Some("end") => State::Ended,
        Some("void") => State::Void {
            fault: last.get("fault").and_then(Value::as_bool) != Some(false),
        },
        _ => State::Cut,
    })
}

/// The last line of a log, when it is complete: a line without its line feed was torn by a kill.
fn last_event(bytes: &[u8]) -> Option<Value> {
    let body = bytes.strip_suffix(b"\n")?;
    let start = body
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |at| at + 1);
    json::parse(std::str::from_utf8(&body[start..]).ok()?).ok()
}

/// Closes the log of a run cut short with a `void` line, after ending any half-written line.
pub fn close_cut(path: &Path, reason: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    let bytes = fs::read(path)?;
    let feed = if bytes.is_empty() || bytes.ends_with(b"\n") {
        ""
    } else {
        "\n"
    };
    let line = Line::new("void")
        .text("reason", reason)
        .raw("fault", true)
        .now()
        .json();
    file.write_all(format!("{feed}{line}\n").as_bytes())?;
    file.sync_data()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_torn_last_line_is_cut_and_closes_as_void() {
        let dir = std::env::temp_dir().join(format!("harness-log-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log");
        let _ = fs::remove_file(&path);
        let mut log = Log::create(&path).unwrap();
        log.write(Line::new("start")).unwrap();
        // A kill inside the two bytes of "é".
        let torn = Line::new("call").text("sent", "é").json();
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&torn.as_bytes()[..torn.len() - 3])
            .unwrap();
        assert_eq!(state(&path).unwrap(), State::Cut);
        close_cut(&path, "cut").unwrap();
        assert_eq!(state(&path).unwrap(), State::Void { fault: true });
        fs::remove_dir_all(dir).unwrap();
    }
}
