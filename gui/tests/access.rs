//! The element tree as AccessKit nodes and AccessKit actions as element input (M5-39), on
//! hand-built trees.

use accesskit::{Action, NodeId, Rect as Bounds, Role, TreeUpdate};
use paka_gui::access::{Ids, WINDOW, action};
use paka_rt::layout::Rect;
use paka_rt::route::{Event, Sent, route};
use paka_rt::tree::{Edit, Node, Step};

type Tree = Node<&'static str>;

fn key(s: &str) -> Step {
    Step::Str(s.into())
}

fn button(label: &str, on: &'static str) -> Tree {
    Node::Button {
        label: label.into(),
        on,
        focus: false,
    }
}

/// A Col of a Text, a Row keyed "k" of "-" and "+", an Input keyed "in" with focus, a hole and
/// an error.
fn form() -> Tree {
    let edit = Edit {
        cursor: 1,
        focus: true,
        ..Edit::default()
    };
    let row = vec![
        (Step::Place(0), button("-", "Dec")),
        (Step::Place(1), button("+", "Inc")),
    ];
    Node::Col {
        gap: 8,
        children: vec![
            (Step::Place(0), Node::Text("0".into())),
            (
                key("k"),
                Node::Row {
                    gap: 8,
                    children: row,
                },
            ),
            (
                key("in"),
                Node::Input {
                    text: "ab".into(),
                    on_change: "Edit",
                    edit,
                },
            ),
            (Step::Place(3), Node::Hole("reset".into())),
            (Step::Place(4), Node::Error("boom".into())),
        ],
    }
}

/// One rect per node in pre-order, each 10 units below the last.
fn rects(n: u64) -> Vec<Rect> {
    let rect = |i| Rect {
        x: 2,
        y: i * 10,
        w: 40,
        h: 8,
    };
    (0..n).map(rect).collect()
}

fn node(update: &TreeUpdate, id: NodeId) -> &accesskit::Node {
    &update.nodes.iter().find(|(i, _)| *i == id).unwrap().1
}

/// The number at `path`.
fn id(ids: &Ids, update: &TreeUpdate, path: &[Step]) -> NodeId {
    let mut found = update.nodes.iter().map(|(i, _)| *i);
    found.find(|i| ids.path(*i) == Some(path)).unwrap()
}

#[test]
fn each_element_gets_its_role_name_value_actions_and_bounds() {
    let mut ids = Ids::default();
    let update = ids.update("counter", &form(), &rects(8), 2.0);
    let window = node(&update, WINDOW);
    assert_eq!(
        (window.role(), window.label()),
        (Role::Window, Some("counter"))
    );
    let at = |path: &[Step]| node(&update, id(&ids, &update, path));
    assert_eq!(window.children(), [id(&ids, &update, &[])]);
    let expect = [
        (vec![], Role::GenericContainer, None, None),
        (vec![Step::Place(0)], Role::Label, None, Some("0")),
        (vec![key("k")], Role::GenericContainer, None, None),
        (
            vec![key("k"), Step::Place(1)],
            Role::Button,
            Some("+"),
            None,
        ),
        (vec![key("in")], Role::TextInput, Some("in"), Some("ab")),
        (vec![Step::Place(3)], Role::Label, None, Some("??reset")),
        (vec![Step::Place(4)], Role::Label, None, Some("boom")),
    ];
    for (path, role, label, value) in expect {
        let n = at(&path);
        assert_eq!(
            (n.role(), n.label(), n.value()),
            (role, label, value),
            "{path:?}"
        );
    }
    let plus = at(&[key("k"), Step::Place(1)]);
    assert!(plus.supports_action(Action::Click) && plus.supports_action(Action::Focus));
    let input = at(&[key("in")]);
    assert!(input.supports_action(Action::Focus) && !input.supports_action(Action::Click));
    assert!(!at(&[Step::Place(0)]).supports_action(Action::Click));
    // The fifth node in pre-order, "+", at y 40 units, in pixels at scale 2.
    assert_eq!(plus.bounds(), Some(Bounds::new(4.0, 80.0, 84.0, 96.0)));
    let row = at(&[key("k")]).children();
    assert_eq!(
        row,
        [
            id(&ids, &update, &[key("k"), Step::Place(0)]),
            id(&ids, &update, &[key("k"), Step::Place(1)])
        ]
    );
    assert_eq!(update.focus, id(&ids, &update, &[key("in")]));
}

#[test]
fn numbers_are_unique_and_stay_with_their_element_across_updates() {
    let mut ids = Ids::default();
    let first = ids.update("counter", &form(), &rects(8), 1.0);
    let mut numbers: Vec<_> = first.nodes.iter().map(|(i, _)| *i).collect();
    numbers.sort();
    numbers.dedup();
    assert_eq!(numbers.len(), 9);
    let kept = |ids: &Ids, update| id(ids, update, &[key("in")]);
    let (input, minus) = (
        kept(&ids, &first),
        id(&ids, &first, &[key("k"), Step::Place(0)]),
    );

    // Drop "-" and the Input's focus; put a new Text first.
    let mut tree = form();
    let Node::Col { children, .. } = &mut tree else {
        unreachable!()
    };
    if let Node::Row { children, .. } = &mut children[1].1 {
        children.remove(0);
    }
    if let Node::Input { edit, .. } = &mut children[2].1 {
        edit.focus = false;
    }
    children.insert(0, (key("new"), Node::Text("hi".into())));
    let second = ids.update("counter", &tree, &rects(8), 1.0);
    assert_eq!(kept(&ids, &second), input);
    assert_eq!(ids.path(minus), None);
    let fresh = id(&ids, &second, &[key("new")]);
    assert!(numbers.iter().all(|n| *n != fresh));
    assert_eq!(second.focus, WINDOW);
}

#[test]
fn a_click_action_sends_the_message_and_a_focus_action_moves_focus_as_tab_does() {
    let mut tree = form();
    let plus = [key("k"), Step::Place(1)];
    let click = action(&tree, &plus, Action::Click).unwrap();
    assert_eq!(click, Event::Click(plus.to_vec(), "Button"));
    assert_eq!(route(&mut tree, &click), Some(Sent::Click(&"Inc")));
    assert_eq!(tree, form());

    for none in [
        action(&tree, &[key("in")], Action::Click),
        action(&tree, &[Step::Place(0)], Action::Focus),
        action(&tree, &[key("gone")], Action::Focus),
        action(&tree, &plus, Action::ScrollIntoView),
    ] {
        assert_eq!(none, None);
    }

    let focus = action(&tree, &plus, Action::Focus).unwrap();
    assert_eq!(focus, Event::Focus(plus.to_vec()));
    let mut tabbed = form();
    route(&mut tree, &focus);
    route(&mut tabbed, &Event::Tab(true));
    assert_eq!(tree, tabbed);
    let back = action(&tree, &[key("in")], Action::Focus).unwrap();
    route(&mut tree, &back);
    assert_eq!(tree, form());
}
