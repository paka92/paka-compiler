//! Visual apps with no window on dev code (M5-25): each script of paka/src/interp/
//! windowless_tests.rs gives the interpreter's tree text, trap count, end and printed text, the
//! reports compared by their first line, and frees all it made. So does a code swap between
//! turns (M5-27), and a reset (M5-29).

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

/// The tree text, each trap report's first line, the end, its report's first line, and the
/// printed text.
type Seen = (String, Vec<String>, Option<(u8, String)>, String);

/// Plays `events` on `text` in the interpreter and on dev code, checks they agree and the dev
/// run frees all, and gives what they saw.
fn play(text: &str, events: Vec<Event>) -> Seen {
    let (core, ..) = paka::check("main.paka", &source(text)).unwrap_or_else(|d| panic!("{d}"));
    let mut out = vec![];
    let mut ui = Windowless::start(core.into(), &[], Host(&mut out));
    ui.send(events.clone());
    let end = ui.end.map(|(code, report)| (code, first_line(&report)));
    let (tree, traps) = (ui.tree.to_string(), ui.traps.iter().map(|r| first_line(r)));
    let want = (tree, traps.collect(), end, String::from_utf8(out).unwrap());

    let mut dev = Dev::default();
    dev.edit(&source(text)).unwrap();
    let mut out = vec![];
    let played = dev.play(&[], &mut Host(&mut out), HEAP, Duration::MAX, events);
    let end = played.end.map(|end| {
        let (code, report) = end.expect("no interrupt");
        (code, first_line(&report))
    });
    let traps = played.traps.iter().map(|r| first_line(r)).collect();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!((played.tree, traps, end, printed), want);
    assert_eq!(played.ledger, (0, 0));
    want
}

/// Plays `events` on `text`, then queues `queued` and swaps to `next` with the same stored types,
/// in the interpreter and on dev code; checks they agree and the dev run frees all, and gives
/// what they saw.
fn swap(text: &str, events: Vec<Event>, queued: Vec<Event>, next: &str) -> Seen {
    swapped(text, events, queued, next, false)
}

/// As `swap`, launching `next` again on a reset.
fn reset(text: &str, events: Vec<Event>, queued: Vec<Event>, next: &str) -> Seen {
    swapped(text, events, queued, next, true)
}

fn swapped(text: &str, events: Vec<Event>, queued: Vec<Event>, next: &str, reset: bool) -> Seen {
    let checked = |text| paka::check("main.paka", &source(text)).unwrap_or_else(|d| panic!("{d}"));
    let ((core, ..), (after, ..)) = (checked(text), checked(next));
    let mut out = vec![];
    let mut ui = Windowless::start(core.into(), &[], Host(&mut out));
    ui.send(events.clone());
    ui.queue(queued.clone());
    if reset {
        ui.reset(after.into(), &[]);
    } else {
        ui.swap(after.into());
    }
    let end = ui.end.map(|(code, report)| (code, first_line(&report)));
    let (tree, traps) = (ui.tree.to_string(), ui.traps.iter().map(|r| first_line(r)));
    let want = (tree, traps.collect(), end, String::from_utf8(out).unwrap());

    let (mut dev, mut next_dev) = (Dev::default(), Dev::default());
    dev.edit(&source(text)).unwrap();
    next_dev.edit(&source(next)).unwrap();
    let mut out = vec![];
    let world = &mut Host(&mut out);
    let played = dev.play_swap(
        &[],
        world,
        HEAP,
        Duration::MAX,
        events,
        queued,
        &next_dev,
        reset,
    );
    let end = played.end.map(|end| {
        let (code, report) = end.expect("no interrupt");
        (code, first_line(&report))
    });
    let traps = played.traps.iter().map(|r| first_line(r)).collect();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!((played.tree, traps, end, printed), want);
    assert_eq!(played.ledger, (0, 0));
    want
}

/// The counter with `??reset` made a button, and its message first, so an old message's tag
/// means `Reset` in the new code.
fn with_reset() -> String {
    COUNTER
        .replace("enum Msg { Inc, Dec }", "enum Msg { Reset, Inc, Dec }")
        .replace(
            "    Inc => s.count += 1",
            "    Reset => s.count = 0\n    Inc => s.count += 1",
        )
        .replace("??reset", "Button(\"reset\", on: Reset)")
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
    let (tree, traps, ..) = play(&source, vec![plus(), plus()]);
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
    let (tree, traps, ..) = play(&source, vec![]);
    assert_eq!((tree, traps.len()), (counter("0"), 1));
}

