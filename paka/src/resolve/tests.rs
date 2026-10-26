//! Hand-written programs and the bindings and diagnostics they must give.

use super::*;
use crate::parser::parse;
use std::collections::BTreeSet;

const SPEC: &str = include_str!("../../../spec/paka.md");

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

/// Every name error: a declaration error alone, else each definition's own, then the app entries'.
fn errors(src: &str) -> Vec<Diagnostic> {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    match resolve("main.paka", &program) {
        Err(d) => vec![d],
        Ok(r) => {
            assert_eq!(r.defs.len(), program.defs.len());
            let (defs, entries) = (r.defs.into_iter(), r.entries.into_values());
            defs.filter_map(Result::err).chain(entries).collect()
        }
    }
}

fn resolved(src: &str) -> Resolved {
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let r = resolve("main.paka", &program).unwrap();
    assert!(
        r.defs.iter().all(Result::is_ok) && r.entries.is_empty(),
        "{src}"
    );
    r
}

/// The first of `errors`.
fn rejection(src: &str) -> Diagnostic {
    errors(src).into_iter().next().expect("no error")
}

/// The code and the position of each error.
fn found(src: &str) -> Vec<(&'static str, Pos)> {
    let all = errors(src);
    all.iter().map(|d| (d.code, d.position.unwrap())).collect()
}

/// The code, the position and the replacement of the fix, if any.
fn rejected(src: &str) -> (&'static str, Pos, Option<String>) {
    let d = rejection(src);
    let detail = d.detailed(src.as_bytes());
    let fix = detail
        .lines()
        .find_map(|l| l.strip_prefix("  fix: replace "));
    let replacement = fix.map(|f| f.split(" with ").nth(1).unwrap().trim_matches('"'));
    (d.code, d.position.unwrap(), replacement.map(String::from))
}

fn assert_rejected(lines: &[&str], code: &str, line: usize, pattern: &str, fix: Option<&str>) {
    let src = program(lines);
    let want = (
        code.to_string(),
        at(&src, line, pattern),
        fix.map(String::from),
    );
    let (found, position, replacement) = rejected(&src);
    assert_eq!((found.to_string(), position, replacement), want, "{src}");
}

/// A function after a header that binds `count` (line 1) and `total` (line 2); `lines` start at 3.
fn body(lines: &[&str]) -> Vec<String> {
    let mut all = vec!["fn f(count: Int) -> Int {", "  let total = count"];
    all.extend(lines);
    all.push("}");
    all.into_iter().map(String::from).collect()
}

fn assert_rejected_in_body(lines: &[&str], code: &str, line: usize, pattern: &str) {
    let all = body(lines);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();
    assert_rejected(&all, code, line, pattern, None);
}

const WHOLE: [&str; 20] = [
    "type Point { x: Int, y: Int = 0 }",
    "enum Msg { Start, Move(dx: Int) }",
    "fn double(n: Int) -> Int {",
    "  let twice = n + n",
    "  twice",
    "}",
    "fn update(m: Msg) -> List<Effect<Msg>> {",
    "  let p = Point { x: 1 }",
    "  for i in 0..<len([1, 2]) { _ = double(i) }",
    "  match m {",
    "    Start => [Exit]",
    "    Move(dx) => [Print(\"{dx} {p.x}\")]",
    "  }",
    "}",
    "app tool {",
    "  state: Point",
    "  start: Start",
    "  update: update",
    "  grants: [Console]",
    "}",
];

