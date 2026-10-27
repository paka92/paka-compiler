//! `paka-bench ui <apps> <results> <normal>`: the visual-app costs of docs/decisions/M6-02.md
//! section 3, items 5 and 6, run with no window on clicks.
//!
//! The `measure` tools sit beside the runner and the normal ones in `<normal>`: each folder holds
//! `paka`, its runtime and `paka-dev-replay`. A measured run counts only after the normal build
//! printed the same trees for the same clicks. A `copy` app clicks each handler `CLICKS` times and
//! gives its state copy per handler; a `frames` app plays its `script` in dev and in release and
//! gives the time per frame in Paka code and in the whole frame, layout included.

use super::{NOISY, ROUNDS, Scratch, median, spread, tools};
use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

const CLICKS: usize = 10_000;
/// Each handler of a `copy` app and the path of its button.
const HANDLERS: [(&str, &str); 2] = [("Field", "#0"), ("Item", "#1")];
const BANNER: &str = "MEASUREMENT BUILD: not a Paka program";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Copy,
    Frames,
}

/// One build of the tools.
struct Tools {
    dir: PathBuf,
    measure: bool,
}

/// A binary to replay: a release build, or the dev replayer on the source.
enum Mode {
    Release(PathBuf),
    Dev(PathBuf, PathBuf),
}

/// What a measured run reports on standard error.
#[derive(Default, Debug, PartialEq)]
struct Figures {
    /// Each handler's state copy: name, nanoseconds, count.
    handlers: Vec<(String, u64, u64)>,
    /// Frames, nanoseconds in Paka code, nanoseconds in all.
    frames: Option<(u64, u64, u64)>,
}

/// Runs every listed app and writes the results; a refused app leaves no results file.
pub fn run(apps: &Path, results: &Path, measured: PathBuf, normal: PathBuf) -> Result<(), String> {
    let list = read_list(apps)?;
    let tools = [
        Tools {
            dir: normal,
            measure: false,
        },
        Tools {
            dir: measured,
            measure: true,
        },
    ];
    for t in &tools {
        for exe in ["paka", "paka-dev-replay"] {
            if !t.dir.join(exe).is_file() {
                return Err(format!("no {exe} in {}", t.dir.display()));
            }
        }
    }
    let clang = tools::pinned("clang", "clang version ", tools::CLANG_PIN.trim())?;
    tools::run("taskset", &["--version"])?;
    let cpu = super::machine::pinned_cpu();
    let dir = Scratch::new()?;
    let mut text = super::machine::facts(apps, cpu);
    text += &format!(
        "compiler paka {clang}\nprotocol clean normal run first; copy: {CLICKS} clicks per handler, one run; frames: 1 warm-up, {ROUNDS} rounds, one rerun above {}% spread\nnot timed: drawing to a screen\n",
        NOISY * 100.0
    );
    let mut summary = String::new();
    for (name, kind) in &list {
        let source = apps.join(name).join("main.paka");
        let [normal, measured] = tools.each_ref().map(|t| {
            let bin = dir.0.join(format!(
                "{name}-{}",
                if t.measure { "measure" } else { "normal" }
            ));
            build(&t.dir, &source, &bin).map(|()| bin)
        });
        let (normal, measured) = (normal?, measured?);
        text += &format!(
            "{name} kind {}\n",
            if *kind == Kind::Copy {
                "copy"
            } else {
                "frames"
            }
        );
        let lines = match kind {
            Kind::Copy => copies(name, &normal, &measured, cpu)?,
            Kind::Frames => {
                let script = fs::read_to_string(apps.join(name).join("script"))
                    .map_err(|e| format!("{name}/script: {e}"))?;
                let dev = |t: &Tools| Mode::Dev(t.dir.join("paka-dev-replay"), source.clone());
                let modes = [
                    [dev(&tools[0]), Mode::Release(normal)],
                    [dev(&tools[1]), Mode::Release(measured)],
                ];
                frames(name, &modes, &script, cpu)?
            }
        };
        text += &lines.0;
        summary += &lines.1;
    }
    fs::write(results, text + &summary)
        .map_err(|e| format!("cannot write {}: {e}", results.display()))?;
    print!("{summary}");
    Ok(())
}

/// The `<name> copy|frames` lines of `<apps>/list`; `#` starts a comment line.
fn read_list(apps: &Path) -> Result<Vec<(String, Kind)>, String> {
    let path = apps.join("list");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lines = text.lines().map(str::trim);
    lines
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            [name, "copy"] => Ok((name.into(), Kind::Copy)),
            [name, "frames"] => Ok((name.into(), Kind::Frames)),
            _ => Err(format!(
                "{}: {l:?} is not `<name> copy|frames`",
                path.display()
            )),
        })
        .collect()
}

