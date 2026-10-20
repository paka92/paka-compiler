//! A visual app with no window (M5-01, M5-03): events and effect results wait in one queue, and
//! each is handled to the end, its view included, before the next.

use super::effects::{End, Queued, World, perform};
use super::eval::{Done, Kind, Machine, Stop, Trap};
use super::value::{Message, Value};
use super::{ended, on_big_stack, place, report, start_message};
use crate::core_form::{App, Program};
use paka_rt::route::{Event, Sent, focused, route};
use paka_rt::tree::{self, Node};
use std::collections::VecDeque;
use std::ffi::OsString;

pub struct Windowless<'a, W> {
    core: &'a Program,
    app: &'a App,
    state: Option<Value>,
    queue: VecDeque<Queued>,
    world: W,
    /// The tree last drawn: one error node if the first view run trapped.
    pub tree: Node<Message>,
    /// The reports of traps that leave the app running: in a handler or a view.
    pub traps: Vec<String>,
    /// How the app ended: by an effect, or by a fault outside a handler or view.
    pub end: Option<End>,
}

impl<'a, W: World + Send> Windowless<'a, W> {
    /// Launches the visual app of `core` (M5-01 section 1.6): state from its defaults, the start
    /// message filled from `args` if the app has one, then the first tree.
    pub fn start(core: &'a Program, args: &[OsString], world: W) -> Self {
        let app = core.app.as_ref().expect("a visual app");
        let mut ui = Windowless {
            core,
            app,
            state: None,
            queue: VecDeque::new(),
            world,
            tree: Node::Error(String::new()),
            traps: vec![],
            end: None,
        };
        ui.on_stack(|ui, m| ui.launch(m, args));
        ui
    }

    /// Queues `events` and handles the queue until it is empty or the app ends.
    pub fn send(&mut self, events: impl IntoIterator<Item = Event>) {
        if self.end.is_none() {
            self.queue.extend(events.into_iter().map(Queued::Event));
            self.on_stack(Self::drain);
        }
    }

    fn on_stack(&mut self, f: impl FnOnce(&mut Self, &mut Machine) -> Result<(), End> + Send) {
        let (core, app) = (self.core, self.app);
        let end = match on_big_stack(|| f(self, &mut Machine::new(core))) {
            Ok(done) => done.err(),
            Err(reason) => Some(ended(core, app, Stop::Internal(reason))),
        };
        if end.is_some() {
            self.end = end;
            self.queue.clear();
        }
    }

    fn launch(&mut self, m: &mut Machine, args: &[OsString]) -> Result<(), End> {
        self.state = m.state(self.app).map_err(|stop| self.ended(stop))?;
        if let Some(start) = self.app.start {
            let message = start_message(self.core, self.app, start, args)?;
            self.handle(m, Ok(message))?;
        }
        self.redraw(m, true)?;
        self.drain(m)
    }

    fn drain(&mut self, m: &mut Machine) -> Result<(), End> {
        while let Some(next) = self.queue.pop_front() {
            self.turn(m, next)?;
        }
        Ok(())
    }

    /// One turn: the message, `update`, its effects, the view; a key's cursor follows last.
    fn turn(&mut self, m: &mut Machine, next: Queued) -> Result<(), End> {
        let mut typed = None;
        let message = match next {
            Queued::Message(message) => Ok(message),
            Queued::Answer(f, arg) => m.answer(f, arg),
            Queued::Event(event) => match route(&mut self.tree, &event) {
                None => return Ok(()),
                Some(Sent::Click(on)) => {
                    on.value.copy().ok_or_else(|| too_deep(self.core, self.app))
                }
                Some(Sent::Change(on, change)) => {
                    let Value::Fn(f) = on.value else {
                        unreachable!("on_change takes a variant");
                    };
                    typed = Some((self.shown(), change.cursor));
                    m.answer(f, Value::Str(change.text.into()))
                }
            },
        };
        if !self.handle(m, message)? {
            return Ok(());
        }
        self.redraw(m, false)?;
        if let Some((before, cursor)) = typed
            && let Some(Node::Input { text, edit, .. }) = focused(&mut self.tree)
        {
            edit.follow(&before, text, Some(cursor));
        }
        Ok(())
    }

    /// Runs `update` on `message` and a copy of the state, which replaces the state once it
    /// returns, then performs its effects; false after a trap, which is recorded and leaves
    /// state, tree and queue as they were.
    fn handle(&mut self, m: &mut Machine, message: Done) -> Result<bool, End> {
        let handled = message.and_then(|message| {
            let state = match &self.state {
                Some(state) => Some(state.copy().ok_or_else(|| too_deep(self.core, self.app))?),
                None => None,
            };
            m.update(self.app, state, message)
        });
        match handled {
            Ok((effects, state)) => {
                self.state = state;
                self.perform(effects)?;
                Ok(true)
            }
            Err(Stop::Trap(trap)) => {
                self.traps.push(report(self.core, &self.app.name, &trap));
                Ok(false)
            }
            Err(stop) => Err(self.ended(stop)),
        }
    }

    fn perform(&mut self, mut effects: Value) -> Result<(), End> {
        if !matches!(effects, Value::List(_)) {
            return Ok(());
        }
        match perform(self.app, &mut effects, &mut self.world, &mut self.queue) {
            Some(end) => Err(end),
            None => Ok(()),
        }
    }

    /// Runs the view on a copy of the state, since a trap there keeps both state and tree.
    fn redraw(&mut self, m: &mut Machine, first: bool) -> Result<(), End> {
        let view = self.app.view.expect("a visual app");
        let state = self.state.as_ref().and_then(Value::copy);
        let drawn = match state {
            Some(state) => m.draw(view, vec![state]),
            None => Err(too_deep(self.core, self.app)),
        };
        match drawn {
            Ok(mut tree) => {
                tree::keep(&self.tree, &mut tree);
                self.tree = tree;
            }
            Err(Stop::Trap(trap)) => {
                if first {
                    let at = place(self.core, trap.site);
                    self.tree = Node::Error(format!("trap {} at {at}", trap.kind.name()));
                }
                self.traps.push(report(self.core, &self.app.name, &trap));
            }
            Err(stop) => return Err(self.ended(stop)),
        }
        Ok(())
    }

    /// The text of the focused Input.
    fn shown(&mut self) -> String {
        match focused(&mut self.tree) {
            Some(Node::Input { text, .. }) => text.clone(),
            _ => unreachable!("a key reached the focused Input"),
        }
    }

    fn ended(&self, stop: Stop) -> End {
        ended(self.core, self.app, stop)
    }
}

/// A copy that nests too deep, a `stack` trap reported at the handler.
fn too_deep(core: &Program, app: &App) -> Stop {
    Trap::bare(Kind::Stack, core.fns[app.update].site).into()
}
