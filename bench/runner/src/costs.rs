//! `paka-bench costs <programs> <results> <normal>`: the section 8 costs of
//! docs/decisions/M6-02.md section 3 that need builds of their own: bounds checks (item 2), the
//! copy of owned trees (item 4) and the numeric kernels once in dev mode (item 6).
//!
//! The `measure` `paka` sits beside the runner; `<normal>` holds the normal `paka`, its runtime
//! and `paka-diff`. A measured or dev run counts only after the normal build printed the same
//! checksum.

use super::{NOISY, ROUNDS, Scratch, agree, machine, median, program, read_list, spread, time};
use super::{text, tools};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// The kernels timed without bounds checks (item 2).
const BOUNDS: [&str; 4] = ["saxpy_dot", "matmul", "stencil", "sieve"];
/// The programs whose copy routine is timed (item 4).
const COPIES: [&str; 3] = ["expr_parse", "binary_trees", "bfs"];
/// A dev run past this many milliseconds is stopped.
const DEV_LIMIT_MS: u64 = 3_600_000;
const BANNER: &str = "MEASUREMENT BUILD: not a Paka program";

/// Runs every cost the list's programs give and writes the results; a refused program leaves no
/// results file.
pub fn run(
    programs: &Path,
    results: &Path,
    measured: PathBuf,
    normal: PathBuf,
) -> Result<(), String> {
    let list = read_list(programs)?;
    for exe in [
        normal.join("paka"),
        normal.join("paka-diff"),
        measured.join("paka"),
    ] {
        if !exe.is_file() {
            return Err(format!("no {}", exe.display()));
        }
    }
    let clang = tools::pinned("clang", "clang version ", tools::CLANG_PIN.trim())?;
    tools::run("taskset", &["--version"])?;
    let cpu = machine::pinned_cpu();
    let dir = Scratch::new()?;
    let mut out = machine::facts(programs, cpu);
    out += &format!(
        "compiler paka {clang}\nprotocol clean normal run first; bounds and copies: 1 warm-up, {ROUNDS} rounds, one rerun above {}% spread; dev: one run\n",
        NOISY * 100.0
    );
    let input = dir.0.join("text");
    fs::write(&input, text::make())
        .map_err(|e| format!("cannot write {}: {e}", input.display()))?;
    for (name, kind, takes) in &list {
        let args = if *takes {
            vec![input.as_path()]
        } else {
            vec![]
        };
        let source = programs.join(name).join("main.paka");
        let work = dir.0.join(name);
        fs::create_dir(&work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
        let bin = |tools: &Path, label: &str| {
            let bin = work.join(label);
            build(tools, &source, label == "no-bounds", &bin).map(|()| bin)
        };
        let release = bin(&normal, "paka")?;
        if BOUNDS.contains(&name.as_str()) {
            let bins = [
                ("paka", release.clone()),
                ("paka-no-bounds", bin(&measured, "no-bounds")?),
            ];
            out += &program(&format!("{name} bounds"), &bins, cpu, &args)?;
        }
        if COPIES.contains(&name.as_str()) {
            out += &copies(name, &release, &bin(&measured, "measure")?, cpu, &args)?;
        }
        if kind != "pointer" {
            out += &dev(
                name,
                &release,
                &normal.join("paka-diff"),
                &source,
                cpu,
                &work,
            )?;
        }
        drop(fs::remove_dir_all(&work));
    }
    fs::write(results, out).map_err(|e| format!("cannot write {}: {e}", results.display()))
}

/// Builds `source` with the `paka` in `tools` into `out`, without bounds checks when `omit`.
fn build(tools: &Path, source: &Path, omit: bool, out: &Path) -> Result<(), String> {
    let mut command = Command::new(tools.join("paka"));
    command.arg("build").arg(source);
    if omit {
        command.args(["--omit", "bounds"]);
    }
    let built = command
        .arg("-o")
        .arg(out)
        .output()
        .map_err(|e| format!("cannot start paka build: {e}"))?;
    if !built.status.success() {
        return Err(format!(
            "building {} with {} failed: {}",
            source.display(),
            tools.display(),
            String::from_utf8_lossy(&built.stderr).trim_end()
        ));
    }
    Ok(())
}

/// The normal and the `measure` build in turn; the measured one also reports its copy routine.
fn copies(
    name: &str,
    normal: &Path,
    measured: &Path,
    cpu: usize,
    args: &[&Path],
) -> Result<String, String> {
    let bins = [("paka", normal.into()), ("paka-measure", measured.into())];
    let checksum = agree(name, &bins, cpu, args)?;
    let mut text = format!("{name} copies checksum {checksum}\n");
    for attempt in 1..=2 {
        let (mut plain, mut whole, mut copy) = (vec![], vec![], vec![]);
        for _ in 0..ROUNDS {
            plain.push(timed(normal, cpu, args, &checksum, false)?.0);
            let (took, stderr) = timed(measured, cpu, args, &checksum, true)?;
            let nanos = copy_nanos(&stderr)
                .ok_or_else(|| format!("{name}: the measure build reported no copy routine"))?;
            whole.push(took);
            copy.push(nanos as f64 / 1e9);
        }
        for (what, t) in [("paka", &plain), ("paka-measure", &whole), ("copy", &copy)] {
            let raw: Vec<_> = t.iter().map(|x| format!("{x:.6}")).collect();
            text += &format!(
                "{name} copies attempt {attempt} {what} median {:.6} s spread {:.1}% times {}\n",
                median(t),
                spread(t) * 100.0,
                raw.join(" ")
            );
        }
        if !rerun(&plain, &whole, &copy) {
            return Ok(text);
        }
    }
    Ok(text + &format!("{name} copies noisy\n"))
}

/// Whether any of an attempt's three figures spreads above the limit; a copy routine that never
/// ran has no spread.
fn rerun(plain: &[f64], whole: &[f64], copy: &[f64]) -> bool {
    [plain, whole, copy].iter().any(|t| spread(t) > NOISY)
}

/// One run of `bin` that must print `checksum`, and a banner only when `measure`: its seconds and
/// standard error.
fn timed(
    bin: &Path,
    cpu: usize,
    args: &[&Path],
    checksum: &str,
    measure: bool,
) -> Result<(f64, String), String> {
    let (took, out, stderr) = time(bin, cpu, args)?;
    if out != checksum {
        return Err(format!("{} printed {out:?}, not {checksum}", bin.display()));
    }
    if stderr.starts_with(BANNER) != measure {
        return Err(format!(
            "{} has the wrong banner: {stderr:?}",
            bin.display()
        ));
    }
    Ok((took.as_secs_f64(), stderr))
}

/// The copy routine's nanoseconds in a `measure` runtime's standard error.
fn copy_nanos(stderr: &str) -> Option<u64> {
    stderr.lines().find_map(|l| {
        let words: Vec<_> = l
            .strip_prefix("MEASUREMENT BUILD: copy routine ")?
            .split(' ')
            .collect();
        match words[..] {
            [nanos, "ns", "in", _, "calls"] => nanos.parse().ok(),
            _ => None,
        }
    })
}

/// One dev-mode run of `source` by `paka-diff --one dev`, after the release build gave the
/// checksum.
fn dev(
    name: &str,
    release: &Path,
    diff: &Path,
    source: &Path,
    cpu: usize,
    scratch: &Path,
) -> Result<String, String> {
    let checksum = time(release, cpu, &[])?.1;
    let folder = scratch.join("dev");
    fs::create_dir_all(&folder).map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
    fs::copy(source, folder.join("main.paka")).map_err(|e| format!("cannot stage {name}: {e}"))?;
    let start = Instant::now();
    let ran = Command::new("taskset")
        .arg("-c")
        .arg(cpu.to_string())
        .arg(diff)
        .args(["--one", "dev", "note"])
        .arg(DEV_LIMIT_MS.to_string())
        .current_dir(&folder)
        .output()
        .map_err(|e| format!("cannot run taskset: {e}"))?;
    let took = start.elapsed().as_secs_f64();
    let out = String::from_utf8_lossy(&ran.stdout).trim_end().to_string();
    if !ran.status.success() || out != checksum {
        let note = fs::read_to_string(folder.join("note")).unwrap_or_default();
        return Err(format!(
            "{name}: the dev run ended {} with {out:?}, not {checksum}: {} {}",
            ran.status,
            note.trim_end(),
            String::from_utf8_lossy(&ran.stderr).trim_end()
        ));
    }
    Ok(format!("{name} dev checksum {checksum} time {took:.6} s\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copy_routine_is_read_from_standard_error() {
        let stderr = "MEASUREMENT BUILD: not a Paka program\n\
            MEASUREMENT BUILD: copy routine 900 ns in 3 calls\n";
        assert_eq!(copy_nanos(stderr), Some(900));
        assert_eq!(
            copy_nanos("MEASUREMENT BUILD: copy routine x ns in 3 calls\n"),
            None
        );
        assert_eq!(copy_nanos(BANNER), None);
    }

    #[test]
    fn a_noisy_copy_figure_reruns_the_attempt() {
        let steady = [2.34, 2.35, 2.33];
        let copy = [0.0221, 0.0220, 0.0246];
        assert!(!rerun(&steady, &steady, &steady));
        assert!(rerun(&steady, &steady, &copy));
        assert!(!rerun(&steady, &steady, &[0.0; 3]));
    }

    #[test]
    fn every_costed_program_is_listed_with_its_kind() {
        let programs = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../programs"));
        let list = read_list(programs).unwrap();
        let kind = |name: &str| list.iter().find(|p| p.0 == name).map(|p| p.1.as_str());
        assert!(
            BOUNDS
                .iter()
                .all(|n| matches!(kind(n), Some("float" | "integer")))
        );
        assert!(COPIES.iter().all(|n| kind(n) == Some("pointer")));
    }
}
