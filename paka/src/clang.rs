//! The one clang the toolchain runs: release builds and foreign hosts accept only the pinned
//! version.

use crate::diagnostic::Diagnostic;
use std::io;
use std::process::{Command, Stdio};

/// The pinned version; tests read the same file.
pub const VERSION: &str = include_str!("../clang-version");

/// Refuses any clang but the pinned one, read off the first line of `clang --version`.
pub fn check(path: &str) -> Result<(), Diagnostic> {
    let out = Command::new("clang")
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| start_failed(path, e))?;
    if !out.status.success() {
        let message = format!("clang --version failed ({})", out.status);
        return Err(Diagnostic::tool(path, "clang-failed", message));
    }
    let pinned = VERSION.trim();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let found = version(&stdout);
    if found == Some(pinned) {
        return Ok(());
    }
    let found = found.unwrap_or("unknown");
    let message = format!("clang {pinned} is required; the clang on PATH is version {found}");
    Err(Diagnostic::tool(path, "clang-version", message))
}

/// `21.1.8` from a first line such as `Ubuntu clang version 21.1.8 (6ubuntu1)`.
fn version(text: &str) -> Option<&str> {
    let (_, rest) = text.lines().next()?.split_once("clang version ")?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    Some(&rest[..end]).filter(|v| !v.is_empty())
}

pub fn start_failed(path: &str, e: io::Error) -> Diagnostic {
    if e.kind() == io::ErrorKind::NotFound {
        Diagnostic::tool(path, "clang-missing", "clang not found on PATH")
    } else {
        Diagnostic::tool(path, "clang-failed", format!("cannot run clang: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_read_off_the_first_line() {
        assert_eq!(
            version("Ubuntu clang version 21.1.8 (6ubuntu1)\nTarget: x"),
            Some("21.1.8")
        );
        assert_eq!(version("clang version 21.1.8-1\n"), Some("21.1.8"));
        assert_eq!(version("Homebrew clang version 20.1.2"), Some("20.1.2"));
        assert_eq!(version("gcc (GCC) 14.2.0"), None);
        assert_eq!(version("x\nclang version 21.1.8"), None);
        assert_eq!(version(""), None);
    }
}
