//! Hand-written programs and the bodies' errors they must give: the code, the position and, for a
//! type error, the expected and found types.

use super::*;
use crate::parser::parse;
use crate::resolve::resolve;

mod app;
mod branch;
mod own;
mod pattern;
mod rows;

/// Definitions the cases use; they follow the function under test, so a mark finds the test first.
const PRELUDE: &[&str] = &[
    "type Counter { count: Int = 0, name: String }",
    "enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }",
    "fn inc(n: Int) -> Int {",
    "  n + 1",
    "}",
    "fn clamp(x: Int; lo: Int, hi: Int) -> Int {",
    "  max(lo, min(x, hi))",
    "}",
    "fn bump(n: inout Int) {",
    "  n += 1",
    "}",
    "fn shout(text: String) {",
    "  _ = text",
    "}",
    "fn before(a: Int, b: Int) -> Bool {",
    "  a < b",
    "}",
];

fn program(lines: &[&str]) -> String {
    lines.join("\n") + "\n"
}

/// `body` as the statements of a function without result, then the prelude.
fn unit(body: &[&str]) -> String {
    let lines: Vec<_> = (body.iter())
        .map(|l| format!("  {l}"))
        .chain(["}".to_string()])
        .collect();
    let lines: Vec<_> = lines.iter().map(String::as_str).collect();
    program(&[&["fn f() {"][..], &lines[..], PRELUDE].concat())
}

/// `lines` and the prelude.
fn full(lines: &[&str]) -> String {
    program(&[lines, PRELUDE].concat())
}

/// The position of the `^` in `mark`, taken in the first line holding the marked text.
fn at(src: &str, mark: &str) -> Option<Pos> {
    let (before, after) = mark.split_once('^').unwrap();
    let text = format!("{before}{after}");
    let (i, line) = src.lines().enumerate().find(|(_, l)| l.contains(&text))?;
    Some((i + 1, line.find(&text).unwrap() + before.len() + 1))
}

fn run(src: &str) -> Bodies {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = check("main.paka", &program, &resolved, &mut table);
    assert_eq!(bodies.defs.len(), program.defs.len());
    assert_eq!(bodies.types.len(), program.defs.len());
    bodies
}

fn errors(src: &str) -> Vec<Diagnostic> {
    run(src).defs.into_iter().flatten().collect()
}

/// The expected and found types of an error, as the tool prints them.
fn types_of(d: &Diagnostic, src: &str) -> Option<(String, String)> {
    let detail = d.detailed(src.as_bytes());
    let field = |key: &str| {
        let prefix = format!("  {key}: ");
        let found = detail.lines().find_map(|l| l.strip_prefix(prefix.as_str()));
        found.map(String::from)
    };
    field("expected").zip(field("found"))
}

/// The program without the prelude.
fn head(src: &str) -> String {
    let end = src.find(PRELUDE[0]).unwrap_or(src.len());
    src[..end].to_string()
}

/// (code, mark, expected and found types, program).
type Case<'a> = (&'a str, &'a str, Option<(&'a str, &'a str)>, String);

fn rejects(cases: &[Case]) {
    let mut wrong = Vec::new();
    for (code, mark, types, src) in cases {
        let found = errors(src);
        let types = types.map(|(e, f)| (e.to_string(), f.to_string()));
        let [d] = &found[..] else {
            wrong.push(format!("want one error in\n{}got {found:?}", head(src)));
            continue;
        };
        let ok = (d.code, d.position) == (*code, at(src, mark));
        if !ok || types_of(d, src) != types {
            let want = format!("want {code} {mark} {types:?} in\n{}", head(src));
            wrong.push(format!("{want}got {d} {:?}", types_of(d, src)));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn accepts(programs: &[String]) {
    for src in programs {
        let found = errors(src);
        assert!(found.is_empty(), "{src}got {found:?}");
    }
}

fn cat<'a>(parts: &[&[&'a str]]) -> Vec<&'a str> {
    parts.concat()
}

/// A case whose body is the statements of a function without result.
fn st<'a>(
    code: &'a str,
    mark: &'a str,
    types: Option<(&'a str, &'a str)>,
    body: &[&str],
) -> Case<'a> {
    (code, mark, types, unit(body))
}

/// A case of whole definitions.
fn df<'a>(
    code: &'a str,
    mark: &'a str,
    types: Option<(&'a str, &'a str)>,
    lines: &[&str],
) -> Case<'a> {
    (code, mark, types, full(lines))
}

const MISMATCH: &str = "type-mismatch";

#[test]
fn accepts_locals_and_assignment() {
    accepts(&[
        unit(&[
            "let a = 1",
            "let b = 2.5",
            "let c = \"x\"",
            "let d = true",
            "_ = a",
        ]),
        unit(&[
            "let xs: List<Int> = []",
            "let m: Map<String, Int> = [:]",
            "let o: Option<Int> = None",
            "let r: Result<Int, String> = Err(\"x\")",
            "let e: Result<Int, String> = Ok(1)",
        ]),
        unit(&[
            "var n = 0",
            "n = 1",
            "n += 2",
            "n -= 1",
            "n *= 3",
            "n /= 1",
            "n %= 2",
        ]),
        unit(&[
            "var c = Counter { name: \"a\" }",
            "c.count = 5",
            "c.count += 1",
            "bump(&c.count)",
            "var xs = [Counter { name: \"b\" }]",
            "xs[0].name = \"z\"",
            "bump(&xs[0].count)",
        ]),
        full(&[
            "fn push(xs: inout List<Int>, x: Int) {",
            "  append(&xs, x)",
            "  xs[0] = 1",
            "}",
        ]),
    ]);
}

#[test]
fn accepts_operators_literals_and_places() {
    accepts(&[
        unit(&[
            "let a = 1 + 2 * 3 - 4 / 2 % 3",
            "let b = 1.5 + 2.5 * 3.0 - 1.0 / 2.0",
            "let c = \"a\" + \"b\"",
            "let d = a < 2 && b >= 1.0 || !(c == \"x\")",
            "let e = \"a\" < \"b\"",
            "let f = [1] == [1]",
            "let g = -a",
            "let h = Counter { name: \"a\" } == Counter { name: \"b\" }",
        ]),
        unit(&["let n = 1", "let s = \"n = {n} {true} {n + 1}\"", "_ = s"]),
        unit(&[
            "let xs = [1, 2, 3]",
            "let m = [\"a\": 1, \"b\": 2]",
            "let c = Counter { name: \"a\" }",
            "let c2 = Counter { count: 2, name: \"b\" }",
            "_ = xs[0] + m[\"a\"] + c.count + c2.count",
            "let ys = xs[0..<2]",
            "let t = \"hello\"",
            "let u = t[1..<3]",
            "let ch = t[0]",
            "_ = len(ys) + len(u) + len(ch)",
        ]),
    ]);
}

