//! Hand-written programs and the signature table they must give.

use super::*;
use crate::builtins::{Kind, ROWS, name};
use crate::parser::parse;
use crate::resolve::resolve;
use std::collections::BTreeSet;

fn program(lines: &[&str]) -> String {
    lines.join("\n") + "\n"
}

/// The position of the `^` in `pattern`, taken as the text of the `line`th line (from 1).
fn at(src: &str, line: usize, pattern: &str) -> Pos {
    let (before, after) = pattern.split_once('^').unwrap();
    let text = src.lines().nth(line - 1).unwrap();
    let found = text.find(&format!("{before}{after}")).unwrap();
    (line, found + before.len() + 1)
}

/// Builds the table of `src` and hands it, with the program, to `f`.
fn with<T>(src: &str, f: impl FnOnce(&Program, &Table) -> T) -> T {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let table = Table::build("main.paka", &program, &resolved);
    assert_eq!(table.defs.len(), program.defs.len());
    f(&program, &table)
}

fn params(t: &Table, ps: &[Param]) -> String {
    let one = |p: &Param| {
        let inout = if p.inout { "inout " } else { "" };
        format!("{}: {inout}{}", p.name, t.types.show(p.ty))
    };
    let join = |labelled| {
        let some: Vec<_> = ps.iter().filter(|p| p.labelled == labelled).collect();
        some.into_iter().map(one).collect::<Vec<_>>().join(", ")
    };
    match (join(false), join(true)) {
        (pos, l) if l.is_empty() => pos,
        (pos, l) if pos.is_empty() => format!("; {l}"),
        (pos, l) => format!("{pos}; {l}"),
    }
}

/// `name(params) -> result`; a row without parentheses when `bare` and it has no parameters.
fn sig_text(t: &Table, name: &str, sig: &Sig, bare: bool) -> String {
    let head = match (bare, sig.params.is_empty()) {
        (true, true) => name.to_string(),
        _ => format!("{name}({})", params(t, &sig.params)),
    };
    match sig.result {
        Some(r) => format!("{head} -> {}", t.types.show(r)),
        None => head,
    }
}

fn fields(t: &Table, fs: &[Field]) -> String {
    let one = |f: &Field| {
        let default = if f.default { " = _" } else { "" };
        format!("{}: {}{default}", f.name, t.types.show(f.ty))
    };
    fs.iter().map(one).collect::<Vec<_>>().join(", ")
}

/// One line per definition: its signature, or its error as the tool prints it.
fn shown(src: &str) -> Vec<String> {
    with(src, |program, t| {
        let line = |(def, result): (&Def, &Result<Item, Diagnostic>)| {
            let name = def.name().text;
            let item = match result {
                Ok(item) => item,
                Err(d) => return d.to_string(),
            };
            match item {
                Item::Record(fs) if fs.is_empty() => format!("type {name} {{}}"),
                Item::Record(fs) => format!("type {name} {{ {} }}", fields(t, fs)),
                Item::Enum(vs) => {
                    let one = |v: &Variant| match v.fields.is_empty() {
                        true => v.name.to_string(),
                        false => format!("{}({})", v.name, fields(t, &v.fields)),
                    };
                    let vs: Vec<_> = vs.iter().map(one).collect();
                    format!("enum {name} {{ {} }}", vs.join(", "))
                }
                Item::Fn(sig) => format!("fn {}", sig_text(t, name, sig, false)),
                Item::Example => format!("example {name}"),
                Item::App { state, update } => {
                    let state = state.map(|s| format!("state: {}", t.types.show(s)));
                    let update = update.map(|u| format!("update: {}", program.defs[u].name().text));
                    let both: Vec<_> = state.into_iter().chain(update).collect();
                    format!("app {name} {{ {} }}", both.join(", "))
                }
            }
        };
        program.defs.iter().zip(&t.defs).map(line).collect()
    })
}

fn error(lines: &[&str], def: usize) -> (String, Diagnostic) {
    let src = program(lines);
    let found = with(&src, |_, t| t.defs[def].as_ref().err().cloned());
    (src, found.expect("no error"))
}

fn assert_error(lines: &[&str], def: usize, code: &str, line: usize, pattern: &str, message: &str) {
    let (src, d) = error(lines, def);
    let want = (code, at(&src, line, pattern), message);
    assert_eq!(
        (d.code, d.position.unwrap(), d.message.as_str()),
        want,
        "{src}"
    );
}

