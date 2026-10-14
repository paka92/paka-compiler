//! App definitions (M2-32): the approved forms of docs/decisions/M2-15.md, and the code, the
//! position and the whole message of each wrong one.

use super::*;

const TALLY: &str = "type Tally { words: Int = 0 }";
const MSG: &str = "enum Msg { Start(path: String), Loaded(text: String) }";
const UPDATE: &str = "fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> { [Exit] }";
const BARE: &str = "fn update(m: Msg) -> List<Effect<Msg>> { [Exit] }";

/// The definitions in `defs`, then an app named `wc` with `entries`.
fn app(defs: &[&str], entries: &[&str]) -> String {
    let entries: Vec<_> = entries.iter().map(|e| format!("  {e}")).collect();
    let entries: Vec<_> = entries.iter().map(String::as_str).collect();
    program(&[defs, &["app wc {"], &entries, &["}"]].concat())
}

const FULL: [&str; 4] = [
    "state: Tally",
    "start: Start",
    "update: update",
    "grants: [Console]",
];

/// The full app with the entry `key` replaced by `line`, over `defs`.
fn replaced(defs: &[&str], key: &str, line: &str) -> String {
    let swap = |e: &str| if e.starts_with(key) { line } else { e }.to_string();
    let entries: Vec<_> = FULL.iter().map(|e| swap(e)).collect();
    app(
        defs,
        &entries.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}

fn full_with(key: &str, line: &str) -> String {
    replaced(&[TALLY, MSG, UPDATE], key, line)
}

/// The full app over `types`, `Msg` and an `update` that takes the state `name`.
fn state(types: &[&str], name: &str) -> String {
    let update = format!("fn update(s: inout {name}, m: Msg) -> List<Effect<Msg>> {{ [Exit] }}");
    replaced(
        &[types, &[MSG, &update]].concat(),
        "state",
        &format!("state: {name}"),
    )
}

#[test]
fn accepts_the_approved_forms() {
    accepts(&[
        app(&[TALLY, MSG, UPDATE], &FULL),
        app(
            &[MSG, BARE],
            &["start: Start", "update: update", "grants: []"],
        ),
        app(
            &[TALLY, MSG, UPDATE],
            &[
                "grants: [ReadFiles, WriteFiles, Console]",
                "update: update",
                "start: Loaded",
                "state: Tally",
            ],
        ),
        app(&[UPDATE, MSG, TALLY], &FULL),
        app(
            &[TALLY, "enum Msg { Start }", UPDATE],
            &[
                "state: Tally",
                "start: Start",
                "update: update",
                "grants: []",
            ],
        ),
        app(
            &[
                "type Inner { names: List<String> = [], seen: List<Int> = [], last: Option<Int> = None }",
                "type Tally { inner: Inner = Inner {}, n: Int = 0 }",
                MSG,
                UPDATE,
            ],
            &FULL,
        ),
    ]);
}

#[test]
fn a_function_type_outside_the_state_is_allowed() {
    let free = "type Free { after: List<fn(Int) -> Int> = [] }";
    accepts(&[app(&[free, TALLY, MSG, UPDATE], &FULL)]);
}

#[test]
fn rejects_wrong_keys() {
    let defs = [TALLY, MSG, UPDATE];
    let twice = app(
        &defs,
        &[FULL[0], FULL[1], FULL[2], FULL[3], "start: Loaded"],
    );
    let no_update = app(&defs, &[FULL[0], FULL[1], FULL[3]]);
    let no_start = app(&defs, &[FULL[0], FULL[2], FULL[3]]);
    let no_grants = app(&defs, &[FULL[0], FULL[1], FULL[2]]);
    let empty = app(&defs, &[]);
    let dup_first = app(&defs, &[FULL[0], FULL[1], FULL[3], "start: Loaded"]);
    says(&[
        (
            "duplicate-app-entry",
            "  ^start: Loaded",
            "`start` is written twice",
            twice,
        ),
        (
            "missing-update",
            "app ^wc",
            "the app has no `update`",
            no_update,
        ),
        (
            "missing-start",
            "app ^wc",
            "the app has no `start`",
            no_start,
        ),
        (
            "missing-grants",
            "app ^wc",
            "the app has no `grants`",
            no_grants,
        ),
        ("missing-start", "app ^wc", "the app has no `start`", empty),
        (
            "duplicate-app-entry",
            "  ^start: Loaded",
            "`start` is written twice",
            dup_first,
        ),
    ]);
}

#[test]
fn rejects_a_wrong_state() {
    let why = "is not a record type with a default on every field; give every field of";
    let enum_state = full_with("state", "state: Msg");
    let no_default = state(&["type Raw { n: Int }"], "Raw");
    let int_state = full_with("state", "state: Int");
    let held = "type Later { after: List<fn(Int) -> Int> = [] }";
    let deep = "type Deep { later: Later = Later {} }";
    let fn_state = state(&[held], "Later");
    let deep_state = state(&[held, deep], "Deep");
    let held_msg = "keep a message value instead and match on it in update";
    says(&[
        (
            "bad-state",
            "state: ^Msg",
            &format!("state Msg {why} Msg a default, or name a record type"),
            enum_state,
        ),
        (
            "bad-state",
            "state: ^Raw",
            &format!("state Raw {why} Raw a default, or name a record type"),
            no_default,
        ),
        (
            "bad-state",
            "state: ^Int",
            &format!("state Int {why} Int a default, or name a record type"),
            int_state,
        ),
        (
            "type-mismatch",
            "state: ^Later",
            &format!("state Later holds a function type in field after; {held_msg}"),
            fn_state,
        ),
        (
            "type-mismatch",
            "state: ^Deep",
            &format!("state Deep holds a function type in field after; {held_msg}"),
            deep_state,
        ),
    ]);
}

#[test]
fn rejects_a_wrong_update_signature() {
    let shape = "update must be fn(s: inout Tally, m: Msg) -> List<Effect<Msg>>";
    let bare = "update must be fn(m: Msg) -> List<Effect<Msg>>";
    let wrong = |f: &str| app(&[TALLY, MSG, f], &FULL);
    let stateless = |f: &str| app(&[MSG, f], &["start: Start", "update: update", "grants: []"]);
    let other = "enum Other { Stop }";
    says(&[
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: String) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(m: Msg) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: Tally, m: Msg) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: Msg) -> Effect<Msg> { Exit }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: inout Msg) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: Msg) { }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally; m: Msg) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong(&format!(
                "{other}\nfn update(s: inout Tally, m: Msg) -> List<Effect<Other>> {{ [Exit] }}"
            )),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: Tally) -> List<Effect<Tally>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            shape,
            wrong("fn update(s: inout Tally, m: Int) -> List<Effect<Int>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            bare,
            stateless("fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> { [Exit] }"),
        ),
        (
            "bad-update-signature",
            "update: ^update",
            bare,
            stateless("fn update(m: String) -> List<Effect<Msg>> { [Exit] }"),
        ),
    ]);
}

