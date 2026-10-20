//! The UI example functions (docs/decisions/M5-06.md): the program's visual app with no window,
//! driven by an example. Each action is handled to the end, its view included. Effects are not
//! performed: a granted one is dropped, so no answer arrives, and `Exit` ends the app.

use super::{Done, Kind, Machine, Stop, Trap};
use crate::builtins::{UI, name};
use crate::core_form::{App, Site};
use crate::interp::effects::{World, perform};
use crate::interp::start_message;
use crate::interp::value::{Message, Value};
use paka_rt::effects::Reason;
use paka_rt::input::Key;
use paka_rt::route::{Event, Sent, focused, route};
use paka_rt::tree::{self, Node, Step};
use std::collections::VecDeque;
use std::mem;

#[derive(Debug, PartialEq)]
pub struct Ui {
    state: Option<Value>,
    tree: Node<Message>,
    /// After `Exit`: reads see the last tree and an action is an assert trap.
    exited: bool,
}

impl<'a> Machine<'a> {
    /// Runs row `k` of `builtins::UI` on `args`, which it may update; the handler runs at depth 1.
    pub(super) fn ui(&mut self, k: usize, args: &mut [Value], site: Site) -> Done {
        let depth = mem::replace(&mut self.depth, 0);
        let done = self.ui_call(name(UI[k].1), args, site);
        self.depth = depth;
        done
    }

    fn ui_call(&mut self, f: &str, args: &mut [Value], site: Site) -> Done {
        let assert = || Stop::from(Trap::bare(Kind::Assert, site));
        let steps = |path: &str| tree::path(path).ok_or_else(assert);
        Ok(match (f, args) {
            ("ui_start", []) => Value::Ui(Box::new(self.ui_start(site)?)),
            ("ui_click" | "ui_type", [Value::Ui(ui), _]) if ui.exited => return Err(assert()),
            ("ui_click", [Value::Ui(ui), Value::Str(path)]) => {
                self.click(ui, steps(path.as_str())?, site)?;
                Value::Unit
            }
            ("ui_type", [Value::Ui(ui), Value::Str(text)]) => {
                for c in text.as_str().chars() {
                    self.key(ui, c, site)?;
                }
                Value::Unit
            }
            ("ui_tree", [Value::Ui(ui)]) => {
                match first(&ui.tree, &|n| matches!(n, Node::Hole(_))) {
                    Some((_, Node::Hole(name))) => return Err(placeholder(name, site)),
                    _ => Value::Str(ui.tree.to_string().as_str().into()),
                }
            }
            ("ui_text", [Value::Ui(ui), Value::Str(path)]) => {
                match find(&ui.tree, &steps(path.as_str())?) {
                    Some(
                        Node::Text(t) | Node::Button { label: t, .. } | Node::Input { text: t, .. },
                    ) => Value::Str(t.as_str().into()),
                    Some(Node::Hole(name)) => return Err(placeholder(name, site)),
                    _ => return Err(assert()),
                }
            }
            ("ui_has", [Value::Ui(ui), Value::Str(path)]) => {
                match find(&ui.tree, &steps(path.as_str())?) {
                    Some(Node::Hole(name)) => return Err(placeholder(name, site)),
                    found => Value::Bool(found.is_some()),
                }
            }
            _ => return Err(Stop::Internal(format!("{f} does not take these arguments"))),
        })
    }

    /// M5-01's launch order: state from the defaults, the start message, the first tree.
    fn ui_start(&mut self, site: Site) -> Done<Ui> {
        let app = self.app();
        let mut ui = Ui {
            state: self.state(app)?,
            tree: Node::Error(String::new()),
            exited: false,
        };
        match app.start {
            Some(start) => {
                let message = start_message(self.core, app, start, &[]);
                self.turn(
                    &mut ui,
                    message.map_err(|(_, line)| Stop::Ended(line))?,
                    None,
                    site,
                )?
            }
            None => self.redraw(&mut ui, None, site)?,
        }
        Ok(ui)
    }

    /// A Button sends its message; an Input takes focus, its cursor at the end.
    fn click(&mut self, ui: &mut Ui, path: Vec<Step>, site: Site) -> Done<()> {
        let kind = match find(&ui.tree, &path) {
            Some(Node::Hole(name)) => return Err(placeholder(name, site)),
            Some(node @ (Node::Button { .. } | Node::Input { .. })) => node.kind(),
            _ => return Err(Trap::bare(Kind::Assert, site).into()),
        };
        let Some(Sent::Click(on)) = route(&mut ui.tree, &Event::Click(path, kind)) else {
            return Ok(());
        };
        let message = on
            .value
            .copy()
            .ok_or_else(|| Trap::bare(Kind::Stack, site))?;
        self.turn(ui, message, None, site)
    }

    /// One code point to the focused Input; `\n` and tab do nothing.
    fn key(&mut self, ui: &mut Ui, c: char, site: Site) -> Done<()> {
        let Some(Node::Input { text, .. }) = focused(&mut ui.tree) else {
            return Err(Trap::bare(Kind::Assert, site).into());
        };
        let before = text.clone();
        let Some(Sent::Change(on, change)) = route(&mut ui.tree, &Event::Key(Key::Type(c))) else {
            return Ok(());
        };
        let Value::Fn(f) = on.value else {
            unreachable!("on_change takes a variant");
        };
        let message = self.answer(f, Value::Str(change.text.as_str().into()))?;
        self.turn(ui, message, Some((before, change.cursor)), site)
    }

