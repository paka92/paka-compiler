//! Built-in rows: letters bound from the arguments, then from the position; `cannot-infer` when
//! neither fixes one; holes as later arguments; the type map after binding (M2-22).

use super::pattern::*;
use super::*;

/// Every type each definition of `src` holds, shown; a letter still open is marked with `!`.
fn held(src: &str) -> Vec<Vec<String>> {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = check("main.paka", &program, &resolved, &mut table);
    let open = |t: Ty| {
        let mut stack = vec![t];
        while let Some(t) = stack.pop() {
            match table.types.node(t) {
                Node::Var(c) if *c != '_' => return true,
                Node::Name(_, args) => stack.extend(args.iter()),
                Node::Fn(params, result) => stack.extend(params.iter().chain(result)),
                _ => {}
            }
        }
        false
    };
    let show = |(k, &t): (&(Pos, usize), &Ty)| {
        let mark = if open(t) { "!" } else { "" };
        format!("{k:?} {mark}{}", table.types.show(t))
    };
    (bodies.types.iter())
        .map(|types| types.iter().map(show).collect())
        .collect()
}

/// Accepted, and every expression holds a type with no open letter.
fn settled(programs: &[String]) {
    accepts(programs);
    for src in programs {
        let open: Vec<_> = held(src)
            .concat()
            .into_iter()
            .filter(|t| t.contains('!'))
            .collect();
        assert!(open.is_empty(), "{}open: {open:?}", head(src));
    }
}

fn holds(src: &str, mark: &str, len: usize, want: &str) {
    let pos = at(src, mark).unwrap_or_else(|| panic!("no {mark} in\n{src}"));
    assert_eq!(type_at(src, pos, len).as_deref(), Some(want), "{mark}");
}

#[test]
fn accepts_every_row() {
    settled(&[
        unit(&[
            "let s = \"a b\"",
            "_ = len(s)",
            "_ = len([1])",
            "_ = len([\"a\": 1])",
            "_ = is_empty(s)",
            "_ = is_empty([1])",
            "_ = is_empty([\"a\": 1])",
            "_ = contains(s, \"a\")",
            "_ = contains([1], 1)",
            "_ = find(s, \"a\")",
            "_ = find([1], 1)",
            "_ = min(1, 2)",
            "_ = min(1.0, 2.0)",
            "_ = min(\"a\", \"b\")",
            "_ = max(1, 2)",
            "_ = max(1.0, 2.0)",
            "_ = max(\"a\", \"b\")",
            "_ = abs(-1)",
            "_ = abs(-1.5)",
        ]),
        unit(&[
            "_ = to_float(1)",
            "_ = to_int(1.5)",
            "_ = sqrt(2.0)",
            "_ = floor(1.5)",
            "_ = ceil(1.5)",
            "_ = trunc(1.5)",
            "_ = round(1.5)",
            "_ = fixed(1.5, digits: 2)",
            "_ = wrap_add(1, 2)",
            "_ = wrap_sub(1, 2)",
            "_ = wrap_mul(1, 2)",
            "_ = bit_and(1, 2)",
            "_ = bit_or(1, 2)",
            "_ = bit_xor(1, 2)",
            "_ = bit_not(1)",
            "_ = shift_left(1, 2)",
            "_ = shift_right(1, 2)",
        ]),
        unit(&[
            "let s = \"a b\"",
            "_ = parse_int(s)",
            "_ = parse_float(s)",
            "_ = code_at(s, 0)",
            "_ = from_code(65)",
            "_ = split(s, \" \")",
            "_ = chars(s)",
            "_ = lines(s)",
            "_ = words(s)",
            "_ = join([\"a\"], \",\")",
            "_ = repeat(s, 2)",
            "_ = pad_left(s, 3)",
            "_ = pad_right(s, 3)",
            "_ = trim(s)",
            "_ = trim_start(s)",
            "_ = trim_end(s)",
            "_ = starts_with(s, \"a\")",
            "_ = ends_with(s, \"b\")",
            "_ = replace(s, \"a\", \"b\")",
            "_ = upper(s)",
            "_ = lower(s)",
        ]),
        unit(&[
            "var xs = [3, 1]",
            "append(&xs, 2)",
            "extend(&xs, [4])",
            "_ = pop(&xs)",
            "insert(&xs, 5, at: 0)",
            "_ = remove_at(&xs, 0)",
            "swap(&xs, 0, 1)",
            "reverse(&xs)",
            "sort(&xs)",
            "var fs = [1.5]",
            "sort(&fs)",
            "var ts = [\"b\"]",
            "sort(&ts)",
            "sort_by(&xs, before)",
            "var grid: List<List<Int>> = list_of([], count: 3)",
            "_ = grid",
        ]),
        unit(&[
            "var m = [\"a\": 1]",
            "_ = get(m, \"a\")",
            "_ = has(m, \"a\")",
            "remove(&m, \"a\")",
            "_ = keys(m)",
            "_ = values(m)",
            "_ = unwrap_or(get(m, \"a\"), 0)",
            "let r: Result<Int, String> = ok_or(parse_int(\"1\"), \"bad\")",
            "_ = r",
            "assert(true)",
            "if false {",
            "  unreachable()",
            "}",
        ]),
        unit(&[
            "let a: Option<Int> = Some(1)",
            "let b: Option<Int> = None",
            "let c: Result<Int, String> = Ok(1)",
            "let d: Result<Int, String> = Err(\"no\")",
            "_ = [a, b]",
            "_ = [c, d]",
        ]),
        full(&[
            "fn got(t: String) -> Shape {",
            "  Dot",
            "}",
            "fn effects() -> List<Effect<Shape>> {",
            "  [Print(\"a\"), ReadFile(\"p\", then: got, or: got), Exit, ExitWith(1)]",
            "}",
            "fn more() -> List<Effect<Shape>> {",
            "  [WriteFile(\"p\", \"t\", then: Dot, or: got)]",
            "}",
        ]),
    ]);
}