#[test]
fn rejects_a_wrong_first_message() {
    let other = "enum Other { Stop }";
    let defs = [TALLY, MSG, other, UPDATE];
    let not_variant = "is not a variant of `Msg`";
    let entries = |start: &str| {
        [FULL[0], start, FULL[2], FULL[3]]
            .map(String::from)
            .to_vec()
    };
    let run = |start: &str| {
        let entries = entries(start);
        app(
            &defs,
            &entries.iter().map(String::as_str).collect::<Vec<_>>(),
        )
    };
    let int_field = app(&[TALLY, "enum Msg { Start(n: Int) }", UPDATE], &FULL);
    let second = app(
        &[
            TALLY,
            "enum Msg { Start(a: String, b: List<String>) }",
            UPDATE,
        ],
        &FULL,
    );
    let msg = |name: &str, field: &str, ty: &str| {
        format!("`{name}` field `{field}` is {ty}; the first message is filled from the arguments")
    };
    says(&[
        (
            "bad-start",
            "start: ^Stop",
            &format!("`Stop` {not_variant}"),
            run("start: Stop"),
        ),
        (
            "bad-start",
            "start: ^Exit",
            &format!("`Exit` {not_variant}"),
            run("start: Exit"),
        ),
        (
            "bad-start",
            "start: ^update",
            &format!("`update` {not_variant}"),
            run("start: update"),
        ),
        (
            "bad-start",
            "start: ^Start(",
            &format!("`Start(..)` {not_variant}"),
            run("start: Start(\"a\")"),
        ),
        (
            "bad-start",
            "start: ^Start",
            &msg("Start", "n", "not a String"),
            int_field,
        ),
        (
            "bad-start",
            "start: ^Start",
            &msg("Start", "b", "not a String"),
            second,
        ),
    ]);
}

