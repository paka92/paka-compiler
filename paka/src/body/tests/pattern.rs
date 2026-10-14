//! `match`, patterns, holes and example bodies (M2-20): the code, the position and, for a type
//! error, the types; the whole message once per distinct text.

use super::*;

/// `fn HEAD { BODY }` and the prelude.
pub(super) fn func(head: &str, body: &[&str]) -> String {
    let open = format!("fn {head} {{");
    full(&[&[open.as_str()][..], body, &["}"]].concat())
}

pub(super) fn bad<'a>(code: &'a str, mark: &'a str, head: &str, body: &[&str]) -> Case<'a> {
    (code, mark, None, func(head, body))
}

pub(super) fn typed<'a>(
    (code, mark): (&'a str, &'a str),
    types: (&'a str, &'a str),
    head: &str,
    body: &[&str],
) -> Case<'a> {
    (code, mark, Some(types), func(head, body))
}

pub(super) fn say<'a>(
    (code, mark): (&'a str, &'a str),
    message: &'a str,
    head: &str,
    body: &[&str],
) -> Say<'a> {
    (code, mark, message, func(head, body))
}

pub(super) const SHAPE: &str = "area(s: Shape) -> Int";
pub(super) const OPTION: &str = "f(o: Option<Int>) -> Int";
pub(super) const INT: &str = "f(n: Int) -> Int";
pub(super) const TEXT: &str = "f(t: String) -> Int";
pub(super) const BOOL: &str = "f(b: Bool) -> Int";
pub(super) const RESULT: &str = "f(r: Result<Int, String>) -> Int";

#[test]
fn accepts_matches_on_enums() {
    accepts(&[
        func(
            "area(s: Shape) -> Float",
            &[
                "match s {",
                "  Dot => 0.0",
                "  Circle(r) => r * r * 3.0",
                "  Rect(w, h) => w * h",
                "}",
            ],
        ),
        func(
            "f(s: Shape)",
            &[
                "match s {",
                "  Dot => {}",
                "  Circle(r) => shout(\"c\")",
                "  Rect(w, _) => {",
                "    _ = w",
                "  }",
                "}",
            ],
        ),
        func(OPTION, &["match o {", "  Some(v) => v", "  None => 0", "}"]),
        func(
            "f(r: Result<Int, String>) -> String",
            &["match r {", "  Err(e) => e", "  Ok(_) => \"ok\"", "}"],
        ),
        func(
            SHAPE,
            &[
                "match s {",
                "  Dot | Circle(_) => 1",
                "  Rect(_, _) => 2",
                "}",
            ],
        ),
        func(
            "f(a: Shape, b: Shape) -> Int",
            &[
                "match a {",
                "  Circle(x) => match b {",
                "    Dot => 1",
                "    Circle(y) => 2",
                "    Rect(_, _) => 3",
                "  }",
                "  Dot | Rect(_, _) => 4",
                "}",
            ],
        ),
    ]);
}

#[test]
fn accepts_matches_on_literals() {
    accepts(&[
        func(
            "f(n: Int) -> String",
            &[
                "match n {",
                "  0 => \"zero\"",
                "  -1 | 1 => \"one\"",
                "  _ => \"many\"",
                "}",
            ],
        ),
        func(
            TEXT,
            &[
                "match t {",
                "  \"a\" => 1",
                "  \"b\" | \"c\" => 2",
                "  _ => 3",
                "}",
            ],
        ),
        func(BOOL, &["match b {", "  true => 1", "  false => 2", "}"]),
        func(BOOL, &["match b {", "  false => 1", "  _ => 2", "}"]),
        func(BOOL, &["match b {", "  _ => 1", "}"]),
    ]);
}

