//! The benchmark runner of docs/decisions/M6-02.md section 3.
//!
//! `paka-bench run <programs> <results>` builds each program `<programs>/list` names in Paka, Rust
//! and C++, refuses one whose checksums differ, times every binary pinned to one CPU and writes
//! the machine facts, medians, spread and every raw time to `<results>`. A program listed with
//! `text` gets the path of the text `text::make` writes as its one argument.

mod machine;
mod text;
mod tools;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tools::{Lang, Toolchain};

const ROUNDS: usize = 10;
/// A spread above this reruns the program once; still above, it is marked noisy.
const NOISY: f64 = 0.05;
const MAX_LOAD: f64 = 1.0;
const LANGS: [Lang; 4] = [Lang::Paka, Lang::Rust, Lang::RustChecked, Lang::Cpp];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [command, programs, results] = &args[..] else {
        return usage();
    };
    if command != "run" {
        return usage();
    }
    let paka = env::current_exe()
        .map(|exe| exe.with_file_name("paka"))
        .unwrap_or_default();
    match idle().and_then(|()| run(Path::new(programs), Path::new(results), paka)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("paka-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: paka-bench run <programs-dir> <results-file>");
    ExitCode::from(2)
}

fn idle() -> Result<(), String> {
    let load = machine::load()?;
    if load > MAX_LOAD {
        return Err(format!(
            "the one-minute load average is {load}, above {MAX_LOAD}; leave the machine idle"
        ));
    }
    Ok(())
}

/// Builds, checks and times every listed program; a refused program leaves no results file.
fn run(programs: &Path, results: &Path, paka: PathBuf) -> Result<(), String> {
    let list = read_list(programs)?;
    let tools = Toolchain::check(paka)?;
    let cpu = machine::pinned_cpu();
    let dir = Scratch::new()?;
    let mut text = header(&tools, programs, cpu);
    let input = dir.0.join("text");
    if list.iter().any(|(_, _, takes)| *takes) {
        let made = text::make();
        fs::write(&input, &made).map_err(|e| format!("cannot write {}: {e}", input.display()))?;
        text += &format!("input text {} bytes\n", made.len());
    }
    for (name, kind, takes) in &list {
        let args = if *takes {
            vec![input.as_path()]
        } else {
            vec![]
        };
        let source = programs.join(name);
        let bins = LANGS
            .into_iter()
            .filter(|&l| l != Lang::RustChecked || kind == "integer")
            .map(|l| {
                let bin = dir.0.join(format!("{name}-{}", l.name()));
                l.build(&tools.paka, &source, &bin).map(|()| (l, bin))
            })
            .collect::<Result<Vec<_>, _>>()?;
        text += &format!("{name} kind {kind}\n");
        text += &program(name, &bins, cpu, &args)?;
        bins.iter().for_each(|(_, bin)| drop(fs::remove_file(bin)));
    }
    fs::write(results, text).map_err(|e| format!("cannot write {}: {e}", results.display()))
}

/// The `<name> <kind> [text]` lines of `<programs>/list`, kind as in M6-02 section 1.3; `#`
/// starts a comment line.
fn read_list(programs: &Path) -> Result<Vec<(String, String, bool)>, String> {
    let path = programs.join("list");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lines = text.lines().map(str::trim);
    lines
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            [
                name,
                kind @ ("float" | "integer" | "pointer"),
                ref text @ ..,
            ] if text.is_empty() || text == ["text"] => {
                Ok((name.into(), kind.into(), !text.is_empty()))
            }
            _ => Err(format!(
                "{}: {l:?} is not `<name> float|integer|pointer [text]`",
                path.display()
            )),
        })
        .collect()
}

fn header(tools: &Toolchain, programs: &Path, cpu: usize) -> String {
    let mut text = machine::facts(programs, cpu);
    text += &format!(
        "compiler paka {}\ncompiler rust {}\ncompiler c++ {}\nlibstdc++ {}\n",
        tools.clang, tools.rustc, tools.clangxx, tools.libstdcxx
    );
    for l in LANGS {
        text += &format!("flags {} {}\n", l.name(), l.flags());
    }
    text + &format!(
        "protocol 1 warm-up, {ROUNDS} rounds, one rerun above {}% spread\n",
        NOISY * 100.0
    )
}

