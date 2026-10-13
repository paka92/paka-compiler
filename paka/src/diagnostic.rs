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
