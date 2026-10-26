//! Deep recursion and large frames, built by `paka build` at full optimization, end in a stack
//! trap and never in a signal (M6-16).

#[allow(dead_code)]
mod common;

use common::{Scratch, build_native};
use std::process::Command;

/// `update` running `call`, after the functions in `defs`.
fn program(call: &str, defs: &str) -> String {
    format!(
        "enum Msg {{ Start }}\n\n{defs}\n\nfn update(msg: Msg) -> List<Effect<Msg>> {{\n  \
         let r = {call}\n  [Exit]\n}}\n\napp deep {{\n  start: Start\n  update: update\n  \
         grants: []\n}}\n"
    )
}

/// Builds and runs `source`: its status and standard error.
fn run(scratch: &Scratch, source: &str) -> (Option<i32>, String) {
    let (path, exe) = (scratch.path("main.paka"), scratch.path("main"));
    std::fs::write(&path, source).unwrap();
    let built = build_native(path.to_str().unwrap(), &exe);
    assert!(built.status.success(), "{built:?}");
    let out = Command::new(&exe).output().unwrap();
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

#[test]
fn deep_recursion_and_large_frames_trap_stack() {
    let fields: Vec<String> = (0..1024).map(|i| format!("f{i}: Int")).collect();
    let values: Vec<String> = (0..1024).map(|i| format!("f{i}: n + {i}")).collect();
    let big = format!(
        "type Big {{ {} }}\n\nfn wide(n: Int) -> Int {{\n  let b = Big {{ {} }}\n  \
         wide(n + 1) + b.f1023\n}}",
        fields.join(", "),
        values.join(", ")
    );
    let cases = [
        (
            "down(0)",
            "fn down(n: Int) -> Int { down(n + 1) + 1 }".to_string(),
        ),
        (
            "ping(0)",
            "fn ping(n: Int) -> Int { pong(n + 1) + 1 }\n\
             fn pong(n: Int) -> Int { ping(n + 1) + 1 }"
                .to_string(),
        ),
        ("wide(0)", big),
    ];
    let scratch = Scratch::new("release-stack");
    for (call, defs) in cases {
        let (status, stderr) = run(&scratch, &program(call, &defs));
        assert_eq!(status, Some(3), "{call}: {stderr}");
        assert!(
            stderr.starts_with("deep: trap stack at main.paka:"),
            "{call}: {stderr}"
        );
    }
}