#[test]
fn letters_bind_from_the_arguments_then_from_the_position() {
    let programs = [
        unit(&[
            "var adj: List<List<Int>> = list_of([], count: 3)",
            "_ = adj",
        ]),
        unit(&["_ = contains([], 1)"]),
        unit(&["_ = find([], \"a\")"]),
        unit(&["_ = unwrap_or(None, 0)"]),
        unit(&["let x: Int = unwrap_or(None, 0)", "_ = x"]),
        unit(&["let r: Result<Int, String> = ok_or(None, \"e\")", "_ = r"]),
        unit(&["let xs: List<Int> = []", "_ = [] == xs"]),
        unit(&["let xs: List<Int> = []", "_ = xs == []"]),
        unit(&["_ = Some([]) == Some([1])"]),
        unit(&["let a: List<Option<Int>> = [None, None]", "_ = a"]),
        unit(&[
            "let xs: List<List<Int>> = []",
            "_ = list_of([], count: 2) == xs",
        ]),
        unit(&["let o: Option<List<Int>> = Some([])", "_ = o"]),
        unit(&["let x: Option<Int> = unwrap_or(None, None)", "_ = x"]),
        unit(&["let a = [[], [1]]", "_ = a"]),
        unit(&["let n: Int = unreachable()", "_ = n"]),
    ];
    settled(&programs);
    holds(&programs[0], "list_of(^[]", 2, "List<Int>");
    holds(&programs[1], "contains(^[]", 2, "List<Int>");
    holds(&programs[2], "find(^[]", 2, "List<String>");
    holds(&programs[3], "unwrap_or(^None", 4, "Option<Int>");
    holds(&programs[4], "unwrap_or(^None", 4, "Option<Int>");
    holds(&programs[5], "ok_or(^None", 4, "Option<Int>");
    holds(&programs[6], "_ = ^[]", 2, "List<Int>");
    holds(&programs[8], "Some(^[])", 2, "List<Int>");
    holds(&programs[8], "_ = ^Some([])", 8, "Option<List<Int>>");
    holds(&programs[9], "[^None", 4, "Option<Int>");
    holds(&programs[10], "list_of(^[]", 2, "List<Int>");
    holds(&programs[11], "Some(^[])", 2, "List<Int>");
    holds(&programs[12], "unwrap_or(^None", 4, "Option<Option<Int>>");
    holds(&programs[12], "None, ^None", 4, "Option<Int>");
    holds(&programs[13], "[^[]", 2, "List<Int>");
}

