//! Helpers for the integration tests that run `paka build`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Once;

pub const PAKA: &str = env!("CARGO_BIN_EXE_paka");
/// The one counting rule for the size of a text: a token is three bytes, rounded up.
pub fn tokens(text: &str) -> usize {
    text.len().div_ceil(3)
}

pub const LIGHT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");

/// Builds `libpaka_rt.a` beside the `paka` binary under test. The profile and target directory
/// are read off that binary's path, so `cargo test` and `cargo test --release` each get their own.
/// Once per test binary: parallel tests wait on the one nested cargo run.
pub fn build_runtime() {
    static BUILT: Once = Once::new();
    BUILT.call_once(|| {
        let profile_dir = Path::new(PAKA).parent().unwrap();
        let mut cargo = Command::new(env!("CARGO"));
        cargo.args([
            "build",
            "-p",
            "paka-rt",
            "--locked",
            "--offline",
            "--target-dir",
        ]);
        cargo.arg(profile_dir.parent().unwrap());
        if profile_dir.ends_with("release") {
            cargo.arg("--release");
        }
        assert!(cargo.status().unwrap().success(), "building paka-rt failed");
    });
}

/// The arguments a case is run with: one per line of its `args` file, none without the file.
/// A file that cannot be read as text is an error, not an empty list.
pub fn case_args(case: &Path) -> Vec<String> {
    match fs::read_to_string(case.join("args")) {
        Ok(text) => text.lines().map(String::from).collect(),
        Err(e) if e.kind() == ErrorKind::NotFound => vec![],
        Err(e) => panic!("{}: args: {e}", case.display()),
    }
}

/// The runtime archive beside the `paka` binary under test, built first.
pub fn runtime_archive() -> PathBuf {
    build_runtime();
    Path::new(PAKA).with_file_name("libpaka_rt.a")
}

/// Runs `paka build source -o output` once the runtime archive exists.
pub fn build_native(source: &str, output: &Path) -> Output {
    build_runtime();
    Command::new(PAKA)
        .args(["build", source, "-o"])
        .arg(output)
        .output()
        .unwrap()
}

/// A private directory under the target directory, removed on drop.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        let dir =
            Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    pub fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }

    pub fn entries(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
