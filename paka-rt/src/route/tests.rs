use super::*;
use crate::tree::{Edit, keep};

type Tree = Node<&'static str>;

fn key(s: &str) -> Step {
    Step::Str(s.into())
}

fn input(text: &str, on_change: &'static str) -> Tree {
    Node::Input {
        text: text.into(),
        on_change,
        edit: Edit::default(),
    }
}

/// A Col of a Text, an Input keyed "a", a Button and an Input keyed "b".
fn form(a: &str, b: &str) -> Tree {
    Node::Col {
        gap: 0,
        children: vec![
            (Step::Place(0), Node::Text("hi".into())),
            (key("a"), input(a, "A")),
            (
                Step::Place(2),
                Node::Button {
                    label: "+".into(),
                    on: "Inc",
                },
            ),
            (key("b"), input(b, "B")),
        ],
    }
}

fn click(path: &[Step], kind: &'static str) -> Event {
    Event::Click(path.to_vec(), kind)
}

fn focus(tree: &Tree) -> Vec<Step> {
    tree.children()
        .iter()
        .filter(|(_, node)| matches!(node, Node::Input { edit, .. } if edit.focus))
        .map(|(step, _)| step.clone())
        .collect()
}

fn typed(text: &str, cursor: usize) -> Change {
    Change {
        text: text.into(),
        cursor,
    }
}

#[test]
fn a_click_on_a_button_sends_its_message_and_leaves_focus() {
    let mut tree = form("", "");
    route(&mut tree, &click(&[key("a")], "Input"));
    assert_eq!(
        route(&mut tree, &click(&[Step::Place(2)], "Button")),
        Some(Sent::Click(&"Inc"))
    );
    assert_eq!(focus(&tree), [key("a")]);
}

#[test]
fn nothing_has_focus_at_launch_so_keys_are_dropped() {
    let mut tree = form("x", "y");
    assert_eq!(route(&mut tree, &Event::Key(Key::Type('k'))), None);
    assert_eq!(tree, form("x", "y"));
}

#[test]
fn a_click_on_an_input_focuses_it_at_the_end_and_keys_reach_only_it() {
    let mut tree = form("mil", "ab");
    assert_eq!(route(&mut tree, &click(&[key("b")], "Input")), None);
    assert_eq!(focus(&tree), [key("b")]);
    assert_eq!(
        route(&mut tree, &Event::Key(Key::Type('c'))),
        Some(Sent::Change(&"B", typed("abc", 3)))
    );
    route(&mut tree, &click(&[key("a")], "Input"));
    assert_eq!(focus(&tree), [key("a")]);
    assert_eq!(
        route(&mut tree, &Event::Key(Key::Type('k'))),
        Some(Sent::Change(&"A", typed("milk", 4)))
    );
}

#[test]
fn clicks_on_text_space_or_nothing_leave_focus() {
    let mut tree = form("", "");
    route(&mut tree, &click(&[key("a")], "Input"));
    for event in [
        click(&[Step::Place(0)], "Text"),
        click(&[], "Col"),
        click(&[key("z")], "Input"),
    ] {
        assert_eq!(route(&mut tree, &event), None);
    }
    assert_eq!(focus(&tree), [key("a")]);
}

#[test]
fn tab_and_enter_are_dropped() {
    let mut tree = form("x", "");
    route(&mut tree, &click(&[key("a")], "Input"));
    for c in ['\t', '\n'] {
        assert_eq!(route(&mut tree, &Event::Key(Key::Type(c))), None);
    }
    assert_eq!(focus(&tree), [key("a")]);
}

#[test]
fn focus_stays_with_its_key_after_a_reorder() {
    let mut old = form("", "b");
    route(&mut old, &click(&[key("b")], "Input"));
    let mut new = form("", "b");
    if let Node::Col { children, .. } = &mut new {
        children.reverse();
    }
    keep(&old, &mut new);
    assert_eq!(focus(&new), [key("b")]);
    assert_eq!(
        route(&mut new, &Event::Key(Key::Type('x'))),
        Some(Sent::Change(&"B", typed("bx", 2)))
    );
}

#[test]
fn focus_is_dropped_with_its_element() {
    let mut old = form("", "b");
    route(&mut old, &click(&[key("b")], "Input"));
    let mut new = form("", "b");
    if let Node::Col { children, .. } = &mut new {
        children.pop();
    }
    keep(&old, &mut new);
    assert_eq!(route(&mut new, &Event::Key(Key::Type('x'))), None);
}

#[test]
fn a_click_aimed_at_an_element_the_next_diff_removed_is_dropped() {
    let queued = [
        click(&[Step::Place(2)], "Button"),
        click(&[key("b")], "Input"),
    ];
    let mut new = Node::Col {
        gap: 0,
        children: vec![(Step::Place(2), Node::Text("+".into()))],
    };
    for event in &queued {
        assert_eq!(route(&mut new, event), None);
    }
    assert!(focus(&new).is_empty());
}