#[test]
fn accepts_calls_and_built_in_rows() {
    accepts(&[
        unit(&[
            "_ = inc(1)",
            "_ = clamp(5, lo: 0, hi: 10)",
            "shout(\"a\")",
            "var n = 1",
            "bump(&n)",
            "let g = inc",
            "_ = g(1)",
            "let s = Circle(1.0)",
            "let z = Dot",
            "let r = Rect(1.0, 2.0)",
            "_ = [s, z, r]",
        ]),
        unit(&[
            "var xs: List<Int> = []",
            "append(&xs, 1)",
            "extend(&xs, [2, 3])",
            "insert(&xs, 4, at: 0)",
            "sort(&xs)",
            "sort_by(&xs, before)",
            "_ = pop(&xs)",
            "_ = len(xs) + unwrap_or(find(xs, 1), 0)",
            "var m: Map<String, Int> = [:]",
            "remove(&m, \"a\")",
            "_ = get(m, \"a\")",
            "_ = keys(m)",
            "let o: Option<Int> = None",
            "_ = unwrap_or(o, 3)",
            "let s = fixed(1.5, digits: 2)",
            "_ = join(chars(s), \"-\")",
            "_ = to_float(1) + 1.5",
            "_ = to_int(2.5) + 1",
        ]),
        full(&[
            "fn apply(f: fn(Int) -> Int, x: Int) -> Int {",
            "  f(x)",
            "}",
            "fn twice() -> Int {",
            "  apply(inc, 1)",
            "}",
        ]),
    ]);
}

#[test]
fn accepts_control_flow_and_values() {
    accepts(&[
        unit(&[
            "var total = 0",
            "for i in 0..<10 {",
            "  if i % 2 == 0 {",
            "    continue",
            "  }",
            "  total += i",
            "}",
            "while total > 0 {",
            "  total -= 1",
            "  if total == 3 {",
            "    break",
            "  }",
            "}",
            "for x in [1, 2] {",
            "  _ = x",
            "}",
        ]),
        unit(&["shout(\"a\")", "_ = inc(1)", "??todo", "unreachable()"]),
        unit(&[
            "let a = if true { 1 } else { 2 }",
            "let b = if a > 1 { \"x\" } else if a > 0 { \"y\" } else { \"z\" }",
            "let c: Option<Int> = if true { None } else { Some(1) }",
            "let d = if true { None } else { Some(1) }",
            "_ = b",
            "_ = c",
            "_ = d",
        ]),
        full(&[
            "fn g() -> Result<Int, String> {",
            "  Ok(1)",
            "}",
            "fn f() -> Result<Int, String> {",
            "  let n = g()?",
            "  Ok(n + 1)",
            "}",
            "fn h(o: Option<Int>) -> Option<Int> {",
            "  let n = o?",
            "  Some(n)",
            "}",
        ]),
        full(&[
            "fn f(c: Bool) -> Int {",
            "  if c {",
            "    return 1",
            "  }",
            "  2",
            "}",
            "fn g(c: Bool) {",
            "  if c {",
            "    return",
            "  }",
            "  shout(\"a\")",
            "}",
            "fn h(c: Bool) -> Int {",
            "  if c { 1 } else { unreachable() }",
            "}",
            "fn u() -> Int {",
            "  unreachable()",
            "}",
        ]),
    ]);
}

#[test]
fn accepts_defaults_and_bare_values() {
    accepts(&[full(&[
        "enum E { A(n: Int), B }",
        "type T { e: E = A(1), xs: List<Int> = [], m: Map<String, Int> = [:], o: Option<Int> = None, n: Int = -3 }",
    ])]);
}

#[test]
fn rejects_mismatched_types() {
    rejects(&[
        st(
            MISMATCH,
            "let x: Int = ^\"a\"",
            Some(("Int", "String")),
            &["let x: Int = \"a\""],
        ),
        st(
            MISMATCH,
            "let x: Float = ^1",
            Some(("Float", "Int")),
            &["let x: Float = 1"],
        ),
        st(
            MISMATCH,
            "let x: Int = ^1.5",
            Some(("Int", "Float")),
            &["let x: Int = 1.5"],
        ),
        st(
            MISMATCH,
            "while ^1",
            Some(("Bool", "Int")),
            &["while 1 {", "  break", "}"],
        ),
        st(
            MISMATCH,
            "if ^1",
            Some(("Bool", "Int")),
            &["if 1 {", "  shout(\"a\")", "}"],
        ),
        st(
            MISMATCH,
            "1 ^+ \"a\"",
            Some(("Int", "String")),
            &["let x = 1 + \"a\""],
        ),
        st(
            MISMATCH,
            "\"a\" ^+ 1",
            Some(("String", "Int")),
            &["let x = \"a\" + 1"],
        ),
        st(
            MISMATCH,
            "true ^+ true",
            Some(("Int, Float or String", "Bool")),
            &["let x = true + true"],
        ),
        st(
            MISMATCH,
            "1.5 ^% 2.0",
            Some(("Int", "Float")),
            &["let x = 1.5 % 2.0"],
        ),
        st(MISMATCH, "^!1", Some(("Bool", "Int")), &["let x = !1"]),
        st(
            MISMATCH,
            "^-\"a\"",
            Some(("Int or Float", "String")),
            &["let x = -\"a\""],
        ),
        st(
            MISMATCH,
            "1 ^== \"a\"",
            Some(("Int", "String")),
            &["let x = 1 == \"a\""],
        ),
        st(
            MISMATCH,
            "c ^< c",
            Some(("Int, Float or String", "Counter")),
            &["let c = Counter { name: \"a\" }", "let x = c < c"],
        ),
        st(
            MISMATCH,
            "[1, ^\"a\"]",
            Some(("Int", "String")),
            &["let x = [1, \"a\"]"],
        ),
        st(
            MISMATCH,
            "\"b\": ^\"c\"",
            Some(("Int", "String")),
            &["let m = [\"a\": 1, \"b\": \"c\"]"],
        ),
        st(
            MISMATCH,
            ", ^2: 3",
            Some(("String", "Int")),
            &["let m = [\"a\": 1, 2: 3]"],
        ),
        st(
            MISMATCH,
            "[^1.5: 1]",
            Some(("Int or String", "Float")),
            &["let m = [1.5: 1]"],
        ),
        st(
            MISMATCH,
            "xs[^\"a\"]",
            Some(("Int", "String")),
            &["let xs = [1]", "let y = xs[\"a\"]"],
        ),
        st(
            MISMATCH,
            "= ^n[0]",
            Some(("List, Map or String", "Int")),
            &["let n = 1", "let y = n[0]"],
        ),
        st(
            MISMATCH,
            "n.^z",
            Some(("a record", "Int")),
            &["let n = 1", "let y = n.z"],
        ),
        st(
            MISMATCH,
            "{^x}",
            Some(("Int, Bool or String", "Float")),
            &["let x = 1.5", "let s = \"a {x}\""],
        ),
        st(
            MISMATCH,
            "= ^shout",
            Some(("a value", "no value")),
            &["let x = shout(\"a\")"],
        ),
    ]);
}