#[test]
fn every_kind_of_name_is_bound() {
    let src = program(&WHOLE);
    let r = resolved(&src);
    let bound = |line, pattern| r.get(at(&src, line, pattern)).unwrap();
    let local = |line, pattern| Binding::Local(at(&src, line, pattern));
    let builtin = Binding::Builtin;

    // types: built-in and defined, in fields, parameters, results and arguments
    assert_eq!(bound(1, "x: ^Int"), builtin("Int"));
    assert_eq!(bound(3, "-> ^Int"), builtin("Int"));
    assert_eq!(bound(7, "(m: ^Msg)"), Binding::Def(1));
    assert_eq!(bound(7, "-> ^List<"), builtin("List"));
    assert_eq!(bound(7, "<^Effect<"), builtin("Effect"));
    assert_eq!(bound(7, "Effect<^Msg>>"), Binding::Def(1));
    assert_eq!(bound(16, "state: ^Point"), Binding::Def(0));
    // parameters, lets, loop names and pattern names
    assert_eq!(bound(4, "= ^n +"), local(3, "(^n:"));
    assert_eq!(bound(4, "+ ^n"), local(3, "(^n:"));
    assert_eq!(bound(5, "^twice"), local(4, "let ^twice"));
    assert_eq!(bound(9, "double(^i)"), local(9, "for ^i"));
    assert_eq!(bound(10, "match ^m"), local(7, "(^m:"));
    assert_eq!(bound(12, "{^dx}"), local(12, "Move(^dx)"));
    assert_eq!(bound(12, "{^p.x}"), local(8, "let ^p"));
    // functions, variants, built-in functions and constructors
    assert_eq!(bound(9, "_ = ^double"), Binding::Def(2));
    assert_eq!(bound(9, "in 0..<^len"), builtin("len"));
    assert_eq!(bound(11, "^Start"), Binding::Variant(1, 0));
    assert_eq!(bound(11, "[^Exit]"), builtin("Exit"));
    assert_eq!(bound(12, "^Move"), Binding::Variant(1, 1));
    assert_eq!(bound(12, "[^Print"), builtin("Print"));
    // records; their labels are the checker's
    assert_eq!(bound(8, "= ^Point"), Binding::Def(0));
    assert_eq!(r.get(at(&src, 8, "{ ^x:")), None);
    // app keys and grants
    assert_eq!(bound(17, "start: ^Start"), Binding::Variant(1, 0));
    assert_eq!(bound(18, "update: ^update"), Binding::Def(3));
    assert_eq!(bound(19, "[^Console]"), builtin("Console"));
    // a declaration is not a use
    assert_eq!(r.get(at(&src, 3, "fn ^double")), None);
}

#[test]
fn a_local_hides_a_function_and_a_built_in() {
    let src = program(&[
        "fn helper(n: Int) -> Int { n }",
        "fn f(count: Int) -> Int {",
        "  let helper = count",
        "  let len = helper",
        "  len",
        "}",
    ]);
    let r = resolved(&src);
    let bound = |line, pattern| r.get(at(&src, line, pattern)).unwrap();
    assert_eq!(
        bound(4, "= ^helper"),
        Binding::Local(at(&src, 3, "let ^helper"))
    );
    assert_eq!(bound(5, "^len"), Binding::Local(at(&src, 4, "let ^len")));
}

#[test]
fn sibling_blocks_and_arms_may_reuse_a_name() {
    let src = program(&[
        "fn f(r: Result<String, String>) -> String {",
        "  if true { let a = 1 }",
        "  if false { let a = 2 }",
        "  for i in [1] { }",
        "  for i in [2] { }",
        "  match r {",
        "    Ok(text) => text",
        "    Err(text) => text",
        "  }",
        "}",
    ]);
    let r = resolved(&src);
    let ok = Binding::Local(at(&src, 7, "Ok(^text)"));
    let err = Binding::Local(at(&src, 8, "Err(^text)"));
    assert_eq!(r.get(at(&src, 7, "=> ^text")), Some(ok));
    assert_eq!(r.get(at(&src, 8, "=> ^text")), Some(err));
}

#[test]
fn a_hole_is_not_a_use() {
    resolved(&program(&["fn f() -> Int {", "  ??todo", "}"]));
}

#[test]
fn a_name_is_unknown_after_its_block_and_inside_its_own_let() {
    let after = ["fn f() {", "  if true { let hidden = 1 }", "  hidden", "}"];
    assert_rejected(&after, "unknown-name", 3, "^hidden", None);
    let own = ["fn f() {", "  let total = total + 1", "}"];
    assert_rejected(&own, "unknown-name", 2, "= ^total", None);
}