    /// `update`, its effects, then the view; a typed key's cursor follows last.
    fn turn(&mut self, ui: &mut Ui, message: Value, typed: Typed, site: Site) -> Done<()> {
        let app = self.app();
        let (mut effects, state) = self.update(app, ui.state.take(), message)?;
        ui.state = state;
        if matches!(effects, Value::List(_)) {
            match perform(app, &mut effects, &mut Dropped, &mut VecDeque::new()) {
                Some((_, line)) if !line.is_empty() => return Err(Stop::Ended(line)),
                Some(_) => ui.exited = true,
                None => {}
            }
        }
        if ui.exited {
            return Ok(());
        }
        self.redraw(ui, typed, site)
    }

    /// Draws a new tree; the first frame that holds an error node is kept for the status.
    fn redraw(&mut self, ui: &mut Ui, typed: Typed, site: Site) -> Done<()> {
        let view = self.app().view.expect("a visual app");
        let state = ui.state.as_ref().and_then(Value::copy);
        let mut drawn = self.draw(
            view,
            vec![state.ok_or_else(|| Trap::bare(Kind::Stack, site))?],
        )?;
        tree::keep(&ui.tree, &mut drawn);
        ui.tree = drawn;
        if let Some((before, cursor)) = typed
            && let Some(Node::Input { text, edit, .. }) = focused(&mut ui.tree)
        {
            edit.follow(&before, text, Some(cursor));
        }
        if self.broken.is_none()
            && let Some((path, Node::Error(why))) =
                first(&ui.tree, &|n| matches!(n, Node::Error(_)))
        {
            self.broken = Some((site, format!("Error {path} {}", paka_rt::ops::quoted(why))));
        }
        Ok(())
    }

    fn app(&self) -> &'a App {
        self.core
            .app
            .as_ref()
            .expect("a program with a UI example has an app")
    }
}

/// The text a typed key leaves the Input showing before its turn, and the cursor it sent.
type Typed = Option<(String, usize)>;

impl Ui {
    /// A deep copy, or `None` when it nests too deep; boxed here to keep `Value::copy`'s frame small.
    pub fn copy(&self) -> Option<Box<Ui>> {
        let state = match &self.state {
            Some(s) => Some(s.copy()?),
            None => None,
        };
        let tree = copy_tree(&self.tree)?;
        Some(Box::new(Ui {
            state,
            tree,
            exited: self.exited,
        }))
    }
}

fn copy_tree(node: &Node<Message>) -> Option<Node<Message>> {
    let message = |m: &Message| {
        let value = m.value.copy()?;
        Some(Message {
            value,
            text: m.text.clone(),
        })
    };
    let kids = |children: &[(Step, Node<Message>)]| {
        let copy = |(s, n): &(Step, Node<Message>)| Some((s.clone(), copy_tree(n)?));
        children.iter().map(copy).collect::<Option<_>>()
    };
    Some(match node {
        Node::Text(t) => Node::Text(t.clone()),
        Node::Button { label, on } => Node::Button {
            label: label.clone(),
            on: message(on)?,
        },
        Node::Input {
            text,
            on_change,
            edit,
        } => Node::Input {
            text: text.clone(),
            on_change: message(on_change)?,
            edit: edit.clone(),
        },
        Node::Row { gap, children } => Node::Row {
            gap: *gap,
            children: kids(children)?,
        },
        Node::Col { gap, children } => Node::Col {
            gap: *gap,
            children: kids(children)?,
        },
        Node::Hole(name) => Node::Hole(name.clone()),
        Node::Error(why) => Node::Error(why.clone()),
    })
}

/// A touched placeholder: the example is unfinished at the call that touched it.
fn placeholder(name: &str, site: Site) -> Stop {
    Trap::new(Kind::Hole, site, format!("??{name}")).into()
}

/// The node at `path`, if any.
fn find<'t>(node: &'t Node<Message>, path: &[Step]) -> Option<&'t Node<Message>> {
    let Some((step, rest)) = path.split_first() else {
        return Some(node);
    };
    let child = node.children().iter().find(|(s, _)| s == step)?;
    find(&child.1, rest)
}

/// The first node in tree order that `is`, with its path.
fn first<'t>(
    node: &'t Node<Message>,
    is: &dyn Fn(&Node<Message>) -> bool,
) -> Option<(String, &'t Node<Message>)> {
    if is(node) {
        return Some((String::new(), node));
    }
    node.children().iter().find_map(|(step, child)| {
        let (path, found) = first(child, is)?;
        let path = if path.is_empty() {
            step.to_string()
        } else {
            format!("{step}/{path}")
        };
        Some((path, found))
    })
}

/// The world of a UI example, which drops every granted effect.
struct Dropped;

impl World for Dropped {
    fn print(&mut self, _: &str) -> Result<(), Reason> {
        Ok(())
    }

    fn read(&mut self, _: &str) -> Result<String, Reason> {
        Err("dropped")
    }

    fn write(&mut self, _: &str, _: &str) -> Result<(), Reason> {
        Ok(())
    }
}
