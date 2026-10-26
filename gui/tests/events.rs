//! Window input as element input (M5-36, M5-37), on a hand-laid-out tree: clicks by press and
//! release at the scale factor, keys, and the input method's text.

use paka_gui::events::{Pointer, focused_input, ime, key};
use paka_gui::input::Key as Edit;
use paka_gui::layout::Rect;
use paka_gui::route::{Event, route};
use paka_gui::tree::{Node, Step};
use winit::event::{ElementState, Ime, MouseButton};
use winit::keyboard::{Key, ModifiersState, NamedKey};

type Tree = Node<&'static str>;

/// A Col of a Button "+" at 0,0 sized 40 by 32 and an Input "in" at 0,40 sized 200 by 32.
fn form() -> (Tree, Vec<Rect>) {
    let tree = Node::Col {
        gap: 8,
        children: vec![
            (
                Step::Place(0),
                Node::Button {
                    label: "+".into(),
                    on: "Inc",
                    focus: false,
                },
            ),
            (
                Step::Str("in".into()),
                Node::Input {
                    text: "ab".into(),
                    on_change: "Edit",
                    edit: Default::default(),
                },
            ),
        ],
    };
    let rect = |y, w| Rect { x: 0, y, w, h: 32 };
    (tree, vec![rect(0, 200), rect(0, 40), rect(40, 200)])
}

/// What pressing and releasing `button` at the two points in pixels at `scale` makes.
fn click(button: MouseButton, down: (f64, f64), up: (f64, f64), scale: f64) -> Option<Event> {
    let (tree, rects) = form();
    let mut pointer = Pointer::default();
    pointer.moved(down.0, down.1, scale);
    let pressed = pointer.button(button, ElementState::Pressed, &tree, &rects);
    assert_eq!(pressed, None);
    pointer.moved(up.0, up.1, scale);
    pointer.button(button, ElementState::Released, &tree, &rects)
}

fn plus() -> Option<Event> {
    Some(Event::Click(vec![Step::Place(0)], "Button"))
}

#[test]
fn a_click_is_a_press_and_release_of_the_main_button_on_one_element() {
    let left = MouseButton::Left;
    assert_eq!(click(left, (5.0, 5.0), (30.0, 20.0), 1.0), plus());
    let input = Some(Event::Click(vec![Step::Str("in".into())], "Input"));
    assert_eq!(click(left, (5.0, 50.0), (150.0, 60.0), 1.0), input);
    assert_eq!(click(left, (5.0, 5.0), (5.0, 50.0), 1.0), None);
    assert_eq!(click(left, (5.0, 36.0), (5.0, 36.0), 1.0), None);
    for other in [MouseButton::Right, MouseButton::Middle] {
        assert_eq!(click(other, (5.0, 5.0), (5.0, 5.0), 1.0), None);
    }
}

#[test]
fn a_release_with_no_press_on_its_element_does_nothing() {
    let (tree, rects) = form();
    let mut pointer = Pointer::default();
    pointer.moved(5.0, 5.0, 1.0);
    let up = ElementState::Released;
    assert_eq!(pointer.button(MouseButton::Left, up, &tree, &rects), None);
    pointer.moved(5.0, 36.0, 1.0);
    pointer.button(MouseButton::Left, ElementState::Pressed, &tree, &rects);
    pointer.button(MouseButton::Left, up, &tree, &rects);
    pointer.moved(5.0, 5.0, 1.0);
    assert_eq!(pointer.button(MouseButton::Left, up, &tree, &rects), None);
}

#[test]
fn pointer_positions_are_divided_by_the_scale_and_rounded_down() {
    let left = MouseButton::Left;
    assert_eq!(click(left, (79.0, 63.0), (79.0, 63.0), 2.0), plus());
    assert_eq!(click(left, (80.0, 10.0), (80.0, 10.0), 2.0), None);
    assert_eq!(click(left, (59.0, 47.0), (59.0, 47.0), 1.5), plus());
    assert_eq!(click(left, (59.0, 48.0), (59.0, 48.0), 1.5), None);
}

fn typed(c: char) -> Event {
    Event::Key(Edit::Type(c))
}