#[test]
fn duplicate_definitions_are_reported_at_the_later_name() {
    let cases: [(&[&str], usize, &str); 12] = [
        (&["enum Msg { Start, Start }"], 1, "Start, ^Start"),
        (
            &["enum Msg { Start }", "enum Other { Start }"],
            2,
            "{ ^Start",
        ),
        (&["enum Msg { Msg }"], 1, "{ ^Msg"),
        (&["enum Msg { A }", "enum Msg { B }"], 2, "enum ^Msg"),
        (&["type Point {}", "enum Point { A }"], 2, "enum ^Point"),
        (&["fn f() {}", "fn f() {}"], 2, "fn ^f"),
        (&["fn f() {}", "example f { true }"], 2, "example ^f"),
        (&["fn f() {}", "app f { }"], 2, "app ^f"),
        (&["enum Msg { Some }"], 1, "{ ^Some"),
        (&["type String {}"], 1, "type ^String"),
        (&["enum Console { A }"], 1, "enum ^Console"),
        (&["type Point { x: Int, x: Int }"], 1, "Int, ^x"),
    ];
    for (lines, line, pattern) in cases {
        assert_rejected(lines, "duplicate-name", line, pattern, None);
    }
    let fields = ["enum Msg { A(n: Int, n: Int) }"];
    assert_rejected(&fields, "duplicate-name", 1, "Int, ^n", None);
}

#[test]
fn a_user_definition_of_a_built_in_name_says_so() {
    for source in ["fn len() {}", "enum Msg { Exit }", "type String {}"] {
        let d = rejection(&program(&[source]));
        let name = ["len", "Exit", "String"]
            .into_iter()
            .find(|n| source.contains(n));
        assert_eq!(
            d.message,
            format!("{} is a built-in; rename it", name.unwrap())
        );
    }
}

/// The part of the language reference between two headings.
fn spec_section(from: &str, to: &str) -> &'static str {
    SPEC.split(from).nth(1).unwrap().split(to).next().unwrap()
}

/// The leading name of each backticked span in `text`, if it starts with `first`.
fn spans(text: &str, first: fn(char) -> bool) -> Vec<&str> {
    let spans = text.split('`').skip(1).step_by(2);
    let words = spans.filter_map(|s| s.split(|c: char| !c.is_ascii_alphanumeric()).next());
    words.filter(|w| w.starts_with(first)).collect()
}

/// The built-in names of the language reference, by kind: sections 3, 9 and 10.
fn spec_builtins() -> Vec<(Kind, Vec<&'static str>)> {
    let rows = |section: &'static str| section.lines().filter(|l| l.starts_with("| `"));
    let types = spec_section("## 3.", "## 4.");
    let cell = |row: &'static str, i| row.split('|').nth(i).unwrap();
    let first_cells: Vec<_> = rows(types).map(|r| cell(r, 1)).collect();
    let written: Vec<_> = rows(types).map(|r| cell(r, 2)).collect();
    let effects = spec_section("## 10.", "## 11.");
    let effect_cells: Vec<_> = rows(effects).map(|r| cell(r, 1)).collect();
    let upper: fn(char) -> bool = |c| c.is_ascii_uppercase();
    let type_names = first_cells.iter().flat_map(|c| spans(c, upper));
    let variants = written
        .iter()
        .chain(&effect_cells)
        .flat_map(|c| spans(c, upper));
    let grants = effects.lines().find(|l| l.contains("grants: [")).unwrap();
    let grants = grants.rsplit_once(": ").unwrap().1.split(", ").collect();
    let code = spec_section("## 9.", "## 10.").split("```").nth(1).unwrap();
    let calls = code.match_indices('(').map(|(i, _)| {
        let head = &code[..i];
        let start = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
        &head[start.map_or(0, |p| p + 1)..]
    });
    let fns = calls.filter(|w| w.starts_with(|c: char| c.is_ascii_lowercase()) && *w != "fn");
    vec![
        (
            Kind::Type,
            type_names.filter(|n| *n != "Effect<Msg>").collect(),
        ),
        (Kind::Variant, variants.collect()),
        (Kind::Grant, grants),
        (Kind::Fn, fns.collect::<BTreeSet<_>>().into_iter().collect()),
    ]
}

