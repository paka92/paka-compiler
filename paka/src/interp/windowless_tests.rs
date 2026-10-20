//! Visual apps run with no window, driven by scripts of events and read through their tree text
//! (M5-01, M5-03). Nothing here touches a display.

use super::Windowless;
use super::effects::Host;
use paka_rt::input::Key;
use paka_rt::route::Event;
use paka_rt::tree::Step;

/// The counter of GENESIS.md section 4.
const COUNTER: &str = "\
type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}

example dec_stops_at_zero {
  var s = Counter {}
  update(&s, Dec)
  s.count == 0
}

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

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}
";

const PLUS: [Step; 2] = [Step::Place(1), Step::Place(1)];
const MINUS: [Step; 2] = [Step::Place(1), Step::Place(0)];

fn click(path: &[Step], kind: &'static str) -> Event {
    Event::Click(path.to_vec(), kind)
}

/// The counter's tree showing `count`.
fn counter(count: &str) -> String {
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
}

/// Launches `source`, sends `events`, and gives the tree text, the trap reports and the end.
fn play(source: &str, events: Vec<Event>) -> (String, Vec<String>, Option<(u8, String)>) {
    play_in(source, events, &mut vec![])
}

/// As `play`, printing to `out`.
fn play_in(
    source: &str,
    events: Vec<Event>,
    out: &mut Vec<u8>,
) -> (String, Vec<String>, Option<(u8, String)>) {
    let source = crate::source::Source::file("main.paka", source.into());
    let core = crate::check("main.paka", &source)
        .unwrap_or_else(|d| panic!("{d}"))
        .0;
    let mut ui = Windowless::start(&core, &[], Host(out));
    ui.send(events);
    (ui.tree.to_string(), ui.traps, ui.end)
}

#[test]
fn the_counter_launches_at_zero_and_counts_clicks() {
    assert_eq!(play(COUNTER, vec![]), (counter("0"), vec![], None));
    let clicks = vec![click(&PLUS, "Button"), click(&PLUS, "Button")];
    assert_eq!(play(COUNTER, clicks), (counter("2"), vec![], None));
    let clicks = vec![
        click(&PLUS, "Button"),
        click(&MINUS, "Button"),
        click(&MINUS, "Button"),
    ];
    assert_eq!(play(COUNTER, clicks).0, counter("0"));
}

#[test]
fn a_click_on_no_button_sends_nothing() {
    let clicks = vec![
        click(&[Step::Place(0)], "Text"),
        click(&[Step::Place(2)], "Hole"),
        click(&PLUS, "Input"),
        click(&[Step::Place(1), Step::Place(2)], "Button"),
    ];
    assert_eq!(play(COUNTER, clicks), (counter("0"), vec![], None));
}

#[test]
fn a_trapping_view_keeps_the_old_tree_and_the_next_click_draws_again() {
    let source = COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / (2 - s.count)}\")");
    let plus = || click(&PLUS, "Button");
    let (tree, traps, end) = play(&source, vec![plus(), plus()]);
    assert_eq!(tree, counter("100"));
    assert_eq!(
        traps,
        ["counter: trap divide_by_zero at main.paka:20:16\n  100 / 0\n  in counter_view\n"]
    );
    assert_eq!(end, None);
    assert_eq!(
        play(&source, vec![plus(), plus(), plus()]).0,
        counter("-100")
    );
}

#[test]
fn a_first_view_that_traps_shows_one_error_node() {
    let source = COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / s.count}\")");
    let (tree, traps, end) = play(&source, vec![click(&PLUS, "Button")]);
    assert_eq!(tree, "Error \"trap divide_by_zero at main.paka:20:16\"\n");
    assert_eq!(traps.len(), 1);
    assert_eq!(end, None);
}

#[test]
fn the_start_message_is_handled_before_the_first_tree() {
    let source = COUNTER
        .replace("Text(\"{s.count}\")", "Text(\"{100 / s.count}\")")
        .replace("  update: update\n", "  update: update\n  start:  Inc\n");
    assert_eq!(play(&source, vec![]), (counter("100"), vec![], None));
}

#[test]
fn a_trapping_start_handler_leaves_the_defaults() {
    let source = COUNTER
        .replace("Inc => s.count += 1", "Inc => s.count = 1 / s.count")
        .replace("  update: update\n", "  update: update\n  start:  Inc\n");
    let (tree, traps, end) = play(&source, vec![]);
    assert_eq!((tree, traps.len(), end), (counter("0"), 1, None));
}