#[test]
fn keys_become_tab_enter_space_and_the_editing_keys() {
    let none = ModifiersState::empty();
    let shift = ModifiersState::SHIFT;
    let named = |k| Key::Named(k);
    let cases = [
        (named(NamedKey::Tab), None, none, vec![Event::Tab(false)]),
        (named(NamedKey::Tab), None, shift, vec![Event::Tab(true)]),
        (named(NamedKey::Enter), Some("\r"), none, vec![typed('\n')]),
        (named(NamedKey::Space), Some(" "), none, vec![typed(' ')]),
        (
            named(NamedKey::Backspace),
            None,
            none,
            vec![Event::Key(Edit::Back)],
        ),
        (
            named(NamedKey::Delete),
            None,
            none,
            vec![Event::Key(Edit::Delete)],
        ),
        (
            named(NamedKey::ArrowLeft),
            None,
            shift,
            vec![Event::Key(Edit::Left(true))],
        ),
        (
            named(NamedKey::ArrowRight),
            None,
            none,
            vec![Event::Key(Edit::Right(false))],
        ),
        (
            named(NamedKey::Home),
            None,
            shift,
            vec![Event::Key(Edit::Home(true))],
        ),
        (
            named(NamedKey::End),
            None,
            none,
            vec![Event::Key(Edit::End(false))],
        ),
        (
            Key::Character("é".into()),
            Some("é"),
            none,
            vec![typed('é')],
        ),
        (
            Key::Character("A".into()),
            Some("A"),
            shift,
            vec![typed('A')],
        ),
        (
            Key::Character("c".into()),
            Some("c"),
            ModifiersState::CONTROL,
            vec![],
        ),
        (
            Key::Character("x".into()),
            Some("x"),
            ModifiersState::ALT,
            vec![],
        ),
        (named(NamedKey::Escape), Some("\u{1b}"), none, vec![]),
        (named(NamedKey::F1), None, none, vec![]),
    ];
    for (k, text, mods, want) in cases {
        assert_eq!(key(&k, text, mods), want, "{k:?} {mods:?}");
    }
}

#[test]
fn tab_then_space_in_the_window_clicks_the_focused_button() {
    let (mut tree, _) = form();
    let mut sent = vec![];
    for (k, text) in [(NamedKey::Tab, None), (NamedKey::Space, Some(" "))] {
        for event in key(&Key::Named(k), text, ModifiersState::empty()) {
            sent.push(format!("{:?}", route(&mut tree, &event)));
        }
    }
    assert_eq!(sent, ["None", "Some(Click(\"Inc\"))"]);
}

#[test]
fn a_preedit_equal_to_the_shown_one_is_dropped() {
    let preedit = |s: &str| Ime::Preedit(s.into(), Some((s.len(), s.len())));
    assert_eq!(ime(preedit("k"), "k"), None);
    assert_eq!(ime(preedit(""), ""), None);
    assert_eq!(ime(preedit("ka"), "k"), Some(Event::Preedit("ka".into())));
    assert_eq!(ime(preedit(""), "k"), Some(Event::Preedit("".into())));
    assert_eq!(ime(Ime::Enabled, ""), None);
    assert_eq!(ime(Ime::Disabled, "k"), Some(Event::Preedit("".into())));
    assert_eq!(
        ime(Ime::Commit("か".into()), "か"),
        Some(Event::Commit("か".into()))
    );
}

#[test]
fn a_composed_word_reaches_the_app_as_one_change() {
    let (mut tree, rects) = form();
    route(
        &mut tree,
        &Event::Click(vec![Step::Str("in".into())], "Input"),
    );
    let steps = [
        Ime::Preedit("n".into(), None),
        Ime::Preedit("に".into(), None),
        Ime::Preedit("に".into(), None),
        Ime::Preedit("にほ".into(), None),
        Ime::Preedit("".into(), None),
        Ime::Commit("日本".into()),
    ];
    let mut changes = vec![];
    let mut sent = 0;
    for step in steps {
        let shown = focused_input(&tree, &rects).map_or("", |(_, _, e)| e.preedit.as_str());
        let Some(event) = ime(step, shown) else {
            continue;
        };
        sent += 1;
        if let Some(change) = route(&mut tree, &event) {
            changes.push(format!("{change:?}"));
        }
    }
    assert_eq!(sent, 5);
    assert_eq!(
        changes,
        ["Change(\"Edit\", Change { text: \"ab日本\", cursor: 4 })"]
    );
    let (r, text, edit) = focused_input(&tree, &rects).unwrap();
    assert_eq!((r.y, text, edit.preedit.as_str()), (40, "ab", ""));
}
