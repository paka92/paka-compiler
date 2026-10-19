//! The shared vectors (tests/vectors/ops.vec) run as dev machine code: each vector becomes a
//! definition, compiled by `paka::emit`, installed and run (M4-26).

use paka::emit::Emitter;
use paka::source::Source;
use paka_installer::{Slots, install};
use paka_rt::abi::dev::Context;
use paka_rt::ops::Trap;
use paka_rt::ops::vectors::{self, Outcome, Value, Vector};
use std::ffi::c_void;

const VECTORS: &str = include_str!("../../tests/vectors/ops.vec");

/// The operations M4-26 compiles: Int, Float and Bool, without text.
fn compiled(op: &str) -> bool {
    let scalar = ["int.", "float.", "bool."]
        .iter()
        .any(|p| op.starts_with(p));
    scalar && !op.ends_with(".text") || op == "to_int" || op == "to_float"
}

fn result_type(op: &str) -> &'static str {
    let name = op.rsplit('.').next().unwrap();
    match name {
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" => "Bool",
        "to_int" => "Int",
        "to_float" => "Float",
        _ => match op.split('.').next().unwrap() {
            "int" => "Int",
            "float" => "Float",
            _ => "Bool",
        },
    }
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

/// A Float literal: Paka has no NaN or infinity literal, and a sign is a negation.
fn float(bits: u64) -> String {
    let negative = bits >> 63 == 1;
    let size = f64::from_bits(bits & !(1 << 63));
    let text = match size {
        _ if size.is_nan() => return "(0.0 / 0.0)".into(),
        _ if size.is_infinite() => "(1.0 / 0.0)".into(),
        _ => {
            let digits = size.to_string();
            if digits.contains('.') {
                digits
            } else {
                digits + ".0"
            }
        }
    };
    match negative {
        true => format!("(-{text})"),
        false => text,
    }
}

fn call(op: &str, inputs: &[Value]) -> String {
    let x: Vec<String> = inputs.iter().map(literal).collect();
    let infix = |sign| format!("{} {sign} {}", x[0], x[1]);
    match op.rsplit('.').next().unwrap() {
        "add" => infix("+"),
        "sub" => infix("-"),
        "mul" => infix("*"),
        "div" => infix("/"),
        "rem" => infix("%"),
        "eq" => infix("=="),
        "ne" => infix("!="),
        "lt" => infix("<"),
        "le" => infix("<="),
        "gt" => infix(">"),
        "ge" => infix(">="),
        "and" => infix("&&"),
        "or" => infix("||"),
        "neg" => format!("-{}", x[0]),
        "not" => format!("!{}", x[0]),
        name => format!("{name}({})", x.join(", ")),
    }
}

fn program(vectors: &[&Vector]) -> String {
    let def = |(n, v): (usize, &&Vector)| {
        let body = call(&v.op, &v.inputs);
        format!("fn v{n}() -> {} {{ {body} }}\n", result_type(&v.op))
    };
    vectors.iter().enumerate().map(def).collect()
}

fn outcome(op: &str, status: i32, ctx: &Context, out: u64) -> Outcome {
    match status {
        0 => Ok(match result_type(op) {
            "Int" => Value::Int(out as i64),
            "Float" => Value::Float(out),
            _ => Value::Bool(out != 0),
        }),
        1 => Err(Trap::from_name(TRAPS[ctx.trap.kind as usize - 1]).unwrap()),
        _ => panic!("status {status}"),
    }
}

/// The trap kinds 1 to 4 of M4-05 section 6.
const TRAPS: [&str; 4] = [
    "overflow",
    "divide_by_zero",
    "out_of_bounds",
    "bad_conversion",
];

#[test]
fn the_vectors_pass_as_machine_code() {
    let all = vectors::parse(VECTORS).unwrap();
    let chosen: Vec<&Vector> = all.iter().filter(|v| compiled(&v.op)).collect();
    let source = program(&chosen);
    let (p, ..) = paka::check("main.paka", &Source::file("main.paka", source.into_bytes()))
        .unwrap_or_else(|d| panic!("{d}"));
    let mut emitter = Emitter::new().unwrap();
    let code: Vec<Vec<u8>> = (0..chosen.len())
        .map(|d| emitter.code(&p, d).unwrap())
        .collect();
    let installed = install(&code.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
    let slots = Slots::new(chosen.len());
    let group: Vec<_> = installed
        .iter()
        .enumerate()
        .map(|(id, c)| (id, c.address))
        .collect();
    slots.swap(&group).unwrap();

    let mut wrong = vec![];
    for (id, v) in chosen.iter().enumerate() {
        let mut ctx = Context::new(0);
        let mut out = 0u64;
        let ctx_ptr = (&raw mut ctx).cast::<c_void>();
        // SAFETY: slot `id` holds `fn v{id}`, compiled as `fn(ctx, out) -> i32`, and `ctx` and
        // `out` live through the run.
        let status = unsafe { slots.run(id, ctx_ptr, (&raw mut out).cast()) }.unwrap();
        let got = outcome(&v.op, status, &ctx, out);
        if !vectors::same(&got, &v.want) {
            let want = &v.want;
            wrong.push(format!(
                "line {}: {} {:?}: got {got:?}, want {want:?}",
                v.line, v.op, v.inputs
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {}:\n{}",
        wrong.len(),
        chosen.len(),
        wrong.join("\n")
    );
}