#[test]
fn a_name_bound_in_an_arm_lives_in_that_arm() {
    accepts(&[func(
        "f(s: Shape) -> Float",
        &[
            "match s {",
            "  Circle(x) => x",
            "  Rect(x, _) => x",
            "  Dot => 0.0",
            "}",
        ],
    )]);
    rejects(&[typed(
        (MISMATCH, "r ^+ 1"),
        ("Float", "Int"),
        "f(s: Shape) -> Float",
        &[
            "match s {",
            "  Circle(r) => r + 1",
            "  Dot => 0.0",
            "  Rect(_, _) => 1.0",
            "}",
        ],
    )]);
    let leaks = [
        "fn f(s: Shape) -> Float {",
        "  match s {",
        "    Circle(r) => r",
        "    Dot => r",
        "    Rect(_, _) => 1.0",
        "  }",
        "}",
        "enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }",
    ];
    assert_eq!(codes(&leaks), ["unknown-name", "ok"]);
}

#[test]
fn types_a_match_by_its_first_arm_that_has_a_type() {
    accepts(&[
        unit(&[
            "let n = 1",
            "let x = match n {",
            "  0 => None",
            "  _ => Some(1)",
            "}",
            "let y: Option<Int> = x",
            "_ = y",
        ]),
        func(
            "f(n: Int) -> Result<Int, String>",
            &["match n {", "  0 => Ok(1)", "  _ => Err(\"no\")", "}"],
        ),
        unit(&[
            "let n = 1",
            "var t = 0",
            "match n {",
            "  0 => t = 1",
            "  _ => t += 2",
            "}",
        ]),
    ]);
    rejects(&[
        typed(
            (MISMATCH, "_ => ^\"a\""),
            ("Int", "String"),
            "f(n: Int)",
            &[
                "let x = match n {",
                "  0 => 1",
                "  _ => \"a\"",
                "}",
                "_ = x",
            ],
        ),
        typed(
            (MISMATCH, "Circle(r) => ^\"x\""),
            ("Float", "String"),
            "f(s: Shape) -> Float",
            &[
                "match s {",
                "  Dot => 0.0",
                "  Circle(r) => \"x\"",
                "  Rect(w, h) => w",
                "}",
            ],
        ),
        typed(
            (MISMATCH, "Rect(w, h) => ^1"),
            ("Float", "Int"),
            "f(s: Shape) -> Float",
            &[
                "let x: Float = match s {",
                "  Dot => 0.0",
                "  Circle(r) => r",
                "  Rect(w, h) => 1",
                "}",
                "x",
            ],
        ),
        typed(
            (MISMATCH, "Dot => ^shout("),
            ("Int", "no value"),
            SHAPE,
            &[
                "match s {",
                "  Dot => shout(\"a\")",
                "  Circle(_) => 1",
                "  Rect(_, _) => 2",
                "}",
            ],
        ),
        bad(
            "unused-result",
            "Dot => ^inc(1)",
            "f(s: Shape)",
            &[
                "match s {",
                "  Dot => inc(1)",
                "  Circle(_) => {}",
                "  Rect(_, _) => {}",
                "}",
            ],
        ),
    ]);
    says(&[(
        "missing-value",
        "^0 => t = 1",
        "this arm must give a value",
        unit(&[
            "let n = 1",
            "var t = 0",
            "let x = match n {",
            "  0 => t = 1",
            "  _ => 2",
            "}",
            "_ = x",
        ]),
    )]);
}

