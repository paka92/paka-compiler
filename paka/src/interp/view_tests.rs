//! Views drawn from source and compared with tree text written by hand (M5-02, M5-06).

use super::draw;
use super::value::{Items, Value};
use crate::sig::Table;
use crate::source::Source;
use crate::{body, lower::lower, parser::parse, resolve::resolve};
use paka_rt::ops::Int;

const COUNTER: &str = "\
type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text(\"{s.count}\")
    Row(gap: 8) {
      Button(\"-\", on: Dec)
      Button(\"+\", on: Inc)
    }
    ??reset
  }
}
";

/// The tree text view `name` draws from `args`, or its trap.
fn drawn(source: &str, name: &str, args: Vec<Value>) -> Result<String, String> {
    let program = parse("main.paka", source.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = body::check("main.paka", &program, &resolved, &mut table);
    assert!(
        bodies.defs.iter().all(Option::is_none),
        "the program checks"
    );
    let source = Source::file("main.paka", source.as_bytes().to_vec());
    let core = lower(&source, &program, &resolved, &table, &bodies).0;
    let id = core.fns.iter().position(|f| f.name == name).unwrap();
    draw(&core, id, args).map(|tree| tree.to_string())
}

fn record(fields: Vec<Value>) -> Value {
    Value::Record(Items(fields))
}

fn int(n: i64) -> Value {
    Value::Int(Int::new(n))
}

fn text(s: &str) -> Value {
    Value::Str(s.into())
}

#[test]
fn the_counter_draws_its_count_and_placeholder() {
    let tree = |count| drawn(COUNTER, "counter_view", vec![record(vec![int(count)])]);
    let want = |count| {
        format!(
            "\
Col gap=8
  Text #0 \"{count}\"
  Row #1 gap=8
    Button #0 \"-\" on=Dec
    Button #1 \"+\" on=Inc
  Hole #2 reset
"
        )
    };
    assert_eq!(tree(0), Ok(want(0)));
    assert_eq!(tree(3), Ok(want(3)));
}

#[test]
fn a_keyed_loop_numbers_its_placeholders_and_collapses_a_shared_key() {
    let source = "\
type Todo { id: Int, name: String }

enum Msg { Add }

view todos(items: List<Todo>) -> View<Msg> {
  Col() {
    Text(\"top\")
    for t in items {
      Text(t.name, key: t.id)
      ??later
    }
    ??footer
  }
}
";
    let todo = |id, name| record(vec![int(id), text(name)]);
    let items = Value::List(Items(vec![todo(1, "a"), todo(2, "b"), todo(1, "c")]));
    let want = "\
Col
  Text #0 \"top\"
  Error 1 \"duplicate key 1: 2 children of Col\"
  Hole #2.0 later
  Text 2 \"b\"
  Hole #2.1 later
  Hole #2.2 later
  Hole #3 footer
";
    assert_eq!(drawn(source, "todos", vec![items]), Ok(want.into()));
}

#[test]
fn view_calls_take_string_keys_and_messages_print_as_values() {
    let source = "\
type Todo { id: Int, name: String }

enum Msg { Draft(text: String), Toggle(id: Int) }

view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 4) {
    Button(\"[ ]\", on: Toggle(t.id))
    if t.id > 1 { Text(t.name) } else { ??short }
  }
}

view todos(items: List<Todo>) -> View<Msg> {
  Col() {
    Input(\"x\", on_change: Draft, key: \"new\")
    for t in items {
      todo_row(t, key: t.name)
    }
  }
}
";
    let todo = |id, name| record(vec![int(id), text(name)]);
    let items = Value::List(Items(vec![todo(1, "it's"), todo(7, "milk")]));
    let want = "\
Col
  Input 'new' \"x\" cursor=0 on_change=Draft
  Row 'it\\'s' gap=4
    Button #0 \"[ ]\" on=Toggle(1)
    Hole #2 short
  Row 'milk' gap=4
    Button #0 \"[ ]\" on=Toggle(7)
    Text #1 \"milk\"
";
    assert_eq!(drawn(source, "todos", vec![items]), Ok(want.into()));
}

#[test]
fn a_negative_computed_gap_traps_and_a_hole_off_element_position_traps() {
    let source = "\
enum Msg { Add }

view spaced(gap: Int) -> View<Msg> {
  Row(gap: gap) {
    Text(\"a\")
  }
}

view named(n: Int) -> View<Msg> {
  Text(if n > 0 { \"x\" } else { ??name })
}
";
    assert_eq!(
        drawn(source, "spaced", vec![int(-1)]),
        Err("trap assert at main.paka:4:3".into())
    );
    assert_eq!(
        drawn(source, "spaced", vec![int(2)]),
        Ok("Row gap=2\n  Text #0 \"a\"\n".into())
    );
    assert_eq!(
        drawn(source, "named", vec![int(0)]),
        Err("trap hole at main.paka:10:32".into())
    );
}
