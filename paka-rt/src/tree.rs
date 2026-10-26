//! The element tree a view builds (M5-02, M5-04) and its text form (M5-06 section 1.3). Nodes
//! carry no rectangles; layout results live beside the tree (M5-05).

use crate::ops::quoted;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

/// A child's step from its parent: its key, or its place number among the element lines of its
/// child block. A placeholder made in a loop also counts that line's placeholders from 0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Int(i64),
    Str(String),
    Place(usize),
    LoopHole(usize, usize),
}

/// What an Input holds for itself (M5-03 section 1.5). Positions count code points.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edit {
    pub cursor: usize,
    pub sel: Range<usize>,
    pub preedit: String,
    pub focus: bool,
}

/// A node of the tree; `M` is a message value. The root is the node with no step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node<M> {
    Text(String),
    Button {
        label: String,
        on: M,
    },
    Input {
        text: String,
        on_change: M,
        edit: Edit,
    },
    Row {
        gap: u64,
        children: Vec<(Step, Node<M>)>,
    },
    Col {
        gap: u64,
        children: Vec<(Step, Node<M>)>,
    },
    Hole(String),
    Error(String),
}

impl<M> Node<M> {
    /// The node at `path` below this one.
    pub fn at(&self, path: &[Step]) -> Option<&Node<M>> {
        let Some((step, rest)) = path.split_first() else {
            return Some(self);
        };
        let child = self.children().iter().find(|(s, _)| s == step)?;
        child.1.at(rest)
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Node::Text(_) => "Text",
            Node::Button { .. } => "Button",
            Node::Input { .. } => "Input",
            Node::Row { .. } => "Row",
            Node::Col { .. } => "Col",
            Node::Hole(_) => "Hole",
            Node::Error(_) => "Error",
        }
    }

    pub fn children(&self) -> &[(Step, Node<M>)] {
        match self {
            Node::Row { children, .. } | Node::Col { children, .. } => children,
            _ => &[],
        }
    }
}

/// `children` with siblings that share a key replaced by one error node at the place of the
/// first, the key as its step (M5-02 section 1.7).
pub fn unique<M>(parent: &str, children: Vec<(Step, Node<M>)>) -> Vec<(Step, Node<M>)> {
    let mut count = BTreeMap::new();
    for (step, _) in &children {
        if let Step::Int(_) | Step::Str(_) = step {
            *count.entry(step.clone()).or_insert(0) += 1;
        }
    }
    if count.values().all(|&n| n < 2) {
        return children;
    }
    let mut kept = vec![];
    for (step, node) in children {
        match count.get_mut(&step) {
            Some(n) if *n > 1 => {
                let key = match &step {
                    Step::Str(s) => quoted(s),
                    step => step.to_string(),
                };
                let message = format!("duplicate key {key}: {n} children of {parent}");
                *n = 0;
                kept.push((step, Node::Error(message)));
            }
            Some(0) => {}
            _ => kept.push((step, node)),
        }
    }
    kept
}

/// Gives `new` the element state of `old` (M5-02 section 1.8). Nodes match by kind; a child
/// matches the old child with the same step under a matched parent. Anything unmatched starts
/// fresh, and so does a duplicate key, since `unique` left an error node in its place. A kept
/// Input follows a text the app set (M5-03 section 1.3).
pub fn keep<M>(old: &Node<M>, new: &mut Node<M>) {
    match (old, new) {
        (
            Node::Input {
                text: before,
                edit: was,
                ..
            },
            Node::Input { text, edit, .. },
        ) => {
            *edit = was.clone();
            edit.follow(before, text, None);
        }
        (Node::Row { children: was, .. }, Node::Row { children, .. })
        | (Node::Col { children: was, .. }, Node::Col { children, .. }) => {
            let was: BTreeMap<_, _> = was.iter().map(|(step, node)| (step, node)).collect();
            for (step, child) in children {
                if let Some(old) = was.get(step) {
                    keep(old, child);
                }
            }
        }
        _ => {}
    }
}

impl Display for Step {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Step::Int(n) => write!(f, "{n}"),
            Step::Str(s) => write!(f, "'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
            Step::Place(n) => write!(f, "#{n}"),
            Step::LoopHole(n, r) => write!(f, "#{n}.{r}"),
        }
    }
}

/// The steps of a UI path (M5-06 point 1.2), if it is written the one way `Step` prints them,
/// joined by `/`. The root is the empty path.
pub fn path(text: &str) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if !steps.is_empty() {
            rest = rest.strip_prefix('/')?;
        }
        let (step, left) = step(rest)?;
        steps.push(step);
        rest = left;
    }
    let printed: Vec<String> = steps.iter().map(Step::to_string).collect();
    (printed.join("/") == text).then_some(steps)
}

/// The first step of `text` and what follows it.
fn step(text: &str) -> Option<(Step, &str)> {
    if let Some(quoted) = text.strip_prefix('\'') {
        let mut key = String::new();
        let mut chars = quoted.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '\'' => return Some((Step::Str(key), &quoted[i + 1..])),
                '\\' => key.push(chars.next()?.1),
                c => key.push(c),
            }
        }
        return None;
    }
    let (word, rest) = text.split_at(text.find('/').unwrap_or(text.len()));
    let step = match word.strip_prefix('#') {
        Some(place) => match place.split_once('.') {
            Some((n, r)) => Step::LoopHole(n.parse().ok()?, r.parse().ok()?),
            None => Step::Place(place.parse().ok()?),
        },
        None => Step::Int(word.parse().ok()?),
    };
    Some((step, rest))
}

/// The tree text: one line per node, two spaces deeper than its parent, each ending in a line
/// feed. Defaults, an empty selection and an empty preedit are left out.
impl<M: Display> Display for Node<M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.lines(f, 0, None)
    }
}

impl<M: Display> Node<M> {
    fn lines(&self, f: &mut Formatter, depth: usize, step: Option<&Step>) -> fmt::Result {
        write!(f, "{:1$}{2}", "", depth * 2, self.kind())?;
        if let Some(step) = step {
            write!(f, " {step}")?;
        }
        match self {
            Node::Text(text) => write!(f, " {}", quoted(text))?,
            Node::Button { label, on } => write!(f, " {} on={on}", quoted(label))?,
            Node::Input {
                text,
                on_change,
                edit,
            } => {
                write!(f, " {} cursor={}", quoted(text), edit.cursor)?;
                if !edit.sel.is_empty() {
                    write!(f, " sel={}..{}", edit.sel.start, edit.sel.end)?;
                }
                if !edit.preedit.is_empty() {
                    write!(f, " preedit={}", quoted(&edit.preedit))?;
                }
                write!(f, " on_change={on_change}")?;
                if edit.focus {
                    write!(f, " focus")?;
                }
            }
            Node::Row { gap, .. } | Node::Col { gap, .. } if *gap != 0 => write!(f, " gap={gap}")?,
            Node::Row { .. } | Node::Col { .. } => {}
            Node::Hole(name) => write!(f, " {name}")?,
            Node::Error(message) => write!(f, " {}", quoted(message))?,
        }
        writeln!(f)?;
        self.children()
            .iter()
            .try_for_each(|(step, child)| child.lines(f, depth + 1, Some(step)))
    }
}

#[cfg(test)]
mod tests;
