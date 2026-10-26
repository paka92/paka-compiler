//! The Look tool on the windowless app (M5-34, docs/decisions/M5-09.md section 2): a run that
//! launches it, its tree, a screenshot, clicks and keys, and each refusal.

#[allow(dead_code)]
mod common;

use common::Scratch;
use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::source::read;
use paka_json::{Value, parse};
use std::fs;

const FORM: &str = r#"type S { count: Int = 0, text: String = "" }

enum Msg { Inc, Edit(text: String), Quit, Hang }

fn update(s: inout S, m: Msg) -> List<Effect<Msg>> {
  match m {
    Inc => {
      s.count += 1
      []
    }
    Edit(text) => {
      s.text = text
      []
    }
    Quit => [Exit]
    Hang => ??hang
  }
}

view v(s: S) -> View<Msg> {
  Col(gap: 8) {
    Text("{s.count}")
    Button("+", on: Inc)
    Input(s.text, on_change: Edit, key: "in")
    Button("quit", on: Quit)
    Button("hang", on: Hang)
  }
}

app form {
  state:  S
  update: update
  view:   v
  grants: []
}
"#;

const FIRST: &str = "\
Col gap=8
  Text #0 \"0\"
  Button #1 \"+\" on=Inc
  Input 'in' \"\" cursor=0 on_change=Edit
  Button #3 \"quit\" on=Quit
  Button #4 \"hang\" on=Hang
";

fn session(scratch: &Scratch) -> Session {
    let file = scratch.path("main.paka");
    fs::write(&file, FORM).unwrap();
    let (source, _) = read(file.as_os_str());
    let workspace = session::workspace(&source.text).unwrap();
    let lock = fs::File::open(&file).unwrap();
    Session::new(
        file.as_os_str(),
        &source,
        workspace,
        lock,
        Runner::new("".into()),
    )
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

fn field<'v>(reply: &'v Value, key: &str) -> &'v Value {
    reply
        .get(key)
        .unwrap_or_else(|| panic!("no `{key}` in {reply}"))
}

fn tree(reply: &Value) -> &str {
    field(reply, "tree").as_str().unwrap()
}

fn refused(s: &mut Session, name: &str, fields: Vec<(&str, Value)>, kind: &str) {
    let reply = call(s, name, fields);
    let error = field(&reply, "error");
    assert_eq!(field(error, "kind").as_str(), Some(kind), "{reply}");
}

#[test]
fn look_drives_the_windowless_app() {
    const INNER: &str = "PAKA_LOOK_NO_DISPLAY";
    if std::env::var_os(INNER).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "look_drives_the_windowless_app"])
            .env(INNER, "1")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let scratch = Scratch::new("look");
    let s = &mut session(&scratch);
    refused(s, "look_tree", vec![], "not_running");

    let run = field(&call(s, "run", vec![("app", "form".into())]), "run").clone();
    assert_eq!(tree(&run), FIRST, "{run}");
    assert_eq!(field(&run, "window"), &Value::Bool(false));
    assert_eq!(field(&run, "status"), &Value::Null);
    assert_eq!(tree(&call(s, "look_tree", vec![])), FIRST);

    for scale in [1usize, 2] {
        let shot = call(s, "look_screenshot", vec![("scale", scale.into())]);
        let png = field(&shot, "png").as_str().unwrap();
        assert!(png.starts_with("iVBORw0KGgo"), "a PNG signature");
        assert_eq!(field(&shot, "width").as_u64(), Some(480 * scale as u64));
        assert_eq!(field(&shot, "height").as_u64(), Some(360 * scale as u64));
    }
    refused(
        s,
        "look_screenshot",
        vec![("scale", 3usize.into())],
        "bad_request",
    );

    let clicked = call(s, "look_click", vec![("path", "#1".into())]);
    assert!(tree(&clicked).contains("Text #0 \"1\""), "{clicked}");
    assert_eq!(field(&clicked, "running"), &Value::Bool(true));
    assert_eq!(field(&clicked, "trap"), &Value::Null);
    let plain = call(s, "look_click", vec![("path", "#0".into())]);
    assert!(tree(&plain).contains("Text #0 \"1\""), "{plain}");

    refused(s, "look_key", vec![("text", "a".into())], "no_focus");
    call(s, "look_click", vec![("path", "'in'".into())]);
    call(s, "look_key", vec![("text", "héj".into())]);
    let keyed = call(s, "look_key", vec![("key", "Backspace".into())]);
    assert!(tree(&keyed).contains("\"hé\" cursor=2"), "{keyed}");
    let moved = call(s, "look_key", vec![("key", "Shift+Home".into())]);
    assert!(tree(&moved).contains("cursor=0 sel=0..2"), "{moved}");

    refused(s, "look_click", vec![("path", "#9".into())], "no_element");
    refused(s, "look_click", vec![("path", "#".into())], "bad_request");
    refused(s, "look_key", vec![("key", "F1".into())], "bad_request");
    refused(s, "look_key", vec![], "bad_request");
    let both = vec![("text", "a".into()), ("key", "Tab".into())];
    refused(s, "look_key", both, "bad_request");

    let trapped = call(s, "look_click", vec![("path", "#4".into())]);
    let trap = field(&trapped, "trap");
    assert_eq!(field(trap, "kind").as_str(), Some("hole"), "{trapped}");
    assert_eq!(field(trap, "hole").as_str(), Some("hang"), "{trapped}");
    assert_eq!(field(&trapped, "running"), &Value::Bool(true));

    let ended = call(s, "look_click", vec![("path", "#3".into())]);
    assert_eq!(field(&ended, "running"), &Value::Bool(false), "{ended}");
    refused(s, "look_tree", vec![], "not_running");

    let again = field(&call(s, "run", vec![("app", "form".into())]), "run").clone();
    assert_eq!(tree(&again), FIRST);
}