/// Builds `source` with the `paka` in `dir` into `out`.
fn build(dir: &Path, source: &Path, out: &Path) -> Result<(), String> {
    let built = Command::new(dir.join("paka"))
        .arg("build")
        .arg(source)
        .arg("-o")
        .arg(out)
        .output()
        .map_err(|e| format!("cannot start paka build: {e}"))?;
    if !built.status.success() {
        return Err(format!(
            "building {} with {} failed: {}",
            source.display(),
            dir.display(),
            String::from_utf8_lossy(&built.stderr).trim_end()
        ));
    }
    Ok(())
}

/// The state copy per handler: a clean normal run, then the measured one; the results lines and
/// the summary.
fn copies(
    name: &str,
    normal: &Path,
    measured: &Path,
    cpu: usize,
) -> Result<(String, String), String> {
    let (mut text, mut summary) = (String::new(), String::new());
    for (handler, path) in HANDLERS {
        let script = format!("click {path}\n").repeat(CLICKS);
        let trees = replay(&Mode::Release(normal.into()), &script, cpu, false)?.0;
        let (checked, figures) = replay(&Mode::Release(measured.into()), &script, cpu, true)?;
        same(name, trees, checked)?;
        let Some(&(_, nanos, count)) = figures.handlers.iter().find(|h| h.0 == handler) else {
            return Err(format!("{name}: no state copy reported for {handler}"));
        };
        if count != CLICKS as u64 {
            return Err(format!(
                "{name}: {count} state copies for {handler}, not {CLICKS}"
            ));
        }
        text += &format!(
            "{name} {handler} trees {trees:016x} state copy {nanos} ns in {count} handlers\n"
        );
        summary += &copy_line(name, handler, nanos, count);
    }
    Ok((text, summary))
}

fn copy_line(name: &str, handler: &str, nanos: u64, count: u64) -> String {
    format!(
        "{name} {handler} state copy {:.1} ns per handler\n",
        nanos as f64 / count as f64
    )
}

/// Frame times in dev and release: clean normal runs of both, then rounds of the measured ones.
/// `modes` is [dev, release], normal then measured.
fn frames(
    name: &str,
    [normal, measured]: &[[Mode; 2]; 2],
    script: &str,
    cpu: usize,
) -> Result<(String, String), String> {
    let mut trees = vec![];
    for mode in normal {
        trees.push(replay(mode, script, cpu, false)?.0);
    }
    same(name, trees[0], trees[1])?;
    for mode in measured {
        same(name, trees[0], replay(mode, script, cpu, true)?.0)?;
    }
    let mut text = String::new();
    for attempt in 1..=2 {
        // Per mode, per round: microseconds per frame in Paka code and in all.
        let mut times = [[vec![], vec![]], [vec![], vec![]]];
        for _ in 0..ROUNDS {
            for (mode, t) in measured.iter().zip(&mut times) {
                let (_, figures) = replay(mode, script, cpu, true)?;
                let Some((count, code, whole)) = figures.frames else {
                    return Err(format!("{name}: no frames reported"));
                };
                t[0].push(code as f64 / count as f64 / 1e3);
                t[1].push(whole as f64 / count as f64 / 1e3);
            }
        }
        for (mode, t) in ["dev", "release"].iter().zip(&times) {
            for (what, t) in ["paka-code", "whole-frame"].iter().zip(t) {
                let raw: Vec<_> = t.iter().map(|x| format!("{x:.3}")).collect();
                text += &format!(
                    "{name} attempt {attempt} {mode} {what} median {:.3} us spread {:.1}% times {}\n",
                    median(t),
                    spread(t) * 100.0,
                    raw.join(" ")
                );
            }
        }
        let quiet = times.iter().flatten().all(|t| spread(t) <= NOISY);
        if quiet || attempt == 2 {
            let m = |mode: usize, what: usize| median(&times[mode][what]);
            let summary = frames_lines(name, [[m(0, 0), m(0, 1)], [m(1, 0), m(1, 1)]], !quiet);
            return Ok((text, summary));
        }
    }
    unreachable!("the second attempt returns")
}

/// The per-frame medians, [dev, release] of [Paka code, whole frame], and both ratios.
fn frames_lines(name: &str, [dev, release]: [[f64; 2]; 2], noisy: bool) -> String {
    let mut text = String::new();
    for (mode, [code, whole]) in [("dev", dev), ("release", release)] {
        text += &format!(
            "{name} {mode} per frame: paka code {code:.3} us, whole frame {whole:.3} us\n"
        );
    }
    text += &format!(
        "{name} dev/release: paka code {:.2}, whole frame {:.2}{}\n",
        dev[0] / release[0],
        dev[1] / release[1],
        if noisy { " (noisy)" } else { "" }
    );
    text
}

/// Refuses two runs whose trees differ.
fn same(name: &str, a: u64, b: u64) -> Result<(), String> {
    if a != b {
        return Err(format!("{name}: the runs printed different trees"));
    }
    Ok(())
}

