use crate::json::Json;
use std::fmt;

/// Per error kind: its code, whether it carries expected and found types, whether it carries a fix.
/// Every code the tool can report is listed (spec/m1.md section 10).
const KINDS: &[(&str, bool, bool)] = &[
    ("file-unreadable", false, false),
    ("clang-missing", false, false),
    ("clang-failed", false, false),
    ("runtime-missing", false, false),
    ("invalid-byte", false, false),
    ("unexpected-character", false, false),
    ("unterminated-string", false, false),
    ("bad-escape", false, false),
    ("unexpected-token", false, false),
    ("unexpected-inout-mark", false, false),
    ("nesting-too-deep", false, false),
    ("unsupported-view", false, false),
    ("unsupported-form", false, false),
    ("duplicate-name", false, false),
    ("duplicate-local", false, false),
    ("no-app", false, false),
    ("multiple-apps", false, false),
    ("unknown-type", false, true),
    ("wrong-type-arguments", false, false),
    ("unknown-name", false, true),
    ("wrong-argument-count", false, false),
    ("type-mismatch", true, false),
    ("duplicate-app-entry", false, false),
    ("missing-start", false, false),
    ("missing-update", false, false),
    ("missing-grants", false, false),
    ("bad-update-signature", false, false),
    ("bad-start", false, false),
    ("bad-state", false, false),
    ("unknown-grant", false, true),
    ("duplicate-grant", false, false),
    ("hole-in-build", false, false),
    ("no-finite-value", false, false),
    ("unused-result", false, false),
    ("nothing-to-discard", false, false),
    ("missing-value", false, false),
    ("unreachable-code", false, false),
    ("break-outside-loop", false, false),
    ("if-needs-else", false, false),
    ("return-last", false, false),
    ("assign-to-let", false, false),
    ("bad-assignment-target", false, false),
    ("cannot-infer", false, false),
    ("wrong-label", false, false),
    ("not-a-function", false, false),
    ("missing-inout-mark", false, false),
    ("bad-inout-target", false, false),
    ("inout-overlap", false, false),
    ("changed-while-read", false, false),
    ("changed-while-walked", false, false),
    ("not-a-value", false, false),
    ("literal-out-of-range", false, false),
    ("missing-arm", false, false),
    ("duplicate-arm", false, false),
    ("wildcard-on-enum", false, false),
];

fn declared(code: &str) -> &'static (&'static str, bool, bool) {
    KINDS
        .iter()
        .find(|k| k.0 == code)
        .unwrap_or_else(|| panic!("error kind `{code}` is not in KINDS"))
}

/// One reported problem. `position` is (line, column); tool errors have none. `len` is the
/// byte length of the offending text, which starts at `position`. A fix replaces that text.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub path: String,
    pub position: Option<(usize, usize)>,
    pub code: &'static str,
    pub message: String,
    len: usize,
    more: Box<More>,
}

/// The optional fields, boxed to keep `Result<_, Diagnostic>` small.
#[derive(Clone, Debug, Default)]
struct More {
    definition: Option<String>,
    types: Option<(String, String)>,
    fix: Option<String>,
}

impl Diagnostic {
    pub fn new(
        path: &str,
        position: (usize, usize),
        len: usize,
        code: &'static str,
        message: impl ToString,
    ) -> Diagnostic {
        Diagnostic {
            path: path.into(),
            position: Some(position),
            code,
            message: message.to_string(),
            len,
            more: Box::default(),
        }
    }

    /// A problem that has no position in the source.
    pub fn tool(path: &str, code: &'static str, message: impl ToString) -> Diagnostic {
        Diagnostic {
            position: None,
            ..Diagnostic::new(path, (0, 0), 0, code, message)
        }
    }

    pub fn in_definition(mut self, name: &str) -> Diagnostic {
        self.more.definition = Some(name.into());
        self
    }

    pub fn with_types(mut self, expected: &impl ToString, found: &impl ToString) -> Diagnostic {
        assert!(declared(self.code).1, "`{}` carries no types", self.code);
        self.more.types = Some((expected.to_string(), found.to_string()));
        self
    }

    pub fn with_fix(mut self, replacement: &str) -> Diagnostic {
        assert!(declared(self.code).2, "`{}` carries no fix", self.code);
        self.more.fix = Some(replacement.into());
        self
    }

    /// Byte range of the offending text in `source`.
    fn span(&self, source: &[u8]) -> Option<(usize, usize)> {
        let (line, col) = self.position?;
        let line_start: usize = (source.split_inclusive(|&b| b == b'\n'))
            .take(line - 1)
            .map(<[u8]>::len)
            .sum();
        let start = (line_start + col - 1).min(source.len());
        Some((start, (start + self.len).min(source.len())))
    }