/// The expected and found types of an error, as the tool prints them.
fn types_of(lines: &[&str], def: usize) -> (String, String) {
    let (src, d) = error(lines, def);
    let detail = d.detailed(src.as_bytes());
    let field = |key: &str| {
        let prefix = format!("  {key}: ");
        let found = detail.lines().find_map(|l| l.strip_prefix(prefix.as_str()));
        found
            .unwrap_or_else(|| panic!("no {key} in {detail}"))
            .to_string()
    };
    (field("expected"), field("found"))
}

fn accepted(lines: &[&str]) {
    let src = program(lines);
    for (i, line) in shown(&src).iter().enumerate() {
        assert!(!line.contains("error["), "definition {i}: {line}\n{src}");
    }
}

fn hashes(lines: &[&str]) -> Vec<Option<u64>> {
    with(&program(lines), |_, t| {
        (0..t.defs.len()).map(|i| t.hash(i)).collect()
    })
}

fn hash(lines: &[&str], def: usize) -> u64 {
    hashes(lines)[def].unwrap_or_else(|| panic!("no hash for definition {def}"))
}

// Signatures of every definition kind

#[test]
fn a_fn_keeps_parameters_marks_labels_and_result() {
    let src = program(&[
        "fn pad(text: String; width: Int) -> String { text }",
        "fn bump(n: inout Int, by: Int) { n += by }",
        "fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }",
        "fn pick(a: fn(), b: fn(String), c: fn(Int, Int) -> Bool) { }",
        "fn none() { }",
    ]);
    assert_eq!(
        shown(&src),
        [
            "fn pad(text: String; width: Int) -> String",
            "fn bump(n: inout Int, by: Int)",
            "fn apply(f: fn(Int) -> Int, x: Int) -> Int",
            "fn pick(a: fn(), b: fn(String), c: fn(Int, Int) -> Bool)",
            "fn none()",
        ]
    );
}

#[test]
fn a_record_keeps_fields_types_and_which_have_a_default() {
    let src = program(&[
        "type Counter { count: Int = 0, name: String, tags: List<String> = [], seen: Map<String, Int> = [:] }",
        "type Empty {}",
        "type Job { done: fn(Int) -> Counter, later: List<fn(String)> }",
    ]);
    assert_eq!(
        shown(&src),
        [
            "type Counter { count: Int = _, name: String, tags: List<String> = _, seen: Map<String, Int> = _ }",
            "type Empty {}",
            "type Job { done: fn(Int) -> Counter, later: List<fn(String)> }",
        ]
    );
}

#[test]
fn an_enum_keeps_variants_and_their_fields() {
    let src = program(&[
        "enum Shape { Circle(radius: Float), Rect(w: Float, h: Float), Dot }",
        "enum Msg { Loaded(text: String), Ticked(at: Option<Int>) }",
    ]);
    assert_eq!(
        shown(&src),
        [
            "enum Shape { Circle(radius: Float), Rect(w: Float, h: Float), Dot }",
            "enum Msg { Loaded(text: String), Ticked(at: Option<Int>) }",
        ]
    );
}

const APP: [&str; 8] = [
    "type Tally { words: Int = 0 }",
    "enum Msg { Start }",
    "fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> { [Exit] }",
    "example first { }",
    "app wc {",
    "  state: Tally",
    "  start: Start",
    "  update: update",
];

#[test]
fn an_example_and_an_app_keep_what_they_name() {
    let mut lines = APP.to_vec();
    lines.extend(["  grants: [Console]", "}"]);
    let src = program(&lines);
    assert_eq!(
        shown(&src),
        [
            "type Tally { words: Int = _ }",
            "enum Msg { Start }",
            "fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>>",
            "example first",
            "app wc { state: Tally, update: update }",
        ]
    );
}

#[test]
fn an_app_without_state_names_only_its_handler() {
    let src = program(&[
        "enum Msg { Start }",
        "fn update(m: Msg) -> List<Effect<Msg>> { [Exit] }",
        "app tool {",
        "  start: Start",
        "  update: update",
        "  grants: []",
        "}",
    ]);
    assert_eq!(shown(&src)[2], "app tool { update: update }");
}

#[test]
fn equal_types_are_one_id() {
    let src =
        program(&["fn f(a: List<Int>, b: List<Int>, c: List<Float>, d: fn(Int), e: fn(Int)) { }"]);
    with(&src, |_, t| {
        let Ok(Item::Fn(sig)) = &t.defs[0] else {
            panic!("not a fn")
        };
        let ids: Vec<_> = sig.params.iter().map(|p| p.ty).collect();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[0], ids[2]);
        assert_eq!(ids[3], ids[4]);
        assert_ne!(ids[0], ids[3]);
    });
}

