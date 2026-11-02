//! The action lines of a UI replay and the result after each (docs/decisions/M7-03.md 1.2), one
//! form for M5-26's log, the windowless loops and `--paka-replay`.

use crate::input::Key;
use crate::route::Event;
use crate::tree::{self, Node, Step};
use std::fmt::{self, Display, Formatter};

/// An action after the launch: a click on a path, or text typed into the focused Input.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Click(Vec<Step>),
    Type(String),
}

impl Action {
    /// The action a line names, `click PATH` or `type "TEXT"`, in the one spelling `Display` gives.
    pub fn parse(line: &str) -> Option<Action> {
        match line.strip_prefix("click ") {
            Some(path) => tree::path(path).map(Action::Click),
            None => unquoted(line.strip_prefix("type ")?).map(Action::Type),
        }
    }

    /// The events it sends to `root`, a key per code point; `None` when a path reaches no node.
    pub fn events<M>(&self, root: &Node<M>) -> Option<Vec<Event>> {
        Some(match self {
            Action::Click(path) => vec![Event::Click(path.clone(), root.at(path)?.kind())],
            Action::Type(text) => text.chars().map(|c| Event::Key(Key::Type(c))).collect(),
        })
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Action::Click(path) => {
                let steps: Vec<_> = path.iter().map(Step::to_string).collect();
                write!(f, "click {}", steps.join("/"))
            }
            Action::Type(text) => write!(f, "type {}", tree::quoted(text)),
        }
    }
}

/// The text `tree::quoted` spells as `text`, if `text` is that spelling.
fn unquoted(text: &str) -> Option<String> {
    let mut chars = text.strip_prefix('"')?.strip_suffix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        out.push(match c {
            '\\' => match chars.next()? {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                c => c,
            },
            c => c,
        });
    }
    (tree::quoted(&out) == text).then_some(out)
}

/// What the launch or an action left (M5-26): the tree text, the tree after `Exit`, a trap's first
/// line, or the line of an end.
#[derive(Debug, Clone, PartialEq)]
pub enum Left {
    Tree(String),
    Exited(String),
    Trapped(String),
    Ended(String),
}

/// As `--paka-replay` writes it, an empty line after: the tree text, `exited` and the tree text,
/// `trapped LINE` or `ended LINE`.
impl Display for Left {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Left::Tree(tree) => writeln!(f, "{tree}"),
            Left::Exited(tree) => writeln!(f, "exited\n{tree}"),
            Left::Trapped(line) => writeln!(f, "trapped {line}\n"),
            Left::Ended(line) => writeln!(f, "ended {}\n", line.trim_end_matches('\n')),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_reads_back_as_written() {
        let typed = Action::Type("a\"\\{}\t\r\né\0".into());
        let click = Action::Click(tree::path("'k\\'y'/#2/7").unwrap());
        for action in [typed, click] {
            assert_eq!(Action::parse(&action.to_string()), Some(action));
        }
        assert_eq!(
            Action::parse(r#"type "a\"\{\}\té""#),
            Some(Action::Type("a\"{}\té".into()))
        );
    }

    #[test]
    fn a_line_in_another_spelling_is_no_action() {
        let lines = [
            "type a",
            "type \"a",
            "type \"\"\"",
            "type \"{\"",
            "type \"\\q\"",
            "type \"\\\"",
            "click",
            "click #1/",
            "click  #1",
            "tap #1",
            "type  \"a\"",
        ];
        for line in lines {
            assert_eq!(Action::parse(line), None, "{line}");
        }
        assert_eq!(
            Action::parse("type \"\""),
            Some(Action::Type(String::new()))
        );
    }

    #[test]
    fn each_result_ends_with_an_empty_line() {
        let tree = "Col gap=0\n".to_string();
        let shown = [
            Left::Tree(tree.clone()),
            Left::Exited(tree),
            Left::Trapped("a: trap overflow at main.paka:1:2".into()),
            Left::Ended("a: effect Print refused: grant Console is missing\n".into()),
        ]
        .map(|l| l.to_string());
        assert_eq!(
            shown,
            [
                "Col gap=0\n\n",
                "exited\nCol gap=0\n\n",
                "trapped a: trap overflow at main.paka:1:2\n\n",
                "ended a: effect Print refused: grant Console is missing\n\n",
            ]
        );
    }
}
