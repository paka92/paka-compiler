//! The foreign host (M7-14, M7-15): the test library in tests/foreign is called through a host in
//! its own process; malformed requests, a crash or a stall in the library leave the caller running.

#[allow(dead_code)]
mod common;

use common::Scratch;
use paka::ast::Def;
use paka::core_form::Library;
use paka::foreign::{Failure, Host, Kind, LIMIT, Value, build, cached, library};
use paka::parser;
use paka_rt::ops::Float;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/foreign");

const PROBE: &str = r#"
foreign Probe {
  source: "probe.c"
  header: "probe.h"
  Sum(text: String) -> Int = "probe_sum"
  Scale(x: Float, by: Int, negate: Bool) -> Float = "probe_scale"
  Odd(n: Int) -> Bool = "probe_odd"
  Count() -> Int = "probe_count"
  Crash() = "probe_crash"
  Stall() = "probe_stall"
}
"#;

/// The one declaration in `source`, whose files are in `dir`.
fn declared(source: &str, dir: &Path) -> Library {
    let program = parser::parse("main.paka", source.as_bytes()).unwrap();
    let [Def::Foreign { name, lines }] = &program.defs[..] else {
        panic!("one foreign declaration expected");
    };
    library(name.text, dir, lines)
}

fn host_of(source: &str, dir: &Path, out: &Path) -> Result<PathBuf, String> {
    build(&declared(source, dir), out)
}

fn float(x: f64) -> Value<'static> {
    Value::Float(Float::from_bits(x.to_bits()))
}

fn probe(scratch: &Scratch) -> Host {
    let exe = host_of(PROBE, Path::new(LIBRARY), &scratch.path("host")).unwrap();
    Host::start(&exe, LIMIT).unwrap()
}

#[test]
fn the_library_answers_through_the_host() {
    let scratch = Scratch::new("foreign-answers");
    let mut host = probe(&scratch);
    let text = "héllo\0x";
    let sum = text.bytes().map(i64::from).sum();
    let calls = [
        (0, vec![Value::String(text)], Kind::Int, Value::Int(sum)),
        (0, vec![Value::String("")], Kind::Int, Value::Int(0)),
        (
            1,
            vec![float(1.5), Value::Int(-4), Value::Bool(true)],
            Kind::Float,
            float(6.0),
        ),
        (
            2,
            vec![Value::Int(i64::MIN + 1)],
            Kind::Bool,
            Value::Bool(true),
        ),
        (2, vec![Value::Int(8)], Kind::Bool, Value::Bool(false)),
    ];
    for (n, args, kind, answer) in calls {
        assert_eq!(
            host.call(n, &args, Some(kind)),
            Ok(Some(answer)),
            "{n} {args:?}"
        );
    }
    // State is kept between calls, and what the library prints or reads misses the pipe.
    for count in 1..=3 {
        assert_eq!(
            host.call(3, &[], Some(Kind::Int)),
            Ok(Some(Value::Int(count)))
        );
    }
}

