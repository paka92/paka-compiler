//! Edits while a visual app runs (M5-28, docs/decisions/M5-07.md): a change to a stored type,
//! reached by field, list item, map value or enum payload, is refused unless a state choice is
//! carried or waiting, and the app keeps answering clicks on the version it runs. An installed
//! version swaps into the app, keeping its state, with a reset launching it again (M5-29), or
//! with a migrate converting it (M5-30), and one that does not finish is stopped (M5-30b).

#[allow(dead_code)]
mod common;

use common::Scratch;
use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::source::read;
use paka_json::{Value, parse};
use paka_rt::route::Event;
use paka_rt::tree::Step;
use std::fs;

const BOARD: &str = "\
type Todo { title: String = \"\", done: Bool = false }

type Tag { name: String = \"\" }

type Note { text: String = \"\" }

enum Mode { Plain, Noted(note: Note) }

type Board {
  count: Int = 0,
  items: List<Todo> = [],
  tags: Map<String, Tag> = [:],
  mode: Mode = Plain
}

enum Msg { Inc }

fn update(s: inout Board, m: Msg) {
  match m {
    Inc => s.count += 1
  }
}

view board_view(s: Board) -> View<Msg> {
  Col(gap: 8) {
    Text(\"{s.count}\")
    Button(\"+\", on: Inc)
  }
}

app board {
  state:  Board
  update: update
  view:   board_view
  grants: []
}
";

const BOARD_TYPE: &str = "\
type Board {
  count: Int = 0,
  items: List<Todo> = [],
  tags: Map<String, Tag> = [:],
  mode: Mode = Plain
}";

fn session(scratch: &Scratch) -> Session {
    let file = scratch.path("main.paka");
    fs::write(&file, BOARD).unwrap();
    let (source, _) = read(file.as_os_str());
    let workspace = session::workspace(&source.text).unwrap();
    let lock = fs::File::open(&file).unwrap();
    let runner = Runner::new("".into());
    Session::new(file.as_os_str(), &source, workspace, lock, runner)
}

fn launch(s: &mut Session) -> String {
    s.launch(vec![]).unwrap_or_else(|f| panic!("{}", f.message))
}

/// The reply to `call` with `fields`.
fn call(s: &mut Session, call: &str, fields: Vec<(&str, Value)>) -> Value {
    let head = [
        ("v", 1usize.into()),
        ("id", 1usize.into()),
        ("call", call.into()),
    ];
    let line = format!("{}\n", Value::object(head.into_iter().chain(fields)));
    let mut out = vec![];
    serve(line.as_bytes(), &mut out, |c| s.handle(c)).unwrap();
    parse(String::from_utf8(out).unwrap().trim_end()).unwrap()
}

/// The reply to an edit of `name` to `text`, carrying `state` if any.
fn edit(s: &mut Session, name: &str, text: &str, state: Option<&str>) -> Value {
    let viewed = call(s, "view", vec![("name", name.into())]);
    let base = viewed.get("version").unwrap().clone();
    let mut fields = vec![("name", name.into()), ("base", base), ("text", text.into())];
    fields.extend(state.map(|c| ("state", c.into())));
    call(s, "edit", fields)
}

/// Clicks `+` and gives the count shown.
fn click(s: &mut Session) -> String {
    let tree = s.send(vec![Event::Click(vec![Step::Place(1)], "Button")]);
    count(&tree.expect("the app runs"))
}

fn count(tree: &str) -> String {
    let line = tree
        .lines()
        .find(|l| l.trim_start().starts_with("Text"))
        .unwrap();
    line.rsplit(' ').next().unwrap().trim_matches('"').into()
}

fn str_of<'v>(reply: &'v Value, key: &str) -> &'v str {
    reply.get(key).and_then(Value::as_str).unwrap()
}

/// Refuses the edit, naming `changed`, and leaves the app answering clicks.
fn refused(s: &mut Session, name: &str, text: &str, changed: &str, clicks: &mut usize) {
    let reply = edit(s, name, text, None);
    let error = reply.get("error").unwrap_or_else(|| panic!("{reply}"));
    assert_eq!(str_of(error, "kind"), "state_changed", "{reply}");
    let lines = error.get("changed").unwrap().to_string();
    assert!(lines.contains(changed), "{changed} not in {lines}");
    *clicks += 1;
    assert_eq!(click(s), clicks.to_string());
}

#[test]
fn a_stored_type_change_is_refused_while_the_app_runs() {
    let scratch = Scratch::new("state-refused");
    let mut s = session(&scratch);
    assert_eq!(count(&launch(&mut s)), "0");
    let mut clicks = 0;
    let state = BOARD_TYPE.replace("count: Int = 0,", "count: Int = 0,\n  step: Int = 1,");
    refused(
        &mut s,
        "Board",
        &state,
        "Board: field step added",
        &mut clicks,
    );
    let list = "type Todo { title: String = \"\", done: Int = 0 }";
    let changed = "Todo: field done is now Int, was Bool";
    refused(&mut s, "Todo", list, changed, &mut clicks);
    let map = "type Tag { label: String = \"\" }";
    let changed = "Tag: field name removed, field label added";
    refused(&mut s, "Tag", map, changed, &mut clicks);
    let payload = "type Note { text: String = \"\", at: Int = 0 }";
    refused(&mut s, "Note", payload, "Note: field at added", &mut clicks);
    let variant = "enum Mode { Plain, Noted(memo: Note) }";
    let changed = "Mode: variant Noted now holds (memo: Note), was (note: Note)";
    refused(&mut s, "Mode", variant, changed, &mut clicks);
    let added = "enum Mode { Plain, Noted(note: Note), Done }";
    refused(
        &mut s,
        "Mode",
        added,
        "Mode: variant Done added",
        &mut clicks,
    );
    let moved = BOARD_TYPE.replace("  count: Int = 0,\n", "");
    let moved = moved.replace(
        "mode: Mode = Plain",
        "mode: Mode = Plain,\n  count: Int = 0",
    );
    refused(
        &mut s,
        "Board",
        &moved,
        "Board: fields reordered",
        &mut clicks,
    );
    let app = BOARD.split("app board").nth(1).unwrap();
    let app = format!("app board{}", app.replace("Board", "Todo"));
    let changed = "state: the app's state type is now Todo, was Board";
    refused(&mut s, "board", app.trim_end(), changed, &mut clicks);
    let file = fs::read_to_string(scratch.path("main.paka")).unwrap();
    assert_eq!(file, BOARD, "a refused edit leaves the source");
}

#[test]
fn defaults_comments_and_other_types_keep_the_state() {
    let scratch = Scratch::new("state-kept");
    let mut s = session(&scratch);
    launch(&mut s);
    let defaults = BOARD_TYPE.replace("count: Int = 0,", "count: Int = 5,");
    let commented = "type Tag {\n  // shown on the board\n  name: String = \"\"\n}";
    for (name, text) in [("Board", defaults.as_str()), ("Tag", commented)] {
        let reply = edit(&mut s, name, text, None);
        assert_eq!(reply.get("installed"), Some(&true.into()), "{reply}");
        assert_eq!(str_of(&reply, "state"), "kept");
    }
    let add = "fn twice(n: Int) -> Int {\n  n * 2\n}";
    let fields = vec![
        ("name", "twice".into()),
        ("base", Value::Null),
        ("text", add.into()),
    ];
    let reply = call(&mut s, "edit", fields);
    assert_eq!(str_of(&reply, "state"), "kept", "{reply}");
    assert_eq!(click(&mut s), "1");
}

const MIGRATE: &str = "migrate(old: Board) -> Board {\n  Board { count: old.count }\n}";

fn with_step() -> String {
    BOARD_TYPE.replace("count: Int = 0,", "count: Int = 0,\n  step: Int = 10,")
}

const UPDATE: &str = "\
fn update(s: inout Board, m: Msg) {
  match m {
    Inc => s.count += s.step
  }
}";

