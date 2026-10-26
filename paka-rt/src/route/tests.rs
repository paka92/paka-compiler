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
                    focus: false,
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

/// The steps of the focused Input or Button in `tree`'s Col.
fn holder(tree: &Tree) -> Vec<Step> {
    let held = |node: &Tree| match node {
        Node::Input { edit, .. } => edit.focus,
        Node::Button { focus, .. } => *focus,
        _ => false,
    };
    let children = tree.children().iter();
    children
        .filter(|(_, node)| held(node))
        .map(|(step, _)| step.clone())
        .collect()
}

#[test]
fn tab_and_shift_tab_move_focus_through_inputs_and_buttons_and_wrap() {
    let mut tree = form("", "");
    for step in [key("a"), Step::Place(2), key("b"), key("a")] {
        assert_eq!(route(&mut tree, &Event::Tab(false)), None);
        assert_eq!(holder(&tree), [step]);
    }
    for step in [key("b"), Step::Place(2), key("a"), key("b")] {
        route(&mut tree, &Event::Tab(true));
        assert_eq!(holder(&tree), [step]);
    }
    let mut tree = form("", "");
    route(&mut tree, &Event::Tab(true));
    assert_eq!(holder(&tree), [key("b")]);
}

#[test]
fn enter_or_space_on_a_focused_button_clicks_it_and_other_keys_do_nothing() {
    let mut tree = form("", "");
    route(&mut tree, &Event::Tab(false));
    route(&mut tree, &Event::Tab(false));
    for c in [' ', '\n'] {
        let sent = route(&mut tree, &Event::Key(Key::Type(c)));
        assert_eq!(sent, Some(Sent::Click(&"Inc")));
    }
    for key in [Key::Type('x'), Key::Back, Key::Left(false)] {
        assert_eq!(route(&mut tree, &Event::Key(key)), None);
    }
    assert_eq!(holder(&tree), [Step::Place(2)]);
}

#[test]
fn in_an_input_space_types_a_space_and_enter_and_tab_type_nothing() {
    let mut tree = form("x", "");
    route(&mut tree, &click(&[key("a")], "Input"));
    assert_eq!(
        route(&mut tree, &Event::Key(Key::Type(' '))),
        Some(Sent::Change(&"A", typed("x ", 2)))
    );
    for c in ['\t', '\n'] {
        assert_eq!(route(&mut tree, &Event::Key(Key::Type(c))), None);
    }
    assert_eq!(holder(&tree), [key("a")]);
}

#[test]
fn a_click_moves_focus_from_a_button_only_to_an_input() {
    let mut tree = form("", "");
    route(&mut tree, &Event::Tab(true));
    route(&mut tree, &Event::Tab(true));
    assert_eq!(holder(&tree), [Step::Place(2)]);
    route(&mut tree, &click(&[Step::Place(2)], "Button"));
    route(&mut tree, &click(&[Step::Place(0)], "Text"));
    assert_eq!(holder(&tree), [Step::Place(2)]);
    route(&mut tree, &click(&[key("b")], "Input"));
    assert_eq!(holder(&tree), [key("b")]);
}

#[test]
fn a_focused_button_keeps_focus_by_its_step_and_loses_it_with_its_element() {
    let mut old = form("", "");
    route(&mut old, &Event::Tab(false));
    route(&mut old, &Event::Tab(false));
    let mut new = form("", "");
    keep(&old, &mut new);
    assert_eq!(holder(&new), [Step::Place(2)]);
    let mut changed = form("", "");
    if let Node::Col { children, .. } = &mut changed {
        children[2].1 = Node::Text("+".into());
    }
    keep(&new, &mut changed);
    assert!(holder(&changed).is_empty());
    assert_eq!(route(&mut changed, &Event::Key(Key::Type(' '))), None);
}

/// `form` with the Input keyed "a" focused, its cursor at the end of `a`.
fn composing(a: &str) -> Tree {
    let mut tree = form(a, "");
    route(&mut tree, &click(&[key("a")], "Input"));
    tree
}

fn preedit(tree: &Tree) -> &str {
    match &tree.children()[1].1 {
        Node::Input { edit, .. } => &edit.preedit,
        _ => unreachable!(),
    }
}

#[test]
fn a_preedit_stays_in_the_element_and_sends_nothing() {
    let mut tree = composing("ab");
    for shown in ["k", "ka", "か"] {
        assert_eq!(route(&mut tree, &Event::Preedit(shown.into())), None);
        assert_eq!(preedit(&tree), shown);
    }
    assert!(tree.to_string().contains("\"ab\" cursor=2 preedit=\"か\""));
}

#[test]
fn a_commit_ends_the_preedit_and_sends_one_change_at_the_cursor() {
    let mut tree = composing("ab");
    route(&mut tree, &Event::Key(Key::Left(false)));
    route(&mut tree, &Event::Preedit("にほ".into()));
    assert_eq!(
        route(&mut tree, &Event::Commit("日本".into())),
        Some(Sent::Change(&"A", typed("a日本b", 3)))
    );
    assert_eq!(preedit(&tree), "");
}

#[test]
fn a_commit_replaces_the_selection_and_an_empty_one_sends_nothing() {
    let mut tree = composing("abc");
    route(&mut tree, &Event::Key(Key::Home(true)));
    assert_eq!(
        route(&mut tree, &Event::Commit("x\ny".into())),
        Some(Sent::Change(&"A", typed("xy", 2)))
    );
    route(&mut tree, &Event::Preedit("z".into()));
    assert_eq!(route(&mut tree, &Event::Commit(String::new())), None);
    assert_eq!(preedit(&tree), "");
}

#[test]
fn a_preedit_moves_with_the_cursor_when_the_app_sets_the_text_and_ends_with_focus() {
    let old = {
        let mut tree = composing("ab");
        route(&mut tree, &Event::Preedit("k".into()));
        tree
    };
    let mut new = form("abcd", "");
    keep(&old, &mut new);
    assert_eq!(preedit(&new), "k");
    let mut short = form("a", "");
    keep(&new, &mut short);
    assert!(short.to_string().contains("\"a\" cursor=1 preedit=\"k\""));
    route(&mut short, &Event::Tab(false));
    assert_eq!(preedit(&short), "");
    assert_eq!(route(&mut short, &Event::Preedit("q".into())), None);
    assert_eq!(preedit(&short), "");
}

#[test]
fn input_method_text_with_no_focused_input_is_dropped() {
    let mut tree = form("", "");
    route(&mut tree, &Event::Preedit("k".into()));
    assert_eq!(route(&mut tree, &Event::Commit("k".into())), None);
    assert_eq!(tree, form("", ""));
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

#[test]
fn a_focus_request_moves_focus_as_tab_does_and_keeps_the_cursor() {
    let mut tree = form("mil", "");
    route(&mut tree, &click(&[key("a")], "Input"));
    route(&mut tree, &Event::Key(Key::Left(false)));
    for step in [Step::Place(2), key("b"), key("a")] {
        assert_eq!(route(&mut tree, &Event::Focus(vec![step.clone()])), None);
        assert_eq!(holder(&tree), [step]);
    }
    for path in [vec![Step::Place(0)], vec![], vec![key("z")]] {
        route(&mut tree, &Event::Focus(path));
        assert_eq!(holder(&tree), [key("a")]);
    }
    assert_eq!(
        route(&mut tree, &Event::Key(Key::Type('k'))),
        Some(Sent::Change(&"A", typed("mikl", 3)))
    );
}