#[test]
fn an_arm_may_leave_or_stop() {
    accepts(&[
        func(
            "f(o: Option<Int>) -> Result<Int, String>",
            &[
                "let n = match o {",
                "  Some(v) => v",
                "  None => {",
                "    return Err(\"missing\")",
                "  }",
                "}",
                "Ok(n)",
            ],
        ),
        func(
            OPTION,
            &[
                "match o {",
                "  Some(v) => v",
                "  None => unreachable()",
                "}",
            ],
        ),
        func(
            "f(o: Option<Int>)",
            &[
                "for i in 0..<3 {",
                "  match o {",
                "    Some(_) => {",
                "      break",
                "    }",
                "    None => {",
                "      continue",
                "    }",
                "  }",
                "}",
            ],
        ),
    ]);
    rejects(&[
        bad(
            "unreachable-code",
            "^let z = 7",
            OPTION,
            &[
                "match o {",
                "  Some(v) => {",
                "    return v",
                "  }",
                "  None => {",
                "    return 0",
                "  }",
                "}",
                "let z = 7",
                "z",
            ],
        ),
        bad(
            "return-last",
            "^return v",
            OPTION,
            &[
                "match o {",
                "  Some(v) => {",
                "    return v",
                "  }",
                "  None => 0",
                "}",
            ],
        ),
        bad(
            "missing-value",
            "_ = v ^}",
            OPTION,
            &["match o {", "  Some(v) => { _ = v }", "  None => 0", "}"],
        ),
    ]);
}

#[test]
fn lists_the_variants_a_match_leaves_out() {
    says(&[
        say(
            ("missing-arm", "^match o"),
            "no arm for variant None; add None => ...",
            OPTION,
            &["match o {", "  Some(v) => v", "}"],
        ),
        say(
            ("missing-arm", "^match r"),
            "no arm for variant Ok; add Ok(_) => ...",
            RESULT,
            &["match r {", "  Err(_) => 0", "}"],
        ),
        say(
            ("missing-arm", "^match s"),
            "no arm for variants Circle, Rect; add Circle(_) => ... and Rect(_, _) => ...",
            SHAPE,
            &["match s {", "  Dot => 0", "}"],
        ),
        say(
            ("missing-arm", "^match s"),
            "no arm for variant Rect; add Rect(_, _) => ...",
            SHAPE,
            &["match s {", "  Dot | Circle(_) => 0", "}"],
        ),
        say(
            ("missing-arm", "^match b"),
            "no arm for false; add false => ...",
            BOOL,
            &["match b {", "  true => 1", "}"],
        ),
        say(
            ("missing-arm", "^match n"),
            "no arm for other Int values; add _ => ...",
            INT,
            &["match n {", "  0 => 1", "  1 => 2", "}"],
        ),
        say(
            ("missing-arm", "^match t"),
            "no arm for other String values; add _ => ...",
            TEXT,
            &["match t {", "  \"a\" => 1", "}"],
        ),
    ]);
}

#[test]
fn a_missing_arm_comes_before_anything_inside_the_match() {
    rejects(&[
        bad(
            "missing-arm",
            "^match s",
            SHAPE,
            &["match s {", "  Dot => 1 + \"a\"", "  Circle(_) => 2", "}"],
        ),
        bad(
            "missing-arm",
            "^match n",
            INT,
            &["match n {", "  0 => inc(\"a\")", "}"],
        ),
    ]);
    let unknown = [
        "fn f(s: Shape) -> Int {",
        "  match s {",
        "    Dot => 1",
        "    Cirle(_) => 2",
        "    Rect(_, _) => 3",
        "  }",
        "}",
        "enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }",
    ];
    assert_eq!(codes(&unknown), ["unknown-name", "ok"]);
}

#[test]
fn the_first_error_is_the_earliest_in_the_source() {
    rejects(&[
        typed(
            (MISMATCH, "x: Int = ^\"a\""),
            ("Int", "String"),
            INT,
            &["let x: Int = \"a\"", "match n {", "  0 => x", "}"],
        ),
        typed(
            (MISMATCH, "1 ^+ \"a\""),
            ("Int", "String"),
            SHAPE,
            &[
                "match s {",
                "  Dot => 1 + \"a\"",
                "  Circle(_) => 2",
                "  Rect(_, _) => 3",
                "  Dot => 4",
                "}",
            ],
        ),
        bad(
            "duplicate-arm",
            "^Dot => 4",
            SHAPE,
            &[
                "match s {",
                "  Dot => 1",
                "  Circle(_) => 2",
                "  Dot => 4",
                "  Rect(_, _) => 3",
                "}",
            ],
        ),
    ]);
}

