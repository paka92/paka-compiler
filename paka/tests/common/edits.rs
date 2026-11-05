//! Edits while the counter runs, through Look (docs/decisions/M5-07.md): a bare change to its
//! stored type is refused and the old version counts on; other edits swap in keeping the count; a
//! reset launches it again; a migrate converts the count, and one that fails its check, traps or
//! does not finish keeps the old version running. Run on the windowless app by
//! paka/tests/look.rs and on the app in a window by installer/tests/window.rs (M5-42).

use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::source::read;
use paka_json::{Value, parse};
use std::fs;
use std::path::Path;

const COUNTER: &str = "type Counter { count: Int = 0 }

enum Msg { Inc }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
  }
}

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text(\"{s.count}\")
    Button(\"+\", on: Inc)
  }
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}
";

const OLD: &str = "type Counter { count: Int = 0 }";
const STEP: &str = "type Counter { count: Int = 0, step: Int = 1 }";
const MIGRATE: &str = "migrate(old: Counter) -> Counter {\n  Counter { count: old.count }\n}";

/// A session on the counter, written to `main.paka` in `dir`, its runs going to `runner`.
pub fn session(dir: &Path, runner: Runner) -> Session {
    let file = dir.join("main.paka");
    fs::write(&file, COUNTER).unwrap();
    let (source, _) = read(file.as_os_str());
    let workspace = session::workspace(&source.text).unwrap();
    let lock = fs::File::open(&file).unwrap();
    Session::new(file.as_os_str(), &source, workspace, lock, runner)
}

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

fn text<'v>(reply: &'v Value, key: &str) -> &'v str {
    let field = reply.get(key).and_then(Value::as_str);
    field.unwrap_or_else(|| panic!("no `{key}` in {reply}"))
}

/// The count the app shows.
fn shown(s: &mut Session) -> String {
    let tree = call(s, "look_tree", vec![]);
    let line = text(&tree, "tree").lines().find(|l| l.contains("Text #0"));
    line.unwrap().split('"').nth(1).unwrap().into()
}

/// Clicks `+`; the count shown after.
fn click(s: &mut Session) -> String {
    call(s, "look_click", vec![("path", "#1".into())]);
    shown(s)
}

/// The reply to an edit of `name` to `text`, carrying `state` and `run` if given.
fn edit(s: &mut Session, name: &str, text: &str, state: Option<&str>, run: Option<Value>) -> Value {
    let viewed = call(s, "view", vec![("name", name.into())]);
    let base = viewed.get("version").unwrap().clone();
    let mut fields = vec![("name", name.into()), ("base", base), ("text", text.into())];
    fields.extend(state.map(|c| ("state", c.into())));
    fields.extend(run.map(|r| ("run", r)));
    call(s, "edit", fields)
}

/// Asserts `reply` installed or not, with the state it says became of the count.
fn took(reply: &Value, installed: bool, state: &str) {
    assert_eq!(reply.get("installed"), Some(&installed.into()), "{reply}");
    assert_eq!(text(reply, "state"), state, "{reply}");
}

/// Drives the counter through edits; `window` is whether its run opens a window.
pub fn drive(s: &mut Session, window: bool) {
    let run = call(s, "run", vec![("app", "counter".into())]);
    assert_eq!(
        run.get("run").and_then(|r| r.get("window")),
        Some(&window.into()),
        "{run}"
    );
    assert_eq!(click(s), "1");

    let refused = edit(s, "Counter", STEP, None, None);
    let error = refused.get("error").unwrap_or_else(|| panic!("{refused}"));
    assert_eq!(text(error, "kind"), "state_changed", "{refused}");
    assert_eq!(click(s), "2", "the old version counts on");

    let view = text(
        &call(s, "view", vec![("name", "counter_view".into())]),
        "text",
    )
    .replace("Button(\"+\"", "Button(\"add\"");
    took(&edit(s, "counter_view", &view, None, None), true, "kept");
    let tree = call(s, "look_tree", vec![]);
    assert!(text(&tree, "tree").contains("Button #1 \"add\""), "{tree}");
    assert_eq!(shown(s), "2", "the swap keeps the count");

    took(
        &edit(s, "Counter", STEP, Some("reset"), None),
        true,
        "reset",
    );
    assert_eq!(shown(s), "0", "the reset launches again");
    took(&edit(s, "Counter", OLD, Some("reset"), None), true, "reset");
    click(s);
    assert_eq!(click(s), "2");

    let wrong = "migrate(old: Msg) -> Counter {\n  Counter {}\n}";
    took(
        &edit(s, "Counter", STEP, Some(wrong), None),
        false,
        "waiting",
    );
    assert_eq!(
        click(s),
        "3",
        "a migrate that fails its check keeps the old version"
    );

    let trapping =
        "migrate(old: Counter) -> Counter {\n  Counter { count: 1 / (old.count - 3) }\n}";
    let trapped = edit(s, "Counter", STEP, Some(trapping), None);
    took(&trapped, false, "migrate_trapped");
    let trap = text(&trapped, "trap");
    assert!(
        trap.starts_with("counter: trap divide_by_zero at migrate:2:"),
        "{trap}"
    );
    assert_eq!(click(s), "4", "a migrate that traps keeps the old version");

    let endless = "migrate(old: Counter) -> Counter {\n  var n = old.count\n  while n >= 0 { n += 1 }\n  Counter { count: n }\n}";
    let run = Value::object([("app", "counter".into()), ("limit_ms", 100usize.into())]);
    let stopped = edit(s, "Counter", STEP, Some(endless), Some(run));
    took(&stopped, false, "migrate_trapped");
    assert_eq!(
        text(&stopped, "trap"),
        "counter: stopped time in migrate, past 100 ms\n"
    );
    assert_eq!(
        click(s),
        "5",
        "a migrate stopped at the limit keeps the old version"
    );

    took(
        &edit(s, "Counter", STEP, Some(MIGRATE), None),
        true,
        "migrated",
    );
    assert_eq!(shown(s), "5", "the migrate keeps the count");
    assert_eq!(click(s), "6");
}
