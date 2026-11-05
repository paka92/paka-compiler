//! The M5-34 Look script, run on the windowless app by paka/tests/look.rs and on the app in a
//! window by installer/tests/window.rs (M5-38).

use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::source::read;
use paka_json::{Value, parse};
use std::fs;
use std::path::Path;

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

/// A session on the form, written to `main.paka` in `dir`, its runs going to `runner`.
pub fn session(dir: &Path, runner: Runner) -> Session {
    let file = dir.join("main.paka");
    fs::write(&file, FORM).unwrap();
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

/// Drives the form through Look as M5-34 does; `window` is whether its runs open a window.
pub fn drive(s: &mut Session, window: bool) {
    refused(s, "look_tree", vec![], "not_running");

    let run = field(&call(s, "run", vec![("app", "form".into())]), "run").clone();
    assert_eq!(tree(&run), FIRST, "{run}");
    assert_eq!(field(&run, "window"), &Value::Bool(window));
    assert_eq!(field(&run, "status"), &Value::Null);
    assert_eq!(tree(&call(s, "look_tree", vec![])), FIRST);

    // A window's size in units is the compositor's to choose; with none it is 480 by 360.
    let mut size = None;
    for scale in [1, 2] {
        let shot = call(
            s,
            "look_screenshot",
            vec![("scale", (scale as usize).into())],
        );
        let png = field(&shot, "png").as_str().unwrap();
        assert!(png.starts_with("iVBORw0KGgo"), "a PNG signature");
        let px = |key| field(&shot, key).as_u64().unwrap();
        let (w, h) = *size.get_or_insert((px("width"), px("height")));
        assert!(window || (w, h) == (480, 360), "{w}x{h}");
        assert_eq!((px("width"), px("height")), (w * scale, h * scale));
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
    let key = |s: &mut Session, name: &str| call(s, "look_key", vec![("key", name.into())]);
    let entered = key(s, "Enter");
    assert!(
        tree(&entered).contains("\"hé\" cursor=0 sel=0..2"),
        "{entered}"
    );
    let spaced = key(s, "Space");
    assert!(tree(&spaced).contains("\" \" cursor=1"), "{spaced}");
    let back = key(s, "Shift+Tab");
    assert!(tree(&back).contains("on=Inc focus"), "{back}");
    let pressed = key(s, "Space");
    assert!(tree(&pressed).contains("Text #0 \"2\""), "{pressed}");
    let pressed = key(s, "Enter");
    assert!(tree(&pressed).contains("Text #0 \"3\""), "{pressed}");
    refused(s, "look_key", vec![("text", "a".into())], "no_focus");
    key(s, "Tab");
    let next = key(s, "Tab");
    assert!(tree(&next).contains("on=Quit focus"), "{next}");
    let to_input = key(s, "Shift+Tab");
    assert!(
        tree(&to_input).contains("on_change=Edit focus"),
        "{to_input}"
    );

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