#[test]
fn every_built_in_name_is_taken() {
    let spec = spec_builtins();
    let mut all = BTreeSet::new();
    for (kind, words) in &spec {
        let rows = builtins::ROWS.iter().filter(|b| b.0 == *kind);
        let table: BTreeSet<_> = rows.map(|b| builtins::name(b.1)).collect();
        let (mut table, mut words) = (Vec::from_iter(table), words.clone());
        table.sort_unstable();
        words.sort_unstable();
        assert_eq!(table, words, "{kind:?}");
        all.extend(table);
    }
    assert_eq!(all.len(), 84);
    let sources = [
        (Kind::Type, "type @ {}"),
        (Kind::Variant, "enum Msg { @ }"),
        (Kind::Grant, "enum Msg { @ }"),
        (Kind::Fn, "fn @() {}"),
    ];
    for (kind, template) in sources {
        for name in spec.iter().find(|s| s.0 == kind).map(|s| &s.1).unwrap() {
            let src = program(&[&template.replace('@', name)]);
            let d = rejection(&src);
            assert_eq!(d.code, "duplicate-name", "{src}");
            assert_eq!(d.message, format!("{name} is a built-in; rename it"));
        }
    }
}

#[test]
fn ui_names_are_taken_from_headless_programs() {
    let elements = builtins::ELEMENTS.iter().map(|e| e.name);
    for name in elements.chain(builtins::UI.iter().map(|&(_, row)| builtins::name(row))) {
        let template = if name.starts_with(char::is_uppercase) {
            "enum Msg { @ }"
        } else {
            "fn @() {}"
        };
        let d = rejection(&program(&[&template.replace('@', name)]));
        assert_eq!(d.message, format!("{name} is a built-in; rename it"));
    }
}

#[test]
fn a_local_may_not_reuse_a_visible_name() {
    let cases: [(&[&str], usize, &str); 5] = [
        (&["  let total = 1"], 3, "let ^total"),
        (&["  let count = 1"], 3, "let ^count"),
        (&["  for count in [1] { }"], 3, "for ^count"),
        (&["  if true { let total = 1 }"], 3, "let ^total"),
        (
            &[
                "  match Some(1) {",
                "    Some(total) => 1",
                "    None => 2",
                "  }",
            ],
            4,
            "Some(^total)",
        ),
    ];
    for (lines, line, pattern) in cases {
        assert_rejected_in_body(lines, "duplicate-local", line, pattern);
    }
    let params = ["fn f(a: Int, a: Int) {}"];
    assert_rejected(&params, "duplicate-local", 1, "Int, ^a", None);
}

