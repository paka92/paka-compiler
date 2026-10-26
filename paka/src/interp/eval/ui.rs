//! The UI example functions (docs/decisions/M5-06.md): the program's visual app with no window,
//! driven by an example. Each action is handled to the end, its view included. Effects are not
//! performed: a granted one is dropped, so no answer arrives, and `Exit` ends the app. Each
//! action is logged with the events that led to it, for M5-26's comparison with dev mode.

use super::{Done, Kind, Machine, Stop, Trap};
use crate::builtins::{UI, name};
use crate::core_form::{App, Site};
use crate::interp::effects::perform;
use crate::interp::value::{Message, Value};
use crate::interp::{report, start_message};
use paka_rt::effects::Dropped;
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
    /// The events sent since the start; `None` once an effect's answer was dropped, as dev mode
    /// would handle it.
    events: Option<Vec<Event>>,
}

/// What a UI example's action left: the tree, the tree after `Exit`, a trap report's first line,
/// or the line of an end.
#[derive(Debug, Clone, PartialEq)]
pub enum Left {
    Tree(String),
    Exited(String),
    Trapped(String),
    Ended(String),
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
                match ui.tree.at(&steps(path.as_str())?) {
                    Some(
                        Node::Text(t) | Node::Button { label: t, .. } | Node::Input { text: t, .. },
                    ) => Value::Str(t.as_str().into()),
                    Some(Node::Hole(name)) => return Err(placeholder(name, site)),
                    _ => return Err(assert()),
                }
            }
            ("ui_has", [Value::Ui(ui), Value::Str(path)]) => {
                match ui.tree.at(&steps(path.as_str())?) {
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
            events: Some(vec![]),
        };
        let done = match app.start {
            Some(start) => match start_message(self.core, app, start, &[]) {
                Ok(message) => self.turn(&mut ui, message, None, site),
                Err((_, line)) => Err(Stop::Ended(line)),
            },
            None => self.redraw(&mut ui, None, site),
        };
        self.log(&ui, &done);
        done.map(|()| ui)
    }

    /// A Button sends its message; an Input takes focus, its cursor at the end.
    fn click(&mut self, ui: &mut Ui, path: Vec<Step>, site: Site) -> Done<()> {
        let kind = match ui.tree.at(&path) {
            Some(Node::Hole(name)) => return Err(placeholder(name, site)),
            Some(node @ (Node::Button { .. } | Node::Input { .. })) => node.kind(),
            _ => return Err(Trap::bare(Kind::Assert, site).into()),
        };
        let event = Event::Click(path, kind);
        let message = match route(&mut ui.tree, &event) {
            Some(Sent::Click(on)) => Some(on.value.copy()),
            _ => None,
        };
        let done = match message {
            Some(Some(message)) => self.turn(ui, message, None, site),
            Some(None) => Err(Trap::bare(Kind::Stack, site).into()),
            None => Ok(()),
        };
        self.sent(ui, event, done)
    }

    /// One code point to the focused Input; `\n` and tab do nothing.
    fn key(&mut self, ui: &mut Ui, c: char, site: Site) -> Done<()> {
        let Some(Node::Input { text, .. }) = focused(&mut ui.tree) else {
            return Err(Trap::bare(Kind::Assert, site).into());
        };
        let before = text.clone();
        let event = Event::Key(Key::Type(c));
        let changed = match route(&mut ui.tree, &event) {
            Some(Sent::Change(on, change)) => {
                let Value::Fn(f) = on.value else {
                    unreachable!("on_change takes a variant");
                };
                Some((f, change))
            }
            _ => None,
        };
        let done = match changed {
            Some((f, change)) => (self.answer(f, Value::Str(change.text.as_str().into())))
                .and_then(|message| self.turn(ui, message, Some((before, change.cursor)), site)),
            None => Ok(()),
        };
        self.sent(ui, event, done)
    }

    /// Notes `event` as sent to `ui` and logs what it left.
    fn sent(&mut self, ui: &mut Ui, event: Event, done: Done<()>) -> Done<()> {
        if let Some(events) = &mut ui.events {
            events.push(event);
        }
        self.log(ui, &done);
        done
    }

    /// Logs what an action left, when the machine keeps a log.
    fn log(&mut self, ui: &Ui, done: &Done<()>) {
        let (core, app) = (self.core, self.app());
        let (Some(log), Some(events)) = (&mut self.ui_log, &ui.events) else {
            return;
        };
        let left = match done {
            Ok(()) if ui.exited => Left::Exited(ui.tree.to_string()),
            Ok(()) => Left::Tree(ui.tree.to_string()),
            Err(Stop::Trap(trap)) => {
                let report = report(core, &app.name, trap);
                Left::Trapped(report.lines().next().unwrap_or_default().into())
            }
            Err(Stop::Ended(line)) => Left::Ended(line.clone()),
            Err(_) => return,
        };
        log.push((events.clone(), left));
    }

    /// `update`, its effects, then the view; a typed key's cursor follows last.
    fn turn(&mut self, ui: &mut Ui, message: Value, typed: Typed, site: Site) -> Done<()> {
        let app = self.app();
        let (mut effects, state) = self.update(app, ui.state.take(), message)?;
        ui.state = state;
        if matches!(effects, Value::List(_)) {
            let mut answers = VecDeque::new();
            let foreign = &self.core.foreign;
            let end = perform(app, foreign, &mut effects, &mut Dropped, &mut answers);
            if !answers.is_empty() {
                ui.events = None;
            }
            match end {
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
            events: self.events.clone(),
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
        Node::Button { label, on, focus } => Node::Button {
            label: label.clone(),
            on: message(on)?,
            focus: *focus,
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