#[test]
fn a_body_error_leaves_the_signature_in_the_table() {
    let src = program(&["fn f(x: Int) -> Int { nope }", "fn g(x: Int) -> Int { x }"]);
    assert_eq!(shown(&src), ["fn f(x: Int) -> Int", "fn g(x: Int) -> Int"]);
}

// Function types stand anywhere a type stands

#[test]
fn a_function_type_stands_in_every_place_but_a_map_key() {
    accepted(&[
        "enum Msg { Ping(after: fn(String) -> Msg), Wait }",
        "type Job { done: fn(Int) -> Msg, many: List<fn(Int)>, byName: Map<String, fn() -> Int>, maybe: Option<fn(Int)> }",
        "fn make(f: fn(fn(Int) -> Int) -> fn(Int) -> Int) -> fn(Int) -> Int { f }",
        "fn all(fs: List<fn(Int) -> Int>) -> Result<fn(Int), String> { fs }",
    ]);
}

// Bad signatures

#[test]
fn a_type_with_the_wrong_number_of_arguments_is_the_resolvers_error() {
    let two = ["type A { xs: List<Int, Int> }"];
    assert_error(
        &two,
        0,
        "wrong-type-arguments",
        1,
        "xs: ^List",
        "List takes 1 type argument",
    );
    let none = ["fn f(e: Effect) { }"];
    assert_error(
        &none,
        0,
        "wrong-type-arguments",
        1,
        "e: ^Effect",
        "Effect takes 1 type argument",
    );
    let user = ["type P {}", "fn f(p: P<Int>) { }"];
    assert_error(
        &user,
        1,
        "wrong-type-arguments",
        2,
        "p: ^P",
        "P takes 0 type arguments",
    );
}

#[test]
fn an_unknown_type_is_the_resolvers_error_and_stops_that_definition() {
    let lines = ["type A { x: Intt, y: Map<Float, Int> }"];
    assert_error(
        &lines,
        0,
        "unknown-type",
        1,
        "x: ^Intt",
        "Intt is not a type; did you mean Int?",
    );
}

#[test]
fn a_name_error_in_a_default_is_the_resolvers_error() {
    let lines = ["type A { n: Int = missing }"];
    assert_error(
        &lines,
        0,
        "unknown-name",
        1,
        "= ^missing",
        "missing is not defined here",
    );
}

#[test]
fn a_map_key_is_an_int_or_a_string() {
    let message = "a Map key is an Int or a String";
    let field = ["type A { m: Map<Float, Int> }"];
    assert_error(&field, 0, "type-mismatch", 1, "Map<^Float", message);
    assert_eq!(
        types_of(&field, 0),
        ("Int or String".into(), "Float".into())
    );
    let param = ["fn f(m: Map<Bool, Int>) { }"];
    assert_error(&param, 0, "type-mismatch", 1, "Map<^Bool", message);
    let result = ["fn f() -> Map<Option<Int>, Int> { }"];
    assert_error(&result, 0, "type-mismatch", 1, "Map<^Option", message);
    assert_eq!(types_of(&result, 0).1, "Option<Int>");
    let variant = ["enum E { A(m: Map<fn(Int), Int>) }"];
    assert_error(&variant, 0, "type-mismatch", 1, "Map<^fn", message);
    assert_eq!(types_of(&variant, 0).1, "fn(Int)");
    let user = ["type P {}", "type A { m: List<Map<P, Int>> }"];
    assert_error(&user, 1, "type-mismatch", 2, "Map<^P", message);
}

#[test]
fn a_map_key_is_checked_before_the_value_to_its_right() {
    let lines = ["type A { m: Map<Float, Intt> }"];
    assert_error(
        &lines,
        0,
        "unknown-type",
        1,
        "Float, ^Intt",
        "Intt is not a type; did you mean Int?",
    );
}

#[test]
fn a_map_of_ints_or_strings_is_accepted() {
    accepted(&["type A { a: Map<Int, Map<String, Bool>>, b: Map<String, List<Int>> }"]);
}

#[test]
fn an_error_names_its_definition() {
    let (src, d) = error(&["type A { m: Map<Float, Int> }"], 0);
    assert!(d.detailed(src.as_bytes()).contains("  definition: A\n"));
}

// Defaults: the shape of the constant and the type it can have

