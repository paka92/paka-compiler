//! The element tree a view builds (M5-02, M5-04) and its text form (M5-06 section 1.3). Nodes
//! carry no rectangles; layout results live beside the tree (M5-05).

use crate::ops::quoted;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

/// A child's step from its parent: its key, or its place number among the element lines of its
/// child block. A placeholder made in a loop also counts that line's placeholders from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
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
