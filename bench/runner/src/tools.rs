//! The pinned compilers, their fixed flags (M6-02 sections 1.1 and 3) and the build of each binary.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const CLANG_PIN: &str = include_str!("../../../paka/clang-version");
const TOOLCHAIN: &str = include_str!("../../../rust-toolchain.toml");

const RUST: [&str; 8] = [
    "--edition",
    "2024",
    "-C",
    "opt-level=3",
    "-C",
    "codegen-units=1",
    "-C",
    "panic=abort",
];
const CHECKED: [&str; 2] = ["-C", "overflow-checks=on"];
const CPP: [&str; 5] = [
    "-O3",
    "-std=c++20",
    "-fno-exceptions",
    "-fno-rtti",
    "-ffp-contract=off",
];

/// One column of the table; `RustChecked` is built for integer kernels only.
#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    Paka,
    Rust,
    RustChecked,
    Cpp,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::Paka => "paka",
            Lang::Rust => "rust",
            Lang::RustChecked => "rust-checked",
            Lang::Cpp => "c++",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Lang::Paka => "main.paka",
            Lang::Rust | Lang::RustChecked => "main.rs",
            Lang::Cpp => "main.cpp",
        }
    }

    /// The compiler and its fixed flags; the runner gives `paka build` none (M6-04).
    fn line(self) -> (&'static str, Vec<&'static str>) {
        match self {
            Lang::Paka => ("paka", vec!["build"]),
            Lang::Rust => ("rustc", RUST.to_vec()),
            Lang::RustChecked => ("rustc", [&RUST[..], &CHECKED].concat()),
            Lang::Cpp => ("clang++", CPP.to_vec()),
        }
    }

    pub fn flags(self) -> String {
        let (tool, flags) = self.line();
        format!("{tool} {}", flags.join(" "))
    }

    /// Builds `program/<source>` into `out`.
    pub fn build(self, paka: &Path, program: &Path, out: &Path) -> Result<(), String> {
        let source = program.join(self.source());
        let (tool, flags) = self.line();
        let tool = if self == Lang::Paka {
            paka.as_os_str()
        } else {
            OsStr::new(tool)
        };
        let built = Command::new(tool)
            .args(flags)
            .arg(&source)
            .arg("-o")
            .arg(out)
            .output();
        match built {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(format!(
                "building {} failed ({}):\n{}",
                source.display(),
                o.status,
                String::from_utf8_lossy(&o.stderr).trim_end()
            )),
            Err(e) => Err(format!("cannot start the {} build: {e}", self.name())),
        }
    }
}

/// The compilers' version lines, read after each was checked against its pin.
pub struct Toolchain {
    pub paka: PathBuf,
    pub clang: String,
    pub clangxx: String,
    pub rustc: String,
    pub libstdcxx: String,
}

impl Toolchain {
    /// Refuses unless clang and clang++ are `paka/clang-version`, rustc is the channel of
    /// `rust-toolchain.toml`, and `paka` and `taskset` exist.
    pub fn check(paka: PathBuf) -> Result<Toolchain, String> {
        if !paka.is_file() {
            return Err(format!(
                "no paka compiler at {}; build it with `cargo build --release -p paka -p paka-rt`",
                paka.display()
            ));
        }
        let clang_pin = CLANG_PIN.trim();
        let rust_pin = rust_channel(TOOLCHAIN).ok_or("rust-toolchain.toml names no channel")?;
        run("taskset", &["--version"])?;
        let rustc = run("rustc", &["--version", "--verbose"])?;
        Ok(Toolchain {
            paka,
            clang: pinned("clang", "clang version ", clang_pin)?,
            clangxx: pinned("clang++", "clang version ", clang_pin)?,
            rustc: rustc_line(&rustc, rust_pin)?,
            libstdcxx: libstdcxx()?,
        })
    }
}

/// `1.96.1` from the `channel = "1.96.1"` line.
fn rust_channel(toml: &str) -> Option<&str> {
    let line = toml
        .lines()
        .find(|l| l.trim_start().starts_with("channel"))?;
    line.split('"').nth(1)
}