#[test]
fn rejects_literals_and_unknown_types() {
    rejects(&[
        st(
            "literal-out-of-range",
            "= ^9223372036854775808",
            None,
            &["let x = 9223372036854775808"],
        ),
        st("cannot-infer", "= ^[]", None, &["let xs = []"]),
        st("cannot-infer", "= ^[:]", None, &["let m = [:]"]),
        st("cannot-infer", "= ^None", None, &["let o = None"]),
        st("cannot-infer", "= ^Ok(1)", None, &["let r = Ok(1)"]),
        st(
            "cannot-infer",
            "= ^unreachable()",
            None,
            &["let x = unreachable()"],
        ),
        st("cannot-infer", "= ^??later", None, &["let x = ??later"]),
        st(
            MISMATCH,
            "Map<^List<Int>, Int>",
            Some(("Int or String", "List<Int>")),
            &["let m: Map<List<Int>, Int> = [:]"],
        ),
        st(
            "unknown-name",
            "c.^z",
            None,
            &["let c = Counter { name: \"a\" }", "let y = c.z"],
        ),
    ]);
}

#[test]
fn rejects_assignments() {
    rejects(&[
        st("assign-to-let", "^x = 2", None, &["let x = 1", "x = 2"]),
        st("assign-to-let", "^x += 2", None, &["let x = 1", "x += 2"]),
        st(
            "assign-to-let",
            "^i = 1",
            None,
            &["for i in 0..<3 {", "  i = 1", "}"],
        ),
        df(
            "assign-to-let",
            "^n = 1",
            None,
            &["fn g(n: Int) {", "  n = 1", "}"],
        ),
        st(
            "assign-to-let",
            "^c.count = 1",
            None,
            &["let c = Counter { name: \"a\" }", "c.count = 1"],
        ),
        st("bad-assignment-target", "^1 = 2", None, &["1 = 2"]),
        st(
            "bad-assignment-target",
            "^inc(1) = 2",
            None,
            &["inc(1) = 2"],
        ),
        st(
            "bad-assignment-target",
            "^s[0] = \"c\"",
            None,
            &["var s = \"ab\"", "s[0] = \"c\""],
        ),
        st(
            "bad-assignment-target",
            "^s[0..<1] = [2]",
            None,
            &["var s = [1, 2]", "s[0..<1] = [2]"],
        ),
        st(
            MISMATCH,
            "n = ^\"a\"",
            Some(("Int", "String")),
            &["var n = 1", "n = \"a\""],
        ),
        st(
            MISMATCH,
            "^s -= \"b\"",
            Some(("Int or Float", "String")),
            &["var s = \"a\"", "s -= \"b\""],
        ),
        st(
            MISMATCH,
            "+= ^1.5",
            Some(("Int", "Float")),
            &["var n = 1", "n += 1.5"],
        ),
        st(
            MISMATCH,
            "] = ^\"a\"",
            Some(("Int", "String")),
            &["var xs = [1]", "xs[0] = \"a\""],
        ),
    ]);
}

#[test]
fn rejects_inout_marks() {
    rejects(&[
        st(
            "bad-inout-target",
            "&^xs",
            None,
            &["let xs = [1]", "append(&xs, 2)"],
        ),
        st(
            "missing-inout-mark",
            "append(^xs",
            None,
            &["var xs = [1]", "append(xs, 2)"],
        ),
        st(
            "unexpected-inout-mark",
            "len(^&xs",
            None,
            &["var xs = [1]", "_ = len(&xs)"],
        ),
        st(
            "bad-inout-target",
            "&^xs[0..<1]",
            None,
            &["var xs = [1]", "append(&xs[0..<1], 2)"],
        ),
        st("bad-inout-target", "&^n", None, &["let n = 1", "bump(&n)"]),
        st(
            "missing-inout-mark",
            "bump(^n",
            None,
            &["var n = 1", "bump(n)"],
        ),
        st(
            "wrong-argument-count",
            "^bump(&n, 2)",
            None,
            &["var n = 1", "bump(&n, 2)"],
        ),
        st(
            "bad-inout-target",
            "&^i",
            None,
            &["for i in 0..<3 {", "  bump(&i)", "}"],
        ),
        df(
            "bad-inout-target",
            "&^n",
            None,
            &["fn g(n: Int) {", "  bump(&n)", "}"],
        ),
        st(
            MISMATCH,
            "xs, ^1)",
            Some(("String", "Int")),
            &["var xs = [\"a\"]", "append(&xs, 1)"],
        ),
        st(
            MISMATCH,
            "sort(^&xs",
            Some(("List<Int>, List<Float> or List<String>", "List<Bool>")),
            &["var xs = [true]", "sort(&xs)"],
        ),
    ]);
}

