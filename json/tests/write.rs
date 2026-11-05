//! The writer: compact, keys in the order given, strings escaped only where JSON needs it.

use paka_json::{Value, parse};

fn string(s: &str) -> String {
    Value::from(s).to_string()
}

#[test]
fn plain_text_is_quoted() {
    assert_eq!(string("abc 1"), r#""abc 1""#);
    assert_eq!(string(""), r#""""#);
}

#[test]
fn quote_and_backslash_are_escaped() {
    assert_eq!(string(r#"a"b\c"#), r#""a\"b\\c""#);
}

#[test]
fn control_characters_are_escaped() {
    assert_eq!(string("a\nb\rc\td"), r#""a\nb\rc\td""#);
    assert_eq!(
        string("\u{0}\u{1}\u{8}\u{c}\u{1f}"),
        r#""\u0000\u0001\u0008\u000c\u001f""#
    );
}

#[test]
fn non_ascii_and_delete_pass_through() {
    assert_eq!(string("é€😀\u{7f}"), "\"é€😀\u{7f}\"");
}

#[test]
fn numbers_bools_and_null() {
    assert_eq!(Value::from(0).to_string(), "0");
    assert_eq!(Value::from(usize::MAX).to_string(), usize::MAX.to_string());
    assert_eq!(Value::from(None::<usize>).to_string(), "null");
    assert_eq!(Value::from(Some("x")).to_string(), r#""x""#);
    assert_eq!(Value::from(true).to_string(), "true");
}

#[test]
fn objects_and_arrays_keep_order_and_nest() {
    let inner = Value::object([("b", 2.into())]);
    let outer = Value::object([
        ("z", 1.into()),
        ("a", inner),
        ("e", Value::object([])),
        ("l", vec!["x", "y"].into()),
    ]);
    assert_eq!(
        outer.to_string(),
        r#"{"z":1,"a":{"b":2},"e":{},"l":["x","y"]}"#
    );
}

#[test]
fn keys_are_escaped_too() {
    let object = Value::object([("a\"b", Value::Null)]);
    assert_eq!(object.to_string(), r#"{"a\"b":null}"#);
}

#[test]
fn what_is_written_reads_back() {
    let text = r#"{"s":"a\"\n\u0001é","n":-1.5e3,"l":[true,false,null,[]],"o":{}}"#;
    let value = parse(text).unwrap();
    assert_eq!(value.to_string(), text);
    assert_eq!(parse(&value.to_string()).unwrap(), value);
}