    fn text(&self, source: &[u8]) -> Option<String> {
        let (start, end) = self.span(source)?;
        Some(String::from_utf8_lossy(&source[start..end]).into_owned())
    }

    /// The one-line form followed by one indented line per field that is present.
    pub fn detailed(&self, source: &[u8]) -> String {
        let mut out = format!("{self}\n");
        let mut field = |name: &str, value: Option<String>| {
            if let Some(value) = value {
                out += &format!("  {name}: {value}\n");
            }
        };
        field("definition", self.more.definition.clone());
        let span = self.span(source);
        field("bytes", span.map(|(start, end)| format!("{start}..{end}")));
        field("text", self.text(source).map(|t| format!("{t:?}")));
        field("expected", self.more.types.as_ref().map(|t| t.0.clone()));
        field("found", self.more.types.as_ref().map(|t| t.1.clone()));
        let fix = (self.more.fix.as_ref()).zip(span);
        field(
            "fix",
            fix.map(|(r, (s, e))| format!("replace {s}..{e} with {r:?}")),
        );
        out
    }

    /// One JSON object and a line feed; every key is always present.
    pub fn json(&self, source: &[u8]) -> String {
        let span = self.span(source);
        let (line, column) = self.position.unzip();
        let (expected, found) = self.more.types.clone().unzip();
        let fix = (self.more.fix.as_ref())
            .zip(span)
            .map(|(replacement, (start, end))| {
                Json::Object(vec![
                    ("start", start.into()),
                    ("end", end.into()),
                    ("replacement", replacement.as_str().into()),
                ])
            });
        let fields = vec![
            ("kind", self.code.into()),
            ("definition", self.more.definition.clone().into()),
            ("message", self.message.as_str().into()),
            ("file", self.path.as_str().into()),
            ("start", span.map(|s| s.0).into()),
            ("end", span.map(|s| s.1).into()),
            ("line", line.into()),
            ("column", column.into()),
            ("text", self.text(source).into()),
            ("expected", expected.into()),
            ("found", found.into()),
            ("fix", fix.into()),
        ];
        format!("{}\n", Json::Object(fields))
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if let Some((line, col)) = self.position {
            write!(f, ":{line}:{col}")?;
        }
        // The one note is a hole (M2-06 section 1.5), the one warning an unused name (M2-02
        // section 5).
        let level = match self.code {
            "hole" => "note",
            "unused-name" => "warning",
            _ => "error",
        };
        write!(f, ": {level}[{}]: {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(position: Option<(usize, usize)>) -> Diagnostic {
        Diagnostic {
            position,
            ..Diagnostic::new("main.paka", (0, 0), 1, "unexpected-token", "expected `{`")
        }
    }

    #[test]
    fn line_with_position() {
        assert_eq!(
            diagnostic(Some((3, 14))).to_string(),
            "main.paka:3:14: error[unexpected-token]: expected `{`"
        );
    }

    #[test]
    fn line_without_position() {
        assert_eq!(
            diagnostic(None).to_string(),
            "main.paka: error[unexpected-token]: expected `{`"
        );
    }

    /// The first cell of every row of the spec's section 10 table.
    fn spec_codes() -> Vec<String> {
        let spec = include_str!("../../spec/m1.md");
        let section = spec.split("## 10.").nth(1).unwrap();
        let section = section.split("## 11.").next().unwrap();
        let cell = |line: &str| {
            line.strip_prefix("| `")?
                .split('`')
                .next()
                .map(String::from)
        };
        section.lines().filter_map(cell).collect()
    }

    #[test]
    fn table_lists_exactly_the_spec_codes() {
        let mut table: Vec<_> = KINDS.iter().map(|k| k.0.to_string()).collect();
        let mut spec = spec_codes();
        table.sort();
        spec.sort();
        assert_eq!(table, spec);
    }

    #[test]
    fn only_declared_kinds_take_types_or_a_fix() {
        assert!(std::panic::catch_unwind(|| diagnostic(None).with_fix("x")).is_err());
        assert!(std::panic::catch_unwind(|| diagnostic(None).with_types(&"a", &"b")).is_err());
    }

    #[test]
    fn spans_follow_lines_and_clamp_to_the_source() {
        let at = |line, col, len| {
            Diagnostic::new("p", (line, col), len, "unknown-name", "m").span(b"ab\ncd\n")
        };
        assert_eq!(at(1, 2, 1), Some((1, 2)));
        assert_eq!(at(2, 1, 2), Some((3, 5)));
        assert_eq!(at(3, 1, 0), Some((6, 6)));
        assert_eq!(at(2, 2, 9), Some((4, 6)));
    }
}