#[test]
fn rejects_unused_values_and_flow() {
    rejects(&[
        st("unused-result", "^len(\"a\")", None, &["len(\"a\")"]),
        st("unused-result", "^1", None, &["1"]),
        st("unused-result", "  ^n", None, &["let n = 1", "n"]),
        st("nothing-to-discard", "^_ =", None, &["_ = shout(\"a\")"]),
        st("break-outside-loop", "^break", None, &["break"]),
        st("break-outside-loop", "^continue", None, &["continue"]),
        st(
            MISMATCH,
            "return ^1",
            Some(("no value", "Int")),
            &["if true {", "  return 1", "}", "shout(\"a\")"],
        ),
        st(
            "unreachable-code",
            "^shout",
            None,
            &["return", "shout(\"a\")"],
        ),
        st(
            "unreachable-code",
            "  ^shout",
            None,
            &["while true {", "  break", "  shout(\"a\")", "}"],
        ),
        st(
            "unreachable-code",
            "^shout",
            None,
            &["unreachable()", "shout(\"a\")"],
        ),
        st(
            MISMATCH,
            "in ^\"ab\"",
            Some(("List", "String")),
            &["for c in \"ab\" {", "  break", "}"],
        ),
        st(
            MISMATCH,
            "in ^5",
            Some(("List", "Int")),
            &["for i in 5 {", "  break", "}"],
        ),
        st(
            MISMATCH,
            "..<^1.5",
            Some(("Int", "Float")),
            &["for i in 0..<1.5 {", "  break", "}"],
        ),
        st("if-needs-else", "= ^if", None, &["let x = if true { 1 }"]),
        st(
            MISMATCH,
            "else { ^\"a\"",
            Some(("Int", "String")),
            &["let x = if true { 1 } else { \"a\" }"],
        ),
    ]);
}

#[test]
fn rejects_results_and_returns() {
    rejects(&[
        df(
            MISMATCH,
            "^\"a\"",
            Some(("Int", "String")),
            &["fn f() -> Int {", "  \"a\"", "}"],
        ),
        df(
            MISMATCH,
            "return ^\"a\"",
            Some(("Int", "String")),
            &[
                "fn f(c: Bool) -> Int {",
                "  if c {",
                "    return \"a\"",
                "  }",
                "  1",
                "}",
            ],
        ),
        df(
            "return-last",
            "^return 1",
            None,
            &["fn f() -> Int {", "  return 1", "}"],
        ),
        df(
            "missing-value",
            "    ^return",
            None,
            &[
                "fn f(c: Bool) -> Int {",
                "  if c {",
                "    return",
                "  }",
                "  1",
                "}",
            ],
        ),
        df(
            "missing-value",
            "^}",
            None,
            &["fn f() -> Int {", "  let x = 1", "}"],
        ),
        df("missing-value", "^}", None, &["fn f() -> Int {", "}"]),
        df(
            "missing-value",
            "  ^}",
            None,
            &[
                "fn f(c: Bool) -> Int {",
                "  if c { 1 } else {",
                "    let x = 2",
                "  }",
                "}",
            ],
        ),
        df(
            MISMATCH,
            "else { ^\"a\"",
            Some(("Int", "String")),
            &["fn f(c: Bool) -> Int {", "  if c { 1 } else { \"a\" }", "}"],
        ),
        df(
            "if-needs-else",
            "^if",
            None,
            &["fn f(c: Bool) -> Int {", "  if c { 1 }", "}"],
        ),
        df("unused-result", "^1", None, &["fn f() {", "  1", "}"]),
    ]);
}

#[test]
fn rejects_the_wrong_use_of_question_marks() {
    let g = ["fn g() -> Result<Int, String> {", "  Ok(1)", "}"];
    let with = |more: &[&'static str]| cat(&[&g, more]);
    rejects(&[
        df(
            MISMATCH,
            "g()^?",
            Some(("Result<T, String>", "Int")),
            &with(&["fn f() -> Int {", "  let n = g()?", "  n", "}"]),
        ),
        df(
            MISMATCH,
            "g()^?",
            Some(("Result<T, String>", "no result")),
            &with(&["fn f() {", "  let n = g()?", "  _ = n", "}"]),
        ),
        df(
            MISMATCH,
            "o^?",
            Some(("Option<T>", "Result<Int, String>")),
            &with(&[
                "fn h(o: Option<Int>) -> Result<Int, String> {",
                "  let n = o?",
                "  Ok(n)",
                "}",
            ]),
        ),
        df(
            MISMATCH,
            "g()^?",
            Some(("Result<T, String>", "Result<Int, Int>")),
            &with(&[
                "fn f() -> Result<Int, Int> {",
                "  let n = g()?",
                "  Ok(n)",
                "}",
            ]),
        ),
        df(
            MISMATCH,
            "1^?",
            Some(("Option or Result", "Int")),
            &[
                "fn f() -> Result<Int, String> {",
                "  let n = 1?",
                "  Ok(n)",
                "}",
            ],
        ),
    ]);
}

#[test]
fn rejects_calls_by_count_label_and_type() {
    rejects(&[
        st(
            "wrong-argument-count",
            "_ = ^inc(1, 2)",
            None,
            &["_ = inc(1, 2)"],
        ),
        st("wrong-argument-count", "_ = ^inc()", None, &["_ = inc()"]),
        st("wrong-label", "5, ^0", None, &["_ = clamp(5, 0, hi: 10)"]),
        st(
            "wrong-label",
            "^hi: 10, lo",
            None,
            &["_ = clamp(5, hi: 10, lo: 0)"],
        ),
        st(
            "wrong-label",
            "^high",
            None,
            &["_ = clamp(5, lo: 0, high: 10)"],
        ),
        st(
            "wrong-label",
            "clamp(^x",
            None,
            &["_ = clamp(x: 5, lo: 0, hi: 10)"],
        ),
        st(
            MISMATCH,
            "lo: ^\"a\"",
            Some(("Int", "String")),
            &["_ = clamp(5, lo: \"a\", hi: 10)"],
        ),
        st("wrong-label", "g(^n", None, &["let g = inc", "_ = g(n: 1)"]),
        st(
            "not-a-function",
            "= ^n(2)",
            None,
            &["let n = 1", "_ = n(2)"],
        ),
        st(
            "wrong-argument-count",
            "= ^g(1, 2)",
            None,
            &["let g = inc", "_ = g(1, 2)"],
        ),
        st(
            "unexpected-inout-mark",
            "g(^&x",
            None,
            &["var x = 1", "let g = inc", "_ = g(&x)"],
        ),
        st("not-a-value", "= ^len", None, &["let g = len"]),
        st(
            "wrong-argument-count",
            "= ^inc",
            None,
            &["let x: Int = inc"],
        ),
        st(
            "wrong-argument-count",
            "= ^Circle",
            None,
            &["let s: Shape = Circle"],
        ),
        st("wrong-argument-count", "= ^Dot()", None, &["let s = Dot()"]),
        st(
            MISMATCH,
            "= ^inc(",
            Some(("String", "Int")),
            &["let s: String = inc(\"a\")"],
        ),
        st(
            MISMATCH,
            "= ^Circle(",
            Some(("String", "Shape")),
            &["let s: String = Circle(\"a\")"],
        ),
        st(
            MISMATCH,
            "Circle(^\"a\"",
            Some(("Float", "String")),
            &["let s = Circle(\"a\")"],
        ),
        st(
            "wrong-argument-count",
            "^unreachable(",
            None,
            &["unreachable(\"why\")"],
        ),
    ]);
}

#[test]
fn rejects_records_by_label_and_type() {
    rejects(&[
        st(
            "wrong-argument-count",
            "= ^Counter {}",
            None,
            &["let c = Counter {}"],
        ),
        st(
            "wrong-label",
            "{ ^nme",
            None,
            &["let c = Counter { nme: \"a\" }"],
        ),
        st(
            MISMATCH,
            "name: ^1",
            Some(("String", "Int")),
            &["let c = Counter { name: 1 }"],
        ),
        st(
            "wrong-label",
            "\"a\", ^count",
            None,
            &["let c = Counter { name: \"a\", count: 1 }"],
        ),
    ]);
}

#[test]
fn rejects_built_in_calls_by_row() {
    rejects(&[
        st(
            MISMATCH,
            "len(^5)",
            Some(("String, List<T> or Map<K, V>", "Int")),
            &["_ = len(5)"],
        ),
        st(
            MISMATCH,
            "1), ^\"a\"",
            Some(("Int", "String")),
            &["_ = unwrap_or(find([1], 1), \"a\")"],
        ),
        st(
            MISMATCH,
            "= ^Ok(\"a\")",
            Some(("Result<Int, String>", "Result<String, String>")),
            &["let r: Result<Int, String> = Ok(\"a\")"],
        ),
        st(
            MISMATCH,
            "= [^\"a\"]",
            Some(("Int", "String")),
            &["let xs: List<Int> = [\"a\"]"],
        ),
        st(
            MISMATCH,
            "= ^None",
            Some(("Int", "Option<T>")),
            &["let n: Int = None"],
        ),
    ]);
}

#[test]
fn rejects_the_contents_of_field_defaults() {
    rejects(&[
        df(
            MISMATCH,
            "A(^\"x\")",
            Some(("Int", "String")),
            &["enum E { A(n: Int) }", "type T { e: E = A(\"x\") }"],
        ),
        df(
            MISMATCH,
            "n: ^\"x\"",
            Some(("Int", "String")),
            &["type In { n: Int }", "type T { i: In = In { n: \"x\" } }"],
        ),
        df(
            "wrong-argument-count",
            "= ^A",
            None,
            &["enum E { A(n: Int), B }", "type T { e: E = A }"],
        ),
    ]);
}

/// (code, mark, message, program): the whole message is pinned.
type Say<'a> = (&'a str, &'a str, &'a str, String);