#[test]
fn rejects_an_arm_that_adds_nothing() {
    says(&[
        say(
            ("duplicate-arm", "^Dot => 2"),
            "Dot is already matched above; drop it",
            SHAPE,
            &[
                "match s {",
                "  Dot => 1",
                "  Dot => 2",
                "  Circle(_) => 3",
                "  Rect(_, _) => 4",
                "}",
            ],
        ),
        say(
            ("duplicate-arm", "| ^Circle(_)"),
            "Circle is already matched above; drop it",
            SHAPE,
            &[
                "match s {",
                "  Circle(_) => 1",
                "  Dot | Circle(_) => 2",
                "  Rect(_, _) => 3",
                "}",
            ],
        ),
        say(
            ("duplicate-arm", "^1 => 2"),
            "1 is already matched above; drop it",
            INT,
            &["match n {", "  1 => 1", "  1 => 2", "  _ => 3", "}"],
        ),
        say(
            ("duplicate-arm", "| ^-1 =>"),
            "-1 is already matched above; drop it",
            INT,
            &["match n {", "  -1 => 1", "  0 | -1 => 2", "  _ => 3", "}"],
        ),
        say(
            ("duplicate-arm", "^\"a\" => 2"),
            "\"a\" is already matched above; drop it",
            TEXT,
            &["match t {", "  \"a\" => 1", "  \"a\" => 2", "  _ => 3", "}"],
        ),
        say(
            ("duplicate-arm", "^true => 3"),
            "true is already matched above; drop it",
            BOOL,
            &[
                "match b {",
                "  true => 1",
                "  false => 2",
                "  true => 3",
                "}",
            ],
        ),
        say(
            ("duplicate-arm", "^0 => 2"),
            "0 is already matched above; drop it",
            INT,
            &["match n {", "  _ => 1", "  0 => 2", "}"],
        ),
        say(
            ("duplicate-arm", "^_ => 2"),
            "_ is already matched above; drop it",
            INT,
            &["match n {", "  _ => 1", "  _ => 2", "}"],
        ),
        say(
            ("duplicate-arm", "^_ => 3"),
            "_ is already matched above; drop it",
            BOOL,
            &["match b {", "  true => 1", "  false => 2", "  _ => 3", "}"],
        ),
    ]);
}

#[test]
fn a_wildcard_is_for_int_string_and_bool() {
    says(&[
        say(
            ("wildcard-on-enum", "^_ => 0"),
            "_ is for Int, String and Bool; name each variant of Option",
            OPTION,
            &["match o {", "  Some(v) => v", "  _ => 0", "}"],
        ),
        say(
            ("wildcard-on-enum", "^_ => 0"),
            "_ is for Int, String and Bool; name each variant of Shape",
            SHAPE,
            &["match s {", "  Dot => 1", "  _ => 0", "}"],
        ),
        say(
            ("wildcard-on-enum", "^_ => 0"),
            "_ is for Int, String and Bool; name each variant of Result",
            RESULT,
            &[
                "match r {",
                "  Ok(v) => v",
                "  Err(_) => 1",
                "  _ => 0",
                "}",
            ],
        ),
    ]);
    rejects(&[
        bad(
            "wildcard-on-enum",
            "^_ => 0",
            SHAPE,
            &["match s {", "  _ => 0", "}"],
        ),
        bad(
            "wildcard-on-enum",
            "| ^_ =>",
            SHAPE,
            &["match s {", "  Dot | _ => 0", "  Circle(_) => 1", "}"],
        ),
    ]);
}

