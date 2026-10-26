//! The element tree as AccessKit nodes, and AccessKit actions as element input (M5-39). Each
//! element's AccessKit number is kept by its path id and stays the same while that id lives
//! (M5-02); the window is node 0.

use std::collections::BTreeMap;
use std::mem;
use std::slice;

use accesskit::{Action, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use paka_rt::layout::Rect;
use paka_rt::route::Event;
use paka_rt::tree::{Node, Step};

/// The window's node; elements are numbered from 1.
pub const WINDOW: NodeId = NodeId(0);

/// The AccessKit number of each path id in the tree last given.
#[derive(Debug, Default)]
pub struct Ids {
    ids: BTreeMap<Vec<Step>, NodeId>,
    last: u64,
}

/// The state of one walk over a tree.
struct Walk<'a> {
    old: BTreeMap<Vec<Step>, NodeId>,
    rects: slice::Iter<'a, Rect>,
    scale: f64,
    nodes: Vec<(NodeId, accesskit::Node)>,
    focus: NodeId,
}

impl Ids {
    /// `tree`, laid out as `rects`, as a whole AccessKit tree in a window titled `title`, with
    /// bounds in pixels at `scale`. A path gone from the tree loses its number; a new one gets one
    /// never used before.
    pub fn update<M>(
        &mut self,
        title: &str,
        tree: &Node<M>,
        rects: &[Rect],
        scale: f64,
    ) -> TreeUpdate {
        let mut walk = Walk {
            old: mem::take(&mut self.ids),
            rects: rects.iter(),
            scale,
            nodes: vec![],
            focus: WINDOW,
        };
        let root = self.add(&mut walk, vec![], tree);
        let mut window = accesskit::Node::new(Role::Window);
        window.set_label(title);
        window.set_children(vec![root]);
        walk.nodes.push((WINDOW, window));
        TreeUpdate {
            nodes: walk.nodes,
            tree: Some(TreeInfo::new(WINDOW)),
            tree_id: TreeId::ROOT,
            focus: walk.focus,
        }
    }

    /// The path the number `id` stands for.
    pub fn path(&self, id: NodeId) -> Option<&[Step]> {
        let (path, _) = self.ids.iter().find(|(_, i)| **i == id)?;
        Some(path)
    }

    /// Adds `node`, at `path`, and its children in pre-order; its number.
    fn add<M>(&mut self, walk: &mut Walk, path: Vec<Step>, node: &Node<M>) -> NodeId {
        let id = walk.old.remove(&path).unwrap_or_else(|| {
            self.last += 1;
            NodeId(self.last)
        });
        let r = walk.rects.next().copied().unwrap_or_default();
        let px = |v: u64| v as f64 * walk.scale;
        let mut out = element(node);
        out.set_bounds(accesskit::Rect::new(
            px(r.x),
            px(r.y),
            px(r.x + r.w),
            px(r.y + r.h),
        ));
        if matches!(node, Node::Button { focus: true, .. })
            || matches!(node, Node::Input { edit, .. } if edit.focus)
        {
            walk.focus = id;
        }
        let children = node.children().iter().map(|(step, child)| {
            let mut below = path.clone();
            below.push(step.clone());
            self.add(walk, below, child)
        });
        out.set_children(children.collect::<Vec<_>>());
        self.ids.insert(path, id);
        walk.nodes.push((id, out));
        id
    }
}

/// `node`'s role, name or value, and actions.
fn element<M>(node: &Node<M>) -> accesskit::Node {
    let role = match node {
        Node::Text(_) | Node::Hole(_) | Node::Error(_) => Role::Label,
        Node::Button { .. } => Role::Button,
        Node::Input { .. } => Role::TextInput,
        Node::Row { .. } | Node::Col { .. } => Role::GenericContainer,
    };
    let mut out = accesskit::Node::new(role);
    match node {
        Node::Text(text) | Node::Error(text) => out.set_value(text.as_str()),
        Node::Hole(name) => out.set_value(format!("??{name}")),
        Node::Button { label, .. } => {
            out.set_label(label.as_str());
            out.add_action(Action::Click);
            out.add_action(Action::Focus);
        }
        Node::Input { text, .. } => {
            out.set_value(text.as_str());
            out.add_action(Action::Focus);
        }
        Node::Row { .. } | Node::Col { .. } => {}
    }
    out
}

/// What `action` on the element at `path` of `tree` makes: a Click on a Button clicks it without
/// moving focus, and a Focus on an Input or a Button moves focus there as Tab would (M5-08).
pub fn action<M>(tree: &Node<M>, path: &[Step], action: Action) -> Option<Event> {
    match (action, tree.at(path)?) {
        (Action::Click, Node::Button { .. }) => Some(Event::Click(path.to_vec(), "Button")),
        (Action::Focus, Node::Button { .. } | Node::Input { .. }) => {
            Some(Event::Focus(path.to_vec()))
        }
        _ => None,
    }
}
