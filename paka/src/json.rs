//! A minimal JSON value and writer: compact output, keys in the order given.

use std::fmt::{self, Display, Write};

pub enum Json {
    Null,
    Uint(usize),
    Str(String),
    Object(Vec<(&'static str, Json)>),
}

impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Uint(n)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.into())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(v: Option<T>) -> Json {
        v.map_or(Json::Null, Into::into)
    }
}

/// A quoted string; control characters are escaped, everything else is written as is.
fn write_str(f: &mut fmt::Formatter, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if c < ' ' => write!(f, "\\u{:04x}", u32::from(c))?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

impl Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Uint(n) => write!(f, "{n}"),
            Json::Str(s) => write_str(f, s),
            Json::Object(fields) => {
                f.write_char('{')?;
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write_str(f, key)?;
                    write!(f, ":{value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(s: &str) -> String {
        Json::from(s).to_string()
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
    fn numbers_and_null() {
        assert_eq!(Json::from(0).to_string(), "0");
        assert_eq!(Json::from(usize::MAX).to_string(), usize::MAX.to_string());
        assert_eq!(Json::from(None::<usize>).to_string(), "null");
        assert_eq!(Json::from(Some("x")).to_string(), r#""x""#);
    }

    #[test]
    fn objects_keep_key_order_and_nest() {
        let inner = Json::Object(vec![("b", 2.into())]);
        let outer = Json::Object(vec![
            ("z", 1.into()),
            ("a", inner),
            ("e", Json::Object(vec![])),
        ]);
        assert_eq!(outer.to_string(), r#"{"z":1,"a":{"b":2},"e":{}}"#);
    }

    #[test]
    fn keys_are_escaped_too() {
        let object = Json::Object(vec![("a\"b", Json::Null)]);
        assert_eq!(object.to_string(), r#"{"a\"b":null}"#);
    }
}