#[test]
fn a_default_of_the_wrong_type_is_a_type_mismatch() {
    let cases: [(&[&str], &str, &str, &str); 6] = [
        (&["type A { n: Int = \"x\" }"], "= ^\"x\"", "Int", "String"),
        (
            &["type A { xs: List<Int> = 3 }"],
            "= ^3",
            "List<Int>",
            "Int",
        ),
        (&["type A { x: Float = 1 }"], "= ^1", "Float", "Int"),
        (&["type A { b: Bool = [:] }"], "= ^[:]", "Bool", "Map<K, V>"),
        (
            &["enum Color { Red }", "type A { o: Option<Int> = Red }"],
            "= ^Red",
            "Option<Int>",
            "Color",
        ),
        (
            &["enum Color { Red }", "type A { c: Color = None }"],
            "= ^None",
            "Color",
            "Option<T>",
        ),
    ];
    for (lines, pattern, expected, found) in cases {
        let def = lines.len() - 1;
        let (src, d) = error(lines, def);
        let line = lines.len();
        let message = format!(
            "the default of {} is {found}, not {expected}",
            field_name(lines)
        );
        assert_eq!(
            (d.code, d.position.unwrap(), d.message.as_str()),
            ("type-mismatch", at(&src, line, pattern), message.as_str())
        );
        assert_eq!(types_of(lines, def), (expected.into(), found.into()));
    }
}

/// The name of the last field of the last line.
fn field_name<'a>(lines: &[&'a str]) -> &'a str {
    let last = lines[lines.len() - 1];
    let after = last.split_once("{ ").unwrap().1;
    after.split(':').next().unwrap()
}

#[test]
fn the_first_bad_field_is_the_error() {
    let lines = ["type A { a: Int = 1, b: Int = \"x\", c: Int = \"y\" }"];
    assert_error(
        &lines,
        0,
        "type-mismatch",
        1,
        "b: Int = ^\"x\"",
        "the default of b is String, not Int",
    );
}

#[test]
fn a_default_is_a_constant() {
    let message =
        "a default is a constant: a literal, [], [:], a variant or a record built from constants";
    let cases: [(&[&str], &str); 6] = [
        (&["type A { n: Int = len([]) }"], "= ^len"),
        (&["type A { n: Int = 1 + 2 }"], "= ^1 +"),
        (&["type A { s: String = \"a{1}\" }"], "= ^\"a{1}\""),
        (
            &[
                "fn f(n: Int) -> Int { n }",
                "type A { g: fn(Int) -> Int = f }",
            ],
            "= ^f",
        ),
        (
            &["enum E { Blue(x: Int) }", "type A { e: E = Blue }"],
            "= ^Blue",
        ),
        (
            &[
                "fn f(n: Int) -> Int { n }",
                "type A { xs: List<Int> = [f(1)] }",
            ],
            "[^f(1)]",
        ),
    ];
    for (lines, pattern) in cases {
        let line = lines.len();
        assert_error(
            lines,
            lines.len() - 1,
            "unexpected-token",
            line,
            pattern,
            message,
        );
    }
}

#[test]
fn constants_of_every_form_are_accepted() {
    accepted(&[
        "enum Color { Red, Blue(shade: Int) }",
        "type P { x: Int = 0 }",
        "type A {",
        "  a: Int = -1, b: Float = -1.5, c: String = \"hi\", d: Bool = true,",
        "  e: List<Int> = [], f: Map<String, Int> = [:], g: List<Int> = [1, 2], h: Map<String, Int> = [\"a\": 1],",
        "  i: Color = Red, j: Color = Blue(2), k: Option<Int> = None, l: Option<Int> = Some(1),",
        "  m: P = P { x: 1 }, n: Result<Int, String> = Ok(1), o: Effect<Color> = Exit, p: Float = 1.5",
        "}",
    ]);
}

#[test]
fn what_a_constant_holds_is_left_to_the_expression_checker() {
    accepted(&["type A { xs: List<Int> = [\"a\"], o: Option<Int> = Some(\"a\") }"]);
}

// Recursive types

const SELF_NODE: &str = "Node holds itself through next with nothing between; write next: Option<Node> or put it in a List or an enum variant";

#[test]
fn a_type_that_holds_itself_by_value_has_no_finite_value() {
    assert_error(
        &["type Node { next: Node }"],
        0,
        "no-finite-value",
        1,
        "type ^Node",
        SELF_NODE,
    );
    let chain = "Chain holds itself through next with nothing between; write next: Option<Chain> or put it in a List or an enum variant";
    assert_error(
        &["enum Chain { Link(next: Chain) }"],
        0,
        "no-finite-value",
        1,
        "enum ^Chain",
        chain,
    );
}

