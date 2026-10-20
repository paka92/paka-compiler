//! Clicks and keys to messages (M5-18). Focus lives in the focused Input's `Edit`, so `keep`
//! carries it by key and drops it with the element (M5-02); at most one Input holds it.

use crate::input::{Change, Key};
use crate::tree::{Node, Step};

/// An event as queued: a click aimed at the element of that kind at that path, or a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Click(Vec<Step>, &'static str),
    Key(Key),
}

/// What an event sends: a Button's message, or an Input's `on_change` with the new text, which
/// the caller turns into a message.
#[derive(Debug, PartialEq, Eq)]
pub enum Sent<'a, M> {
    Click(&'a M),
    Change(&'a M, Change),
}

/// Works `event` out against `tree`, the tree of its turn in the queue (M5-03). A click whose
/// element is gone or changed kind is dropped; a click on an Input moves focus there, any other
/// leaves it (M5-04). A key with no focused Input is dropped.
pub fn route<'a, M>(tree: &'a mut Node<M>, event: &Event) -> Option<Sent<'a, M>> {
    match event {
        Event::Key(key) => match focused(tree)? {
            Node::Input {
                text,
                on_change,
                edit,
            } => edit
                .key(text, *key)
                .map(|change| Sent::Change(on_change, change)),
            _ => None,
        },
        Event::Click(path, kind) => {
            if at(tree, path)?.kind() != *kind {
                return None;
            }
            if *kind == "Input"
                && let Some(Node::Input { edit, .. }) = focused(tree)
            {
                edit.focus = false;
            }
            match at(tree, path)? {
                Node::Button { on, .. } => Some(Sent::Click(on)),
                Node::Input { edit, .. } => {
                    edit.focus = true;
                    None
                }
                _ => None,
            }
        }
    }
}

fn at<'a, M>(node: &'a mut Node<M>, path: &[Step]) -> Option<&'a mut Node<M>> {
    let Some((step, rest)) = path.split_first() else {
        return Some(node);
    };
    let child = kids(node).iter_mut().find(|(s, _)| s == step)?;
    at(&mut child.1, rest)
}

/// The Input that holds focus, if any.
pub fn focused<M>(node: &mut Node<M>) -> Option<&mut Node<M>> {
    if matches!(node, Node::Input { edit, .. } if edit.focus) {
        return Some(node);
    }
    kids(node).iter_mut().find_map(|(_, child)| focused(child))
}

fn kids<M>(node: &mut Node<M>) -> &mut [(Step, Node<M>)] {
    match node {
        Node::Row { children, .. } | Node::Col { children, .. } => children,
        _ => &mut [],
    }
}

#[cfg(test)]
mod tests;
