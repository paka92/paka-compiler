//! Visual apps with no window on dev code (M5-25): each script of paka/src/interp/
//! windowless_tests.rs gives the interpreter's tree text, trap count and end, the reports
//! compared by their first line, and frees all it made.

mod common;

use common::{Dev, source};
use paka::interp::Windowless;
use paka_rt::effects::Host;
use paka_rt::input::Key;
use paka_rt::route::Event;
use paka_rt::tree::Step;
use std::time::Duration;

const HEAP: u64 = 1 << 30;

const COUNTER: &str = "\
type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
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

const PLUS: [Step; 2] = [Step::Place(1), Step::Place(1)];
const MINUS: [Step; 2] = [Step::Place(1), Step::Place(0)];

fn click(path: &[Step], kind: &'static str) -> Event {
    Event::Click(path.to_vec(), kind)
}

fn first_line(report: &str) -> String {
    report.lines().next().unwrap_or_default().into()
}

/// The tree text, each trap report's first line and the end, its report's first line.
type Seen = (String, Vec<String>, Option<(u8, String)>);

/// Plays `events` on `text` in the interpreter and on dev code, checks they agree and the dev
/// run frees all, and gives what they saw.
fn play(text: &str, events: Vec<Event>) -> Seen {
    let (core, ..) = paka::check("main.paka", &source(text)).unwrap_or_else(|d| panic!("{d}"));
    let mut ui = Windowless::start(&core, &[], Host(vec![]));
    ui.send(events.clone());
    let end = ui.end.map(|(code, report)| (code, first_line(&report)));
    let want = (
        ui.tree.to_string(),
        ui.traps.iter().map(|r| first_line(r)).collect(),
        end,
    );

    let mut dev = Dev::default();
    dev.edit(&source(text)).unwrap();
    let played = dev.play(&[], &mut Host(vec![]), HEAP, Duration::MAX, events);
    let end = played.end.map(|end| {
        let (code, report) = end.expect("no interrupt");
        (code, first_line(&report))
    });
    let traps = played.traps.iter().map(|r| first_line(r)).collect();
    assert_eq!((played.tree, traps, end), want);
    assert_eq!(played.ledger, (0, 0));
    want
}

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

#[test]
fn the_counter_launches_at_zero_and_counts_clicks() {
    assert_eq!(play(COUNTER, vec![]).0, counter("0"));
    let plus = || click(&PLUS, "Button");
    assert_eq!(play(COUNTER, vec![plus(), plus()]).0, counter("2"));
    let clicks = vec![plus(), click(&MINUS, "Button"), click(&MINUS, "Button")];
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
    assert_eq!(play(COUNTER, clicks).0, counter("0"));
}

#[test]
fn a_trapping_view_keeps_the_old_tree_and_the_next_click_draws_again() {
    let source = COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / (2 - s.count)}\")");
    let plus = || click(&PLUS, "Button");
    let (tree, traps, _) = play(&source, vec![plus(), plus()]);
    assert_eq!((tree, traps.len()), (counter("100"), 1));
    assert_eq!(
        play(&source, vec![plus(), plus(), plus()]).0,
        counter("-100")
    );
}

#[test]
fn a_first_view_that_traps_shows_one_error_node() {
    let source = COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / s.count}\")");
    let tree = play(&source, vec![click(&PLUS, "Button")]).0;
    assert_eq!(tree, "Error \"trap divide_by_zero at main.paka:14:16\"\n");
}

#[test]
fn the_start_message_is_handled_and_a_trapping_one_leaves_the_defaults() {
    let start = |s: &str| s.replace("  update: update\n", "  update: update\n  start:  Inc\n");
    let source = start(&COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / s.count}\")"));
    assert_eq!(play(&source, vec![]).0, counter("100"));
    let source = start(&COUNTER.replace("Inc => s.count += 1", "Inc => s.count = 1 / s.count"));
    let (tree, traps, _) = play(&source, vec![]);
    assert_eq!((tree, traps.len()), (counter("0"), 1));
}

#[test]
fn a_trapping_handler_ends_the_app_with_the_last_tree() {
    let source = COUNTER.replace("Inc => s.count += 1", "Inc => s.count = 1 / s.count");
    let end = play(&source, vec![click(&PLUS, "Button")]).2;
    let report = "counter: trap divide_by_zero at main.paka:7:24";
    assert_eq!(end, Some((3, report.into())));
}

#[test]
fn exit_ends_the_app_and_drops_what_is_queued() {
    let source = COUNTER.replace(
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
    );
    let clicks = vec![
        click(&PLUS, "Button"),
        click(&MINUS, "Button"),
        click(&PLUS, "Button"),
    ];
    let (tree, _, end) = play(&source, clicks);
    assert_eq!((tree, end), (counter("1"), Some((0, String::new()))));
}

#[test]
fn keys_reach_the_focused_input_and_its_cursor_follows_the_text_kept() {
    let typed = |text: &str| {
        text.chars()
            .map(|c| Event::Key(Key::Type(c)))
            .collect::<Vec<_>>()
    };
    let mut events = typed("x");
    events.push(click(&[Step::Str("new".into())], "Input"));
    events.extend(typed("milk"));
    events.push(Event::Key(Key::Left(false)));
    let want = "\
Col
  Input 'new' \"mil\" cursor=2 on_change=Draft focus
  Text #1 \"mil\"
";
    assert_eq!(play(FORM, events.clone()).0, want);
    events.push(Event::Key(Key::Back));
    let want = "\
Col
  Input 'new' \"ml\" cursor=1 on_change=Draft focus
  Text #1 \"ml\"
";
    assert_eq!(play(FORM, events).0, want);
}