#[test]
fn a_cycle_of_several_types_is_reported_at_each_member() {
    let lines = ["type A { b: B }", "type B { c: C }", "type C { a: A }"];
    let through = |t, f, w| {
        format!(
            "{t} holds itself through {f} with nothing between; write {f}: Option<{w}> or put it in a List or an enum variant"
        )
    };
    assert_error(
        &lines,
        0,
        "no-finite-value",
        1,
        "type ^A",
        &through("A", "b", "B"),
    );
    assert_error(
        &lines,
        1,
        "no-finite-value",
        2,
        "type ^B",
        &through("B", "c", "C"),
    );
    assert_error(
        &lines,
        2,
        "no-finite-value",
        3,
        "type ^C",
        &through("C", "a", "A"),
    );
}

#[test]
fn the_field_named_is_the_first_that_leads_back() {
    let lines = ["type Ok1 {}", "type Node { a: Ok1, b: Node, c: Node }"];
    let message = "Node holds itself through b with nothing between; write b: Option<Node> or put it in a List or an enum variant";
    assert_error(&lines, 1, "no-finite-value", 2, "type ^Node", message);
    let both = ["enum E { X(a: E), Y(b: E) }"];
    let message = "E holds itself through a with nothing between; write a: Option<E> or put it in a List or an enum variant";
    assert_error(&both, 0, "no-finite-value", 1, "enum ^E", message);
}

#[test]
fn a_result_is_grounded_when_one_side_is() {
    let both = ["type Tree { r: Result<Tree, Tree> }"];
    let message = "Tree holds itself through r with nothing between; write r: Option<Tree> or put it in a List or an enum variant";
    assert_error(&both, 0, "no-finite-value", 1, "type ^Tree", message);
    accepted(&[
        "type Tree { r: Result<Tree, Int> }",
        "type Leaf { r: Result<Int, Leaf> }",
    ]);
}

#[test]
fn a_type_may_hold_itself_where_a_finite_value_exists() {
    accepted(&["type Node { next: Option<Node> = None }"]);
    accepted(&["type Node { kids: List<Node> = [] }"]);
    accepted(&["type Node { kids: Map<String, Node> = [:] }"]);
    accepted(&["type Node { f: fn(Node) -> Node }"]);
    accepted(&["type Node { e: Effect<Node> }"]);
    accepted(&["enum Chain { End, Link(next: Chain) }"]);
    accepted(&["enum Expr { Num(value: Int), Add(left: Expr, right: Expr) }"]);
    accepted(&[
        "enum Chain { End, Link(next: Chain) }",
        "type Wrap { c: Chain }",
    ]);
    accepted(&["type Empty {}", "type Holder { e: Empty }"]);
}

#[test]
fn a_type_that_only_holds_a_bad_cycle_is_not_itself_in_error() {
    let lines = [
        "enum Chain { Link(next: Chain) }",
        "type Holder { c: Chain }",
        "fn use(h: Holder, c: Chain) { }",
    ];
    let shown = shown(&program(&lines));
    assert!(shown[0].contains("error[no-finite-value]"), "{shown:?}");
    assert_eq!(
        shown[1..],
        ["type Holder { c: Chain }", "fn use(h: Holder, c: Chain)"]
    );
}

#[test]
fn a_cycle_is_found_with_the_other_definitions_unchanged() {
    let lines = [
        "type A { b: B }",
        "type B { a: A }",
        "type C { a: A, n: Int = 0 }",
    ];
    let shown = shown(&program(&lines));
    assert!(
        shown[0].contains("error[no-finite-value]") && shown[1].contains("error[no-finite-value]")
    );
    assert_eq!(shown[2], "type C { a: A, n: Int = _ }");
}

// An app's state holds no function type

const LATER: &str = "state Later holds a function type in field after; keep a message value instead and match on it in update";

fn app(types: &[&str], state: &str) -> Vec<String> {
    let mut lines = vec!["enum Msg { Start }"];
    lines.extend(types);
    lines.extend([
        "fn update(s: inout Later, m: Msg) -> List<Effect<Msg>> { [Exit] }",
        "app wc {",
    ]);
    let state = format!("  state: {state}");
    lines.extend([
        state.as_str(),
        "  start: Start",
        "  update: update",
        "  grants: []",
        "}",
    ]);
    lines.into_iter().map(String::from).collect()
}

fn state_error(types: &[&str], state: &str, message: &str) {
    let lines = app(types, state);
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let def = lines
        .iter()
        .filter(|l| l.starts_with("type ") || l.starts_with("enum ") || l.starts_with("fn "))
        .count();
    let line = lines.iter().position(|l| l.contains("state:")).unwrap() + 1;
    assert_error(
        &lines,
        def,
        "type-mismatch",
        line,
        &format!("state: ^{state}"),
        message,
    );
}

