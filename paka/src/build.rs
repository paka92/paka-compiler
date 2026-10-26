//! `paka build`: emits the IR and has clang compile and link it with the runtime
//! (docs/abi/m1.md section 9).

use crate::core_form::Program;
use crate::diagnostic::Diagnostic;
use crate::llvm;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const RUNTIME: &str = "libpaka_rt.a";
/// The one clang release builds accept; tests read the same file.
const CLANG_VERSION: &str = include_str!("../clang-version");
/// The optimization flags and target CPU for every release build (M6-02 section 1.1): `-O3` and
/// no CPU flag, which keeps clang's baseline x86-64.
const OPTIMIZE: [&str; 1] = ["-O3"];
const SYSTEM_LIBS: [&str; 7] = [
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

/// Writes the executable for `core` to `output`; `path` is the source path for diagnostics.
/// The back end builds the milestone 1 subset only.
pub fn build(path: &str, core: &Program, output: &OsStr) -> Result<(), Diagnostic> {
    let view = llvm::view(core).ok_or_else(|| {
        let message = "the back end builds only the milestone 1 subset";
        Diagnostic::tool(path, "unsupported-form", message)
    })?;
    let text = llvm::emit(&view);
    let broken = llvm::lint::lint(&text);
    // The emitter alone keeps the promise table (M6-01 section 1.1); a finding is its bug.
    assert!(broken.is_empty(), "release IR breaks M6-01: {broken:?}");
    let runtime = env::current_exe()
        .map(|exe| exe.with_file_name(RUNTIME))
        .ok()
        .filter(|archive| archive.is_file())
        .ok_or_else(|| {
            Diagnostic::tool(
                path,
                "runtime-missing",
                format!("no {RUNTIME} next to paka"),
            )
        })?;
    check_clang(path)?;
    let mut ir = output.to_os_string();
    ir.push(format!(".{}.ll", std::process::id()));
    let ir = Path::new(".").join(ir);
    let write_failed = |e: io::Error| {
        Diagnostic::tool(
            path,
            "clang-failed",
            // Debug form: a line feed in the path is escaped.
            format!("cannot write {ir:?}: {e}"),
        )
    };
    // Only a file created here is removed; one that already had the name is never touched.
    let mut file = File::create_new(&ir).map_err(write_failed)?;
    let result = file
        .write_all(text.as_bytes())
        .map_err(write_failed)
        .and_then(|()| link(path, &ir, &runtime, output));
    let _ = fs::remove_file(&ir);
    result
}

fn link(path: &str, ir: &Path, runtime: &Path, output: &OsStr) -> Result<(), Diagnostic> {
    let status = Command::new("clang")
        .args(OPTIMIZE)
        .args([ir, runtime])
        .args(SYSTEM_LIBS)
        .arg("-o")
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(Diagnostic::tool(
            path,
            "clang-failed",
            format!("clang failed ({status})"),
        )),
        Err(e) => Err(start_failed(path, e)),
    }
}

/// Refuses any clang but the pinned one, read off the first line of `clang --version`.
fn check_clang(path: &str) -> Result<(), Diagnostic> {
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
    let pinned = CLANG_VERSION.trim();
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

fn start_failed(path: &str, e: io::Error) -> Diagnostic {
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
