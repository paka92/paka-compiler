//! `paka-bench [--gated] [DEFS [SAMPLES]]`: edit-to-running time (M4-44, docs/decisions/M4-01.md
//! section 6).
//! Writes a program of DEFS definitions, 5,000 by default, one file each, serves it with the `paka`
//! beside this binary, and times edit requests that ask for a run, from request written to reply
//! read. Prints the median and 95th percentile of each edit class over SAMPLES edits, 200 by
//! default; the worst of three runs counts. `--gated` times only the gated classes. Exits 1 when a
//! gated class fails.

use paka_json::{Value, array, object, parse, quote};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, io, process};

const DEFS: &str = include_str!("../../../bench/generators/defs.paka");
const RUNS: usize = 3;
const GATE: Duration = Duration::from_millis(100);
/// Definitions besides the fillers: two enums, three benchmark functions, `t`, `total`, `Msg`,
/// `update` and the app.
const FIXED: usize = 10;
/// The fillers that use `t`, the signature edit's direct users.
const USERS: usize = 10;
const SMALLEST: usize = FIXED + USERS + 1;

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let gated = args.first().is_some_and(|a| a == "--gated");
    args.drain(..usize::from(gated));
    let arg = |at: usize, default: usize| args.get(at).map_or(Ok(default), |a| a.parse());
    let (Ok(size @ SMALLEST..), Ok(samples @ 1..)) = (arg(0, 5000), arg(1, 200)) else {
        eprintln!("usage: paka-bench [--gated] [DEFS [SAMPLES]], DEFS at least {SMALLEST}");
        return ExitCode::from(2);
    };
    let dir = env::temp_dir().join(format!("paka-bench-{}", process::id()));
    let result = bench(&dir, size, samples, gated);
    let _ = fs::remove_dir_all(&dir);
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("paka-bench: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Whether every gated class passes.
fn bench(dir: &Path, size: usize, samples: usize, gated: bool) -> io::Result<bool> {
    write_program(dir, size)?;
    let paka = env::current_exe()?.with_file_name("paka");
    let mut server = Server::start(&paka, dir)?;
    server.call([
        ("call", quote("run")),
        ("app", quote("bench")),
        ("args", array([quote("x")])),
    ])?;
    let mut base = vec![];
    for (name, _) in &CLASSES[..4] {
        let (_, found) = server.call([("call", quote("find")), ("name", quote(name))])?;
        let version = found
            .get("found")
            .and_then(|f| first(f)?.get("version")?.as_str());
        base.push(
            version
                .ok_or_else(|| fail("no version", &found))?
                .to_string(),
        );
    }
    let mut times = vec![vec![]; CLASSES.len()];
    for run in 0..RUNS {
        let mut run_times = vec![vec![]; CLASSES.len()];
        for s in 0..samples {
            let n = run * samples + s + 1;
            for (class, (name, _)) in CLASSES[..4].iter().enumerate() {
                if gated && !CLASSES[class].1 {
                    continue;
                }
                let (time, version) =
                    server.edit(name, Some(&base[class]), Some(&body(name, n)))?;
                base[class] = version;
                run_times[class].push(time);
            }
            if gated {
                continue;
            }
            let added = format!("g{n}");
            let text = format!("fn {added}(x: Int) -> Int {{\n  x + {n}\n}}");
            let (time, version) = server.edit(&added, None, Some(&text))?;
            run_times[4].push(time);
            run_times[5].push(server.edit(&added, Some(&version), None)?.0);
        }
        for (all, mut these) in times.iter_mut().zip(run_times) {
            these.sort();
            all.push(these);
        }
    }
    let passed = report(size, samples, &times);
    server.stop()?;
    Ok(passed)
}

/// Each class: its name and whether it is gated.
const CLASSES: [(&str, bool); 6] = [
    ("step", true),
    ("walk", true),
    ("simulate", true),
    ("t", false),
    ("add", false),
    ("delete", false),
];

/// Prints each timed class; whether every gated one passes.
fn report(size: usize, samples: usize, times: &[Vec<Vec<Duration>>]) -> bool {
    let mut passed = true;
    println!("{size} definitions, {samples} edits per class, worst of {RUNS} runs\n");
    println!("{:<12} {:>10} {:>10}  gate", "class", "median", "p95");
    let labels = [
        "body-small",
        "body-medium",
        "body-large",
        "signature",
        "add",
        "delete",
    ];
    for ((label, (_, gated)), runs) in labels.iter().zip(CLASSES).zip(times) {
        if runs.iter().all(Vec::is_empty) {
            continue;
        }
        let worst =
            |at: fn(&[Duration]) -> Duration| runs.iter().map(|r| at(r)).max().unwrap_or_default();
        let p95 = worst(|r| r[(r.len() * 95).div_ceil(100) - 1]);
        let gate = match gated {
            false => "reported",
            true if p95 < GATE => "pass",
            true => "FAIL",
        };
        passed &= gate != "FAIL";
        println!(
            "{label:<12} {:>10} {:>10}  {gate}",
            ms(worst(|r| r[r.len() / 2])),
            ms(p95)
        );
    }
    passed
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

/// The text of `name` for sample `n`: a body edit changes one constant; `t` turns between Int
/// and Float, so its users recheck and take its new signature.
fn body(name: &str, n: usize) -> String {
    let def = |name: &str| {
        definitions()
            .find(|d| d.starts_with(&format!("fn {name}(")))
            .unwrap_or_default()
    };
    match name {
        "step" => def(name).replace("Hold => x", &format!("Hold => x + {n}")),
        "walk" => def(name).replace("next > 1000", &format!("next > {}", 1000 + n)),
        "simulate" => def(name).replace("cap * 1000", &format!("cap * {}", 1000 + n)),
        _ if n.is_multiple_of(2) => format!("fn t(x: Int) -> Int {{\n  x / 2 + {n}\n}}"),
        _ => format!("fn t(x: Int) -> Float {{\n  to_float(x) / 2.0 + {n}.0\n}}"),
    }
}

/// The definitions of the benchmark file, without its comments and examples.
fn definitions() -> impl Iterator<Item = String> {
    let mut defs: Vec<String> = vec![];
    for line in DEFS
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
    {
        match defs.last_mut() {
            Some(d) if line.starts_with([' ', '}']) => {
                d.push('\n');
                d.push_str(line);
            }
            _ => defs.push(line.into()),
        }
    }
    defs.into_iter().filter(|d| !d.starts_with("example"))
}

/// The program: the benchmark's definitions, an app that calls each once, `t`, and fillers of
/// about 20 numeric lines in a binary call tree from `f0`.
fn write_program(dir: &Path, size: usize) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let fillers = size - FIXED;
    let mut files: Vec<(String, String)> = definitions()
        .map(|d| {
            (
                d.split([' ', '(']).nth(1).unwrap_or_default().to_string(),
                d,
            )
        })
        .collect();
    files.extend([
        ("t".into(), "fn t(x: Int) -> Int {\n  x / 2\n}".into()),
        ("msg".into(), "enum Msg { Start(arg: String) }".into()),
        ("total".into(), "fn total(x: Int) -> Int {\n  f0(x) + step(x, Up, by: 2) + walk(x, 40, d: 3) + simulate(x, 50, cap: 7)\n}".into()),
        ("update".into(), "fn update(m: Msg) -> List<Effect<Msg>> {\n  match m {\n    Start(arg) => [Print(\"{total(len(arg))}\"), Exit]\n  }\n}".into()),
        ("app".into(), "app bench {\n  start: Start\n  update: update\n  grants: [Console]\n}".into()),
    ]);
    files.extend((0..fillers).map(|i| (format!("f{i:05}"), filler(i, fillers))));
    for (name, text) in files {
        fs::write(dir.join(format!("{name}.paka")), text + "\n")?;
    }
    Ok(())
}

fn filler(i: usize, count: usize) -> String {
    let mut lines = vec![format!("  let a0 = x % 1000 + {i}")];
    lines.extend((1..18).map(|k| {
        format!(
            "  let a{k} = (a{} * {} + a{} + {k}) % 9973",
            k - 1,
            k + 2,
            k / 2
        )
    }));
    if (1..=USERS).contains(&i) {
        lines.push("  let a18 = if t(a17) == t(a17) { a17 } else { 0 }".into());
    } else {
        lines.push("  let a18 = a17".into());
    }
    let calls: String = [2 * i + 1, 2 * i + 2]
        .iter()
        .filter(|&&c| c < count)
        .map(|c| format!(" + f{c}(a18)"))
        .collect();
    lines.push(format!("  a18{calls}"));
    format!("fn f{i}(x: Int) -> Int {{\n{}\n}}", lines.join("\n"))
}

/// The run every edit asks for.
fn run() -> String {
    object([("app", quote("bench")), ("args", array([quote("x")]))])
}

fn first(v: &Value) -> Option<&Value> {
    match v {
        Value::Array(items) => items.first(),
        _ => None,
    }
}

fn fail(what: &str, reply: &Value) -> io::Error {
    io::Error::other(format!("{what}: {reply:?}"))
}

/// A `paka serve` child and its pipes.
struct Server {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    id: u64,
}

impl Server {
    fn start(paka: &Path, dir: &Path) -> io::Result<Server> {
        let mut child = Command::new(paka)
            .arg("serve")
            .arg(dir)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let (input, output) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        Ok(Server {
            child,
            input,
            output: BufReader::new(output),
            id: 0,
        })
    }

    /// Sends a call of `fields`; how long its reply took, and the reply, refused unless ok.
    fn call<'a>(
        &mut self,
        fields: impl IntoIterator<Item = (&'a str, String)>,
    ) -> io::Result<(Duration, Value)> {
        self.id += 1;
        let head = [("v", "1".into()), ("id", self.id.to_string())];
        let request = object(head.into_iter().chain(fields));
        let start = Instant::now();
        writeln!(self.input, "{request}")?;
        self.input.flush()?;
        let mut line = String::new();
        self.output.read_line(&mut line)?;
        let time = start.elapsed();
        let reply = parse(&line).map_err(|e| io::Error::other(format!("{e}: {line}")))?;
        if reply.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(fail("refused", &reply));
        }
        Ok((time, reply))
    }

    /// Times an edit that asks for a run; the new version. The edit must install and run cleanly.
    fn edit(
        &mut self,
        name: &str,
        base: Option<&str>,
        text: Option<&str>,
    ) -> io::Result<(Duration, String)> {
        let json = |s: Option<&str>| s.map_or("null".into(), quote);
        let fields = [
            ("call", quote("edit")),
            ("name", quote(name)),
            ("base", json(base)),
            ("text", json(text)),
            ("run", run()),
        ];
        let (time, reply) = self.call(fields)?;
        let status = reply.get("run").and_then(|r| r.get("status")?.as_u64());
        if reply.get("installed").and_then(Value::as_bool) != Some(true) || status != Some(0) {
            return Err(fail(&format!("edit of {name} did not run"), &reply));
        }
        Ok((
            time,
            reply
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        ))
    }

    fn stop(mut self) -> io::Result<()> {
        drop(self.input);
        self.child.wait()?;
        Ok(())
    }
}