/// Plays `script` on `mode` pinned to `cpu`: a hash of the trees it printed and, from a measured
/// run, its figures. A run that fails, or whose banner is not as `measure` says, is refused.
fn replay(mode: &Mode, script: &str, cpu: usize, measure: bool) -> Result<(u64, Figures), String> {
    let mut command = Command::new("taskset");
    command.arg("-c").arg(cpu.to_string());
    let shown = match mode {
        Mode::Release(bin) => {
            command.arg(bin).arg("--paka-replay");
            bin
        }
        Mode::Dev(replayer, source) => {
            command.arg(replayer).arg(source);
            source
        }
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run taskset: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped");
    let script = script.to_string();
    let writer = thread::spawn(move || stdin.write_all(script.as_bytes()));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("cannot wait for {}: {e}", shown.display()))?;
    let wrote = writer.join().expect("the writer does not panic");
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() || wrote.is_err() {
        return Err(format!(
            "{} failed ({}): {}",
            shown.display(),
            out.status,
            stderr.trim_end()
        ));
    }
    if stderr.starts_with(BANNER) != measure {
        let want = if measure {
            "a measure build"
        } else {
            "a normal build"
        };
        return Err(format!("{} is not {want}", shown.display()));
    }
    let mut hash = DefaultHasher::new();
    hash.write(&out.stdout);
    Ok((hash.finish(), figures(&stderr)))
}

/// The state copy and frame lines of a `measure` runtime's standard error.
fn figures(stderr: &str) -> Figures {
    let mut figures = Figures::default();
    for line in stderr
        .lines()
        .filter_map(|l| l.strip_prefix("MEASUREMENT BUILD: "))
    {
        let words: Vec<_> = line.split(' ').collect();
        let n = |i: usize| words[i].parse::<u64>().ok();
        match words[..] {
            ["state", "copy", name, _, "ns", "in", _, "handlers"] => {
                if let (Some(nanos), Some(count)) = (n(3), n(6)) {
                    figures.handlers.push((name.into(), nanos, count));
                }
            }
            ["frames", _, "paka", "code", _, "ns", "whole", _, "ns"] => {
                figures.frames = n(1).zip(n(4)).zip(n(7)).map(|((f, c), w)| (f, c, w));
            }
            _ => {}
        }
    }
    figures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_are_read_from_standard_error() {
        let stderr = "MEASUREMENT BUILD: not a Paka program\n\
            MEASUREMENT BUILD: copy routine 900 ns in 3 calls\n\
            MEASUREMENT BUILD: state copy Field 500 ns in 2 handlers\n\
            MEASUREMENT BUILD: state copy Item x ns in 2 handlers\n\
            MEASUREMENT BUILD: frames 4 paka code 40 ns whole 100 ns\n\
            held: trap assert\n";
        let got = figures(stderr);
        assert_eq!(got.handlers, [("Field".into(), 500, 2)]);
        assert_eq!(got.frames, Some((4, 40, 100)));
        assert_eq!(
            figures("MEASUREMENT BUILD: frames 4 paka code 40 ns\n"),
            Figures::default()
        );
    }

    #[test]
    fn the_summary_gives_per_handler_and_both_ratios() {
        assert_eq!(
            copy_line("copy_1000", "Item", 15_000, 10_000),
            "copy_1000 Item state copy 1.5 ns per handler\n"
        );
        let text = frames_lines("todo", [[30.0, 90.0], [10.0, 60.0]], true);
        assert_eq!(
            text,
            "todo dev per frame: paka code 30.000 us, whole frame 90.000 us\n\
             todo release per frame: paka code 10.000 us, whole frame 60.000 us\n\
             todo dev/release: paka code 3.00, whole frame 1.50 (noisy)\n"
        );
    }

    #[test]
    fn the_app_list_is_read_and_a_bad_line_refused() {
        let dir = Scratch::new().unwrap();
        fs::write(dir.0.join("list"), "# c\ncopy_1000 copy\ntodo frames\n").unwrap();
        let got = read_list(&dir.0).unwrap();
        assert_eq!(
            got,
            [
                ("copy_1000".into(), Kind::Copy),
                ("todo".into(), Kind::Frames)
            ]
        );
        fs::write(dir.0.join("list"), "todo window\n").unwrap();
        let e = read_list(&dir.0).unwrap_err();
        assert!(
            e.ends_with("\"todo window\" is not `<name> copy|frames`"),
            "{e}"
        );
    }

    #[test]
    fn the_committed_list_reads() {
        let apps = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../ui"));
        let list = read_list(apps).unwrap();
        assert_eq!(list.len(), 4);
        let script = fs::read_to_string(apps.join("todo/script")).unwrap();
        assert_eq!(script.lines().filter(|l| *l == "click #1").count(), 1000);
        assert_eq!(script.lines().count(), 2000);
    }
}
