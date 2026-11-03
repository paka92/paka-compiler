//! The hand-written exemption list of the harness's release mode (M6-06).

use std::collections::BTreeSet;

/// The exemption list `text`: the cases under each kind, one `KIND CASE` per line, `#` lines and
/// blank lines aside. The kinds are `hole`, a case release must refuse by its holes' names, and
/// `unfinished`, an example-status case with a hole, compared in interpreter and dev only.
pub fn exempt(text: &str) -> Result<[BTreeSet<String>; 2], String> {
    let mut sets = [BTreeSet::new(), BTreeSet::new()];
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (kind, case) = line.split_once(' ').unwrap_or((line, ""));
        let k = (["hole", "unfinished"].iter().position(|w| *w == kind))
            .filter(|_| !case.trim().is_empty())
            .ok_or(format!("not `hole CASE` or `unfinished CASE`: {line:?}"))?;
        sets[k].insert(case.trim().to_owned());
    }
    Ok(sets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exemption_list_has_two_kinds() {
        let [holes, unfinished] = exempt("# x\nhole a/b\nunfinished c/d\n").unwrap();
        assert_eq!((holes.len(), unfinished.len()), (1, 1));
        assert!(exempt("skip a/b\n").is_err());
        assert!(exempt("examples c/d\n").is_err());
        assert!(exempt("hole\n").is_err());
        exempt(include_str!("../../tests/conformance/release-exempt")).unwrap();
    }
}
