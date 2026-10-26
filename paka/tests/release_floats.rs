//! The Float vectors of tests/vectors/ops.vec, NaN rows included, built by `paka build` at full
//! optimization and compared bit for bit (M6-08). Each vector is a function; `update` asserts its
//! result, so a wrong one traps at its own line. Each trapping vector is a program of its own.

#[allow(dead_code)]
mod common;

use common::{Scratch, build_native};
use paka_rt::ops::vectors::{self, Value, Vector};
use std::process::Command;

const VECTORS: &str = include_str!("../../tests/vectors/ops.vec");

const SAME: &str = "
fn same(x: Float, want: Float) -> Bool {
  if want != want { x != x } else { x == want && 1.0 / x == 1.0 / want }
}
";

fn floats(v: &Vector) -> bool {
    v.op.starts_with("float.") || v.op == "to_int" || v.op == "to_float"
}

fn literal(v: &Value) -> String {
    match *v {
        Value::Int(i64::MIN) => "(-9223372036854775807 - 1)".into(),
        Value::Int(n) if n < 0 => format!("({n})"),
        Value::Int(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Float(bits) => float(bits),
        ref other => panic!("not a scalar: {other:?}"),
    }
}

/// Paka has no NaN or infinity literal, and a sign is a negation.
fn float(bits: u64) -> String {
    let size = f64::from_bits(bits & !(1 << 63));
    let text = match size {
        _ if size.is_nan() => "(0.0 / 0.0)".into(),
        _ if size.is_infinite() => "(1.0 / 0.0)".into(),
        _ => {
            let digits = size.to_string();
            match digits.contains('.') {
                true => digits,
                false => digits + ".0",
            }
        }
    };
    match bits >> 63 {
        1 => format!("(-{text})"),
        _ => text,
    }
}

fn call(v: &Vector) -> String {
    let x: Vec<String> = v.inputs.iter().map(literal).collect();
    let infix = |sign| format!("{} {sign} {}", x[0], x[1]);
    match v.op.rsplit('.').next().unwrap() {
        "add" => infix("+"),
        "sub" => infix("-"),
        "mul" => infix("*"),
        "div" => infix("/"),
        "eq" => infix("=="),
        "ne" => infix("!="),
        "lt" => infix("<"),
        "le" => infix("<="),
        "gt" => infix(">"),
        "ge" => infix(">="),
        "neg" => format!("-{}", x[0]),
        name => format!("{name}({})", x.join(", ")),
    }
}

/// `update` running `checks`, one per line from line 4, after the functions in `defs`.
fn program(checks: &[String], defs: &str) -> String {
    let body: String = checks.iter().map(|c| format!("  {c}\n")).collect();
    format!(
        "enum Msg {{ Start }}\n\nfn update(msg: Msg) -> List<Effect<Msg>> {{\n{body}  [Exit]\n}}\n\n\
         app floats {{\n  start: Start\n  update: update\n  grants: []\n}}\n{defs}"
    )
}

/// Builds `source` and runs it: its status and standard error.
fn run(scratch: &Scratch, source: &str) -> (Option<i32>, String) {
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    std::fs::write(&path, source).unwrap();
    let built = build_native(path.to_str().unwrap(), &exe);
    assert!(built.status.success(), "{built:?}");
    let out = Command::new(&exe).output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    (out.status.code(), stderr)
}

#[test]
fn the_float_vectors_pass_bit_for_bit_in_release() {
    let all = vectors::parse(VECTORS).unwrap();
    let chosen: Vec<&Vector> = all.iter().filter(|v| floats(v)).collect();
    let nan = |v: &Value| matches!(v, Value::Float(b) if b & !(1 << 63) > 0x7FF0_0000_0000_0000);
    let nans = chosen
        .iter()
        .filter(|v| v.inputs.iter().any(nan) || v.want.as_ref().is_ok_and(nan));
    assert!(nans.count() > 40, "the NaN vectors run too");
    let (good, traps): (Vec<&Vector>, Vec<&Vector>) = chosen.iter().partition(|v| v.want.is_ok());
    let scratch = Scratch::new("release-floats");

    let mut defs = SAME.to_string();
    let mut checks = vec![];
    for (n, v) in good.iter().enumerate() {
        let want = v.want.as_ref().unwrap();
        let ty = match want {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            _ => "Bool",
        };
        defs += &format!("\nfn v{n}() -> {ty} {{\n  {}\n}}\n", call(v));
        checks.push(match want {
            Value::Float(_) => format!("assert(same(v{n}(), {}))", literal(want)),
            _ => format!("assert(v{n}() == {})", literal(want)),
        });
    }
    let (status, stderr) = run(&scratch, &program(&checks, &defs));
    let failed = || {
        let line: usize = stderr.split(':').nth(2)?.parse().ok()?;
        good.get(line.checked_sub(4)?)
            .map(|v| (v.line, &v.op, &v.inputs))
    };
    assert_eq!(status, Some(0), "{stderr} vector {:?}", failed());

    for v in traps {
        let kind = v.want.as_ref().unwrap_err().name();
        let source = program(&[format!("let n = {}", call(v))], "");
        let (status, stderr) = run(&scratch, &source);
        let want = format!("floats: trap {kind} at main.paka:4:11\n");
        assert_eq!((status, stderr), (Some(3), want), "line {}", v.line);
    }
}