#[test]
fn a_duplicate_local_names_the_first_line() {
    let src = program(
        &body(&["  let total = 1"])
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    assert!(rejection(&src).message.contains("already bound at line 2"));
}

#[test]
fn an_unknown_name_gets_the_nearest_known_name_as_its_fix() {
    let msg = ["enum Msg { Start }", "fn f() -> Msg {", "  Stat", "}"];
    assert_rejected(&msg, "unknown-name", 3, "^Stat", Some("Start"));
    let typo = |line, pattern, fix| {
        let all = body(&[line]);
        let all: Vec<&str> = all.iter().map(String::as_str).collect();
        assert_rejected(&all, "unknown-name", 3, pattern, fix);
    };
    typo("  count + totl", "+ ^totl", Some("total"));
    typo("  lenn(count)", "^lenn", Some("len"));
    typo("  zzzzzz", "^zzzzzz", None);
    typo("  Count", "^Count", None);
}

#[test]
fn the_nearest_name_is_the_first_in_byte_order_on_a_tie() {
    let lines = ["fn abx() {}", "fn abc() {}", "fn f() {", "  abd", "}"];
    assert_rejected(&lines, "unknown-name", 4, "^abd", Some("abc"));
}

#[test]
fn a_name_farther_than_a_third_of_its_length_has_no_fix() {
    let fix = |name: &str| {
        let lines = ["fn target() {}".to_string(), format!("fn f() {{ {name} }}")];
        let src = program(&[&lines[0], &lines[1]]);
        rejected(&src).2
    };
    assert_eq!(fix("tarxxt"), Some("target".into()));
    assert_eq!(fix("tarxyt"), Some("target".into()));
    assert_eq!(fix("taxxyt"), None);
    assert_eq!(fix("tar"), None);
    assert_eq!(fix("Target"), None);
}

#[test]
fn only_values_are_names_in_an_expression() {
    let ty = ["enum Msg { Start }", "fn f() -> Msg {", "  Msg", "}"];
    assert_rejected(&ty, "unknown-name", 3, "^Msg", None);
    let example = ["example check { true }", "fn f() -> Bool {", "  check", "}"];
    assert_rejected(&example, "unknown-name", 3, "^check", None);
    let grant = ["fn f() {", "  Console", "}"];
    assert_rejected(&grant, "unknown-name", 2, "^Console", None);
}

#[test]
fn a_record_names_a_record_type_and_leaves_its_labels_to_the_checker() {
    let lines = [
        "type Point { x: Int }",
        "fn f() {",
        "  _ = Pont { x: 1 }",
        "}",
    ];
    assert_rejected(&lines, "unknown-type", 3, "= ^Pont", Some("Point"));
    let lines = ["enum Msg { Start }", "fn f() {", "  _ = Msg {}", "}"];
    assert_rejected(&lines, "unknown-type", 3, "= ^Msg", None);
    let lines = [
        "type Size { width: Int, height: Int }",
        "fn f() {",
        "  _ = Size { width: 1, widht: 2 }",
        "}",
    ];
    resolved(&program(&lines));
}

#[test]
fn an_unknown_type_is_found_anywhere_a_type_stands() {
    let cases: [(&[&str], usize, &str, Option<&str>); 6] = [
        (&["fn f(m: Message) {}"], 1, "m: ^Message", None),
        (
            &["enum Msg { Start }", "fn f(m: Start) {}"],
            2,
            "m: ^Start",
            None,
        ),
        (
            &["fn f(xs: List<Strng>) {}"],
            1,
            "List<^Strng>",
            Some("String"),
        ),
        (&["fn f(g: fn(Int) -> Nope) {}"], 1, "-> ^Nope", None),
        (&["fn f() { let x: Nope = 1 }"], 1, "x: ^Nope", None),
        (
            &["type Point { x: Int }", "fn f(p: Pont) {}"],
            2,
            "p: ^Pont",
            Some("Point"),
        ),
    ];
    for (lines, line, pattern, fix) in cases {
        assert_rejected(lines, "unknown-type", line, pattern, fix);
    }
}

#[test]
fn an_unknown_pattern_variant_gets_a_fix() {
    let lines = [
        "enum Msg { Start }",
        "fn f(m: Msg) -> Int {",
        "  match m {",
        "    Stat => 1",
        "  }",
        "}",
    ];
    assert_rejected(&lines, "unknown-name", 4, "^Stat", Some("Start"));
}

const APP: [&str; 6] = [
    "enum Msg { Start }",
    "fn handle(m: Msg) -> List<Effect<Msg>> { [] }",
    "app a {",
    "  start: Start",
    "  update: handle",
    "  grants: [Console]",
];

fn app_with(key: usize, line: &str) -> Vec<&str> {
    let mut lines = APP.to_vec();
    lines[key] = line;
    lines.push("}");
    lines
}

#[test]
fn app_keys_name_functions_variants_and_grants() {
    let lines = app_with(4, "  update: handel");
    assert_rejected(&lines, "unknown-name", 5, "^handel", Some("handle"));
    let lines = app_with(4, "  update: len");
    assert_rejected(&lines, "unknown-name", 5, "^len", None);
    let lines = app_with(4, "  update: a");
    assert_rejected(&lines, "unknown-name", 5, "update: ^a", None);
    let lines = app_with(3, "  start: Stat");
    assert_rejected(&lines, "unknown-name", 4, "^Stat", Some("Start"));
    let lines = app_with(5, "  grants: [Consol]");
    assert_rejected(&lines, "unknown-grant", 6, "^Consol", Some("Console"));
    let lines = app_with(5, "  grants: [Camera]");
    assert_rejected(&lines, "unknown-grant", 6, "^Camera", None);
    let lines = app_with(5, "  grants: [ReadFiles, WriteFiles]");
    resolved(&program(&lines));
    let lines = app_with(5, "  state: Nope");
    assert_rejected(&lines, "unknown-type", 6, "^Nope", None);
}

#[test]
fn declarations_come_first_then_each_definition_in_source_order() {
    let lines = ["fn f() -> Strng {}", "fn f() {}"];
    assert_rejected(&lines, "duplicate-name", 2, "fn ^f", None);
    let lines = ["fn f() {", "  nothing", "}", "fn g(m: Bad) {}"];
    let src = program(&lines);
    assert_eq!(
        found(&src),
        [("unknown-name", (2, 3)), ("unknown-type", (4, 9))]
    );
}

#[test]
fn a_type_is_tested_outside_in() {
    let lines = ["enum Msg { Start }", "fn f(m: Msg<Bad>) {}"];
    assert_rejected(&lines, "wrong-type-arguments", 2, "m: ^Msg", None);
    let lines = ["fn f(m: List<Bad, Int>) {}"];
    assert_rejected(&lines, "wrong-type-arguments", 1, "m: ^List", None);
    let lines = ["fn f(m: Bad<Worse>) {}"];
    assert_rejected(&lines, "unknown-type", 1, "m: ^Bad", None);
    let lines = ["fn f(m: Int, n: Bad) -> Worse {}"];
    assert_rejected(&lines, "unknown-type", 1, "n: ^Bad", None);
}

#[test]
fn each_app_entry_keeps_its_own_error() {
    let lines = [
        "enum Msg { Start }",
        "fn handle(m: Msg) -> List<Effect<Msg>> { [] }",
        "app a {",
        "  state: Nope",
        "  start: Begin",
        "  update: nothing",
        "  grants: [Camera, Console, Mic]",
        "}",
    ];
    let src = program(&lines);
    let program = parse("main.paka", src.as_bytes()).unwrap();
    let r = resolve("main.paka", &program).unwrap();
    assert!(r.defs.iter().all(Result::is_ok));
    let entry = |line, pattern| r.entry_error(at(&src, line, pattern)).unwrap();
    assert_eq!(entry(4, "state: ^Nope").code, "unknown-type");
    assert_eq!(entry(5, "start: ^Begin").code, "unknown-name");
    assert_eq!(entry(6, "update: ^nothing").code, "unknown-name");
    assert_eq!(entry(7, "[^Camera").code, "unknown-grant");
    assert_eq!(entry(7, "Console, ^Mic").code, "unknown-grant");
    assert!(r.entry_error(at(&src, 7, "Camera, ^Console")).is_none());
    assert!(
        entry(4, "state: ^Nope")
            .detailed(src.as_bytes())
            .contains("  definition: a\n")
    );
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
            let r = resolved(&src);
            assert!(r.get(at(&src, 2, "(^x")).is_some());
        });
    }
}