/// The tool's first `--version` line, when it names `<prefix><pin>`.
pub fn pinned(tool: &str, prefix: &str, pin: &str) -> Result<String, String> {
    let text = run(tool, &["--version"])?;
    let line = text.lines().next().unwrap_or("").to_string();
    if names_version(&line, prefix, pin) {
        Ok(line)
    } else {
        Err(format!(
            "{tool} {pin} is required; {tool} --version says {line:?}"
        ))
    }
}

/// `rustc 1.96.1 (...)` with the LLVM version from `rustc --version --verbose`.
fn rustc_line(text: &str, pin: &str) -> Result<String, String> {
    let first = text.lines().next().unwrap_or("");
    if !names_version(first, "rustc ", pin) {
        return Err(format!(
            "rustc {pin} is required; rustc --version says {first:?}"
        ));
    }
    let llvm = text.lines().find_map(|l| l.strip_prefix("LLVM version: "));
    Ok(format!("{first} (LLVM {})", llvm.unwrap_or("unknown")))
}

/// True when `line` holds `<prefix><pin>` and the version does not go on past the pin.
pub fn names_version(line: &str, prefix: &str, pin: &str) -> bool {
    line.split_once(prefix)
        .and_then(|(_, rest)| rest.strip_prefix(pin))
        .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.'))
}

/// The libstdc++ release and date clang++ compiles against, from its predefined macros.
fn libstdcxx() -> Result<String, String> {
    let out = Command::new("clang++")
        .args(["-std=c++20", "-x", "c++", "-dM", "-E", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"#include <cstddef>\n")?;
            child.wait_with_output()
        })
        .map_err(|e| format!("cannot run clang++: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let macro_value = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("#define {name} ")))
            .unwrap_or("unknown")
            .to_string()
    };
    Ok(format!(
        "{} ({})",
        macro_value("_GLIBCXX_RELEASE"),
        macro_value("__GLIBCXX__")
    ))
}

/// The standard output of `tool args`, refused when the tool is missing or fails.
pub fn run(tool: &str, args: &[&str]) -> Result<String, String> {
    match Command::new(tool).args(args).stdin(Stdio::null()).output() {
        Ok(Output { status, stdout, .. }) if status.success() => {
            Ok(String::from_utf8_lossy(&stdout).into_owned())
        }
        Ok(o) => Err(format!("{tool} {} failed ({})", args.join(" "), o.status)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("{tool} not found on PATH; the runner needs it"))
        }
        Err(e) => Err(format!("cannot run {tool}: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_matched_whole() {
        let clang = "Ubuntu clang version 21.1.8 (6ubuntu1)";
        assert!(names_version(clang, "clang version ", "21.1.8"));
        assert!(!names_version(clang, "clang version ", "21.1"));
        assert!(!names_version(
            "clang version 21.1.80",
            "clang version ",
            "21.1.8"
        ));
        assert!(names_version(
            "rustc 1.96.1 (31fca3adb 2026-06-26)",
            "rustc ",
            "1.96.1"
        ));
        assert!(!names_version(
            "gcc (GCC) 14.2.0",
            "clang version ",
            "21.1.8"
        ));
        assert_eq!(
            rust_channel("[toolchain]\nchannel = \"1.96.1\"\n"),
            Some("1.96.1")
        );
    }

    #[test]
    fn a_missing_compiler_is_refused_by_name() {
        let e = pinned("paka-bench-no-such-compiler", "clang version ", "21.1.8").unwrap_err();
        assert_eq!(
            e,
            "paka-bench-no-such-compiler not found on PATH; the runner needs it"
        );
    }

    #[test]
    fn a_missing_paka_is_refused() {
        let e = Toolchain::check(PathBuf::from("/no/such/paka"))
            .err()
            .unwrap();
        assert!(e.starts_with("no paka compiler at /no/such/paka"), "{e}");
    }
}