#[test]
fn malformed_requests_are_refused_and_the_host_goes_on() {
    let scratch = Scratch::new("foreign-malformed");
    let mut host = probe(&scratch);
    let long = [&0u32.to_le_bytes()[..], &u64::MAX.to_le_bytes(), b"ab"].concat();
    let requests: [&[u8]; 7] = [
        b"",
        &[0, 0],
        &[9, 0, 0, 0],
        &[0, 0, 0, 0, 1, 0],
        &long,
        &[2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        &[3, 0, 0, 0, 0],
    ];
    for request in requests {
        assert_eq!(host.exchange(request), Ok(vec![1]), "{request:?}");
    }
    let bad_bool = [&1u32.to_le_bytes()[..], &[0; 16], &[2]].concat();
    assert_eq!(host.exchange(&bad_bool), Ok(vec![1]));
    // An answer of another kind than expected is a bad reply, not a value.
    assert_eq!(host.call(3, &[], None), Err(Failure::BadReply));
    assert_eq!(
        host.call(2, &[Value::Int(3)], Some(Kind::Int)),
        Err(Failure::BadReply)
    );
    assert_eq!(host.call(3, &[], Some(Kind::Int)), Ok(Some(Value::Int(2))));
}

#[test]
fn a_crashing_library_takes_down_only_its_host() {
    let scratch = Scratch::new("foreign-crash");
    let mut host = probe(&scratch);
    assert_eq!(host.call(3, &[], Some(Kind::Int)), Ok(Some(Value::Int(1))));
    assert_eq!(host.call(4, &[], None), Err(Failure::Died));
    assert_eq!(host.call(3, &[], Some(Kind::Int)), Err(Failure::Died));
    // A new host starts afresh, its state lost.
    let mut host = Host::start(&scratch.path("host/host"), LIMIT).unwrap();
    assert_eq!(host.call(3, &[], Some(Kind::Int)), Ok(Some(Value::Int(1))));
}

#[test]
fn a_stalled_call_times_out() {
    let scratch = Scratch::new("foreign-stall");
    let exe = host_of(PROBE, Path::new(LIBRARY), &scratch.path("host")).unwrap();
    let mut host = Host::start(&exe, Duration::from_millis(200)).unwrap();
    let started = Instant::now();
    assert_eq!(host.call(5, &[], None), Err(Failure::TimedOut));
    assert!(started.elapsed() < LIMIT);
}

#[test]
fn a_host_is_built_once_per_declaration_and_files() {
    let scratch = Scratch::new("foreign-cache");
    for file in ["probe.c", "probe.h"] {
        fs::copy(Path::new(LIBRARY).join(file), scratch.path(file)).unwrap();
    }
    let probe = declared(PROBE, &scratch.path(""));
    let cache = scratch.path("cache");
    let exe = cached(&probe, &cache).unwrap();
    assert_eq!(cached(&probe, &cache), Ok(exe.clone()));
    let fewer = declared(
        &PROBE.replace("  Stall() = \"probe_stall\"\n", ""),
        &scratch.path(""),
    );
    assert_ne!(cached(&fewer, &cache).unwrap(), exe);
    fs::write(scratch.path("probe.c"), "#include \"probe.h\"\n").unwrap();
    assert!(cached(&probe, &cache).is_err());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 2);
}

#[test]
fn a_header_type_other_than_declared_fails_the_build() {
    let scratch = Scratch::new("foreign-types");
    let wrong = PROBE.replace("Odd(n: Int) -> Bool", "Odd(n: Int) -> Int");
    let error = host_of(&wrong, Path::new(LIBRARY), &scratch.path("host")).unwrap_err();
    assert!(
        error.contains("incompatible function pointer types"),
        "{error}"
    );
}

#[test]
fn a_file_not_named_is_not_there() {
    let scratch = Scratch::new("foreign-named");
    for file in ["probe.c", "probe.h"] {
        fs::copy(Path::new(LIBRARY).join(file), scratch.path(file)).unwrap();
    }
    fs::write(scratch.path("near.h"), "#include \"probe.h\"\n").unwrap();
    let near = PROBE.replace("probe.h\"", "near.h\"");
    let error = host_of(&near, &scratch.path(""), &scratch.path("host")).unwrap_err();
    assert!(error.contains("'probe.h' file not found"), "{error}");
}

#[test]
fn a_system_library_is_linked_by_name() {
    let scratch = Scratch::new("foreign-library");
    fs::write(scratch.path("math.h"), "#include <math.h>\n").unwrap();
    let source = r#"
foreign M {
  library: "m"
  header: "math.h"
  Floor(x: Float) -> Float = "floor"
}
"#;
    let exe = host_of(source, &scratch.path(""), &scratch.path("host")).unwrap();
    let mut host = Host::start(&exe, LIMIT).unwrap();
    let floor = host.call(0, &[float(-2.5)], Some(Kind::Float));
    assert_eq!(floor, Ok(Some(float(-3.0))));
}
