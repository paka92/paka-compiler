//! The code generator benchmark of docs/decisions/M4-01.md, for Cranelift.
//!
//! `paka-bench` runs the whole method and prints the report; `warm` and `cold` are the measured
//! processes it starts.

mod code;
mod lower;

use code::{Code, Trap};
use cranelift_codegen::Context;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::isa::{self, OwnedTargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use paka::core_form::Program;
use paka::source::Source;
use std::process::Command;
use std::time::{Duration, Instant};
use std::{env, fs};

const DEFS: &str = include_str!("../defs.paka");
const UP: i64 = 0;
const DOWN: i64 = 1;
const HOLD: i64 = 2;
const NAMES: [&str; 3] = ["step", "walk", "simulate"];

/// Each call of section 2 and what it must give; the first of each definition is the timed one.
const CASES: [(&str, [i64; 3], Result<i64, Trap>); 13] = [
    ("step", [5, UP, 3], Ok(8)),
    ("step", [5, DOWN, 3], Ok(2)),
    ("step", [5, HOLD, 3], Ok(5)),
    ("walk", [0, 40, 3], Ok(59)),
    ("walk", [990, 100, 7], Ok(8112)),
    ("walk", [-5, 1, 1], Ok(8)),
    ("simulate", [1, 50, 7], Ok(2164)),
    ("simulate", [42, 500, 13], Ok(137150)),
    ("simulate", [2026, 2000, 101], Ok(343598)),
    ("step", [i64::MAX, UP, 1], Err(Trap::Overflow)),
    ("walk", [0, 40, 0], Err(Trap::DivideByZero)),
    ("simulate", [1, 50, 0], Err(Trap::DivideByZero)),
    ("simulate", [1, 50, i64::MAX], Err(Trap::Overflow)),
];

fn main() {
    let program = load();
    match env::args().nth(1).as_deref() {
        Some("warm") => warm(&program),
        Some("cold") => cold(&program),
        _ => report(&program),
    }
}

fn load() -> Program {
    let source = Source::file("defs.paka", DEFS.as_bytes().to_vec());
    paka::check("defs.paka", &source)
        .unwrap_or_else(|d| panic!("{d}"))
        .0
}

/// The generator: one ISA and one set of contexts, reused for every compile.
struct Cranelift {
    isa: OwnedTargetIsa,
    builder: FunctionBuilderContext,
    context: Context,
    slots: Vec<u64>,
    codes: Vec<Option<Code>>,
}

impl Cranelift {
    fn new(p: &Program, verify: bool) -> Cranelift {
        let mut flags = settings::builder();
        flags.set("opt_level", "none").unwrap();
        flags.set("enable_verifier", &verify.to_string()).unwrap();
        let mut native = isa::lookup_by_name("x86_64-unknown-linux-gnu").unwrap();
        for (flag, has) in [
            ("has_sse3", is_x86_feature_detected!("sse3")),
            ("has_ssse3", is_x86_feature_detected!("ssse3")),
            ("has_sse41", is_x86_feature_detected!("sse4.1")),
            ("has_sse42", is_x86_feature_detected!("sse4.2")),
            ("has_popcnt", is_x86_feature_detected!("popcnt")),
            ("has_avx", is_x86_feature_detected!("avx")),
            ("has_avx2", is_x86_feature_detected!("avx2")),
            ("has_bmi1", is_x86_feature_detected!("bmi1")),
            ("has_bmi2", is_x86_feature_detected!("bmi2")),
            ("has_lzcnt", is_x86_feature_detected!("lzcnt")),
        ] {
            native.set(flag, &has.to_string()).unwrap();
        }
        Cranelift {
            isa: native.finish(settings::Flags::new(flags)).unwrap(),
            builder: FunctionBuilderContext::new(),
            context: Context::new(),
            slots: code::slots(p.fns.len()),
            codes: p.fns.iter().map(|_| None).collect(),
        }
    }

    /// Compiles definition `id` from the core form and installs it in its slot.
    fn install(&mut self, p: &Program, id: usize) {
        self.context.func = lower::function(&*self.isa, p, id, &mut self.builder);
        let compiled = self
            .context
            .compile(&*self.isa, &mut ControlPlane::default());
        let compiled = compiled.unwrap_or_else(|e| panic!("{:?}", e.inner));
        assert!(
            compiled.buffer.relocs().is_empty(),
            "the code must not need relocating"
        );
        let code = Code::install(compiled.code_buffer());
        self.context.clear();
        self.slots[id] = code.address();
        self.codes[id] = Some(code);
    }

    fn call(&self, id: usize, args: [i64; 3]) -> Result<i64, Trap> {
        code::call(&self.slots, id, args)
    }
}

fn id(p: &Program, name: &str) -> usize {
    p.fns
        .iter()
        .position(|f| f.name == name)
        .expect("the definition is in defs.paka")
}

/// A generator with every definition installed, checked against every case once.
fn verified(p: &Program, verify: bool) -> Cranelift {
    let mut cl = Cranelift::new(p, verify);
    for name in NAMES {
        cl.install(p, id(p, name));
    }
    for (name, args, want) in CASES {
        let got = cl.call(id(p, name), args);
        assert_eq!(got, want, "{name}{args:?}");
    }
    cl
}

/// Per definition: 20 untimed compiles, then 200 timed ones, each to the first right result.
fn warm(p: &Program) {
    let mut cl = verified(p, false);
    for name in NAMES {
        let id = id(p, name);
        let &(_, args, want) = CASES.iter().find(|c| c.0 == name).unwrap();
        let mut times: Vec<_> = (0..220)
            .map(|_| {
                let start = Instant::now();
                cl.install(p, id);
                let got = cl.call(id, args);
                let time = start.elapsed();
                assert_eq!(got, want, "{name}{args:?}");
                time
            })
            .skip(20)
            .collect();
        times.sort();
        println!(
            "{name} {} {}",
            nanos(percentile(&times, 50)),
            nanos(percentile(&times, 95))
        );
    }
    println!("peak {}", peak_kib());
}

/// From the generator's first call to the first right result of the large definition.
fn cold(p: &Program) {
    let start = Instant::now();
    let mut cl = Cranelift::new(p, false);
    for name in NAMES {
        cl.install(p, id(p, name));
    }
    let got = cl.call(id(p, "simulate"), CASES[6].1);
    let time = start.elapsed();
    assert_eq!(got, CASES[6].2);
    println!("cold {}", nanos(time));
}

fn report(p: &Program) {
    verified(p, true);
    println!("Cranelift 0.136.3, opt_level=none; every result and trap right (verifier on)\n");
    let runs: Vec<_> = (0..3).map(|_| measure("warm")).collect();
    let worst = |key: &str, at: usize| runs.iter().map(|run| field(run, key)[at]).max().unwrap();
    println!("warm, 200 compiles after 20, worst of three processes:");
    for name in NAMES {
        println!(
            "  {name:<9} p50 {:>6.3} ms   p95 {:>6.3} ms",
            ms(worst(name, 0)),
            ms(worst(name, 1))
        );
    }
    let (large, peak) = (ms(worst("simulate", 1)), worst("peak", 0));
    measure("cold");
    let mut cold: Vec<_> = (0..20)
        .map(|_| field(&measure("cold"), "cold")[0])
        .collect();
    cold.sort();
    let cold = ms(cold[18]);
    println!("cold, second slowest of 20: {cold:.3} ms\n");
    let pass = large <= 20.0 && cold <= 50.0;
    println!(
        "rule: warm p95 (large) <= 20 ms and cold <= 50 ms: {}",
        if pass { "pass" } else { "fail" }
    );
    println!("\nOutside the rule:");
    println!(
        "  run time of simulate(2026, 2000, cap: 101): {:.3} ms",
        ms(run_time(p))
    );
    println!("  peak memory of a warm process: {peak} KiB");
    let outside = include_str!("../Cargo.lock")
        .matches("source = \"registry+")
        .count();
    println!("  outside crates: {outside}");
    let sources = [
        include_str!("main.rs"),
        include_str!("lower.rs"),
        include_str!("code.rs"),
    ];
    let unsafe_lines = sources
        .iter()
        .flat_map(|s| s.lines())
        .filter(|l| l.contains("unsafe"))
        .count();
    println!("  lines with unsafe: {unsafe_lines}");
}

/// Median of 21 runs, after installing every definition.
fn run_time(p: &Program) -> u64 {
    let cl = verified(p, false);
    let mut times: Vec<_> = (0..21)
        .map(|_| {
            let start = Instant::now();
            cl.call(id(p, "simulate"), CASES[8].1).unwrap();
            nanos(start.elapsed())
        })
        .collect();
    times.sort();
    times[10]
}

/// Runs this program as `mode` in a new process; gives its output lines split into words.
fn measure(mode: &str) -> Vec<Vec<String>> {
    let out = Command::new(env::current_exe().unwrap())
        .arg(mode)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    text.lines()
        .map(|l| l.split(' ').map(String::from).collect())
        .collect()
}

/// The numbers on the output line that starts with `key`.
fn field(lines: &[Vec<String>], key: &str) -> Vec<u64> {
    let line = lines.iter().find(|l| l[0] == key).unwrap();
    line[1..].iter().map(|w| w.parse().unwrap()).collect()
}

fn percentile(sorted: &[Duration], q: usize) -> Duration {
    sorted[(sorted.len() * q).div_ceil(100) - 1]
}

fn nanos(d: Duration) -> u64 {
    d.as_nanos() as u64
}

fn ms(nanos: u64) -> f64 {
    nanos as f64 / 1e6
}

/// The process's peak resident memory.
fn peak_kib() -> u64 {
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmHWM:")).unwrap();
    line.split_whitespace().nth(1).unwrap().parse().unwrap()
}