fn says(cases: &[Say]) {
    let mut wrong = Vec::new();
    for (code, mark, message, src) in cases {
        let found = errors(src);
        let [d] = &found[..] else {
            wrong.push(format!("want one error in\n{}got {found:?}", head(src)));
            continue;
        };
        if (d.code, d.position, d.message.as_str()) != (*code, at(src, mark), *message) {
            let want = format!("want {code} {mark} {message:?} in\n{}", head(src));
            wrong.push(format!("{want}got {d}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn pins_the_message_of_each_text() {
    says(&[
        (
            "type-mismatch",
            "x: Int = ^\"a\"",
            "expected `Int`, found `String`",
            unit(&["let x: Int = \"a\""]),
        ),
        (
            "type-mismatch",
            "x: Float = ^1",
            "expected `Float`, found `Int`; write 1.0",
            unit(&["let x: Float = 1"]),
        ),
        (
            "type-mismatch",
            "x: Float = ^n",
            "expected `Float`, found `Int`; write to_float(n)",
            unit(&["let n = 1", "let x: Float = n"]),
        ),
        (
            "type-mismatch",
            "x: Int = ^1.5",
            "expected `Int`, found `Float`; write to_int(1.5)",
            unit(&["let x: Int = 1.5"]),
        ),
        (
            "type-mismatch",
            "x: Int = ^y",
            "expected `Int`, found `Float`; write to_int(y)",
            unit(&["let y = 1.5", "let x: Int = y"]),
        ),
        (
            "return-last",
            "^return 1",
            "return on the last line; drop return: the last line is the value",
            full(&["fn f() -> Int {", "  return 1", "}"]),
        ),
        (
            "return-last",
            "^return",
            "return on the last line; drop return: the last line is the value",
            full(&["fn f() {", "  shout(\"a\")", "  return", "}"]),
        ),
        (
            "return-last",
            "^return 1",
            "return on the last line; drop return: the last line is the value",
            full(&["fn f() -> Int {", "  return 1 + \"a\"", "}"]),
        ),
        (
            "return-last",
            "^return 1",
            "return on the last line; drop return: the last line is the value",
            full(&[
                "fn f(c: Bool) -> Int {",
                "  if c { return 1 } else { return 2 }",
                "}",
            ]),
        ),
        (
            "unused-result",
            "^find",
            "the value of find is not used; it is Option<Int>: match it, pass it up with find(.., 1)?, or write `_ = find(.., 1)`",
            unit(&["find([1], 1)"]),
        ),
        (
            "unused-result",
            "^inc",
            "the value of inc is not used; write _ = ...",
            unit(&["inc(1)"]),
        ),
        (
            "unused-result",
            "^g(1)",
            "the value of g is not used; write _ = ...",
            unit(&["let g = inc", "g(1)"]),
        ),
        (
            "unused-result",
            "^1",
            "the value of 1 is not used; write _ = ...",
            unit(&["1"]),
        ),
        (
            "unused-result",
            "  ^n",
            "the value of n is not used; write _ = ...",
            unit(&["let n = 1", "n"]),
        ),
        (
            "unused-result",
            "^o?",
            "the value of o? is not used; write `_ = o?`",
            full(&["fn h(o: Option<Int>) -> Option<Int> {", "  o?", "  o", "}"]),
        ),
        (
            "nothing-to-discard",
            "^_",
            "shout has no value: drop _ =",
            unit(&["_ = shout(\"a\")"]),
        ),
        (
            "if-needs-else",
            "_ = ^if",
            "an if used as a value needs an else",
            unit(&["_ = if true { }"]),
        ),
        (
            "missing-value",
            "^}",
            "this block must end in a value of type Int",
            full(&["fn f() -> Int {", "  let x = 1", "}"]),
        ),
        (
            "missing-value",
            "  ^}",
            "this block must end in a value of type Int",
            full(&[
                "fn f(c: Bool) -> Int {",
                "  if c { 1 } else {",
                "    let x = 2",
                "  }",
                "}",
            ]),
        ),
        (
            "missing-value",
            "^return",
            "return needs a value of type Int",
            full(&[
                "fn f(c: Bool) -> Int {",
                "  if c {",
                "    return",
                "  }",
                "  1",
                "}",
            ]),
        ),
        (
            "cannot-infer",
            "= ^[]",
            "cannot infer the type of xs; write it: let xs: List<T> = ...",
            unit(&["let xs = []"]),
        ),
        (
            "cannot-infer",
            "= ^None",
            "cannot infer the type of o; write it: var o: Option<T> = ...",
            unit(&["var o = None"]),
        ),
        (
            "cannot-infer",
            "= ^Ok(1)",
            "cannot infer the type of r; write it: let r: Result<Int, E> = ...",
            unit(&["let r = Ok(1)"]),
        ),
        (
            "cannot-infer",
            "{ ^Ok(1) }",
            "cannot infer the type of r; write it: let r: Result<Int, E> = ...",
            unit(&["let r = if true { Ok(1) } else { Ok(2) }"]),
        ),
        (
            "wrong-label",
            "^hi: 10, lo",
            "labels come in declared order",
            unit(&["_ = clamp(5, hi: 10, lo: 0)"]),
        ),
        (
            "wrong-label",
            "5, ^0",
            "write lo: 0",
            unit(&["_ = clamp(5, 0, hi: 10)"]),
        ),
        (
            "wrong-label",
            "clamp(^x",
            "x is bare: write 5",
            unit(&["_ = clamp(x: 5, lo: 0, hi: 10)"]),
        ),
        (
            "wrong-label",
            "{ ^nme",
            "Counter has no field nme",
            unit(&["let c = Counter { nme: \"a\" }"]),
        ),
        (
            "wrong-label",
            "P { b: 1, ^a",
            "labels come in declared order",
            full(&[
                "type P { a: Int, b: Int }",
                "fn f() {",
                "  _ = P { b: 1, a: 2 }",
                "}",
            ]),
        ),
        (
            "wrong-label",
            "\"a\", ^count",
            "labels come in declared order",
            unit(&["let c = Counter { name: \"a\", count: 1 }"]),
        ),
        (
            "assign-to-let",
            "^x = 2",
            "x is a let; declare it with var",
            unit(&["let x = 1", "x = 2"]),
        ),
        (
            "assign-to-let",
            "^n = 1",
            "n is a parameter; declare it with var",
            full(&["fn g(n: Int) {", "  n = 1", "}"]),
        ),
        (
            "assign-to-let",
            "^i = 1",
            "i is a for name; declare it with var",
            unit(&["for i in 0..<3 {", "  i = 1", "}"]),
        ),
        (
            "assign-to-let",
            "^c.count = 1",
            "c is a let; declare it with var",
            unit(&["let c = Counter { name: \"a\" }", "c.count = 1"]),
        ),
        (
            "bad-assignment-target",
            "^1 = 2",
            "only a var local, an inout parameter or their fields and items can be assigned",
            unit(&["1 = 2"]),
        ),
        (
            "bad-assignment-target",
            "^s[0] = \"c\"",
            "a String changes only as a whole",
            unit(&["var s = \"ab\"", "s[0] = \"c\""]),
        ),
        (
            "bad-assignment-target",
            "^d.shape.w = 3.0",
            "w is a field of the variant Rect; build Rect(..) and assign d.shape",
            full(&[
                "type D { shape: Shape = Dot }",
                "fn f(d: inout D) {",
                "  d.shape.w = 3.0",
                "}",
            ]),
        ),
        (
            "not-a-function",
            "= ^n(2)",
            "n is an Int, not a function",
            unit(&["let n = 1", "_ = n(2)"]),
        ),
        (
            "not-a-value",
            "= ^len",
            "len is a built-in and cannot be a value; write fn wrap(...) -> R { len(...) } and pass wrap",
            unit(&["let g = len"]),
        ),
        (
            "not-a-value",
            "= ^Some",
            "Some is a built-in variant and cannot be a value; write fn some_of(value: T) -> Option<T> { Some(value) }",
            unit(&["let g = Some"]),
        ),
        (
            "missing-inout-mark",
            "append(^xs",
            "append changes xs: write &xs",
            unit(&["var xs = [1]", "append(xs, 2)"]),
        ),
        (
            "missing-inout-mark",
            "bump(^n",
            "bump changes n: write &n",
            unit(&["var n = 1", "bump(n)"]),
        ),
        (
            "unexpected-inout-mark",
            "len(^&xs",
            "x is read-only in len; drop the &",
            unit(&["var xs = [1]", "_ = len(&xs)"]),
        ),
        (
            "bad-inout-target",
            "&^xs",
            "xs is a let; declare it with var",
            unit(&["let xs = [1]", "append(&xs, 2)"]),
        ),
        (
            "bad-inout-target",
            "&^n",
            "n is read-only; write n: inout Int and call g(&ys)",
            full(&["fn g(n: Int) {", "  bump(&n)", "}"]),
        ),
        (
            "bad-inout-target",
            "&^i",
            "i is a for name; it is a copy",
            unit(&["for i in 0..<3 {", "  bump(&i)", "}"]),
        ),
        (
            "bad-inout-target",
            "&^xs[0..<1]",
            "a slice cannot be passed with &; pass the list and the positions",
            unit(&["var xs = [1]", "append(&xs[0..<1], 2)"]),
        ),
        (
            "wrong-argument-count",
            "^inc(1, 2)",
            "inc takes 1 argument (n), got 2",
            unit(&["_ = inc(1, 2)"]),
        ),
        (
            "wrong-argument-count",
            "^inc()",
            "inc takes 1 argument (n), got 0",
            unit(&["_ = inc()"]),
        ),
        (
            "wrong-argument-count",
            "^g(1, 2)",
            "g takes 1 argument, got 2",
            unit(&["let g = inc", "_ = g(1, 2)"]),
        ),
        (
            "wrong-argument-count",
            "^unreachable(",
            "unreachable takes no arguments",
            unit(&["unreachable(\"why\")"]),
        ),
        (
            "wrong-argument-count",
            "= ^inc",
            "inc takes 1 argument (n), got 0; write inc(...)",
            unit(&["let x: Int = inc"]),
        ),
        (
            "wrong-argument-count",
            "= ^Circle",
            "Circle takes 1 argument (r), got 0; write Circle(...)",
            unit(&["let s: Shape = Circle"]),
        ),
        (
            "wrong-argument-count",
            "^Dot()",
            "Dot has no fields: write Dot",
            unit(&["let s = Dot()"]),
        ),
        (
            "wrong-argument-count",
            "^Exit()",
            "Exit has no fields: write Exit",
            unit(&["let e: Effect<Shape> = Exit()"]),
        ),
        (
            "wrong-argument-count",
            "^None()",
            "None has no fields: write None",
            unit(&["let o: Option<Int> = None()"]),
        ),
        (
            "wrong-argument-count",
            "^Print",
            "Print takes 1 argument (text), got 0; write Print(...)",
            unit(&["let es: List<Effect<Shape>> = [Print, Exit]"]),
        ),
        (
            "wrong-argument-count",
            "^ExitWith",
            "ExitWith takes 1 argument (status), got 0; write ExitWith(...)",
            unit(&["let e: Effect<Shape> = ExitWith"]),
        ),
        (
            "wrong-argument-count",
            "= ^Counter {}",
            "Counter needs name: write name: ...",
            unit(&["let c = Counter {}"]),
        ),
        (
            "type-mismatch",
            "Circle(^\"a\"",
            "expected `Float`, found `String`",
            unit(&["let s = Circle(\"a\")"]),
        ),
        (
            "type-mismatch",
            "n.^z",
            "Int has no fields",
            unit(&["let n = 1", "let y = n.z"]),
        ),
        (
            "type-mismatch",
            "s.^r",
            "r is a field of the variant Circle; match on the value to read it",
            unit(&["let s = Circle(1.0)", "_ = s.r"]),
        ),
        (
            "unknown-name",
            "c.^cnt",
            "Counter has no field cnt",
            unit(&["let c = Counter { name: \"a\" }", "_ = c.cnt"]),
        ),
        (
            "type-mismatch",
            "= ^shout",
            "shout has no result",
            unit(&["let x = shout(\"a\")"]),
        ),
        (
            "type-mismatch",
            "= ^fs[0]",
            "expected `Int`, found `no value`",
            unit(&[
                "let fs: List<fn(String)> = [shout]",
                "let x: Int = fs[0](\"a\")",
            ]),
        ),
        (
            "type-mismatch",
            "return ^1",
            "f has no result; drop the value or write -> Int",
            full(&[
                "fn f(c: Bool) {",
                "  if c {",
                "    return 1",
                "  }",
                "  shout(\"a\")",
                "}",
            ]),
        ),
        (
            "type-mismatch",
            "len(^5)",
            "len takes a String, List or Map",
            unit(&["_ = len(5)"]),
        ),
        (
            "type-mismatch",
            "max(^true",
            "max takes an Int, Float or String",
            unit(&["_ = max(true, 1)"]),
        ),
        (
            "type-mismatch",
            "max(1, ^2.0",
            "max takes two of one type; write 1.0",
            unit(&["_ = max(1, 2.0)"]),
        ),
        (
            "type-mismatch",
            "max(1.5, ^2",
            "max takes two of one type; write 2.0",
            unit(&["_ = max(1.5, 2)"]),
        ),
        (
            "type-mismatch",
            "x ^% 2.0",
            "no % on Float: write x - y * trunc(x / y)",
            unit(&["let x = 5", "_ = x % 2.0"]),
        ),
        (
            "type-mismatch",
            "x ^% 2.0",
            "no % on Float: write x - y * trunc(x / y)",
            unit(&["let x = 5.0", "_ = x % 2.0"]),
        ),
        (
            "type-mismatch",
            "1 ^+ 2.0",
            "expected `Int`, found `Float`; write 1.0",
            unit(&["_ = 1 + 2.0"]),
        ),
        (
            "type-mismatch",
            "1.5 ^+ 2",
            "expected `Float`, found `Int`; write 2.0",
            unit(&["_ = 1.5 + 2"]),
        ),
        (
            "type-mismatch",
            "1 ^+ \"a\"",
            "no + between Int and String; write an interpolation: \"{1}\"",
            unit(&["_ = 1 + \"a\""]),
        ),
        (
            "type-mismatch",
            "x ^+ y",
            "no + on List; write extend(&x, y)",
            unit(&["let x = [1]", "let y = [2]", "_ = x + y"]),
        ),
        (
            "type-mismatch",
            "true ^+ true",
            "expected `Int, Float or String`, found `Bool`",
            unit(&["_ = true + true"]),
        ),
        (
            "type-mismatch",
            "c ^< c",
            "Counter has no order; compare a field: c.count < c.count",
            unit(&["let c = Counter { name: \"a\" }", "_ = c < c"]),
        ),
        (
            "type-mismatch",
            "^!1",
            "expected `Bool`, found `Int`",
            unit(&["_ = !1"]),
        ),
        (
            "type-mismatch",
            "^-\"a\"",
            "expected `Int or Float`, found `String`",
            unit(&["_ = -\"a\""]),
        ),
        (
            "type-mismatch",
            "g()^?",
            "? passes Err(String) up, but f returns Int; make it Result<T, String> or match it",
            full(&[
                "fn g() -> Result<Int, String> {",
                "  Ok(1)",
                "}",
                "fn f() -> Int {",
                "  let n = g()?",
                "  n",
                "}",
            ]),
        ),
        (
            "type-mismatch",
            "g()^?",
            "? passes Err(String) up, but f has no result; match it",
            full(&[
                "fn g() -> Result<Int, String> {",
                "  Ok(1)",
                "}",
                "fn f() {",
                "  _ = g()?",
                "}",
            ]),
        ),
        (
            "type-mismatch",
            "o^?",
            "? passes None up, but h returns Result<Int, String>; make it Option<T>, write ok_or(..., error)? or match it",
            full(&[
                "fn h(o: Option<Int>) -> Result<Int, String> {",
                "  Ok(o?)",
                "}",
            ]),
        ),
        (
            "type-mismatch",
            "1^?",
            "? needs an Option or Result; 1 is Int; drop the ?",
            full(&["fn f() -> Result<Int, String> {", "  Ok(1?)", "}"]),
        ),
    ]);
}

/// The result of each definition, as the code of its error or `ok`.
fn codes(lines: &[&str]) -> Vec<&'static str> {
    let found = run(&program(lines)).defs;
    found
        .iter()
        .map(|d| d.as_ref().map_or("ok", |d| d.code))
        .collect()
}

#[test]
fn the_earliest_error_of_a_definition_wins() {
    let order = |lines: &[&str]| codes(lines);
    assert_eq!(
        order(&["fn f() {", "  let x: Int = \"a\"", "  _ = nope", "}"]),
        ["type-mismatch"]
    );
    assert_eq!(
        order(&["fn f() {", "  _ = nope", "  let x: Int = \"a\"", "}"]),
        ["unknown-name"]
    );
    assert_eq!(
        order(&["fn f(x: Nope) {", "  let y: Int = \"a\"", "}"]),
        ["unknown-type"]
    );
    assert_eq!(
        order(&["fn f() {", "  let y: Int = \"a\"", "  let z: Nope = 1", "}"]),
        ["type-mismatch"]
    );
    assert_eq!(
        order(&["fn f() {", "  let y: Nope = nope", "  _ = 1 + \"a\"", "}"]),
        ["unknown-type"]
    );
}

#[test]
fn a_call_is_checked_by_count_then_labels_then_arguments() {
    rejects(&[
        st(
            "wrong-argument-count",
            "^clamp(\"a\", 0)",
            None,
            &["_ = clamp(\"a\", 0)"],
        ),
        st(
            "wrong-label",
            "\"a\", ^0",
            None,
            &["_ = clamp(\"a\", 0, hi: 10)"],
        ),
        st(
            MISMATCH,
            "clamp(^\"a\"",
            Some(("Int", "String")),
            &["_ = clamp(\"a\", lo: 0, hi: 10)"],
        ),
    ]);
}

#[test]
fn a_call_through_a_value_errs_at_the_callee() {
    rejects(&[
        st(
            "wrong-argument-count",
            "^fs[0](1, 2)",
            None,
            &["let fs = [inc]", "_ = fs[0](1, 2)"],
        ),
        st(
            MISMATCH,
            "fs[0](^\"a\")",
            Some(("Int", "String")),
            &["let fs = [inc]", "_ = fs[0](\"a\")"],
        ),
    ]);
}

#[test]
fn a_built_in_row_is_picked_by_the_first_argument() {
    rejects(&[st(
        MISMATCH,
        "max(\"a\", ^1)",
        Some(("String", "Int")),
        &["_ = max(\"a\", 1)"],
    )]);
}

#[test]
fn a_bare_built_in_variant_fits_its_own_type_only() {
    accepts(&[unit(&[
        "let es: List<Effect<Shape>> = [Exit, Print(\"a\"), ExitWith(2)]",
        "_ = es",
    ])]);
}

#[test]
fn does_not_look_into_match_arms() {
    accepts(&[unit(&[
        "let n = 1",
        "let m = match n {",
        "  _ => 2",
        "}",
        "_ = m",
    ])]);
}

#[test]
fn an_error_in_one_body_never_changes_another_result() {
    let good = ["fn good() -> Int {", "  bad()", "}"];
    let bad = |body: &'static str| ["fn bad() -> Int {", body, "}"];
    assert_eq!(
        codes(&cat(&[&bad("  \"a\""), &good])),
        ["type-mismatch", "ok"]
    );
    assert_eq!(codes(&cat(&[&bad("  1"), &good])), ["ok", "ok"]);
    assert_eq!(
        codes(&cat(&[&good, &bad("  \"a\"")])),
        ["ok", "type-mismatch"]
    );
    let both = cat(&[&bad("  1 + \"a\""), &good, &["fn other() {", "  1", "}"]]);
    assert_eq!(codes(&both), ["type-mismatch", "ok", "unused-result"]);
}

#[test]
fn a_signature_with_an_error_exposes_its_good_parts() {
    let lines = [
        "fn bad(x: Nope, y: Int) -> Nope {",
        "  x",
        "}",
        "fn good() -> Int {",
        "  let a: Int = bad(1, 2)",
        "  let b: String = bad(\"s\", 2)",
        "  a",
        "}",
        "fn wrong() -> Int {",
        "  let a: Int = bad(1, \"s\")",
        "  a",
        "}",
    ];
    assert_eq!(codes(&lines), ["unknown-type", "ok", "type-mismatch"]);
}

#[test]
fn a_type_with_an_error_in_a_default_fails_alone() {
    let lines = [
        "enum E { A(n: Int) }",
        "type T { e: E = A(\"x\") }",
        "type U { n: Int = 1 }",
    ];
    assert_eq!(codes(&lines), ["ok", "type-mismatch", "ok"]);
}

#[test]
fn holds_the_type_of_each_expression_met() {
    let src = program(&["fn f(n: Int) -> Float {", "  let x = n + 2", "  1.5", "}"]);
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = check("main.paka", &program, &resolved, &mut table);
    let shown = |pos, len| {
        bodies.types[0]
            .get(&(pos, len))
            .map(|&t| table.types.show(t))
    };
    assert_eq!(shown((2, 11), 5).as_deref(), Some("Int"));
    assert_eq!(shown((2, 11), 1).as_deref(), Some("Int"));
    assert_eq!(shown((3, 3), 3).as_deref(), Some("Float"));
}

/// Runs `f` on the 8 MiB of a program's main thread; a debug test thread has only 2 MiB.
fn deep(f: impl FnOnce() + Send) {
    std::thread::scope(|s| {
        let run = std::thread::Builder::new().stack_size(8 << 20);
        run.spawn_scoped(s, f).unwrap().join().unwrap();
    });
}

/// 62 levels of brackets, each holding a chain of 1,000 links, with the next level at the chain's
/// end or its head. The innermost level starts from `base`.
fn nested_chains(head: bool, base: &str) -> String {
    let links = " + x".repeat(999);
    let wrap = |inner: String| match head {
        true => format!("({inner}{links} + x)"),
        false => format!("(x{links} + {inner})"),
    };
    let expr = (0..62).fold(base.to_string(), |inner, _| wrap(inner));
    program(&["fn f(x: Int) -> Int {", &format!("  {expr}"), "}"])
}

#[test]
fn the_deepest_allowed_chains_fit_the_stack() {
    for head in [false, true] {
        deep(|| {
            assert!(errors(&nested_chains(head, "x")).is_empty());
            assert_eq!(errors(&nested_chains(head, "\"x\"")).len(), 1);
        });
    }
}