#[test]
fn a_state_that_holds_a_function_type_is_refused() {
    state_error(&["type Later { after: fn(String) -> Msg }"], "Later", LATER);
}

#[test]
fn a_function_type_is_found_through_every_holder() {
    let later = |field: &str| {
        format!(
            "state Later holds a function type in field {field}; keep a message value instead and match on it in update"
        )
    };
    state_error(
        &["type Later { after: List<fn(Int)> }"],
        "Later",
        &later("after"),
    );
    state_error(
        &["type Later { after: Option<fn(Int)> }"],
        "Later",
        &later("after"),
    );
    state_error(
        &["type Later { after: Map<String, fn(Int)> }"],
        "Later",
        &later("after"),
    );
    state_error(
        &["type Later { after: Result<Int, List<fn(Int)>> }"],
        "Later",
        &later("after"),
    );
    state_error(
        &["type Later { job: Job }", "type Job { done: fn(Int) }"],
        "Later",
        &later("done"),
    );
    state_error(
        &[
            "type Later { mode: Mode }",
            "enum Mode { Idle, Wait(then: fn(Int) -> Mode) }",
        ],
        "Later",
        &later("then"),
    );
    state_error(
        &[
            "type Later { first: Later2 }",
            "type Later2 { back: List<Later>, f: fn() }",
        ],
        "Later",
        &later("f"),
    );
}

#[test]
fn a_function_type_outside_the_state_is_fine() {
    let types = [
        "type Later { after: fn(String) -> Msg }",
        "type Tally { words: Int = 0, kids: List<Tally> = [] }",
    ];
    let lines = app(&types, "Tally");
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let shown = shown(&program(&lines));
    assert_eq!(
        shown.last().unwrap(),
        "app wc { state: Tally, update: update }"
    );
}

#[test]
fn a_state_that_is_a_bad_definition_adds_no_second_error() {
    let lines = app(&["type Later { m: Map<Float, fn()> }"], "Later");
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let shown = shown(&program(&lines));
    assert!(shown[2].contains("error[type-mismatch]"), "{shown:?}");
    assert_eq!(
        shown.last().unwrap(),
        "app wc { state: Later, update: update }"
    );
}

// One definition's error never changes another's result

#[test]
fn an_error_in_one_definition_changes_no_other_result() {
    let bad = |field: &str| {
        program(&[
            "type Good { x: Int = 0 }",
            &format!("type Bad {{ m: {field} }}"),
            "fn uses(b: Bad, g: Good) -> Good { g }",
            "fn broken(x: Intt) { }",
            "enum Loop { Link(next: Loop) }",
            "fn after(l: Loop) -> Good { Good { } }",
        ])
    };
    let one = shown(&bad("Map<Float, Int>"));
    let other = shown(&bad("Map<Bool, Int>"));
    for i in [0, 2, 5] {
        assert_eq!(one[i], other[i]);
        assert!(!one[i].contains("error["), "{}", one[i]);
    }
    assert_eq!(one[2], "fn uses(b: Bad, g: Good) -> Good");
    for i in [1, 3, 4] {
        assert!(one[i].contains("error["), "{}", one[i]);
    }
    let hashes = |field: &str| {
        let src = bad(field);
        with(&src, |_, t| [0, 2, 5].map(|i| t.hash(i)))
    };
    assert_eq!(hashes("Map<Float, Int>"), hashes("Map<Bool, Int>"));
    assert!(hashes("Map<Float, Int>").iter().all(Option::is_some));
}

#[test]
fn a_rejected_definition_has_no_hash() {
    let lines = [
        "type Bad { m: Map<Float, Int> }",
        "example e { }",
        "type Good {}",
    ];
    let h = hashes(&lines);
    assert_eq!((h[0], h[1]), (None, None));
    assert!(h[2].is_some());
}

// The interface hash