#[test]
fn a_hole_after_the_first_argument_takes_the_bound_type() {
    let programs = [
        unit(&["let x: Int = unwrap_or(None, ??a)", "_ = x"]),
        unit(&["let o: Option<Int> = Some(??a)", "_ = o"]),
        unit(&["let r: Result<Int, String> = Ok(??a)", "_ = r"]),
        unit(&["let r: Result<Int, String> = Err(??a)", "_ = r"]),
        unit(&["let r: Result<Int, String> = ok_or(Some(1), ??e)", "_ = r"]),
        unit(&["var xs = [1]", "append(&xs, ??a)"]),
        unit(&["_ = contains([1], ??a)"]),
        unit(&["_ = min(1, ??a)"]),
        unit(&["_ = to_float(??n)"]),
        unit(&["var m = [\"a\": 1]", "_ = get(m, ??k)"]),
    ];
    settled(&programs);
    let holes = [
        "Int", "Int", "Int", "String", "String", "Int", "Int", "Int", "Int", "String",
    ];
    for (src, want) in programs.iter().zip(holes) {
        let pos = at(src, "^??").unwrap();
        assert_eq!(type_at(src, pos, 3).as_deref(), Some(want), "{}", head(src));
    }
}

#[test]
fn a_letter_fixed_by_neither_is_cannot_infer() {
    rejects(&[
        st("cannot-infer", "_ = ^[] == []", None, &["_ = [] == []"]),
        st("cannot-infer", "_ = ^[] != []", None, &["_ = [] != []"]),
        st(
            "cannot-infer",
            "_ = ^None == None",
            None,
            &["_ = None == None"],
        ),
        st("cannot-infer", "_ = ^Ok(1)", None, &["_ = Ok(1) == Ok(1)"]),
        st(
            "cannot-infer",
            "_ = ^Ok(1)",
            None,
            &["_ = Ok(1) == Err(\"a\")"],
        ),
        st("cannot-infer", "len(^[])", None, &["_ = len([])"]),
        st("cannot-infer", "len(^[:])", None, &["_ = len([:])"]),
        st("cannot-infer", "is_empty(^[])", None, &["_ = is_empty([])"]),
        st(
            "cannot-infer",
            "is_empty(^[:])",
            None,
            &["_ = is_empty([:])"],
        ),
        st("cannot-infer", "_ = ^[]", None, &["_ = []"]),
        st("cannot-infer", "_ = ^[:]", None, &["_ = [:]"]),
        st("cannot-infer", "_ = ^None", None, &["_ = None"]),
        st("cannot-infer", "Some(^[])", None, &["_ = Some([])"]),
        st(
            "cannot-infer",
            "list_of(^None",
            None,
            &["_ = list_of(None, count: 2)"],
        ),
        st(
            "cannot-infer",
            "list_of(^[]",
            None,
            &["_ = len(list_of([], count: 2))"],
        ),
        st(
            "cannot-infer",
            "unwrap_or(^None",
            None,
            &["_ = unwrap_or(None, None)"],
        ),
        st(
            "cannot-infer",
            "unwrap_or(None, ^??a",
            None,
            &["_ = unwrap_or(None, ??a)"],
        ),
        st(
            "cannot-infer",
            "contains([], ^??a",
            None,
            &["_ = contains([], ??a)"],
        ),
        st(
            "cannot-infer",
            "{ ^[] }",
            None,
            &["_ = if true { [] } else { [] }"],
        ),
        st(
            "cannot-infer",
            "in ^[]",
            None,
            &["for x in [] {", "  _ = x", "}"],
        ),
        st(
            "cannot-infer",
            "list_of(^[]",
            None,
            &["let g = list_of([], count: 3)"],
        ),
        st("cannot-infer", "Some(^None)", None, &["let x = Some(None)"]),
        st("cannot-infer", "len(^[])", None, &["let n = len([])"]),
        st("cannot-infer", "^Err", None, &["let e = Err(\"no\")"]),
        st(
            "cannot-infer",
            "^unreachable",
            None,
            &["let x = unreachable()"],
        ),
    ]);
}

#[test]
fn a_hole_that_picks_or_binds_a_row_is_cannot_infer() {
    rejects(&[
        st(
            "cannot-infer",
            "contains(^??xs",
            None,
            &["_ = contains(??xs, 1)"],
        ),
        st("cannot-infer", "min(^??a", None, &["_ = min(??a, 1)"]),
        st(
            "cannot-infer",
            "list_of(^??a",
            None,
            &["_ = list_of(??a, count: 2)"],
        ),
        st("cannot-infer", "Some(^??a)", None, &["let o = Some(??a)"]),
        st("cannot-infer", "Some(^??n)", None, &["_ = Some(??n)"]),
        st("cannot-infer", "Ok(^??n)", None, &["_ = Ok(??n)"]),
        st("cannot-infer", "^??e)", None, &["_ = ok_or(Some(1), ??e)"]),
    ]);
}