#[test]
fn a_pattern_has_the_type_of_the_subject() {
    rejects(&[
        typed(
            (MISMATCH, "^\"a\" => 1"),
            ("Int", "String"),
            INT,
            &["match n {", "  \"a\" => 1", "  _ => 2", "}"],
        ),
        typed(
            (MISMATCH, "^1 => 1"),
            ("String", "Int"),
            TEXT,
            &["match t {", "  1 => 1", "  _ => 2", "}"],
        ),
        typed(
            (MISMATCH, "^true => 1"),
            ("Int", "Bool"),
            INT,
            &["match n {", "  true => 1", "  _ => 2", "}"],
        ),
        typed(
            (MISMATCH, "^1 => 1"),
            ("Bool", "Int"),
            BOOL,
            &["match b {", "  1 => 1", "  _ => 2", "}"],
        ),
        typed(
            (MISMATCH, "^Dot => 1"),
            ("Int", "Shape"),
            INT,
            &["match n {", "  Dot => 1", "  _ => 2", "}"],
        ),
        typed(
            (MISMATCH, "^1 => 1"),
            ("Shape", "Int"),
            SHAPE,
            &["match s {", "  1 => 1", "  Dot => 2", "}"],
        ),
        typed(
            (MISMATCH, "^Some(v)"),
            ("Shape", "Option"),
            SHAPE,
            &["match s {", "  Some(v) => 1", "  None => 2", "}"],
        ),
        typed(
            (MISMATCH, "^Dot =>"),
            ("Option<Int>", "Shape"),
            OPTION,
            &["match o {", "  Dot => 1", "  None => 2", "}"],
        ),
        typed(
            (MISMATCH, "^Ok(v)"),
            ("Option<Int>", "Result"),
            OPTION,
            &["match o {", "  Ok(v) => 1", "  None => 2", "}"],
        ),
    ]);
}

#[test]
fn a_subject_of_unknown_type_is_not_held_to_its_patterns() {
    let lines = [
        "fn f(x: Nope) -> Int {",
        "  match x {",
        "    Dot => 1",
        "    Circle(r) => 2",
        "    7 => 3",
        "  }",
        "}",
        "enum Shape { Dot, Circle(r: Float) }",
    ];
    assert_eq!(codes(&lines), ["unknown-type", "ok"]);
}

#[test]
fn only_an_int_string_bool_or_enum_is_matched() {
    let want = "Int, String, Bool or an enum";
    rejects(&[
        typed(
            (MISMATCH, "match ^x {"),
            (want, "Float"),
            "f(x: Float) -> Int",
            &["match x {", "  _ => 1", "}"],
        ),
        typed(
            (MISMATCH, "match ^c {"),
            (want, "Counter"),
            "f(c: Counter) -> Int",
            &["match c {", "  _ => 1", "}"],
        ),
        typed(
            (MISMATCH, "match ^xs {"),
            (want, "List<Int>"),
            "f(xs: List<Int>) -> Int",
            &["match xs {", "  _ => 1", "}"],
        ),
    ]);
}

/// A match on a Shape whose Circle arm is `circle`.
fn circle_arm(circle: &str) -> Vec<&str> {
    vec!["match s {", "  Dot => 1", circle, "  Rect(_, _) => 3", "}"]
}

#[test]
fn a_variant_pattern_names_each_field() {
    says(&[
        say(
            ("wrong-argument-count", "^Circle(a, b)"),
            "Circle has 1 field (r): write Circle(_)",
            SHAPE,
            &circle_arm("  Circle(a, b) => 2"),
        ),
        say(
            ("wrong-argument-count", "^Circle =>"),
            "Circle has 1 field (r): write Circle(_)",
            SHAPE,
            &circle_arm("  Circle => 2"),
        ),
        say(
            ("wrong-argument-count", "^Rect(w)"),
            "Rect has 2 fields (w, h): write Rect(_, _)",
            SHAPE,
            &[
                "match s {",
                "  Dot => 1",
                "  Circle(_) => 2",
                "  Rect(w) => 3",
                "}",
            ],
        ),
        say(
            ("wrong-argument-count", "^Dot(_)"),
            "Dot has no fields: write Dot",
            SHAPE,
            &[
                "match s {",
                "  Dot(_) => 1",
                "  Circle(_) => 2",
                "  Rect(_, _) => 3",
                "}",
            ],
        ),
        say(
            ("wrong-argument-count", "^None(_)"),
            "None has no fields: write None",
            OPTION,
            &["match o {", "  Some(v) => v", "  None(_) => 0", "}"],
        ),
    ]);
}

