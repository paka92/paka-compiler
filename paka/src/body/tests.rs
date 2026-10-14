//! Hand-written programs and the bodies' errors they must give: the code, the position and, for a
//! type error, the expected and found types.

use super::*;
use crate::parser::parse;
use crate::resolve::resolve;

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
fn at(src: &str, mark: &str) -> Pos {
    let (before, after) = mark.split_once('^').unwrap();
    let text = format!("{before}{after}");
    let found = src.lines().enumerate().find(|(_, l)| l.contains(&text));
    let (i, line) = found.unwrap_or_else(|| panic!("no `{text}` in\n{src}"));
    (i + 1, line.find(&text).unwrap() + before.len() + 1)
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

/// (code, mark, expected and found types, program).
type Case<'a> = (&'a str, &'a str, Option<(&'a str, &'a str)>, String);

fn rejects(cases: &[Case]) {
    for (code, mark, types, src) in cases {
        let found = errors(src);
        let [d] = &found[..] else {
            panic!("want one error in\n{src}got {found:?}");
        };
        assert_eq!(
            (d.code, d.position),
            (*code, Some(at(src, mark))),
            "{src}{d}"
        );
        let types = types.map(|(e, f)| (e.to_string(), f.to_string()));
        assert_eq!(types_of(d, src), types, "{src}");
    }
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
            "_ = [b, c, d]",
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
            "fn g() {",
            "  return",
            "}",
            "fn h(c: Bool) -> Int {",
            "  if c { 1 } else { unreachable() }",
            "}",
            "fn k(c: Bool) -> Int {",
            "  if c { return 1 } else { return 2 }",
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
        "type T { e: E = A(1), xs: List<Int> = [1, 2], m: Map<String, Int> = [\"a\": 1], o: Option<Int> = None, n: Int = -3 }",
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
            "1 + ^\"a\"",
            Some(("Int", "String")),
            &["let x = 1 + \"a\""],
        ),
        st(
            MISMATCH,
            "\"a\" + ^1",
            Some(("String", "Int")),
            &["let x = \"a\" + 1"],
        ),
        st(
            MISMATCH,
            "= ^true + true",
            Some(("Int, Float or String", "Bool")),
            &["let x = true + true"],
        ),
        st(
            MISMATCH,
            "= ^1.5 % 2.0",
            Some(("Int", "Float")),
            &["let x = 1.5 % 2.0"],
        ),
        st(MISMATCH, "!^1", Some(("Bool", "Int")), &["let x = !1"]),
        st(
            MISMATCH,
            "-^\"a\"",
            Some(("Int or Float", "String")),
            &["let x = -\"a\""],
        ),
        st(
            MISMATCH,
            "1 == ^\"a\"",
            Some(("Int", "String")),
            &["let x = 1 == \"a\""],
        ),
        st(
            MISMATCH,
            "= ^c < c",
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
            "= ^n.z",
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
        st("cannot-infer", "let ^xs", None, &["let xs = []"]),
        st("cannot-infer", "let ^m", None, &["let m = [:]"]),
        st("cannot-infer", "let ^o", None, &["let o = None"]),
        st("cannot-infer", "let ^r", None, &["let r = Ok(1)"]),
        st("cannot-infer", "let ^x", None, &["let x = unreachable()"]),
        st("cannot-infer", "let ^x", None, &["let x = ??later"]),
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
        st("bad-inout-target", "&^inc(1)", None, &["bump(&inc(1))"]),
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
            Some(("List<Int> or List<Float> or List<String>", "List<Bool>")),
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
            &["return 1"],
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
            "= ^g()?",
            Some(("Result<T, String>", "Int")),
            &with(&["fn f() -> Int {", "  let n = g()?", "  n", "}"]),
        ),
        df(
            MISMATCH,
            "= ^g()?",
            Some(("Result<T, String>", "no result")),
            &with(&["fn f() {", "  let n = g()?", "  _ = n", "}"]),
        ),
        df(
            MISMATCH,
            "= ^o?",
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
            "= ^g()?",
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
            "= ^1?",
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
            Some(("String or List<T> or Map<K, V>", "Int")),
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
            "= ^[\"a\"]",
            Some(("List<Int>", "List<String>")),
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
            "[1, ^\"a\"]",
            Some(("Int", "String")),
            &["type T { xs: List<Int> = [1, \"a\"] }"],
        ),
        df(
            "wrong-argument-count",
            "= ^A",
            None,
            &["enum E { A(n: Int), B }", "type T { e: E = A }"],
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
        "type T { xs: List<Int> = [1, \"a\"] }",
        "type U { n: Int = 1 }",
    ];
    assert_eq!(codes(&lines), ["type-mismatch", "ok"]);
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
/// end or its head.
fn nested_chains(head: bool) -> String {
    let links = " + x".repeat(999);
    let wrap = |inner: String| match head {
        true => format!("({inner}{links} + x)"),
        false => format!("(x{links} + {inner})"),
    };
    let expr = (0..62).fold("x".to_string(), |inner, _| wrap(inner));
    program(&["fn f(x: Int) -> Int {", &format!("  {expr}"), "}"])
}

#[test]
fn the_deepest_allowed_chains_fit_the_stack() {
    for head in [false, true] {
        deep(|| {
            let src = nested_chains(head);
            assert!(errors(&src).is_empty());
            let bad = src.replacen("(x", "(\"x\"", 1);
            assert_eq!(errors(&bad).len(), 1);
        });
    }
}
