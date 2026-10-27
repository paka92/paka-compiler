//! `paka build`: emits the IR and has clang compile and link it with the runtime
//! (docs/abi/m1.md section 9), and puts the host of each granted foreign declaration beside the
//! executable (section 13).

use crate::core_form::Program;
use crate::diagnostic::Diagnostic;
use crate::llvm::{self, Checks};
use paka::clang;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const RUNTIME: &str = "libpaka_rt.a";
/// The optimization flags and target CPU for every release build (M6-02 section 1.1): `-O3` and
/// no CPU flag, which keeps clang's baseline x86-64.
const OPTIMIZE: [&str; 1] = ["-O3"];
/// Drops the runtime code a binary never reaches: a headless app links no window.
const GC_SECTIONS: &str = "-Wl,--gc-sections";
const SYSTEM_LIBS: [&str; 7] = [
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

/// Writes the executable for `core` to `output` with `checks`; `path` is the source path for
/// diagnostics.
pub fn build(
    path: &str,
    core: &mut Program,
    output: &OsStr,
    checks: Checks,
) -> Result<(), Diagnostic> {
    let text = llvm::ir(core, checks).ok_or_else(|| {
        let message = "the back end does not build this program yet";
        Diagnostic::tool(path, "unsupported-form", message)
    })?;
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
    clang::check(path)?;
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
    result.map(|()| hosts(core, output))
}

/// Puts the host of each granted declaration of `core` beside `output` as `<output>.<name>`; one
/// that does not build is left out, so its calls answer `no host`.
fn hosts(core: &Program, output: &OsStr) {
    let granted = core.app.as_ref().map(|a| &a.foreign);
    for library in core.libraries.iter() {
        if granted.is_some_and(|g| g.contains(&library.name)) {
            let mut beside = output.to_os_string();
            beside.push(format!(".{}", library.name));
            let _ = fs::remove_file(&beside);
            if let Some(host) = paka::foreign::host(library) {
                let _ = fs::copy(host, beside);
            }
        }
    }
}

fn link(path: &str, ir: &Path, runtime: &Path, output: &OsStr) -> Result<(), Diagnostic> {
    let status = Command::new("clang")
        .args(OPTIMIZE)
        .arg(GC_SECTIONS)
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
        Err(e) => Err(clang::start_failed(path, e)),
    }
}