#[test]
fn a_bound_letter_holds_against_later_arguments() {
    rejects(&[
        st(
            MISMATCH,
            "[1], ^\"a\"",
            Some(("Int", "String")),
            &["_ = contains([1], \"a\")"],
        ),
        st(
            MISMATCH,
            "list_of(^[]",
            Some(("Int", "List<T>")),
            &["let xs: List<Int> = list_of([], count: 3)"],
        ),
        st(
            MISMATCH,
            "None, ^None",
            Some(("Int", "Option<T>")),
            &["let x: Int = unwrap_or(None, None)"],
        ),
        st(
            MISMATCH,
            "m, ^1",
            Some(("String", "Int")),
            &["var m: Map<String, Int> = [:]", "_ = get(m, 1)"],
        ),
        st(
            MISMATCH,
            "Some(1), ^\"a\"",
            Some(("Int", "String")),
            &["_ = unwrap_or(Some(1), \"a\")"],
        ),
        st(
            MISMATCH,
            "sort(^&ps",
            Some(("List<Int>, List<Float> or List<String>", "List<Counter>")),
            &["var ps = [Counter { name: \"a\" }]", "sort(&ps)"],
        ),
    ]);
}

#[test]
fn pins_each_cannot_infer_text() {
    says(&[
        (
            "cannot-infer",
            "_ = ^[] ==",
            "cannot infer the type of []; write it: let x: List<T> = ...",
            unit(&["_ = [] == []"]),
        ),
        (
            "cannot-infer",
            "_ = ^None ==",
            "cannot infer the type of None; write it: let x: Option<T> = ...",
            unit(&["_ = None == None"]),
        ),
        (
            "cannot-infer",
            "len(^[:])",
            "cannot infer the type of [:]; write it: let x: Map<K, V> = ...",
            unit(&["_ = len([:])"]),
        ),
        (
            "cannot-infer",
            "_ = ^Ok(1)",
            "cannot infer the type of Ok; write it: let x: Result<Int, E> = ...",
            unit(&["_ = Ok(1) == Ok(1)"]),
        ),
        (
            "cannot-infer",
            "len(^[])",
            "cannot infer the type of []; write it: let x: List<T> = ...",
            unit(&["let n = len([])"]),
        ),
        (
            "cannot-infer",
            "list_of(^[]",
            "cannot infer the type of g; write it: let g: List<List<T>> = ...",
            unit(&["let g = list_of([], count: 3)"]),
        ),
        (
            "cannot-infer",
            "Some(^None)",
            "cannot infer the type of x; write it: let x: Option<Option<T>> = ...",
            unit(&["let x = Some(None)"]),
        ),
        (
            "cannot-infer",
            "= ^[:]",
            "cannot infer the type of m; write it: var m: Map<K, V> = ...",
            unit(&["var m = [:]"]),
        ),
        (
            "cannot-infer",
            "^Err",
            "cannot infer the type of e; write it: let e: Result<T, String> = ...",
            unit(&["let e = Err(\"no\")"]),
        ),
        (
            "cannot-infer",
            "Some(^??a)",
            "cannot infer the type of ??a; write it: let a: T = ??a",
            unit(&["let o = Some(??a)"]),
        ),
        (
            "cannot-infer",
            "^??e)",
            "cannot infer the type of ??e; write it: let e: T = ??e",
            unit(&["_ = ok_or(Some(1), ??e)"]),
        ),
    ]);
}

#[test]
fn sort_on_an_item_without_order_offers_sort_by() {
    says(&[
        (
            "type-mismatch",
            "sort(^&xs",
            "Bool has no order; write sort_by(&xs, before) with fn before(a: Bool, b: Bool) -> Bool",
            unit(&["var xs = [true]", "sort(&xs)"]),
        ),
        (
            "type-mismatch",
            "sort(^&ps",
            "Counter has no order; write sort_by(&ps, before) with fn before(a: Counter, b: Counter) -> Bool",
            unit(&["var ps = [Counter { name: \"a\" }]", "sort(&ps)"]),
        ),
    ]);
}

