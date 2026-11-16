//! `paka-bench --visual|--window [ITEMS [SAMPLES]]`: edit-to-frame time on a running visual app
//! (M5-43). Serves the to-do sample of bench/ui/todo, runs it, fills it with ITEMS items, 1,000 by
//! default, and times SAMPLES view-body and handler-body edits each, 50 by default, from request
//! written to reply read: by then the app's new tree is built. `--visual` runs the app with no
//! window, in the server; `--window` runs it in a window, in the runner, and also times each
//! view edit to the log line of its presented frame. A handler edit leaves the tree as it was, so
//! no new frame follows it. Exits 1 when a 95th percentile reaches 100 ms.

use super::{GATE, Server, exit, fail, ms, percentiles, scratch};
use paka_json::{Value, array, quote};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use std::{env, fs, io};

const TODO: &str = include_str!("../../../../bench/ui/todo/main.paka");
/// How long a frame may take to be logged.
const FRAME: Duration = Duration::from_secs(10);

/// Whether `flag` asks for a window; `None` when it is not this benchmark's.
pub fn mode(flag: &str) -> Option<bool> {
    match flag {
        "--visual" => Some(false),
        "--window" => Some(true),
        _ => None,
    }
}

pub fn main(args: &[String], window: bool) -> ExitCode {
    let arg = |at: usize, default: usize| args.get(at).map_or(Ok(default), |a| a.parse());
    let (Ok(items), Ok(samples @ 1..)) = (arg(0, 1000), arg(1, 50)) else {
        eprintln!("usage: paka-bench --visual|--window [ITEMS [SAMPLES]]");
        return ExitCode::from(2);
    };
    if window && env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("paka-bench: --window needs a Wayland display");
        return ExitCode::from(2);
    }
    let dir = scratch();
    let result = bench(&dir, items, samples, window);
    let _ = fs::remove_dir_all(&dir);
    exit(result)
}

/// Whether every 95th percentile is under the gate.
fn bench(dir: &Path, items: usize, samples: usize, window: bool) -> io::Result<bool> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("main.paka"), TODO)?;
    let paka = env::current_exe()?.with_file_name("paka");
    let mut server = Server::start(&paka, dir, window)?;
    let launched = Instant::now();
    let (_, ran) = server.call([
        ("call", quote("run")),
        ("app", quote("todo")),
        ("args", array([])),
    ])?;
    let shown = ran.get("run").and_then(|r| r.get("window"));
    if shown != Some(&Value::Bool(window)) {
        return Err(fail("the app did not run as asked", &ran));
    }
    if window {
        server.frame(launched)?;
    }
    fill(&mut server, items)?;
    // Each kind: view edits, their frames, handler edits.
    let mut times = [vec![], vec![], vec![]];
    let (row, add) = (original("todo_row"), original("add_item"));
    for n in 1..=samples {
        let gap = format!("Row(gap: {})", 8 + n % 2);
        let start = Instant::now();
        times[0].push(server.edit_def("todo_row", row.replace("Row(gap: 8)", &gap))?);
        if window {
            times[1].push(server.frame(start)?);
        }
        let draft = format!("s.draft = \"{n}\"");
        times[2].push(server.edit_def("add_item", add.replace("s.draft = \"\"", &draft))?);
    }
    let passed = report(items, samples, &mut times);
    server.stop()?;
    Ok(passed)
}

/// Gives the app `items` items: `add_item` appends that many for one click on Add, then takes
/// its own text back.
fn fill(server: &mut Server, items: usize) -> io::Result<()> {
    let many = format!(
        "fn add_item(s: inout Todos) {{\n  for i in 0..<{items} {{\n    append(&s.items, Todo {{ id: len(s.items) + 1, title: \"item {{i}}\" }})\n  }}\n}}"
    );
    server.edit_def("add_item", many)?;
    server.call([("call", quote("look_click")), ("path", quote("#1"))])?;
    server.edit_def("add_item", original("add_item"))?;
    let (_, tree) = server.call([("call", quote("look_tree"))])?;
    let rows = tree
        .get("tree")
        .and_then(Value::as_str)
        .map_or(0, |t| t.matches("Row ").count());
    if rows != items {
        return Err(io::Error::other(format!("{rows} rows shown, not {items}")));
    }
    Ok(())
}

/// The sample's text of the function or view `name`.
fn original(name: &str) -> String {
    let start = ["fn", "view"]
        .iter()
        .find_map(|kind| TODO.find(&format!("\n{kind} {name}(")))
        .expect("a definition of the sample")
        + 1;
    let end = TODO[start..].find("\n}").expect("a closed definition") + start + 2;
    TODO[start..end].into()
}

/// Prints the median and 95th percentile of each kind timed; whether each is under the gate.
fn report(items: usize, samples: usize, times: &mut [Vec<Duration>; 3]) -> bool {
    println!("to-do app with {items} items, {samples} edits per kind\n");
    println!("{:<14} {:>10} {:>10}  gate", "edit", "median", "p95");
    let mut passed = true;
    for (label, t) in ["view", "view frame", "handler"].iter().zip(times) {
        if t.is_empty() {
            continue;
        }
        t.sort();
        let (median, p95) = percentiles(t);
        passed &= p95 < GATE;
        let gate = if p95 < GATE { "pass" } else { "FAIL" };
        println!("{label:<14} {:>10} {:>10}  {gate}", ms(median), ms(p95));
    }
    passed
}

impl Server {
    /// Times an edit of `name` to `text`, which must install.
    fn edit_def(&mut self, name: &str, text: String) -> io::Result<Duration> {
        let (_, viewed) = self.call([("call", quote("view")), ("name", quote(name))])?;
        let base = viewed
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let fields = [
            ("call", quote("edit")),
            ("name", quote(name)),
            ("base", quote(base)),
            ("text", quote(&text)),
        ];
        let (time, reply) = self.call(fields)?;
        if reply.get("installed").and_then(Value::as_bool) != Some(true) {
            return Err(fail(&format!("edit of {name} did not install"), &reply));
        }
        Ok(time)
    }

    /// How long after `start` the first frame presented since was logged.
    fn frame(&mut self, start: Instant) -> io::Result<Duration> {
        let log = self.log.as_ref().expect("a window logs its frames");
        loop {
            let (at, line) = log
                .recv_timeout(FRAME)
                .map_err(|_| io::Error::other("no frame presented"))?;
            if at >= start && line.contains("frame presented") {
                return Ok(at.duration_since(start));
            }
        }
    }
}
