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
        .write_all(llvm::emit(&view).as_bytes())
        .map_err(write_failed)
        .and_then(|()| link(path, &ir, &runtime, output));
    let _ = fs::remove_file(&ir);
    result
}

fn link(path: &str, ir: &Path, runtime: &Path, output: &OsStr) -> Result<(), Diagnostic> {
    let status = Command::new("clang")
        .arg("-O2")
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
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(Diagnostic::tool(
            path,
            "clang-missing",
            "clang not found on PATH",
        )),
        Err(e) => Err(Diagnostic::tool(
            path,
            "clang-failed",
            format!("cannot run clang: {e}"),
        )),
    }
}
