//! Random programs from paka-gen (M4-21): each checks, finishes in the interpreter, and together
//! they produce every core-form node kind but a view's. `PAKA_GEN_SEEDS` sets how many (default 10,000).

use paka::coverage::core::{CORE_KINDS, VIEW_KINDS, kinds};
use paka::source::Source;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

fn seeds() -> u64 {
    std::env::var("PAKA_GEN_SEEDS").map_or(10_000, |n| n.parse().unwrap())
}

#[test]
fn generated_programs_check_and_finish() {
    let n = seeds();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = Arc::new(AtomicU64::new(0));
    let found = Arc::new(Mutex::new(Found::default()));
    let workers: Vec<_> = (0..threads)
        .map(|_| {
            let (next, found) = (next.clone(), found.clone());
            // The main thread's stack, which `paka` checks on.
            let worker = std::thread::Builder::new().stack_size(8 << 20);
            worker.spawn(move || {
                loop {
                    let seed = next.fetch_add(1, Ordering::Relaxed);
                    if seed >= n {
                        break;
                    }
                    let one = one(seed);
                    found.lock().unwrap().add(seed, one);
                }
            })
        })
        .collect();
    workers.into_iter().for_each(|w| w.unwrap().join().unwrap());
    let found = found.lock().unwrap();
    println!("statuses over {n} programs: {:?}", found.statuses);
    for kind in CORE_KINDS {
        println!("{:>6} {kind}", found.kinds.get(kind).unwrap_or(&0));
    }
    assert!(found.failures.is_empty(), "{}", found.failures.join("\n"));
    let missing: Vec<_> = CORE_KINDS
        .iter()
        .filter(|k| !found.kinds.contains_key(*k) && !VIEW_KINDS.contains(k))
        .collect();
    assert!(missing.is_empty(), "never produced: {missing:?}");
}

#[derive(Default)]
struct Found {
    /// Programs that produced each kind.
    kinds: BTreeMap<&'static str, u64>,
    statuses: BTreeMap<i32, u64>,
    failures: Vec<String>,
}

impl Found {
    fn add(&mut self, seed: u64, one: Result<(Vec<&'static str>, i32), String>) {
        match one {
            Ok((kinds, status)) => {
                kinds
                    .into_iter()
                    .for_each(|k| *self.kinds.entry(k).or_default() += 1);
                *self.statuses.entry(status).or_default() += 1;
            }
            Err(why) => self.failures.push(format!("seed {seed}: {why}")),
        }
    }
}

/// The kinds of program `seed` and the status it ran to.
fn one(seed: u64) -> Result<(Vec<&'static str>, i32), String> {
    let text = paka_gen::program(seed);
    let source = Source::file("main.paka", text.clone().into_bytes());
    let (program, ..) = paka::check("main.paka", &source).map_err(|d| d.to_string())?;
    let kinds = kinds(&program).0.into_iter().collect();
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("gen-{seed}.paka"));
    std::fs::write(&path, &text).unwrap();
    let status = run(&path)?;
    if matches!(status, 0 | 3 | 5..=255) {
        std::fs::remove_file(&path).unwrap();
        return Ok((kinds, status));
    }
    Err(format!("status {status}, kept at {}", path.display()))
}

fn run(path: &Path) -> Result<i32, String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_paka"))
        .arg("run")
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Ok(status.code().unwrap_or(-1));
        }
        if start.elapsed() > TIMEOUT {
            child.kill().unwrap();
            child.wait().unwrap();
            return Err(format!(
                "no end within {TIMEOUT:?}, kept at {}",
                path.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
