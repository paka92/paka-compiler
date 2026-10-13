//! `paka build`: emits the IR and has clang compile and link it with the runtime
//! (docs/abi/m1.md section 9).

use crate::diagnostic::Diagnostic;
use crate::llvm;
use crate::lower::Core;
use std::env;
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
pub fn build(path: &str, core: &Core, output: &str) -> Result<(), Diagnostic> {
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
    let ir = Path::new(".").join(format!("{output}.{}.ll", std::process::id()));
    let write_failed = |e: io::Error| {
        Diagnostic::tool(
            path,
            "clang-failed",
            format!("cannot write {}: {e}", ir.display()),
        )
    };
    // Only a file created here is removed; one that already had the name is never touched.
    let mut file = File::create_new(&ir).map_err(write_failed)?;
    let result = file
        .write_all(llvm::emit(core).as_bytes())
        .map_err(write_failed)
        .and_then(|()| link(path, &ir, &runtime, output));
    let _ = fs::remove_file(&ir);
    result
}

fn link(path: &str, ir: &Path, runtime: &Path, output: &str) -> Result<(), Diagnostic> {
    let status = Command::new("clang")
        .arg("-O2")
        .args([ir, runtime])
        .args(SYSTEM_LIBS)
        .args(["-o", output])
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