#[test]
fn a_trapping_handler_keeps_the_old_tree_and_the_next_click_is_answered() {
    let source = COUNTER.replace("Inc => s.count += 1", "Inc => s.count = 1 / s.count");
    let (tree, traps, end, _) = play(&source, vec![click(&PLUS, "Button")]);
    let report = "counter: trap divide_by_zero at main.paka:7:24";
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
        .replace("grants: []", "grants: [Console]");
    let plus = || click(&PLUS, "Button");
    let clicks = vec![plus(), plus(), plus(), click(&MINUS, "Button")];
    let (tree, traps, end, printed) = play(&source, clicks);
    assert_eq!((tree, traps.len(), end), (counter("0"), 2, None));
    assert_eq!(printed, "inc 1\n1\n");
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
    let (tree, _, end, _) = play(&source, clicks);
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

#[test]
fn a_handler_that_traps_on_a_key_leaves_text_and_cursor_as_they_were() {
    let source = FORM.replace(
        "if len(text) <= 3 { s.draft = text }",
        "{\n      s.draft = text\n      _ = 1 / (4 - len(text))\n    }",
    );
    let mut events = vec![click(&[Step::Str("new".into())], "Input")];
    events.extend("milk".chars().map(|c| Event::Key(Key::Type(c))));
    let (tree, traps, end, _) = play(&source, events.clone());
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

fn reset_counter(count: &str) -> String {
    counter(count).replace("Hole #2 reset", "Button #2 \"reset\" on=Reset")
}

#[test]
fn a_swap_shows_the_new_button_and_keeps_the_count() {
    let plus = || click(&PLUS, "Button");
    let next = with_reset();
    assert_eq!(
        swap(COUNTER, vec![plus(), plus()], vec![], &next).0,
        reset_counter("2")
    );
    let reset = click(&[Step::Place(2)], "Button");
    let queued = vec![plus(), reset, plus()];
    assert_eq!(
        swap(COUNTER, vec![plus()], queued, &next).0,
        reset_counter("1")
    );
}

#[test]
fn a_click_queued_before_a_swap_is_routed_through_the_new_tree() {
    let plus = || click(&PLUS, "Button");
    let hole = click(&[Step::Place(2)], "Hole");
    let (tree, traps, end, _) = swap(COUNTER, vec![plus()], vec![plus(), hole], &with_reset());
    assert_eq!((tree, traps.len(), end), (reset_counter("2"), 0, None));
}

#[test]
fn a_view_that_traps_at_a_swap_shows_one_error_node_and_sends_nothing() {
    let next = COUNTER.replace("Text(\"{s.count}\")", "Text(\"{100 / (2 - s.count)}\")");
    let plus = || click(&PLUS, "Button");
    let (tree, traps, ..) = swap(COUNTER, vec![plus(), plus()], vec![plus()], &next);
    let error = "Error \"trap divide_by_zero at main.paka:14:16\"\n";
    assert_eq!((tree.as_str(), traps.len()), (error, 1));
}

#[test]
fn a_swap_keeps_the_focused_input_and_its_cursor() {
    let mut events = vec![click(&[Step::Str("new".into())], "Input")];
    events.extend("mi".chars().map(|c| Event::Key(Key::Type(c))));
    events.push(Event::Key(Key::Left(false)));
    let next = FORM.replace("Text(s.draft)", "Text(\"draft: {s.draft}\")");
    let queued = vec![Event::Key(Key::Type('l'))];
    let want = "\
Col
  Input 'new' \"mli\" cursor=2 on_change=Draft focus
  Text #1 \"draft: mli\"
";
    assert_eq!(swap(FORM, events, queued, &next).0, want);
}

#[test]
fn a_reset_installs_a_state_change_and_launches_again_dropping_the_queue() {
    let next = COUNTER
        .replace("{ count: Int = 0 }", "{ count: Int = 0, step: Int = 1 }")
        .replace("Inc => s.count += 1", "Inc => s.count += s.step")
        .replace("  update: update\n", "  update: update\n  start:  Inc\n");
    let plus = || click(&PLUS, "Button");
    let (tree, traps, end, _) = reset(COUNTER, vec![plus(), plus()], vec![plus(), plus()], &next);
    assert_eq!((tree, traps.len(), end), (counter("1"), 0, None));
}

#[test]
fn a_reset_drops_focus_and_element_state() {
    let mut events = vec![click(&[Step::Str("new".into())], "Input")];
    events.extend("mi".chars().map(|c| Event::Key(Key::Type(c))));
    let queued = vec![Event::Key(Key::Type('l'))];
    let launched = play(FORM, vec![]).0;
    assert!(launched.contains("Input 'new' \"\" cursor=0 on_change=Draft\n"));
    assert_eq!(reset(FORM, events, queued, FORM).0, launched);
}