#[test]
fn a_trapping_handler_keeps_the_old_tree_and_the_next_click_is_answered() {
    let source = COUNTER.replace("Inc => s.count += 1", "Inc => s.count = 1 / s.count");
    let (tree, traps, end) = play(&source, vec![click(&PLUS, "Button")]);
    let report = "counter: trap divide_by_zero at main.paka:7:24\n  1 / 0\n  in update\n";
    assert_eq!(
        (tree, traps, end),
        (counter("0"), vec![report.into()], None)
    );
}

#[test]
fn a_trapping_handler_rolls_back_its_state_and_performs_none_of_its_effects() {
    let source = COUNTER
        .replace(
            "\
fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1",
            "\
fn update(s: inout Counter, m: Msg) -> List<Effect<Msg>> {
  match m {
    Inc => {
      s.count += 1
      return [Print(\"inc {s.count}\"), Print(\"{1 / (2 - s.count)}\")]
    }",
        )
        .replace("s.count - 1)\n  }\n}", "s.count - 1)\n  }\n  []\n}")
        .replace("  update(&s, Dec)", "  _ = update(&s, Dec)")
        .replace("grants: []", "grants: [Console]");
    let mut out = vec![];
    let plus = || click(&PLUS, "Button");
    let clicks = vec![plus(), plus(), plus(), click(&MINUS, "Button")];
    let (tree, traps, end) = play_in(&source, clicks, &mut out);
    assert_eq!((tree, traps.len(), end), (counter("0"), 2, None));
    assert_eq!(String::from_utf8(out).unwrap(), "inc 1\n1\n");
}

#[test]
fn exit_ends_the_app_and_drops_what_is_queued() {
    let source = COUNTER
        .replace(
            "\
fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}",
            "\
fn update(s: inout Counter, m: Msg) -> List<Effect<Msg>> {
  match m {
    Inc => s.count += 1
    Dec => {
      return [Exit]
    }
  }
  []
}",
        )
        .replace("  update(&s, Dec)", "  _ = update(&s, Dec)");
    let clicks = vec![
        click(&PLUS, "Button"),
        click(&MINUS, "Button"),
        click(&PLUS, "Button"),
    ];
    assert_eq!(
        play(&source, clicks),
        (counter("1"), vec![], Some((0, String::new())))
    );
}

const FORM: &str = "\
type Form { draft: String = \"\" }

enum Msg { Draft(text: String) }

fn update(s: inout Form, m: Msg) {
  match m {
    Draft(text) => if len(text) <= 3 { s.draft = text }
  }
}

view form_view(s: Form) -> View<Msg> {
  Col() {
    Input(s.draft, on_change: Draft, key: \"new\")
    Text(s.draft)
  }
}

app form {
  state:  Form
  update: update
  view:   form_view
  grants: []
}
";

#[test]
fn keys_reach_the_focused_input_and_its_cursor_follows_the_text_kept() {
    let typed = |text: &str| {
        text.chars()
            .map(|c| Event::Key(Key::Type(c)))
            .collect::<Vec<_>>()
    };
    let new = [Step::Str("new".into())];
    let mut events = typed("x");
    events.push(click(&new, "Input"));
    events.extend(typed("milk"));
    events.push(Event::Key(Key::Left(false)));
    let (tree, ..) = play(FORM, events.clone());
    let want = "\
Col
  Input 'new' \"mil\" cursor=2 on_change=Draft focus
  Text #1 \"mil\"
";
    assert_eq!(tree, want);
    events.push(Event::Key(Key::Back));
    let want = "\
Col
  Input 'new' \"ml\" cursor=1 on_change=Draft focus
  Text #1 \"ml\"
";
    assert_eq!(play(FORM, events).0, want);
}

#[test]
fn a_handler_that_traps_on_a_key_leaves_text_and_cursor_as_they_were() {
    let source = FORM.replace(
        "if len(text) <= 3 { s.draft = text }",
        "{\n      s.draft = text\n      _ = 1 / (4 - len(text))\n    }",
    );
    let mut events = vec![click(&[Step::Str("new".into())], "Input")];
    events.extend("milk".chars().map(|c| Event::Key(Key::Type(c))));
    let (tree, traps, end) = play(&source, events.clone());
    let want = "\
Col
  Input 'new' \"mil\" cursor=3 on_change=Draft focus
  Text #1 \"mil\"
";
    assert_eq!((tree.as_str(), traps.len(), end), (want, 1, None));
    events.push(Event::Key(Key::Left(false)));
    let want = "\
Col
  Input 'new' \"mil\" cursor=2 on_change=Draft focus
  Text #1 \"mil\"
";
    assert_eq!(play(&source, events).0, want);
}
