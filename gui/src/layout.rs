//! Layout of the element tree and hit testing (M5-05). One pass hands widths down and heights
//! up; rectangles come out in a Vec parallel to a pre-order walk of the tree, which is also the
//! drawing order.

use crate::tree::{Node, Step};

/// A node's place in whole units. Content that does not fit sticks out of its parent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u64,
    pub y: u64,
    pub w: u64,
    pub h: u64,
}

impl Rect {
    fn holds(&self, x: u64, y: u64) -> bool {
        (self.x..self.x.saturating_add(self.w)).contains(&x)
            && (self.y..self.y.saturating_add(self.h)).contains(&y)
    }
}

/// Text measuring, rounded up to whole units.
pub trait Measure {
    /// The width of `text` set on one line.
    fn width(&self, text: &str) -> u64;
    /// The lines `text` takes wrapped to `width`, at least one.
    fn lines(&self, text: &str, width: u64) -> u64;
}

const LINE: u64 = 20;
const TALL: u64 = 32;
const PAD: u64 = 12;
const FLOOR: u64 = 40;

/// The rectangle of every node of `root`, in pre-order, laid out in a window `width` wide.
pub fn layout<M>(root: &Node<M>, width: u64, measure: &impl Measure) -> Vec<Rect> {
    let mut out = Layout {
        measure,
        rects: Vec::new(),
    };
    out.place(root, 0, 0, out.hug(root).unwrap_or(width));
    out.rects
}

struct Layout<'a, T> {
    measure: &'a T,
    rects: Vec<Rect>,
}

impl<T: Measure> Layout<'_, T> {
    /// A hugging node's width; filling nodes take what they are handed.
    fn hug<M>(&self, node: &Node<M>) -> Option<u64> {
        match node {
            Node::Button { label: s, .. } | Node::Hole(s) => Some(self.measure.width(s) + 2 * PAD),
            _ => None,
        }
    }

    /// Places `node` at `x`,`y`, `w` wide, and answers its height.
    fn place<M>(&mut self, node: &Node<M>, x: u64, y: u64, w: u64) -> u64 {
        let at = self.rects.len();
        self.rects.push(Rect { x, y, w, h: 0 });
        let h = match node {
            Node::Text(text) => LINE * self.measure.lines(text, w),
            Node::Button { .. } | Node::Input { .. } | Node::Hole(_) => TALL,
            Node::Error(message) => LINE * self.measure.lines(message, w.saturating_sub(PAD)) + PAD,
            Node::Col { gap, children } => {
                let mut bottom = y;
                for (i, (_, child)) in children.iter().enumerate() {
                    if i > 0 {
                        bottom = bottom.saturating_add(*gap);
                    }
                    let cw = self.hug(child).unwrap_or(w);
                    bottom = bottom.saturating_add(self.place(child, x, bottom, cw));
                }
                bottom - y
            }
            Node::Row { gap, children } => self.row(*gap, children, x, y, w),
        };
        self.rects[at].h = h;
        h
    }

    /// Hugging children take their width first; filling ones share the rest equally, earlier
    /// ones taking the spare units, none under the floor. Each is centred top to bottom.
    fn row<M>(&mut self, gap: u64, children: &[(Step, Node<M>)], x: u64, y: u64, w: u64) -> u64 {
        let (mut used, mut fills) = (0u64, 0u64);
        for (i, (_, child)) in children.iter().enumerate() {
            used = used.saturating_add(if i > 0 { gap } else { 0 });
            match self.hug(child) {
                Some(cw) => used = used.saturating_add(cw),
                None => fills += 1,
            }
        }
        let rest = w.saturating_sub(used);
        let share = rest.checked_div(fills).unwrap_or(0);
        let spare = rest.checked_rem(fills).unwrap_or(0);
        let first = self.rects.len();
        let (mut left, mut filled, mut h) = (x, 0, 0);
        for (i, (_, child)) in children.iter().enumerate() {
            if i > 0 {
                left = left.saturating_add(gap);
            }
            let cw = self.hug(child).unwrap_or_else(|| {
                filled += 1;
                (share + u64::from(filled <= spare)).max(FLOOR)
            });
            h = h.max(self.place(child, left, y, cw));
            left = left.saturating_add(cw);
        }
        let mut at = first;
        for (_, child) in children {
            let end = at + count(child);
            let dy = (h - self.rects[at].h) / 2;
            self.rects[at..end].iter_mut().for_each(|r| r.y += dy);
            at = end;
        }
        h
    }
}

fn count<M>(node: &Node<M>) -> usize {
    1 + node.children().iter().map(|(_, c)| count(c)).sum::<usize>()
}

/// The path of the last-drawn element under `x`,`y`, given the `rects` of `layout`. Row and Col
/// are never hit; an element sticking out of its parent is.
pub fn hit<M>(root: &Node<M>, rects: &[Rect], x: u64, y: u64) -> Option<Vec<Step>> {
    let mut find = Find {
        rects,
        x,
        y,
        next: 0,
        path: Vec::new(),
        best: None,
    };
    find.walk(root);
    find.best.map(|path| path.into_iter().cloned().collect())
}

struct Find<'a, 'r> {
    rects: &'r [Rect],
    x: u64,
    y: u64,
    next: usize,
    path: Vec<&'a Step>,
    best: Option<Vec<&'a Step>>,
}

impl<'a> Find<'a, '_> {
    fn walk<M>(&mut self, node: &'a Node<M>) {
        let rect = self.rects.get(self.next);
        self.next += 1;
        let container = matches!(node, Node::Row { .. } | Node::Col { .. });
        if !container && rect.is_some_and(|r| r.holds(self.x, self.y)) {
            self.best = Some(self.path.clone());
        }
        for (step, child) in node.children() {
            self.path.push(step);
            self.walk(child);
            self.path.pop();
        }
    }
}

#[cfg(test)]
mod tests;
