//! Clicks and keys to messages (M5-18). Focus lives in the focused element, an Input's `Edit` or a
//! Button's mark, so `keep` carries it by key and drops it with the element (M5-02); at most one
//! element holds it.

use crate::input::{Change, Key};
use crate::tree::{Node, Step};

/// An event as queued: a click aimed at the element of that kind at that path, a key, Tab
/// (Shift+Tab when true), or the input method's preedit or commit. Enter arrives as
/// `Key::Type('\n')`, Space as `Key::Type(' ')`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Click(Vec<Step>, &'static str),
    Key(Key),
    Tab(bool),
    Preedit(String),
    Commit(String),
}

/// What an event sends: a Button's message, or an Input's `on_change` with the new text, which
/// the caller turns into a message.
#[derive(Debug, PartialEq, Eq)]
pub enum Sent<'a, M> {
    Click(&'a M),
    Change(&'a M, Change),
}

/// Works `event` out against `tree`, the tree of its turn in the queue (M5-03). A click whose
/// element is gone or changed kind is dropped; a click on an Input moves focus there with the
/// cursor at the end (M5-06), any other leaves it. Tab and Shift+Tab move focus through Inputs
/// and Buttons in tree order and wrap; Enter or Space on a focused Button clicks it (M5-08). A key
/// with nothing focused is dropped. A preedit stays in the focused Input and sends nothing; a
/// commit ends it and sends one change (M5-03).
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
            Node::Button { on, .. } if matches!(key, Key::Type(' ' | '\n')) => {
                Some(Sent::Click(on))
            }
            _ => None,
        },
        Event::Preedit(shown) => {
            if let Node::Input { edit, .. } = focused(tree)? {
                edit.preedit.clone_from(shown);
            }
            None
        }
        Event::Commit(put) => match focused(tree)? {
            Node::Input {
                text,
                on_change,
                edit,
            } => edit
                .commit(text, put)
                .map(|change| Sent::Change(on_change, change)),
            _ => None,
        },
        Event::Tab(back) => {
            tab(tree, *back);
            None
        }
        Event::Click(path, kind) => {
            if at(tree, path)?.kind() != *kind {
                return None;
            }
            if *kind == "Input"
                && let Some(node) = focused(tree)
            {
                set_focus(node, false);
            }
            match at(tree, path)? {
                Node::Button { on, .. } => Some(Sent::Click(on)),
                Node::Input { text, edit, .. } => {
                    edit.focus = true;
                    edit.key(text, Key::End(false));
                    None
                }
                _ => None,
            }
        }
    }
}

/// Moves focus to the next Input or Button in tree order, or the one before when `back`, wrapping
/// around; with nothing focused, to the first or the last.
fn tab<M>(tree: &mut Node<M>, back: bool) {
    let mut stops = vec![];
    focusable(tree, &mut stops);
    let n = stops.len();
    let at = stops.iter().position(|node| has_focus(node));
    let next = match (at, back) {
        _ if n == 0 => return,
        (None, false) => 0,
        (None, true) => n - 1,
        (Some(i), false) => (i + 1) % n,
        (Some(i), true) => (i + n - 1) % n,
    };
    if let Some(i) = at.filter(|&i| i != next) {
        set_focus(stops[i], false);
    }
    set_focus(stops[next], true);
}

fn focusable<'a, M>(node: &'a mut Node<M>, out: &mut Vec<&'a mut Node<M>>) {
    match node {
        Node::Input { .. } | Node::Button { .. } => out.push(node),
        Node::Row { children, .. } | Node::Col { children, .. } => {
            for (_, child) in children {
                focusable(child, out);
            }
        }
        _ => {}
    }
}

fn has_focus<M>(node: &Node<M>) -> bool {
    match node {
        Node::Input { edit, .. } => edit.focus,
        Node::Button { focus, .. } => *focus,
        _ => false,
    }
}

/// Gives or takes focus; an Input that loses it drops its preedit (M5-03).
fn set_focus<M>(node: &mut Node<M>, on: bool) {
    match node {
        Node::Input { edit, .. } => {
            edit.focus = on;
            if !on {
                edit.preedit.clear();
            }
        }
        Node::Button { focus, .. } => *focus = on,
        _ => {}
    }
}

fn at<'a, M>(node: &'a mut Node<M>, path: &[Step]) -> Option<&'a mut Node<M>> {
    let Some((step, rest)) = path.split_first() else {
        return Some(node);
    };
    let child = kids(node).iter_mut().find(|(s, _)| s == step)?;
    at(&mut child.1, rest)
}

/// The Input or Button that holds focus, if any.
pub fn focused<M>(node: &mut Node<M>) -> Option<&mut Node<M>> {
    if has_focus(node) {
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
