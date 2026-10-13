use std::fmt;

/// One reported problem. `position` is (line, column); tool errors have none.
#[derive(Debug)]
pub struct Diagnostic {
    pub path: String,
    pub position: Option<(usize, usize)>,
    pub code: &'static str,
    pub message: String,
}

impl Diagnostic {
    /// A tool failure: no position.
    pub fn tool(path: &str, code: &'static str, message: impl ToString) -> Diagnostic {
        Diagnostic {
            path: path.into(),
            position: None,
            code,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if let Some((line, col)) = self.position {
            write!(f, ":{line}:{col}")?;
        }
        write!(f, ": error[{}]: {}", self.code, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(position: Option<(usize, usize)>) -> Diagnostic {
        Diagnostic {
            path: "main.paka".into(),
            position,
            code: "unexpected-token",
            message: "expected `{`".into(),
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
}