#[test]
fn a_pattern_name_is_a_copy() {
    says(&[
        say(
            ("assign-to-let", "^w = 1.0"),
            "w is a pattern name; declare it with var",
            "f(s: Shape)",
            &[
                "match s {",
                "  Rect(w, _) => w = 1.0",
                "  Dot => {}",
                "  Circle(_) => {}",
                "}",
            ],
        ),
        say(
            ("bad-inout-target", "bump(&^w)"),
            "w is a pattern name; it is a copy",
            "f(o: Option<Int>)",
            &["match o {", "  Some(w) => bump(&w)", "  None => {}", "}"],
        ),
    ]);
}

#[test]
fn rejects_a_pattern_integer_that_does_not_exist() {
    says(&[say(
        ("literal-out-of-range", "^99999999999999999999 =>"),
        "largest Int is 9223372036854775807",
        INT,
        &["match n {", "  99999999999999999999 => 1", "  _ => 2", "}"],
    )]);
}

#[test]
fn takes_the_type_asked_for_in_a_hole() {
    accepts(&[
        unit(&["let n: Int = ??n"]),
        unit(&["let s: Shape = ??s", "_ = s"]),
        unit(&["_ = inc(??a)"]),
        unit(&["shout(??t)"]),
        unit(&["let c = Counter { name: ??nm }", "_ = c"]),
        unit(&["var n = 1", "n = ??next", "_ = n"]),
        unit(&["if ??c {", "}"]),
        unit(&["while ??c {", "}"]),
        unit(&["let xs: List<Int> = [1, ??two]", "_ = xs"]),
        unit(&["let xs: List<Int> = [??one]", "_ = xs"]),
        unit(&["let m: Map<String, Int> = [\"a\": ??v]", "_ = m"]),
        unit(&["??todo"]),
        unit(&["??first", "??second"]),
        func("f() -> Int", &["??body"]),
        func(
            "f() -> Int",
            &["let a = 1", "if a == 1 {", "  return ??early", "}", "a"],
        ),
        func("f(n: Int) -> Int", &["if n == 0 { ??zero } else { n }"]),
        func("f(n: Int) -> Int", &["if n == 0 { n } else { ??other }"]),
        func(
            "f(n: Int) -> Int",
            &["match n {", "  0 => ??zero", "  _ => ??other", "}"],
        ),
        func("f(n: Int) -> Int", &["??a + 1"]),
        func("f(n: Int) -> Int", &["1 + ??b"]),
        func("f(n: Int) -> Bool", &["??a == n"]),
        func("f(n: Int) -> Bool", &["n < 1 && ??b"]),
        func("f(t: String) -> String", &["??a + t"]),
        func("f(n: Int) -> Int", &["n * ??a + ??b * 2"]),
    ]);
}

/// The type held for the expression at `at` of length `len` in the first definition.
pub(super) fn type_at(src: &str, at: Pos, len: usize) -> Option<String> {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = check("main.paka", &program, &resolved, &mut table);
    (bodies.types[0].get(&(at, len))).map(|&t| table.types.show(t))
}

#[test]
fn a_hole_beside_a_typed_operand_takes_its_type() {
    let left = func("f(n: Int) -> Bool", &["??a == n"]);
    assert_eq!(type_at(&left, (2, 1), 3).as_deref(), Some("Int"));
    let right = func("f(t: String) -> String", &["t + ??a"]);
    assert_eq!(type_at(&right, (2, 5), 3).as_deref(), Some("String"));
    rejects(&[
        typed(
            (MISMATCH, "??a ^&& 1"),
            ("Bool", "Int"),
            "f() -> Bool",
            &["??a && 1"],
        ),
        typed(
            (MISMATCH, "??a ^- \"s\""),
            ("Int or Float", "String"),
            "f() -> Int",
            &["??a - \"s\""],
        ),
    ]);
}

