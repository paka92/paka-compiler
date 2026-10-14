//! Branch typing, holes beside operators, error positions and message texts (M2-20).

use super::pattern::*;
use super::*;

/// The type held for the `len` characters at `mark`.
fn holds(src: &str, mark: &str, len: usize, want: &str) {
    let pos = at(src, mark).unwrap_or_else(|| panic!("no {mark} in\n{src}"));
    assert_eq!(type_at(src, pos, len).as_deref(), Some(want), "{mark}");
}

#[test]
fn a_subject_without_a_type_of_its_own_is_cannot_infer() {
    says(&[(
        "cannot-infer",
        "match ^None",
        "cannot infer the type of None; match a value with a known type",
        unit(&["match None {", "  None => {}", "}"]),
    )]);
    rejects(&[
        st(
            "cannot-infer",
            "match ^Ok(1)",
            None,
            &["match Ok(1) {", "  Ok(_) => {}", "  Err(_) => {}", "}"],
        ),
        st(
            "cannot-infer",
            "match ^Err(\"a\")",
            None,
            &["match Err(\"a\") {", "  Ok(_) => {}", "  Err(_) => {}", "}"],
        ),
    ]);
}

#[test]
fn the_first_branch_with_a_type_gives_it_to_the_others() {
    let n = "f(n: Int)";
    let programs = [
        func(
            n,
            &["let v = match n {", "  0 => ??x", "  _ => 2", "}", "_ = v"],
        ),
        func(
            n,
            &["let v = match n {", "  0 => 2", "  _ => ??x", "}", "_ = v"],
        ),
        func(n, &["let v = if n > 0 { ??x } else { 2 }", "_ = v"]),
        func(n, &["let v = if n > 0 { 2 } else { ??x }", "_ = v"]),
        func(
            n,
            &[
                "let v = if n > 0 {",
                "  _ = 1",
                "  ??x",
                "} else {",
                "  2",
                "}",
                "_ = v",
            ],
        ),
        func(n, &["let xs = [??x, 1]", "_ = xs"]),
        func(n, &["let xs = [1, ??x]", "_ = xs"]),
        func(
            n,
            &[
                "let v = if n > 0 { ??x } else if n < 0 { ??y } else { 2 }",
                "_ = v",
            ],
        ),
    ];
    accepts(&programs);
    holds(&programs[0], "0 => ^??x", 3, "Int");
    holds(&programs[1], "_ => ^??x", 3, "Int");
    holds(&programs[2], "{ ^??x }", 3, "Int");
    holds(&programs[4], "  ^??x", 3, "Int");
    holds(&programs[5], "[^??x, 1]", 3, "Int");
    holds(&programs[7], "{ ^??y }", 3, "Int");
}

#[test]
fn a_branch_without_a_type_is_recorded_with_the_one_it_gets() {
    let n = "f(n: Int)";
    let first = func(
        n,
        &[
            "let v = match n {",
            "  0 => None",
            "  _ => Some(n)",
            "}",
            "_ = v",
        ],
    );
    let last = func(
        n,
        &[
            "let v = match n {",
            "  0 => Some(n)",
            "  _ => None",
            "}",
            "_ = v",
        ],
    );
    let branch = func(n, &["let v = if n > 0 { None } else { Some(n) }", "_ = v"]);
    let list = func(n, &["let xs = [None, Some(n)]", "_ = xs"]);
    accepts(&[first.clone(), last.clone(), branch.clone(), list.clone()]);
    holds(&first, "0 => ^None", 4, "Option<Int>");
    holds(&last, "_ => ^None", 4, "Option<Int>");
    holds(&branch, "{ ^None }", 4, "Option<Int>");
    holds(&list, "[^None,", 4, "Option<Int>");
}

#[test]
fn a_branch_that_does_not_fit_the_typed_one_is_the_error() {
    rejects(&[
        typed(
            (MISMATCH, "0 => ^None"),
            ("Int", "Option<T>"),
            "f(n: Int)",
            &["let v = match n {", "  0 => None", "  _ => 2", "}", "_ = v"],
        ),
        typed(
            (MISMATCH, "{ ^None }"),
            ("Int", "Option<T>"),
            "f(n: Int)",
            &["let v = if n > 0 { None } else { 2 }", "_ = v"],
        ),
    ]);
}

#[test]
fn branches_that_are_all_open_report_at_the_first() {
    rejects(&[
        bad(
            "cannot-infer",
            "0 => ^??a",
            "f(n: Int)",
            &[
                "let v = match n {",
                "  0 => ??a",
                "  _ => ??b",
                "}",
                "_ = v",
            ],
        ),
        bad(
            "cannot-infer",
            "0 => ^None",
            "f(n: Int)",
            &[
                "let v = match n {",
                "  0 => None",
                "  _ => None",
                "}",
                "_ = v",
            ],
        ),
        bad(
            "cannot-infer",
            "{ ^??a }",
            "f(n: Int)",
            &["let v = if n > 0 { ??a } else { ??b }", "_ = v"],
        ),
        bad(
            "cannot-infer",
            "= [^??a, ??b]",
            "f(n: Int)",
            &["let v = [??a, ??b]", "_ = v"],
        ),
    ]);
    says(&[say(
        ("cannot-infer", "0 => ^None"),
        "cannot infer the type of v; write it: let v: Option<T> = ...",
        "f(n: Int)",
        &[
            "let v = match n {",
            "  0 => None",
            "  _ => None",
            "}",
            "_ = v",
        ],
    )]);
}