#[test]
fn a_diagnostic_names_its_definition() {
    let src = program(&["fn f() {", "  nothing", "}"]);
    assert!(
        rejection(&src)
            .detailed(src.as_bytes())
            .contains("  definition: f\n")
    );
}

/// Each `unused-name` warning: its name and position (docs/decisions/M2-02.md section 5).
fn unused(lines: &[&str]) -> Vec<(String, Pos)> {
    let r = resolved(&program(lines));
    let name = |d: &Diagnostic| {
        d.message
            .strip_suffix(" is never used")
            .unwrap()
            .to_string()
    };
    r.unused
        .iter()
        .map(|d| (name(d), d.position.unwrap()))
        .collect()
}

#[test]
fn every_kind_of_binding_warns_when_never_named() {
    let found = unused(&[
        "enum Msg { Go(n: Int) }",
        "fn f(a: Int, b: Int) -> Int {",
        "  let c = 1",
        "  var d = 2",
        "  for e in 0..<3 {",
        "  }",
        "  match Go(1) {",
        "    Go(g) => {}",
        "  }",
        "  a",
        "}",
    ]);
    let want = [
        ("b", (2, 14)),
        ("c", (3, 7)),
        ("d", (4, 7)),
        ("e", (5, 7)),
        ("g", (8, 8)),
    ];
    assert_eq!(found, want.map(|(n, at)| (n.to_string(), at)));
}

