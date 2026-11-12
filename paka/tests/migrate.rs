//! A migrate checked against the running version of a program and the new one (M5-30a,
//! docs/decisions/M5-07.md sections 1.3, 1.4 and 2).

use paka::migrate::check_migrate;
use paka::source::Source;

/// An app storing `state`, with `types` defining it.
fn app(types: &str, state: &str) -> Source {
    let text = format!(
        "{types}\n\nenum Msg {{ Inc }}\n\nfn update(s: inout {state}, m: Msg) {{\n  match m {{\n    \
         Inc => {{}}\n  }}\n}}\n\nview show(s: {state}) -> View<Msg> {{\n  Button(\"+\", on: Inc)\n}}\n\n\
         app main {{\n  state: {state}\n  update: update\n  view: show\n  grants: []\n}}\n"
    );
    let source = Source::file("main.paka", text.into());
    if let Err(d) = paka::check("main.paka", &source) {
        panic!("the app does not check: {}", source.locate(d).0);
    }
    source
}

const COUNTER: &str = "type Counter { count: Int = 0 }";
const COUNTER_STEP: &str = "type Counter { count: Int = 0, step: Int = 1 }";

const TODOS: &str = "\
type Todo { id: Int, title: String, done: Bool = false }

type Todos { draft: String = \"\", items: List<Todo> = [] }";

const TODOS_NOTED: &str = "\
type Todo { id: Int, title: String, done: Bool = false, note: String = \"\" }

type Todos { draft: String = \"\", items: List<Todo> = [] }";

const MODE: &str = "\
enum Mode { Plain, Noted(text: String) }

type Board { count: Int = 0, mode: Mode = Plain }";

const MODE_MORE: &str = "\
enum Mode { Plain, Noted(text: String), Starred }

type Board { count: Int = 0, mode: Mode = Plain }";

fn accepts(old: &Source, new: &Source, migrate: &str) {
    if let Err(d) = check_migrate(old, new, migrate) {
        panic!("refused: {d}");
    }
}

/// The code and the `migrate:line:col` of the refusal.
fn refusal(old: &Source, new: &Source, migrate: &str) -> (&'static str, String) {
    match check_migrate(old, new, migrate) {
        Ok(_) => panic!("accepted: {migrate}"),
        Err(d) => {
            let (line, col) = d.position.expect("a refusal has a position");
            (d.code, format!("{}:{line}:{col}", d.path))
        }
    }
}

fn counter_step() -> (Source, Source) {
    (app(COUNTER, "Counter"), app(COUNTER_STEP, "Counter"))
}

#[test]
fn counter_gains_a_field() {
    let (old, new) = counter_step();
    let migrate = "migrate(old: Counter) -> Counter {\n  Counter { count: old.count }\n}\n";
    let checked = check_migrate(&old, &new, migrate).unwrap_or_else(|d| panic!("{d}"));
    assert_eq!(checked.function, checked.program.fns.len() - 1);
}

#[test]
fn todos_gain_a_note() {
    let (old, new) = (app(TODOS, "Todos"), app(TODOS_NOTED, "Todos"));
    accepts(
        &old,
        &new,
        "migrate(old: Todos) -> Todos {
  var items: List<Todo> = []
  for t in old.items {
    append(&items, Todo { id: t.id, title: t.title, done: t.done, note: \"\" })
  }
  Todos { draft: old.draft, items: items }
}",
    );
}

#[test]
fn nothing_changed() {
    let counter = app(COUNTER, "Counter");
    accepts(
        &counter,
        &counter,
        "migrate(old: Counter) -> Counter { old }",
    );
}

#[test]
fn a_hole_is_accepted() {
    let (old, new) = counter_step();
    accepts(&old, &new, "migrate(old: Counter) -> Counter { ??later }");
}

#[test]
fn new_functions_on_new_values() {
    let (old, new) = (app(TODOS, "Todos"), app(TODOS_NOTED, "Todos"));
    accepts(
        &old,
        &new,
        "migrate(old: Todos) -> Todos {
  let n = old.items[0].title
  Todos { draft: max(old.draft, n), items: [] }
}",
    );
}

#[test]
fn every_old_variant_matched() {
    let (old, new) = (app(MODE, "Board"), app(MODE_MORE, "Board"));
    accepts(
        &old,
        &new,
        "migrate(old: Board) -> Board {
  let mode = match old.mode {
    Plain => Plain
    Noted(t) => Noted(t)
  }
  Board { count: old.count, mode: mode }
}",
    );
}

#[test]
fn wrong_parameter_type() {
    let (old, new) = counter_step();
    let migrate = "migrate(old: Int) -> Counter { Counter { count: old } }";
    assert_eq!(
        refusal(&old, &new, migrate),
        ("bad-migrate-signature", "migrate:1:14".into())
    );
}

#[test]
fn wrong_result_type() {
    let (old, new) = counter_step();
    let migrate = "migrate(old: Counter) -> Int { old.count }";
    assert_eq!(
        refusal(&old, &new, migrate),
        ("bad-migrate-signature", "migrate:1:26".into())
    );
}

#[test]
fn inout_parameter() {
    let (old, new) = counter_step();
    let migrate = "migrate(old: inout Counter) -> Counter { Counter { count: 0 } }";
    assert_eq!(refusal(&old, &new, migrate).0, "bad-migrate-signature");
}

#[test]
fn effect_is_impure() {
    let (old, new) = counter_step();
    let migrate = "migrate(old: Counter) -> Counter {\n  let e: Effect<Int> = Print(\"x\")\n  \
                   Counter { count: old.count }\n}";
    assert_eq!(
        refusal(&old, &new, migrate),
        ("effect-in-migrate", "migrate:2:24".into())
    );
}

#[test]
fn old_value_passed_to_a_function() {
    let (old, new) = (app(TODOS, "Todos"), app(TODOS_NOTED, "Todos"));
    let migrate =
        "migrate(old: Todos) -> Todos {\n  let n = len(old.items)\n  Todos { draft: old.draft }\n}";
    assert_eq!(
        refusal(&old, &new, migrate),
        ("old-value-use", "migrate:2:15".into())
    );
}

#[test]
fn old_value_compared() {
    let (old, new) = (app(MODE, "Board"), app(MODE_MORE, "Board"));
    let migrate = "migrate(old: Board) -> Board {\n  let same = old.mode == old.mode\n  \
                   Board { count: old.count }\n}";
    assert_eq!(
        refusal(&old, &new, migrate),
        ("old-value-use", "migrate:2:14".into())
    );
}

#[test]
fn old_value_printed() {
    let (old, new) = (app(MODE, "Board"), app(MODE_MORE, "Board"));
    let migrate =
        "migrate(old: Board) -> Board {\n  let t = \"{old.mode}\"\n  Board { count: old.count }\n}";
    assert_eq!(refusal(&old, &new, migrate).0, "type-mismatch");
}

#[test]
fn old_variant_missed() {
    let (old, new) = (app(MODE, "Board"), app(MODE_MORE, "Board"));
    let migrate = "migrate(old: Board) -> Board {
  let n = match old.mode {
    Plain => 0
  }
  Board { count: n }
}";
    assert_eq!(refusal(&old, &new, migrate).0, "missing-arm");
}

#[test]
fn changed_type_holding_itself() {
    let tree =
        "enum Tree { Leaf, Node(left: Tree, right: Tree) }\n\ntype Garden { tree: Tree = Leaf }";
    let grown = "enum Tree { Leaf, Bud, Node(left: Tree, right: Tree) }\n\ntype Garden { tree: Tree = Leaf }";
    let (old, new) = (app(tree, "Garden"), app(grown, "Garden"));
    assert_eq!(
        refusal(&old, &new, "migrate(old: Garden) -> Garden { Garden {} }"),
        ("migrate-needs-reset", "migrate:1:14".into())
    );
}