#[test]
fn the_hash_is_fnv_one_a() {
    assert_eq!(hash::fnv(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(hash::fnv("a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn a_fn_hash_ignores_the_body_the_name_and_positional_names() {
    let base = hash(&["fn f(a: Int) -> Int { a }"], 0);
    assert_eq!(base, hash(&["fn f(b: Int) -> Int { 0 }"], 0));
    assert_eq!(
        base,
        hash(&["fn g(a: Int) -> Int {", "  let x = a", "  x", "}"], 0)
    );
}

#[test]
fn a_fn_hash_covers_types_marks_labels_order_and_result() {
    let base = hash(&["fn f(a: Int, w: Float; n: Int) -> Int { a }"], 0);
    let others = [
        "fn f(a: Float, w: Float; n: Int) -> Int { a }",
        "fn f(a: inout Int, w: Float; n: Int) -> Int { a }",
        "fn f(w: Float, a: Int; n: Int) -> Int { a }",
        "fn f(a: Int; w: Float, n: Int) -> Int { a }",
        "fn f(a: Int, w: Float; m: Int) -> Int { a }",
        "fn f(a: Int, w: Float; n: Int) -> Float { a }",
        "fn f(a: Int, w: Float; n: Int) { a }",
        "fn f(a: Int, w: Float) -> Int { a }",
    ];
    for other in others {
        assert_ne!(base, hash(&[other], 0), "{other}");
    }
}

// Stack safety

#[test]
fn a_very_long_chain_of_types_is_walked_by_loops() {
    let n = 20_000;
    let mut src = String::new();
    for i in 0..n - 1 {
        src += &format!("type T{i} {{ next: T{} }}\n", i + 1);
    }
    src += &format!("type T{} {{}}\nfn f(t: T0) {{ }}\n", n - 1);
    with(&src, |_, t| {
        assert!(t.defs.iter().all(Result::is_ok));
        assert!(t.hash(0).is_some() && t.hash(0) != t.hash(1));
        assert!(t.hash(n).is_some());
    });
}

#[test]
fn a_very_long_cycle_of_types_is_walked_by_loops() {
    let n = 20_000;
    let mut src = String::new();
    for i in 0..n {
        src += &format!("type T{i} {{ next: Option<T{}> }}\n", (i + 1) % n);
    }
    with(&src, |_, t| {
        assert!(t.defs.iter().all(Result::is_ok));
        assert!(t.hash(0) != t.hash(1));
    });
    let mut bad = String::new();
    for i in 0..n {
        bad += &format!("type T{i} {{ next: T{} }}\n", (i + 1) % n);
    }
    with(&bad, |_, t| assert!(t.defs.iter().all(Result::is_err)));
}

#[test]
fn deeply_nested_types_are_shown_and_walked() {
    let depth = 60;
    let ty = format!("{}Int{}", "Option<".repeat(depth), ">".repeat(depth));
    let src = program(&[&format!("fn f(a: {ty}) {{ }}")]);
    assert_eq!(shown(&src), [format!("fn f(a: {ty})")]);
}

// The built-in table

fn rows(t: &Table) -> Vec<String> {
    let fns = t
        .fns
        .iter()
        .flat_map(|(n, ss)| ss.iter().map(move |s| sig_text(t, n, s, false)));
    let variants = t.variants.iter().map(|(n, s)| sig_text(t, n, s, true));
    fns.chain(variants).collect()
}

fn built_in_rows() -> Vec<String> {
    with("", |_, t| rows(t))
}

#[test]
fn the_table_holds_63_functions_in_76_rows_and_9_variants() {
    with("", |_, t| {
        assert_eq!(t.fns.len(), 63);
        assert_eq!(t.fns.values().map(Vec::len).sum::<usize>(), 76);
        assert_eq!(t.variants.len(), 9);
    });
}

#[test]
fn the_rows_are_the_signatures_of_the_decision() {
    let have = built_in_rows();
    let want = [
        "len(x: String) -> Int",
        "len(xs: List<T>) -> Int",
        "len(m: Map<K, V>) -> Int",
        "is_empty(m: Map<K, V>) -> Bool",
        "contains(text: String, part: String) -> Bool",
        "contains(xs: List<T>, item: T) -> Bool",
        "find(text: String, part: String) -> Option<Int>",
        "find(xs: List<T>, item: T) -> Option<Int>",
        "min(a: Int, b: Int) -> Int",
        "min(a: Float, b: Float) -> Float",
        "max(a: String, b: String) -> String",
        "abs(x: Int) -> Int",
        "abs(x: Float) -> Float",
        "to_float(n: Int) -> Float",
        "to_int(x: Float) -> Int",
        "round(x: Float) -> Float",
        "fixed(x: Float; digits: Int) -> String",
        "wrap_mul(a: Int, b: Int) -> Int",
        "bit_not(a: Int) -> Int",
        "shift_left(a: Int, count: Int) -> Int",
        "parse_int(text: String) -> Option<Int>",
        "parse_float(text: String) -> Option<Float>",
        "code_at(text: String, i: Int) -> Int",
        "from_code(n: Int) -> String",
        "split(text: String, sep: String) -> List<String>",
        "join(parts: List<String>, sep: String) -> String",
        "repeat(text: String, n: Int) -> String",
        "pad_left(text: String, width: Int) -> String",
        "replace(text: String, old: String, new: String) -> String",
        "append(xs: inout List<T>, item: T)",
        "extend(xs: inout List<T>, more: List<T>)",
        "pop(xs: inout List<T>) -> T",
        "insert(xs: inout List<T>, item: T; at: Int)",
        "remove_at(xs: inout List<T>, i: Int) -> T",
        "swap(xs: inout List<T>, i: Int, j: Int)",
        "reverse(xs: inout List<T>)",
        "sort(xs: inout List<Int>)",
        "sort(xs: inout List<Float>)",
        "sort(xs: inout List<String>)",
        "sort_by(xs: inout List<T>, before: fn(T, T) -> Bool)",
        "list_of(item: T; count: Int) -> List<T>",
        "get(m: Map<K, V>, key: K) -> Option<V>",
        "has(m: Map<K, V>, key: K) -> Bool",
        "remove(m: inout Map<K, V>, key: K)",
        "keys(m: Map<K, V>) -> List<K>",
        "values(m: Map<K, V>) -> List<V>",
        "unwrap_or(opt: Option<T>, fallback: T) -> T",
        "ok_or(opt: Option<T>, error: E) -> Result<T, E>",
        "assert(cond: Bool)",
        "unreachable() -> T",
        "Some(value: T) -> Option<T>",
        "None -> Option<T>",
        "Ok(value: T) -> Result<T, E>",
        "Err(error: E) -> Result<T, E>",
        "Print(text: String) -> Effect<M>",
        "ReadFile(path: String; then: fn(String) -> M, or: fn(String) -> M) -> Effect<M>",
        "WriteFile(path: String, text: String; then: M, or: fn(String) -> M) -> Effect<M>",
        "Exit -> Effect<M>",
        "ExitWith(status: Int) -> Effect<M>",
    ];
    for row in want {
        assert!(have.iter().any(|h| h == row), "missing {row}");
    }
}

#[test]
fn the_rows_follow_the_rules_of_the_decision() {
    with("", |_, t| {
        for (name, sigs) in &t.fns {
            for sig in sigs {
                let inout: Vec<_> = sig.params.iter().map(|p| p.inout).collect();
                assert!(
                    inout.iter().filter(|&&i| i).count() <= 1,
                    "{name}: two inout"
                );
                assert!(
                    !inout.iter().skip(1).any(|&i| i),
                    "{name}: inout is not first"
                );
                let labelled = sig.params.iter().any(|p| p.labelled);
                assert_eq!(
                    labelled,
                    ["fixed", "insert", "list_of"].contains(name),
                    "{name}"
                );
                let first_label = sig.params.iter().position(|p| p.labelled);
                let last_bare = sig.params.iter().rposition(|p| !p.labelled);
                assert!(
                    first_label.zip(last_bare).is_none_or(|(l, b)| b < l),
                    "{name}"
                );
            }
        }
        let growing = [
            "append",
            "extend",
            "pop",
            "insert",
            "remove_at",
            "swap",
            "reverse",
            "sort",
            "sort_by",
            "remove",
        ];
        for name in growing {
            assert!(t.fns[name].iter().all(|s| s.params[0].inout), "{name}");
        }
        let no_result = [
            "append", "extend", "insert", "swap", "reverse", "sort", "sort_by", "remove", "assert",
        ];
        for (name, sigs) in &t.fns {
            let gives = sigs.iter().all(|s| s.result.is_some());
            assert_eq!(gives, !no_result.contains(name), "{name}");
        }
    });
}

#[test]
fn the_names_of_the_table_are_the_names_the_resolver_binds() {
    let mut table = BTreeSet::new();
    with("", |_, t| {
        table.extend(t.fns.keys().copied());
        table.extend(t.variants.keys().copied());
    });
    let named = |kind| ROWS.iter().filter(move |r| r.0 == kind).map(|r| name(r.1));
    let mut want: BTreeSet<_> = named(Kind::Fn).chain(named(Kind::Variant)).collect();
    assert_eq!(table, want);
    want.extend(named(Kind::Type).chain(named(Kind::Grant)));
    assert_eq!(want.len(), 84);
}

#[test]
fn every_row_is_built_from_the_one_table() {
    let rows_of = |kind| ROWS.iter().filter(move |r| r.0 == kind).count();
    assert_eq!(rows_of(Kind::Fn), 76);
    assert_eq!(rows_of(Kind::Variant), 9);
    assert_eq!(rows_of(Kind::Type), 9);
    assert_eq!(rows_of(Kind::Grant), 3);
}
