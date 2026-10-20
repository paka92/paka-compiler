//! Compiled views (M5-24): the tree a view writes through `rt.view` decodes to the text the
//! interpreter draws (paka/src/interp/view_tests.rs), and a trap comes back as trap status.

mod common;

use common::{Dev, source};
use paka_rt::abi::dev::app::Stopped;
use paka_rt::abi::dev::{ASSERT, Context, HOLE, on_runner};

const HEAP: u64 = 1 << 30;

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

fn dev(text: &str) -> Dev {
    let mut dev = Dev::default();
    dev.edit(&source(text)).unwrap();
    dev
}

/// The tree text view `name` draws from argument words `args` and the ledger's leaks and faults
/// once the drawing is released; or the trap's kind and `line:col`.
fn drawn(dev: &Dev, name: &str, args: &[u64]) -> Result<(String, usize, usize), String> {
    let p = &dev.program;
    let id = p.fns.iter().position(|f| f.name == name).unwrap();
    on_runner(HEAP, |ctx: &mut Context| match dev.draw(ctx, id, args) {
        Ok(drawing) => {
            let text = drawing.root.to_string();
            drawing.release(ctx);
            Ok((text, ctx.heap.leaks().len(), ctx.heap.faults().len()))
        }
        Err(Stopped::Trap) => {
            let (_, at) = p.place(ctx.trap.site);
            Err(format!("{} at {}:{}", ctx.trap.kind, at.line, at.col))
        }
        Err(Stopped::Time) => Err("time".into()),
    })
}

fn clean(text: &str) -> Result<(String, usize, usize), String> {
    Ok((text.into(), 0, 0))
}

#[test]
fn the_counter_draws_as_the_interpreter_does() {
    let dev = dev(COUNTER);
    for count in [0u64, 3] {
        let want = format!(
            "\
Col gap=8
  Text #0 \"{count}\"
  Row #1 gap=8
    Button #0 \"-\" on=Dec
    Button #1 \"+\" on=Inc
  Hole #2 reset
"
        );
        let state = [count];
        let at = state.as_ptr().expose_provenance() as u64;
        assert_eq!(drawn(&dev, "counter_view", &[at]), clean(&want));
    }
}

#[test]
fn a_keyed_loop_numbers_its_placeholders_and_collapses_a_shared_key() {
    let dev = dev("\
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

view sample() -> View<Msg> {
  todos([Todo { id: 1, name: \"a\" }, Todo { id: 2, name: \"b\" }, Todo { id: 1, name: \"c\" }])
}
");
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
    assert_eq!(drawn(&dev, "sample", &[]), clean(want));
}

#[test]
fn string_keys_and_messages_with_payloads_draw_and_are_released() {
    let dev = dev("\
type Todo { id: Int, name: String }

enum Msg { Draft(text: String), Toggle(id: Int), Rename(to: String) }

view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 4) {
    Button(\"[ ]\", on: Toggle(t.id))
    if t.id > 1 { Text(t.name) } else { ??short }
    Button(\"x\", on: Rename(t.name))
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

view sample() -> View<Msg> {
  todos([Todo { id: 1, name: \"it's\" }, Todo { id: 7, name: \"milk\" }])
}
");
    let want = "\
Col
  Input 'new' \"x\" cursor=0 on_change=Draft
  Row 'it\\'s' gap=4
    Button #0 \"[ ]\" on=Toggle(1)
    Hole #2 short
    Button #3 \"x\" on=Rename(\"it's\")
  Row 'milk' gap=4
    Button #0 \"[ ]\" on=Toggle(7)
    Text #1 \"milk\"
    Button #3 \"x\" on=Rename(\"milk\")
";
    assert_eq!(drawn(&dev, "sample", &[]), clean(want));
}

#[test]
fn a_negative_gap_and_a_hole_return_trap_status() {
    let dev = dev("\
enum Msg { Add }

view spaced(gap: Int) -> View<Msg> {
  Row(gap: gap) {
    Text(\"a\")
  }
}

view named(n: Int) -> View<Msg> {
  Text(if n > 0 { \"x\" } else { ??name })
}
");
    assert_eq!(
        drawn(&dev, "spaced", &[-1i64 as u64]),
        Err(format!("{ASSERT} at 4:3"))
    );
    assert_eq!(
        drawn(&dev, "spaced", &[2]),
        clean("Row gap=2\n  Text #0 \"a\"\n")
    );
    assert_eq!(drawn(&dev, "named", &[0]), Err(format!("{HOLE} at 10:32")));
}