#[test]
fn rejects_an_unknown_or_repeated_grant() {
    let grants = |g: &str| full_with("grants", g);
    says(&[
        (
            "unknown-grant",
            "[^Files]",
            "Files is not a grant; did you mean ReadFiles?",
            grants("grants: [Files]"),
        ),
        (
            "unknown-grant",
            "[Console, ^Network]",
            "Network is not a grant",
            grants("grants: [Console, Network]"),
        ),
        (
            "unknown-grant",
            "[^Consol, Console]",
            "Consol is not a grant; did you mean Console?",
            grants("grants: [Consol, Console]"),
        ),
        (
            "duplicate-grant",
            "ReadFiles, ^ReadFiles",
            "`ReadFiles` is listed twice",
            grants("grants: [ReadFiles, ReadFiles]"),
        ),
        (
            "unknown-grant",
            "[Console, ^Files, Console]",
            "Files is not a grant; did you mean ReadFiles?",
            grants("grants: [Console, Files, Console]"),
        ),
    ]);
}

#[test]
fn the_unknown_grant_carries_its_fix() {
    let src = full_with("grants", "grants: [Files]");
    let [d] = &errors(&src)[..] else {
        panic!("want one error")
    };
    assert!(d.detailed(src.as_bytes()).contains(" with \"ReadFiles\"\n"));
}

#[test]
fn reports_in_the_order_state_update_start_grants() {
    let defs = [TALLY, MSG, UPDATE];
    let bad = |entries: [&str; 4]| app(&defs, &entries);
    let all = [
        "state: Msg",
        "start: Stop",
        "update: step",
        "grants: [Files]",
    ];
    let no_state = [FULL[0], "start: Stop", "update: step", "grants: [Files]"];
    let no_update = [FULL[0], "start: Stop", FULL[2], "grants: [Files]"];
    let no_start = [FULL[0], FULL[1], FULL[2], "grants: [Files]"];
    let late = ["grants: [Files]", "start: Stop", "update: step", FULL[0]];
    let kinds = |src: String| errors(&src).iter().map(|d| d.code).collect::<Vec<_>>();
    assert_eq!(kinds(bad(all)), ["bad-state"]);
    assert_eq!(kinds(bad(no_state)), ["unknown-name"]);
    assert_eq!(kinds(bad(no_update)), ["unknown-name"]);
    assert_eq!(kinds(bad(no_start)), ["unknown-grant"]);
    assert_eq!(kinds(bad(late)), ["unknown-name"]);
}

#[test]
fn an_unknown_type_is_reported_once_and_cascades_nowhere() {
    let broken = "fn update(s: inout Tally, m: Mgs) -> List<Effect<Mgs>> { [Exit] }";
    let src = app(&[TALLY, MSG, broken], &FULL);
    assert_eq!(
        errors(&src).iter().map(|d| d.code).collect::<Vec<_>>(),
        ["unknown-type"]
    );
    let state = full_with("state", "state: Tallie");
    let found = errors(&state);
    let [d] = &found[..] else {
        panic!("want one error")
    };
    assert_eq!(
        (d.code, d.position),
        ("unknown-type", at(&state, "state: ^Tallie"))
    );
}

#[test]
fn the_first_error_of_an_app_does_not_touch_other_definitions() {
    let src = full_with("grants", "grants: [Files]");
    let run = run(&src);
    let bad: Vec<_> = run.defs.iter().map(Option::is_some).collect();
    assert_eq!(bad, [false, false, false, true]);
}