/// The warm-up agrees on the checksum, then rounds run every binary in turn; a noisy attempt
/// is rerun once. The last attempt's medians are the result.
fn program(
    name: &str,
    bins: &[(Lang, PathBuf)],
    cpu: usize,
    args: &[&Path],
) -> Result<String, String> {
    let checksum = agree(name, bins, cpu, args)?;
    let mut text = format!("{name} checksum {checksum}\n");
    for attempt in 1..=2 {
        let times = rounds(bins, cpu, args, &checksum)?;
        for ((lang, _), t) in bins.iter().zip(&times) {
            let raw: Vec<_> = t.iter().map(|x| format!("{x:.6}")).collect();
            text += &format!(
                "{name} attempt {attempt} {} median {:.6} s spread {:.1}% times {}\n",
                lang.name(),
                median(t),
                spread(t) * 100.0,
                raw.join(" ")
            );
        }
        if times.iter().all(|t| spread(t) <= NOISY) {
            return Ok(text);
        }
    }
    Ok(text + &format!("{name} noisy\n"))
}

/// The one Int line every binary prints, refused unless all agree.
fn agree(
    name: &str,
    bins: &[(Lang, PathBuf)],
    cpu: usize,
    args: &[&Path],
) -> Result<String, String> {
    let outs = bins
        .iter()
        .map(|(_, bin)| time(bin, cpu, args).map(|(_, out)| out))
        .collect::<Result<Vec<_>, _>>()?;
    let first = &outs[0];
    if first.parse::<i64>().is_ok() && outs.iter().all(|o| o == first) {
        return Ok(first.clone());
    }
    let got: Vec<_> = bins
        .iter()
        .zip(&outs)
        .map(|((l, _), o)| format!("{} {o:?}", l.name()))
        .collect();
    Err(format!(
        "{name}: the checksums differ or are not one Int line: {}",
        got.join(", ")
    ))
}

/// Seconds per run, one list per binary.
fn rounds(
    bins: &[(Lang, PathBuf)],
    cpu: usize,
    args: &[&Path],
    checksum: &str,
) -> Result<Vec<Vec<f64>>, String> {
    let mut times = vec![Vec::with_capacity(ROUNDS); bins.len()];
    for _ in 0..ROUNDS {
        for ((_, bin), t) in bins.iter().zip(&mut times) {
            let (took, out) = time(bin, cpu, args)?;
            if out != checksum {
                return Err(format!("{} printed {out:?}, not {checksum}", bin.display()));
            }
            t.push(took.as_secs_f64());
        }
    }
    Ok(times)
}

