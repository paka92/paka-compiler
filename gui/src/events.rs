//! Window input as element input (M5-36, M5-37). A click is a press and release of the main
//! button on the same element; keys and the input method's text become `Event`s for the app's
//! queue, which works them out against the tree of their turn.

use crate::input;
use crate::layout::{Rect, hit};
use crate::route::Event;
use crate::tree::{Edit, Node, Step};
use winit::event::{ElementState, Ime, MouseButton};
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// The pointer: where it is in units, and the element the main button went down on.
#[derive(Debug, Default)]
pub struct Pointer {
    at: (u64, u64),
    pressed: Option<Vec<Step>>,
}

impl Pointer {
    /// Moves to `x`,`y` in pixels, divided by `scale` and rounded down (M5-05).
    pub fn moved(&mut self, x: f64, y: f64, scale: f64) {
        self.at = ((x / scale) as u64, (y / scale) as u64);
    }

    /// A mouse button went down or up over `tree`, laid out as `rects`: a click when the main
    /// button comes up over the element it went down on. Other buttons do nothing.
    pub fn button<M>(
        &mut self,
        button: MouseButton,
        state: ElementState,
        tree: &Node<M>,
        rects: &[Rect],
    ) -> Option<Event> {
        if button != MouseButton::Left {
            return None;
        }
        let under = hit(tree, rects, self.at.0, self.at.1);
        if state == ElementState::Pressed {
            self.pressed = under;
            return None;
        }
        let pressed = self.pressed.take();
        let path = under.filter(|p| pressed.as_ref() == Some(p))?;
        let kind = tree.at(&path)?.kind();
        Some(Event::Click(path, kind))
    }
}

/// What a key press makes. `text` is what the key types; with Control, Alt or Super held it
/// types nothing. Enter and Space are the named keys M5-08 gives a focused Button.
pub fn key(key: &Key, text: Option<&str>, mods: ModifiersState) -> Vec<Event> {
    let shift = mods.shift_key();
    let edit = match key {
        Key::Named(NamedKey::Tab) => return vec![Event::Tab(shift)],
        Key::Named(NamedKey::Enter) => input::Key::Type('\n'),
        Key::Named(NamedKey::Space) => input::Key::Type(' '),
        Key::Named(NamedKey::Backspace) => input::Key::Back,
        Key::Named(NamedKey::Delete) => input::Key::Delete,
        Key::Named(NamedKey::ArrowLeft) => input::Key::Left(shift),
        Key::Named(NamedKey::ArrowRight) => input::Key::Right(shift),
        Key::Named(NamedKey::Home) => input::Key::Home(shift),
        Key::Named(NamedKey::End) => input::Key::End(shift),
        _ if mods.control_key() || mods.alt_key() || mods.super_key() => return vec![],
        _ => {
            let typed = text.unwrap_or_default().chars().filter(|c| !c.is_control());
            return typed.map(|c| Event::Key(input::Key::Type(c))).collect();
        }
    };
    vec![Event::Key(edit)]
}

/// What the input method's `ime` makes, `shown` being the focused Input's preedit. A preedit
/// equal to the shown one is dropped (M5-08): moving the cursor area makes the input method send
/// its preedit again, and showing it again would repaint without end. Enabling or disabling the
/// input method ends any preedit.
pub fn ime(ime: Ime, shown: &str) -> Option<Event> {
    let preedit = match ime {
        Ime::Commit(text) => return Some(Event::Commit(text)),
        Ime::Preedit(text, _) => text,
        Ime::Enabled | Ime::Disabled => String::new(),
    };
    (preedit != shown).then_some(Event::Preedit(preedit))
}

/// The focused Input of `tree`, laid out as `rects`: its rectangle, its text and its cursor
/// and preedit.
pub fn focused_input<'a, M>(
    tree: &'a Node<M>,
    rects: &[Rect],
) -> Option<(Rect, &'a str, &'a Edit)> {
    let mut nodes = Vec::new();
    pre_order(tree, &mut nodes);
    nodes
        .into_iter()
        .zip(rects)
        .find_map(|(node, r)| match node {
            Node::Input { text, edit, .. } if edit.focus => Some((*r, text.as_str(), edit)),
            _ => None,
        })
}

fn pre_order<'a, M>(node: &'a Node<M>, out: &mut Vec<&'a Node<M>>) {
    out.push(node);
    for (_, child) in node.children() {
        pre_order(child, out);
    }
}