#[test]
fn a_body_is_checked_the_same_whatever_its_callers() {
    let bodies: [&[&str]; 4] = [
        &[
            "fn f(xs: List<Int>) -> Int {",
            "  let n = unwrap_or(find(xs, 3), 0)",
            "  n + len(xs)",
            "}",
        ],
        &[
            "fn f(xs: List<Int>) -> Int {",
            "  let ys = []",
            "  len(xs) + len(ys)",
            "}",
        ],
        &[
            "fn f(xs: List<Int>) -> Int {",
            "  let a: List<List<Int>> = unwrap_or(None, [[], [:]])",
            "  len(xs) + len(a)",
            "}",
        ],
        &[
            "fn f(xs: List<Int>) -> Int {",
            "  let m = unwrap_or(None, ??a).count",
            "  len(xs) + m",
            "}",
        ],
    ];
    let callers: &[&str] = &[
        "fn g() -> Int {",
        "  f([])",
        "}",
        "fn h() -> Int {",
        "  f(list_of(1, count: 2))",
        "}",
        "fn k() -> Int {",
        "  ??n",
        "}",
    ];
    for body in bodies {
        let alone = full(body);
        let called = full(&[callers, body].concat());
        let first = |src: &str, i: usize| {
            let shown = |d: &Diagnostic| (d.code, d.position.map(|p| p.1), d.message.clone());
            run(src).defs[i].as_ref().map(shown)
        };
        assert_eq!(first(&alone, 0), first(&called, 3), "{}", head(&called));
        let columns = |src: &str, i: usize| {
            let shown = held(src).swap_remove(i);
            shown
                .into_iter()
                .map(|t| t.split_once(", ").map(|(_, r)| r.to_string()))
        };
        assert!(
            columns(&alone, 0).eq(columns(&called, 3)),
            "{}",
            head(&called)
        );
    }
    assert!(run(&full(&[callers, bodies[1]].concat())).defs[3].is_some());
}

#[test]
fn an_open_part_is_checked_against_the_type_it_gets() {
    rejects(&[
        typed(
            (MISMATCH, "[[], ^[:]]"),
            ("List<T>", "Map<K, V>"),
            "f()",
            &[
                "let a: List<List<Int>> = unwrap_or(None, [[], [:]])",
                "_ = a",
            ],
        ),
        typed(
            (MISMATCH, "[[], ^[:]]"),
            ("List<T>", "Map<K, V>"),
            "f()",
            &["let a: Option<List<List<Int>>> = Some([[], [:]])", "_ = a"],
        ),
        typed(
            (MISMATCH, "None, ^[]]"),
            ("Option<Int>", "List<T>"),
            "f()",
            &["let a: List<Option<Int>> = [None, []]", "_ = a"],
        ),
        typed(
            (MISMATCH, "None, ^[]]"),
            ("Option<Int>", "List<T>"),
            "f() -> List<Option<Int>>",
            &["[None, []]"],
        ),
        typed(
            (MISMATCH, "Some([]), ^[:])"),
            ("List<T>", "Map<K, V>"),
            "f()",
            &[
                "var m: Map<String, Int> = [:]",
                "_ = unwrap_or(Some([]), [:]) == m",
            ],
        ),
        typed(
            (MISMATCH, "Some([]), ^[:])"),
            ("List<T>", "Map<K, V>"),
            "f()",
            &["let z = unwrap_or(Some([]), [:])", "_ = z"],
        ),
        typed(
            (MISMATCH, "[[], ^[:]]"),
            ("List<T>", "Map<K, V>"),
            "f()",
            &["_ = [[], [:]]"],
        ),
        typed(
            (MISMATCH, "[Some([]), ^Some([:])]"),
            ("Option<List<T>>", "Option<Map<K, V>>"),
            "f()",
            &["_ = [Some([]), Some([:])]"],
        ),
    ]);
}

#[test]
fn the_first_argument_that_fails_is_the_error() {
    rejects(&[typed(
        (MISMATCH, "list_of(^[]"),
        ("String", "List<T>"),
        "f()",
        &["let g: List<String> = list_of([], count: \"x\")", "_ = g"],
    )]);
}