#[test]
fn a_hole_with_no_type_is_cannot_infer() {
    rejects(&[
        st("cannot-infer", "= ^??n", None, &["let n = ??n"]),
        st("cannot-infer", "= ^??x", None, &["var x = ??x"]),
        st("cannot-infer", "_ = ^??x", None, &["_ = ??x"]),
        st("cannot-infer", "^??a.f", None, &["_ = ??a.f"]),
        st("cannot-infer", "^??a[0]", None, &["_ = ??a[0]"]),
        st("cannot-infer", "-^??a", None, &["_ = -??a"]),
        st("cannot-infer", "{^??t}", None, &["_ = \"{??t}\""]),
        st("cannot-infer", "in ^??xs", None, &["for x in ??xs {", "}"]),
        st(
            "cannot-infer",
            "match ^??s",
            None,
            &["match ??s {", "  _ => {}", "}"],
        ),
        st("cannot-infer", "_ = ^??a", None, &["_ = ??a + ??b"]),
        st("cannot-infer", "_ = ^??a", None, &["_ = ??a == ??b"]),
        st("cannot-infer", "len(^??xs)", None, &["_ = len(??xs)"]),
        st(
            "cannot-infer",
            "{ ^??a }",
            None,
            &["let c = true", "let x = if c { ??a } else { ??b }", "_ = x"],
        ),
    ]);
    says(&[
        (
            "cannot-infer",
            "= ^??n",
            "cannot infer the type of ??n; write it: let n: T = ??n",
            unit(&["let n = ??n"]),
        ),
        (
            "cannot-infer",
            "= ^??item",
            "cannot infer the type of ??item; write it: let item: T = ??item",
            unit(&["var x = ??item"]),
        ),
    ]);
}

#[test]
fn a_hole_stays_silent_where_an_error_elsewhere_left_the_type_unknown() {
    let lines = [
        "fn bad(x: Nope, y: Int) -> Int {",
        "  y",
        "}",
        "fn good() -> Int {",
        "  bad(??h, 1)",
        "}",
    ];
    assert_eq!(codes(&lines), ["unknown-type", "ok"]);
}

#[test]
fn checks_an_example_as_a_function_that_gives_a_bool() {
    accepts(&[
        full(&["example one {", "  inc(1) == 2", "}"]),
        full(&[
            "example two {",
            "  let a = inc(1)",
            "  _ = inc(a)",
            "  a == 2",
            "}",
        ]),
        full(&[
            "example three {",
            "  let a = inc(1)",
            "  if a == 2 {",
            "    return true",
            "  }",
            "  false",
            "}",
        ]),
        full(&["example four {", "  ??todo", "}"]),
        full(&["example five {", "  let n: Int = ??n", "  n == 1", "}"]),
        full(&[
            "example six {",
            "  match Some(1) {",
            "    Some(v) => v == 1",
            "    None => false",
            "  }",
            "}",
        ]),
    ]);
    rejects(&[
        df(
            MISMATCH,
            "^1 + 1",
            Some(("Bool", "Int")),
            &["example bad {", "  1 + 1", "}"],
        ),
        df(
            MISMATCH,
            "^inc(a)",
            Some(("Bool", "Int")),
            &["example bad {", "  let a = 1", "  inc(a)", "}"],
        ),
        df("missing-value", "^}", None, &["example bad {", "}"]),
        df(
            "missing-value",
            "^}",
            None,
            &["example bad {", "  _ = inc(1)", "}"],
        ),
        df(
            MISMATCH,
            "return ^1",
            Some(("Bool", "Int")),
            &[
                "example bad {",
                "  if true {",
                "    return 1",
                "  }",
                "  true",
                "}",
            ],
        ),
        df(
            "cannot-infer",
            "= ^??n",
            None,
            &["example bad {", "  let n = ??n", "  true", "}"],
        ),
        df(
            "unused-result",
            "^inc(1)",
            None,
            &["example bad {", "  inc(1)", "  true", "}"],
        ),
    ]);
}