#[test]
fn a_write_uses_a_name() {
    let found = unused(&[
        "type Box { n: Int = 0 }",
        "fn f(s: inout Box, t: inout Box, k: Int) {",
        "  s.n = 1",
        "  var v = 0",
        "  v = 2",
        "  if true {",
        "    t = Box { n: k }",
        "  }",
        "}",
    ]);
    assert_eq!(found, []);
}

#[test]
fn a_name_used_in_a_sibling_block_does_not_use_this_one() {
    let found = unused(&[
        "fn f(c: Bool) {",
        "  if c {",
        "    let x = 1",
        "    _ = x",
        "  } else {",
        "    let x = 2",
        "  }",
        "}",
    ]);
    assert_eq!(found, [("x".to_string(), (6, 9))]);
}

#[test]
fn the_update_message_and_an_example_local_warn_and_underscore_never_does() {
    let src = program(&[
        "enum Msg { Go(n: Int) }",
        "fn update(m: Msg) -> List<Effect<Msg>> {",
        "  [Exit]",
        "}",
        "fn g(m: Msg) -> Int {",
        "  match m {",
        "    Go(_) => 0",
        "  }",
        "}",
        "example e {",
        "  let unused = 1",
        "  true",
        "}",
    ]);
    let r = resolved(&src);
    let shown: Vec<_> = (r.unused.iter())
        .map(|d| d.detailed(src.as_bytes()))
        .collect();
    let line = |at: &str, name: &str, def: &str, bytes: &str| {
        format!(
            "main.paka:{at}: warning[unused-name]: {name} is never used\n  definition: {def}\n  \
             bytes: {bytes}\n  text: \"{name}\"\n"
        )
    };
    assert_eq!(
        shown,
        [
            line("2:11", "m", "update", "34..35"),
            line("11:7", "unused", "e", "149..155"),
        ]
    );
}

#[test]
fn a_foreign_declaration_keeps_its_shape() {
    let decl = |line: &str| {
        let lines = [
            "foreign Sum {",
            "  source: \"sum.c\"",
            "  header: \"sum.h\"",
            line,
            "}",
        ];
        program(&lines)
    };
    assert!(errors(&decl("  Add(a: Int, b: String) -> Float = \"sum_add\"")).is_empty());
    let cases = [
        ("  header: \"two.h\"", "bad-foreign", "^header"),
        ("  library: \"../lib\"", "bad-foreign", "\"^../lib"),
        ("  Add(a: List<Int>) = \"add\"", "bad-foreign", "^List"),
        ("  Add() -> String = \"add\"", "bad-foreign", "^String"),
        ("  Add(a: Int) = \"9add\"", "bad-foreign", "\"^9add"),
        (
            "  Add(a: Int, a: Int) = \"add\"",
            "duplicate-name",
            "Int, ^a",
        ),
        ("  Add(then: Int) = \"add\"", "duplicate-name", "(^then"),
    ];
    for (line, code, pattern) in cases {
        let src = decl(line);
        assert_eq!(found(&src), [(code, at(&src, 4, pattern))], "{line}");
    }
    let bare = program(&["foreign Sum {", "  header: \"sum.h\"", "}"]);
    assert_eq!(found(&bare), [("bad-foreign", (1, 9))]);
    let taken = program(&[
        "foreign Sum {",
        "  header: \"sum.h\"",
        "  Exit() = \"p\"",
        "}",
    ]);
    assert_eq!(found(&taken)[0].0, "duplicate-name");
}