#[test]
fn ok_and_err_in_two_branches_do_not_complete_each_other() {
    rejects(&[
        bad(
            "cannot-infer",
            "{ ^Ok(1) }",
            "f(c: Bool)",
            &["let r = if c { Ok(1) } else { Err(\"x\") }", "_ = r"],
        ),
        bad(
            "cannot-infer",
            "{ ^Err(\"x\") }",
            "f(c: Bool)",
            &["let r = if c { Err(\"x\") } else { Ok(1) }", "_ = r"],
        ),
        bad(
            "cannot-infer",
            "0 => ^Ok(1)",
            "f(n: Int)",
            &[
                "let r = match n {",
                "  0 => Ok(1)",
                "  _ => Err(\"x\")",
                "}",
                "_ = r",
            ],
        ),
    ]);
    accepts(&[
        func(
            "f(c: Bool)",
            &[
                "let r: Result<Int, String> = if c { Ok(1) } else { Err(\"x\") }",
                "_ = r",
            ],
        ),
        func(
            "f(c: Bool) -> Result<Int, String>",
            &["if c { Ok(1) } else { Err(\"x\") }"],
        ),
        func(
            "f(n: Int) -> Result<Int, String>",
            &["match n {", "  0 => Ok(1)", "  _ => Err(\"x\")", "}"],
        ),
    ]);
}

#[test]
fn cannot_infer_stands_inside_parentheses() {
    rejects(&[
        st("cannot-infer", "= (^??a)", None, &["let x = (??a)"]),
        st("cannot-infer", "= ( ^??a )", None, &["let x = ( ??a )"]),
        st("cannot-infer", "= ((^??a))", None, &["let x = ((??a))"]),
        st("cannot-infer", "= (^None)", None, &["let x = (None)"]),
        st("cannot-infer", "= (^Ok(1))", None, &["let x = (Ok(1))"]),
    ]);
}

#[test]
fn a_missing_arm_comes_before_an_error_in_the_subject() {
    let g = ["fn g(n: Int) -> Option<Int> {", "  Some(n)", "}"];
    let with = |arms: &[&str]| {
        let head = ["fn f() -> Int {", "  match g(\"a\") {"];
        df("", "", None, &[&head[..], arms, &["  }", "}"], &g].concat()).3
    };
    let partial = with(&["    Some(v) => v"]);
    let whole = with(&["    Some(v) => v", "    None => 0"]);
    let found = errors(&partial);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].code, found[0].position),
        ("missing-arm", at(&partial, "^match g"))
    );
    let found = errors(&whole);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].code, found[0].position),
        (MISMATCH, at(&whole, "g(^\"a\")"))
    );
}

#[test]
fn a_hole_beside_a_range_takes_the_type_of_the_other_side() {
    let low = func("f(n: Int)", &["for i in ??lo..<n {", "  _ = i", "}"]);
    let high = func("f()", &["for i in 0..<??hi {", "  _ = i", "}"]);
    let slices = unit(&["let xs = [1, 2, 3]", "_ = xs[??a..<2]", "_ = xs[0..<??b]"]);
    accepts(&[low.clone(), high.clone(), slices.clone()]);
    holds(&low, "in ^??lo", 4, "Int");
    holds(&high, "..<^??hi", 4, "Int");
    holds(&slices, "[^??a..<2]", 3, "Int");
    holds(&slices, "0..<^??b]", 3, "Int");
    let both = func("f()", &["for i in ??a..<??b {", "  _ = i", "}"]);
    accepts(std::slice::from_ref(&both));
    holds(&both, "in ^??a", 3, "Int");
    holds(&both, "..<^??b", 3, "Int");
}

#[test]
fn a_hole_beside_a_logical_operator_is_a_bool() {
    let both = unit(&["_ = ??a && ??b", "_ = ??c || true", "_ = true && ??d"]);
    let not = unit(&["_ = !??e"]);
    accepts(&[both.clone(), not.clone()]);
    holds(&both, "_ = ^??a &&", 3, "Bool");
    holds(&both, "&& ^??b", 3, "Bool");
    holds(&both, "_ = ^??c", 3, "Bool");
    holds(&both, "&& ^??d", 3, "Bool");
    holds(&not, "!^??e", 3, "Bool");
}

#[test]
fn a_hole_has_no_type_to_write_in_return() {
    says(&[say(
        ("type-mismatch", "return ^??x"),
        "f has no result; drop the value",
        "f()",
        &["let c = true", "if c {", "  return ??x", "}", "_ = c"],
    )]);
}