#[test]
fn an_example_has_no_question_mark() {
    let lines = [
        "fn pick() -> Option<Int> {",
        "  Some(1)",
        "}",
        "example bad {",
        "  let v = pick()?",
        "  v == 1",
        "}",
    ];
    let src = program(&lines);
    let found = errors(&src);
    let [d] = &found[..] else {
        panic!("want one error, got {found:?}")
    };
    assert_eq!(
        (d.code, d.position, d.message.as_str()),
        (
            MISMATCH,
            at(&src, "pick()^?"),
            "an example has no ?; match it"
        )
    );
    assert_eq!(
        types_of(d, &src),
        Some(("Option<T>".to_string(), "Bool".to_string()))
    );
}

#[test]
fn an_example_fails_alone() {
    let lines = [
        "example bad {",
        "  1",
        "}",
        "example good {",
        "  true",
        "}",
        "fn f() -> Int {",
        "  1",
        "}",
    ];
    assert_eq!(codes(&lines), ["type-mismatch", "ok", "ok"]);
}

/// A program with a large enum, a match with an arm for each variant, one with a single wide
/// `|` arm and one over as many integers.
fn large(n: usize) -> String {
    let names: Vec<_> = (0..n).map(|i| format!("V{i}")).collect();
    let each: Vec<_> = (0..n).map(|i| format!("V{i} => {i}")).collect();
    let ints: Vec<_> = (0..n).map(|i| format!("{i} => {i}")).collect();
    let decl = format!("enum Big {{ {} }}", names.join(", "));
    let (each, ints) = (each.join("\n"), ints.join("\n"));
    let wide = format!("{} => 1", names.join(" | "));
    program(&[
        &decl,
        "fn each(b: Big) -> Int {",
        "  match b {",
        &each,
        "  }",
        "}",
        "fn wide(b: Big) -> Int {",
        "  match b {",
        &wide,
        "  }",
        "}",
        "fn lit(n: Int) -> Int {",
        "  match n {",
        &ints,
        "  _ => 0",
        "  }",
        "}",
    ])
}

#[test]
fn a_large_enum_with_many_arms_is_checked_in_one_pass() {
    let n = 20_000;
    let found = errors(&large(n));
    assert!(found.is_empty(), "{:?}", found.first());
    let last = format!("V{} => {}", n - 1, n - 1);
    let [d] = &errors(&large(n).replace(&last, ""))[..] else {
        panic!("want one error")
    };
    let want = format!("no arm for variant V{0}; add V{0} => ...", n - 1);
    assert_eq!((d.code, d.message.as_str()), ("missing-arm", want.as_str()));
    let twice = large(n).replace("V0 | V1", "V0 | V0 | V1");
    let [d] = &errors(&twice)[..] else {
        panic!("want one error")
    };
    assert_eq!(d.code, "duplicate-arm");
}

/// `depth` matches inside each other, each in the first arm of the one before.
fn nested_matches(depth: usize) -> String {
    let mut lines = vec!["fn f(n: Int) {".to_string()];
    for _ in 0..depth {
        lines.extend(["match n {".to_string(), "0 => {".to_string()]);
    }
    for _ in 0..depth {
        lines.extend(["}".to_string(), "_ => {}".to_string(), "}".to_string()]);
    }
    lines.push("}".to_string());
    lines.join("\n") + "\n"
}

#[test]
fn the_deepest_allowed_matches_fit_the_stack() {
    let parses = |d: usize| parse("main.paka", nested_matches(d).as_bytes()).is_ok();
    let deepest = (1..64).take_while(|&d| parses(d)).last().unwrap();
    assert!(deepest >= 20, "{deepest}");
    deep(|| assert!(errors(&nested_matches(deepest)).is_empty()));
}