#[test]
fn the_refused_edit_resent_with_a_migrate_installs_and_keeps_the_count() {
    let scratch = Scratch::new("state-migrated");
    let mut s = session(&scratch);
    launch(&mut s);
    assert_eq!((click(&mut s), click(&mut s)), ("1".into(), "2".into()));
    let reply = edit(&mut s, "Board", &with_step(), None);
    assert_eq!(str_of(reply.get("error").unwrap(), "kind"), "state_changed");
    let reply = edit(&mut s, "Board", &with_step(), Some(MIGRATE));
    assert_eq!(reply.get("installed"), Some(&true.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "migrated");
    assert_eq!(click(&mut s), "3");
    let reply = edit(&mut s, "update", UPDATE, None);
    assert_eq!(
        str_of(&reply, "state"),
        "kept",
        "the migrate is used up: {reply}"
    );
    assert_eq!(click(&mut s), "13", "the new field holds its default");
}

#[test]
fn a_trapping_migrate_leaves_the_old_version_running() {
    let scratch = Scratch::new("state-migrate-trapped");
    let mut s = session(&scratch);
    launch(&mut s);
    click(&mut s);
    let trapping = "migrate(old: Board) -> Board {\n  Board { count: 1 / (old.count - 1) }\n}";
    let reply = edit(&mut s, "Board", &with_step(), Some(trapping));
    assert_eq!(reply.get("installed"), Some(&false.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "migrate_trapped");
    let trap = str_of(&reply, "trap");
    assert!(
        trap.starts_with("board: trap divide_by_zero at migrate:2:"),
        "{trap}"
    );
    assert_eq!(click(&mut s), "2", "the old code counts on");
    assert_eq!(click(&mut s), "3", "the migrate is not run again by itself");
    let reply = edit(&mut s, "Board", &with_step(), Some(MIGRATE));
    assert_eq!(str_of(&reply, "state"), "migrated", "{reply}");
    assert_eq!(click(&mut s), "4");
}

#[test]
fn a_migrate_that_does_not_finish_is_stopped_at_the_run_limit_and_counts_as_trapped() {
    let scratch = Scratch::new("state-migrate-endless");
    let mut s = session(&scratch);
    launch(&mut s);
    click(&mut s);
    let endless = "\
migrate(old: Board) -> Board {
  var n = old.count
  while n >= 0 { n += 1 }
  Board { count: n }
}";
    let viewed = call(&mut s, "view", vec![("name", "Board".into())]);
    let run = Value::object([("app", "board".into()), ("limit_ms", 100usize.into())]);
    let fields = vec![
        ("name", "Board".into()),
        ("base", viewed.get("version").unwrap().clone()),
        ("text", with_step().into()),
        ("state", endless.into()),
        ("run", run),
    ];
    let reply = call(&mut s, "edit", fields);
    assert_eq!(reply.get("installed"), Some(&false.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "migrate_trapped");
    assert_eq!(
        str_of(&reply, "trap"),
        "board: stopped time in migrate, past 100 ms\n"
    );
    assert_eq!(reply.get("run"), Some(&Value::Null));
    assert_eq!(click(&mut s), "2", "the old code counts on");
    let reply = edit(&mut s, "Board", &with_step(), Some(MIGRATE));
    assert_eq!(str_of(&reply, "state"), "migrated", "{reply}");
    assert_eq!(click(&mut s), "3");
}

#[test]
fn a_migrate_that_fails_its_check_blocks_the_install() {
    let scratch = Scratch::new("state-migrate-wrong");
    let mut s = session(&scratch);
    launch(&mut s);
    click(&mut s);
    let wrong = "migrate(old: Todo) -> Board {\n  Board {}\n}";
    let reply = edit(&mut s, "Board", &with_step(), Some(wrong));
    assert_eq!(reply.get("installed"), Some(&false.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "waiting");
    assert_eq!(reply.get("broke").unwrap().to_string(), r#"["migrate"]"#);
    let errors = reply.get("errors").unwrap().to_string();
    assert!(errors.contains("bad-migrate-signature"), "{errors}");
    assert_eq!(click(&mut s), "2");
    let reply = edit(&mut s, "update", UPDATE, None);
    assert_eq!(str_of(&reply, "state"), "waiting", "{reply}");
    assert_eq!(reply.get("broke").unwrap().to_string(), r#"["migrate"]"#);
    assert_eq!(click(&mut s), "3");
    let reply = edit(&mut s, "Board", &with_step(), Some("cleanup"));
    assert_eq!(str_of(reply.get("error").unwrap(), "kind"), "bad_request");
    let reply = edit(&mut s, "Board", &with_step(), Some(MIGRATE));
    assert_eq!(str_of(&reply, "state"), "migrated", "{reply}");
    assert_eq!(click(&mut s), "13");
}

#[test]
fn with_nothing_running_a_choice_is_ignored() {
    let scratch = Scratch::new("state-ignored");
    let mut s = session(&scratch);
    let state = BOARD_TYPE.replace("count: Int = 0,", "count: Int = 0,\n  step: Int = 1,");
    let reply = edit(&mut s, "Board", &state, Some("reset"));
    assert_eq!(reply.get("installed"), Some(&true.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "ignored");
    let reply = edit(&mut s, "Board", BOARD_TYPE, None);
    assert_eq!(reply.get("state"), None, "{reply}");
    assert!(s.send(vec![]).is_none());
}

/// Adds definition `text` named `name`.
fn add(s: &mut Session, name: &str, text: &str) -> Value {
    let fields = vec![
        ("name", name.into()),
        ("base", Value::Null),
        ("text", text.into()),
    ];
    call(s, "edit", fields)
}

#[test]
fn an_installed_edit_swaps_into_the_running_app_keeping_its_state() {
    let scratch = Scratch::new("state-swapped");
    let mut s = session(&scratch);
    launch(&mut s);
    click(&mut s);
    let view = BOARD
        .split("view board_view")
        .nth(1)
        .unwrap()
        .split("app board")
        .next();
    let view = format!("view board_view{}", view.unwrap()).replace("{s.count}", "n {s.count}");
    let reply = edit(&mut s, "board_view", view.trim_end(), None);
    assert_eq!(str_of(&reply, "state"), "kept", "{reply}");
    let tree = s.send(vec![]).unwrap();
    assert!(tree.contains("\"n 1\""), "{tree}");
}

#[test]
fn the_refused_edit_resent_with_a_reset_installs_and_launches_again() {
    let scratch = Scratch::new("state-reset");
    let mut s = session(&scratch);
    let launched = launch(&mut s);
    assert_eq!((click(&mut s), click(&mut s)), ("1".into(), "2".into()));
    let state = BOARD_TYPE.replace("count: Int = 0,", "count: Int = 0,\n  step: Int = 1,");
    let reply = edit(&mut s, "Board", &state, None);
    assert_eq!(str_of(reply.get("error").unwrap(), "kind"), "state_changed");
    let reply = edit(&mut s, "Board", &state, Some("reset"));
    assert_eq!(reply.get("installed"), Some(&true.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "reset");
    assert_eq!(s.send(vec![]).unwrap(), launched);
    assert_eq!(click(&mut s), "1");
    let reply = edit(&mut s, "Board", &state, None);
    assert_eq!(
        str_of(&reply, "state"),
        "kept",
        "the reset is used up: {reply}"
    );
}

#[test]
fn a_reset_comes_alone_or_waits_until_the_program_checks() {
    let scratch = Scratch::new("state-reset-waits");
    let mut s = session(&scratch);
    launch(&mut s);
    click(&mut s);
    let reply = edit(&mut s, "Board", BOARD_TYPE, Some("reset"));
    assert_eq!(str_of(&reply, "state"), "reset", "{reply}");
    assert_eq!(click(&mut s), "1");
    let broken = BOARD_TYPE.replace(
        "count: Int = 0,",
        "count: Int = 0,\n  flag: Flag = Flag {},",
    );
    let reply = edit(&mut s, "Board", &broken, Some("reset"));
    assert_eq!(str_of(&reply, "state"), "waiting", "{reply}");
    assert_eq!(click(&mut s), "2");
    let reply = add(&mut s, "Flag", "type Flag { on: Bool = false }");
    assert_eq!(reply.get("installed"), Some(&true.into()), "{reply}");
    assert_eq!(str_of(&reply, "state"), "reset");
    assert_eq!(click(&mut s), "1");
}