#[test]
fn an_integer_out_of_range_names_the_largest() {
    says(&[(
        "literal-out-of-range",
        "= ^99999999999999999999",
        "largest Int is 9223372036854775807",
        unit(&["_ = 99999999999999999999"]),
    )]);
}

#[test]
fn an_exit_status_is_zero_or_five_to_255() {
    let exit = |n: &str| unit(&[&format!("let e: Effect<Shape> = ExitWith({n})"), "_ = e"]);
    accepts(
        &[
            "0", "5", "255", "1 + 3", "-1 + 6", "--5", "-(-5)", "- 1", "-(1)",
        ]
        .map(exit),
    );
    let message =
        |n: &str| format!("{n} is not an exit status a program may choose; write 0 or 5 to 255");
    for n in ["1", "4", "256", "-1"] {
        let (message, mark) = (message(n), format!("ExitWith(^{n})"));
        says(&[("literal-out-of-range", &mark, &message, exit(n))]);
    }
    // the parser keeps no parentheses: the literal's node starts at the opening one
    let (message, src) = (message("3"), exit("(3)"));
    says(&[("literal-out-of-range", "ExitWith(^(3))", &message, src)]);
}

#[test]
fn a_wrong_arity_is_reported_before_a_repeat() {
    rejects(&[
        bad(
            "wrong-argument-count",
            "^Circle =>",
            SHAPE,
            &[
                "match s {",
                "  Circle(_) => 1",
                "  Circle => 2",
                "  Dot => 3",
                "  Rect(_, _) => 4",
                "}",
            ],
        ),
        bad(
            "wrong-argument-count",
            "^Circle(a, b)",
            SHAPE,
            &[
                "match s {",
                "  Circle(_) => 1",
                "  Circle(a, b) => 2",
                "  Dot => 3",
                "  Rect(_, _) => 4",
                "}",
            ],
        ),
    ]);
}

#[test]
fn a_pattern_of_another_enum_is_found_by_the_bare_enum_name() {
    rejects(&[typed(
        (MISMATCH, "^Exit =>"),
        ("Shape", "Effect"),
        SHAPE,
        &["match s {", "  Exit => 1", "  Dot => 2", "}"],
    )]);
}

#[test]
fn an_effect_is_not_matched() {
    rejects(&[typed(
        (MISMATCH, "match ^e {"),
        ("Int, String, Bool or an enum", "Effect<Shape>"),
        "f(e: Effect<Shape>) -> Int",
        &["match e {", "  _ => 1", "}"],
    )]);
}

#[test]
fn a_missing_arm_names_up_to_five_variants() {
    let eight = "enum Eight { A, B, C(x: Int, y: Int), D, E, F, G, H }";
    let lines = |arms: &[&str]| {
        let head = ["fn f(e: Eight) -> Int {", "  match e {"];
        full(&[&head[..], arms, &["  }", "}", eight]].concat())
    };
    let seven = lines(&["    A => 1"]);
    let five = lines(&["    A | G | H => 1"]);
    says(&[
        (
            "missing-arm",
            "^match e",
            "no arm for variants B, C, D, E, F and 2 more; \
             add B => ..., C(_, _) => ..., D => ..., E => ..., F => ... and 2 more",
            seven,
        ),
        (
            "missing-arm",
            "^match e",
            "no arm for variants B, C, D, E, F; \
             add B => ..., C(_, _) => ..., D => ..., E => ... and F => ...",
            five,
        ),
    ]);
    says(&[say(
        ("missing-arm", "^match o"),
        "no arm for variant Some; add Some(_) => ...",
        OPTION,
        &["match o {", "  None => 0", "}"],
    )]);
}

#[test]
fn a_pattern_that_does_not_fit_comes_before_a_missing_arm() {
    rejects(&[
        typed(
            (MISMATCH, "^\"a\" =>"),
            ("Int", "String"),
            INT,
            &["match n {", "  0 => 1", "  \"a\" => 2", "}"],
        ),
        typed(
            (MISMATCH, "^1 => 2"),
            ("Shape", "Int"),
            SHAPE,
            &["match s {", "  Dot => 1", "  1 => 2", "}"],
        ),
    ]);
}

#[test]
fn a_wildcard_on_an_enum_is_never_a_missing_arm() {
    rejects(&[bad(
        "wildcard-on-enum",
        "^_ => 4",
        SHAPE,
        &[
            "match s {",
            "  Dot => 1",
            "  Circle(_) => 2",
            "  Rect(_, _) => 3",
            "  _ => 4",
            "}",
        ],
    )]);
}

#[test]
fn an_arm_that_gives_no_value_does_not_name_the_type() {
    says(&[(
        "missing-value",
        "0 => ^t = 1",
        "this arm must give a value",
        unit(&[
            "let n = 1",
            "var t = 0",
            "let x: Int = match n {",
            "  0 => t = 1",
            "  _ => 2",
            "}",
            "_ = x",
        ]),
    )]);
}
