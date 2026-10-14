//! The JSON reader: strict, so an unexpected shape is an error and never a default.

use harness::json::{Value, parse, quote};

fn num(text: &str) -> Value {
    Value::Num(text.into())
}

fn obj(fields: Vec<(&str, Value)>) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[test]
fn scalars() {
    assert_eq!(parse("null"), Ok(Value::Null));
    assert_eq!(parse("true"), Ok(Value::Bool(true)));
    assert_eq!(parse(" false\n"), Ok(Value::Bool(false)));
    assert_eq!(parse("-12.5e+3"), Ok(num("-12.5e+3")));
    assert_eq!(parse("0"), Ok(num("0")));
    assert_eq!(parse(r#""a b""#), Ok(Value::Str("a b".into())));
}

#[test]
fn nested_values_keep_order() {
    let want = obj(vec![
        ("b", Value::Array(vec![num("1"), Value::Null, obj(vec![])])),
        ("a", Value::Str("x".into())),
    ]);
    assert_eq!(
        parse(r#" { "b" : [ 1 , null , { } ] , "a" : "x" } "#),
        Ok(want)
    );
}

#[test]
fn string_escapes() {
    let got = parse(r#""\"\\\/\b\f\n\r\té😀""#);
    assert_eq!(got, Ok(Value::Str("\"\\/\u{8}\u{c}\n\r\té😀".into())));
}

#[test]
fn bad_text_is_an_error() {
    let bad = [
        "",
        "nul",
        "[1,]",
        "{\"a\":1,}",
        "{'a':1}",
        "[1] x",
        "01",
        "1.",
        "-",
        "+1",
        "\"open",
        "\"a\nb\"",
        r#""\x""#,
        r#""\u12""#,
        r#""\ud83d""#,
        r#""\ude00""#,
        r#"{"a":1,"a":2}"#,
        r#"{"a" 1}"#,
        "[1 2]",
    ];
    for text in bad {
        assert!(parse(text).is_err(), "{text:?} was accepted");
    }
}

fn nested(levels: usize) -> String {
    format!("{}{}", "[".repeat(levels), "]".repeat(levels))
}

#[test]
fn nesting_is_bounded_at_64_levels() {
    assert!(parse(&nested(64)).is_ok());
    assert!(parse(&nested(65)).is_err());
    assert!(parse(&"[".repeat(100_000)).is_err());
}

#[test]
fn unescaped_non_ascii_text_is_kept() {
    assert_eq!(parse("\"é😀\""), Ok(Value::Str("é😀".into())));
    assert_eq!(parse("\"\\u00e9\\ud83d\\ude00\""), Ok(Value::Str("é😀".into())));
}

#[test]
fn accessors() {
    let v = parse(r#"{"n":12,"s":"t","b":true,"l":[1]}"#).unwrap();
    assert_eq!(v.get("n").and_then(Value::as_u64), Some(12));
    assert_eq!(v.get("s").and_then(Value::as_str), Some("t"));
    assert_eq!(v.get("b").and_then(Value::as_bool), Some(true));
    assert_eq!(
        v.get("l").and_then(Value::as_array).map(<[Value]>::len),
        Some(1)
    );
    assert!(v.get("missing").is_none());
    assert_eq!(v.get("s").and_then(Value::as_u64), None);
    assert_eq!(Value::Null.get("n"), None);
}

#[test]
fn only_whole_non_negative_numbers_are_counts() {
    for (text, want) in [
        ("0", Some(0)),
        ("18446744073709551615", Some(u64::MAX)),
        ("18446744073709551616", None),
        ("-1", None),
        ("1.5", None),
        ("1e3", None),
        ("2.0", None),
    ] {
        assert_eq!(parse(text).unwrap().as_u64(), want, "{text}");
    }
}

#[test]
fn quoted_text_reads_back() {
    for text in [
        "",
        "plain",
        "a\"b\\c",
        "line\nbreak\ttab\r",
        "\u{0}\u{1f}é€😀",
    ] {
        assert_eq!(parse(&quote(text)), Ok(Value::Str(text.into())), "{text:?}");
    }
    assert_eq!(quote("a\nb"), r#""a\nb""#);
    assert_eq!(quote("\u{1}"), r#""\u0001""#);
}