/// Runs `bin args` pinned to `cpu`, timed from start to exit; returns its trimmed standard output.
fn time(bin: &Path, cpu: usize, args: &[&Path]) -> Result<(Duration, String), String> {
    let start = Instant::now();
    let out = Command::new("taskset")
        .arg("-c")
        .arg(cpu.to_string())
        .arg(bin)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run taskset: {e}"))?;
    let took = start.elapsed();
    if !out.status.success() {
        return Err(format!(
            "{} failed ({}): {}",
            bin.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim_end()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
    Ok((took, stdout))
}

fn sorted(times: &[f64]) -> Vec<f64> {
    let mut t = times.to_vec();
    t.sort_by(f64::total_cmp);
    t
}

fn median(times: &[f64]) -> f64 {
    let t = sorted(times);
    let mid = t.len() / 2;
    if t.len().is_multiple_of(2) {
        (t[mid - 1] + t[mid]) / 2.0
    } else {
        t[mid]
    }
}

/// (slowest - fastest) / median.
fn spread(times: &[f64]) -> f64 {
    let t = sorted(times);
    (t[t.len() - 1] - t[0]) / median(times)
}

/// A fresh directory under the temporary directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Scratch, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("paka-bench-{}-{n}", process::id()));
        fs::create_dir(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../sample");

    /// `paka` and its runtime, built into this test's own target directory and profile.
    fn paka() -> PathBuf {
        static BUILT: Once = Once::new();
        let exe = env::current_exe().unwrap();
        let profile = exe.parent().unwrap().parent().unwrap();
        BUILT.call_once(|| {
            let mut cargo = Command::new(env!("CARGO"));
            cargo.args([
                "build",
                "-p",
                "paka",
                "-p",
                "paka-rt",
                "--locked",
                "--offline",
            ]);
            cargo.arg("--target-dir").arg(profile.parent().unwrap());
            if profile.ends_with("release") {
                cargo.arg("--release");
            }
            assert!(cargo.status().unwrap().success(), "building paka failed");
        });
        profile.join("paka")
    }

    /// The machine lacks a pinned compiler: the refusal is the covered path.
    fn compiler_missing(e: &str) -> bool {
        let missing = e.contains(" is required; ") || e.contains(" not found on PATH");
        if missing {
            eprintln!("refused as it must be: {e}");
        }
        missing
    }

    #[test]
    fn the_sample_is_built_agreed_and_timed() {
        let dir = Scratch::new().unwrap();
        let results = dir.0.join("results");
        match run(Path::new(SAMPLE), &results, paka()) {
            Err(e) if compiler_missing(&e) => return,
            r => r.unwrap(),
        }
        let text = fs::read_to_string(&results).unwrap();
        assert!(text.starts_with("cpu "), "{text}");
        assert!(text.contains("\nsteps checksum 849637\n"), "{text}");
        for lang in ["paka", "rust", "rust-checked", "c++"] {
            let line = format!("\nsteps attempt 1 {lang} median ");
            let line = text
                .split(&line)
                .nth(1)
                .expect(lang)
                .lines()
                .next()
                .unwrap();
            assert_eq!(line.split(' ').count(), 5 + ROUNDS, "{line}");
        }
    }

    #[test]
    fn mismatched_checksums_are_refused() {
        let dir = Scratch::new().unwrap();
        let steps = dir.0.join("steps");
        fs::create_dir(&steps).unwrap();
        fs::copy(format!("{SAMPLE}/list"), dir.0.join("list")).unwrap();
        for file in ["main.paka", "main.rs"] {
            fs::copy(format!("{SAMPLE}/steps/{file}"), steps.join(file)).unwrap();
        }
        let cpp = fs::read_to_string(format!("{SAMPLE}/steps/main.cpp")).unwrap();
        let off_by_one = cpp.replace("total += steps(i)", "total += steps(i) + 1");
        assert_ne!(cpp, off_by_one);
        fs::write(steps.join("main.cpp"), off_by_one).unwrap();
        let results = dir.0.join("results");
        let e = run(&dir.0, &results, paka()).unwrap_err();
        if !compiler_missing(&e) {
            assert!(e.starts_with("steps: the checksums differ"), "{e}");
        }
        assert!(!results.exists());
    }

    #[test]
    fn median_and_spread() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert_eq!(spread(&[1.0, 2.0, 3.0]), 1.0);
    }

    #[test]
    fn a_bad_list_line_is_refused() {
        let dir = Scratch::new().unwrap();
        let list = "# c\nsieve integer\nword_count pointer text\nstencil vector\n";
        fs::write(dir.0.join("list"), list).unwrap();
        let e = read_list(&dir.0).unwrap_err();
        assert!(
            e.ends_with("\"stencil vector\" is not `<name> float|integer|pointer [text]`"),
            "{e}"
        );
        fs::write(dir.0.join("list"), &list[..list.len() - 15]).unwrap();
        let got = read_list(&dir.0).unwrap();
        assert_eq!(got[1], ("word_count".into(), "pointer".into(), true));
        assert!(!got[0].2);
    }
}