#[test]
fn an_open_value_that_fixes_no_type_is_cannot_infer_where_it_is_met() {
    let hole = "cannot infer the type of ??a; write it: let a: T = ??a";
    let list = "cannot infer the type of []; write it: let x: List<T> = ...";
    says(&[
        (
            "cannot-infer",
            "or(None, ^??a)",
            hole,
            unit(&["let n: Int = unwrap_or(None, ??a).count", "_ = n"]),
        ),
        (
            "cannot-infer",
            "Int = ^[][0]",
            list,
            unit(&["let n: Int = [][0]", "_ = n"]),
        ),
        (
            "cannot-infer",
            "Int = ^[:][\"a\"]",
            "cannot infer the type of [:]; write it: let x: Map<K, V> = ...",
            unit(&["let x: Int = [:][\"a\"]", "_ = x"]),
        ),
        (
            "cannot-infer",
            "Int = ^unreachable()",
            "cannot infer the type of unreachable; write it: let x: T = ...",
            unit(&["let n: Int = unreachable().count", "_ = n"]),
        ),
        (
            "cannot-infer",
            "-unwrap_or(None, ^??a)",
            hole,
            unit(&["_ = -unwrap_or(None, ??a)"]),
        ),
        (
            "cannot-infer",
            "{unwrap_or(None, ^??a)}",
            hole,
            unit(&["_ = \"{unwrap_or(None, ??a)}\""]),
        ),
        (
            "cannot-infer",
            "match unwrap_or(None, ^??a)",
            hole,
            unit(&["match unwrap_or(None, ??a) {", "  _ => {}", "}"]),
        ),
    ]);
}

#[test]
fn a_bool_operand_and_a_range_side_settle() {
    let programs = [
        unit(&["if !unwrap_or(None, ??a) {", "}"]),
        unit(&[
            "let n = 3",
            "for i in unwrap_or(None, ??lo)..<n {",
            "  _ = i",
            "}",
        ]),
        unit(&["for i in ??a..<??b {", "  _ = i", "}"]),
        unit(&["_ = unwrap_or(Some([1]), [])[0]"]),
        unit(&["_ = len(unwrap_or(None, [1]))"]),
        unit(&["let xs = [1, 2]", "_ = xs[unwrap_or(None, ??a)..<2]"]),
        unit(&["_ = unwrap_or(None, ??a) + 1"]),
    ];
    settled(&programs);
    holds(&programs[0], "unwrap_or(None, ^??a", 3, "Bool");
    holds(&programs[1], "unwrap_or(None, ^??lo", 4, "Int");
    holds(&programs[1], "in ^unwrap_or(None, ??lo)", 21, "Int");
    holds(&programs[3], "^unwrap_or(Some([1]), [])", 24, "List<Int>");
}

#[test]
fn a_letter_met_by_an_unknown_type_is_silent() {
    let bad = "type R { x: Nope, n: Int }";
    for body in [
        "  let w = unwrap_or(None, r.x)",
        "  let l = list_of(r.x, count: 2)",
    ] {
        let src = full(&[bad, "fn f(r: R) {", body, "}"]);
        let found = errors(&src);
        assert!(found.iter().all(|d| d.code == "unknown-type"), "{found:?}");
    }
}

#[test]
fn a_cannot_infer_text_fills_the_letters_that_are_known() {
    says(&[
        (
            "cannot-infer",
            "keys(^[:])",
            "cannot infer the type of [:]; write it: let x: Map<String, V> = ...",
            unit(&["let a: List<String> = keys([:])", "_ = a"]),
        ),
        (
            "cannot-infer",
            "has(^[:]",
            "cannot infer the type of [:]; write it: let x: Map<String, V> = ...",
            unit(&["_ = has([:], \"a\")"]),
        ),
        (
            "cannot-infer",
            "values(^[:])",
            "cannot infer the type of [:]; write it: let x: Map<K, Int> = ...",
            unit(&["let a: List<Int> = values([:])", "_ = a"]),
        ),
        (
            "cannot-infer",
            "[^None, None",
            "cannot infer the type of None; write it: let x: Option<List<T>> = ...",
            unit(&["_ = [None, None, Some([])]"]),
        ),
        (
            "cannot-infer",
            "{ ^[] } else",
            "cannot infer the type of []; write it: let x: List<List<T>> = ...",
            unit(&["_ = if true { [] } else { [[]] }"]),
        ),
        (
            "wrong-label",
            "(None, ^fallback",
            "fallback is bare: write ??a",
            unit(&["_ = unwrap_or(None, fallback: ??a)"]),
        ),
    ]);
}

#[test]
fn a_question_mark_settles_the_error_type_from_the_result() {
    let src = func(
        "g() -> Result<Int, String>",
        &["let x: Int = Ok(1)?", "Ok(x)"],
    );
    accepts(std::slice::from_ref(&src));
    holds(&src, "= ^Ok(1)", 5, "Result<Int, String>");
}
