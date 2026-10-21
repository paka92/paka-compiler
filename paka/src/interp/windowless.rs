//! A visual app with no window (M5-01, M5-03): events and effect results wait in one queue, and
//! each is handled to the end, its view included, before the next.

use super::effects::{End, Queued, World, perform};
use super::eval::{Done, Kind, Machine, Stop, Trap};
use super::value::{Message, Value};
use super::{ended, on_big_stack, place, report, start_message};
use crate::core_form::{App, FnId, Program};
use crate::migrate::late;
use paka_rt::limit::until;
use paka_rt::route::{Event, Sent, focused, route};
use paka_rt::tree::{self, Node};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct Windowless<W> {
    core: Arc<Program>,
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

impl<W: World + Send> Windowless<W> {
    /// Launches the visual app of `core` (M5-01 section 1.6): state from its defaults, the start
    /// message filled from `args` if the app has one, then the first tree.
    pub fn start(core: Arc<Program>, args: &[OsString], world: W) -> Self {
        let mut ui = Windowless {
            core,
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
        self.queue(events);
        if self.end.is_none() {
            self.on_stack(Self::drain);
        }
    }

    /// Queues `events` without handling them, as input that comes while a turn runs.
    pub fn queue(&mut self, events: impl IntoIterator<Item = Event>) {
        if self.end.is_none() {
            self.queue.extend(events.into_iter().map(Queued::Event));
        }
    }

    /// Installs `core`, the next version of the app with the same stored types, between turns
    /// (M5-07): the state is kept, the view runs again keeping element state as M5-02 matches it,
    /// then the queue is handled. Only events wait here, since a turn's answers are handled in
    /// the same `send`; each is worked out against the new tree. A view that traps here shows one
    /// error node, since the old tree's messages are the old code's.
    pub fn swap(&mut self, core: Arc<Program>) {
        if self.end.is_none() {
            self.core = core;
            self.on_stack(|ui, m| {
                ui.redraw(m, true)?;
                ui.drain(m)
            });
        }
    }

    /// Installs `core`, any next version of the app, as a launch again with `args` (M5-07
    /// section 1.7): the queue is emptied, so later answers to old effects are dropped; state
    /// comes from the defaults, the start message is handled again, and the view runs with no
    /// element state or focus kept.
    pub fn reset(&mut self, core: Arc<Program>, args: &[OsString]) {
        if self.end.is_none() {
            self.core = core;
            self.queue.clear();
            self.tree = Node::Error(String::new());
            self.on_stack(|ui, m| ui.launch(m, args));
        }
    }

    /// Installs `core`, the next version of the app with a migrate as its function `function`,
    /// between turns (M5-07 section 1.6): the migrate runs on a copy of the state, and its result
    /// is the state as `swap` goes on. If it traps, or the interrupt stops it past `limit`, its
    /// report is recorded and the old version runs on, its state and tree as they were, and
    /// handles the queue; false then.
    pub fn migrate(&mut self, core: Arc<Program>, function: FnId, limit: Duration) -> bool {
        if self.end.is_some() {
            return false;
        }
        let old = mem::replace(&mut self.core, core);
        let mut migrated = false;
        self.on_stack(|ui, m| {
            let state = ui.state.as_ref().and_then(Value::copy);
            let state = state.ok_or_else(|| too_deep(&ui.core));
            let flag = Arc::new(AtomicBool::new(false));
            m.interrupt = Some(Arc::clone(&flag));
            let interrupt = || flag.store(true, Ordering::Relaxed);
            let (ran, _) = until(limit, interrupt, || {
                state.and_then(|s| m.run(function, vec![s]))
            });
            m.interrupt = None;
            let name = &app(&ui.core).name;
            match ran {
                Ok(state) => {
                    (ui.state, migrated) = (Some(state), true);
                    ui.redraw(m, true)?;
                    ui.drain(m)
                }
                Err(Stop::Trap(trap)) => {
                    ui.traps.push(report(&ui.core, name, &trap));
                    Ok(())
                }
                Err(Stop::Time) => {
                    ui.traps.push(late(name, limit));
                    Ok(())
                }
                Err(stop) => Err(ui.ended(stop)),
            }
        });
        if !migrated && self.end.is_none() {
            self.core = old;
            self.on_stack(Self::drain);
        }
        migrated
    }

    fn on_stack(&mut self, f: impl FnOnce(&mut Self, &mut Machine) -> Result<(), End> + Send) {
        let core = Arc::clone(&self.core);
        let end = match on_big_stack(|| f(self, &mut Machine::new(&core))) {
            Ok(done) => done.err(),
            Err(reason) => Some(ended(&core, app(&core), Stop::Internal(reason))),
        };
        if end.is_some() {
            self.end = end;
            self.queue.clear();
        }
    }

    fn launch(&mut self, m: &mut Machine, args: &[OsString]) -> Result<(), End> {
        self.state = m.state(app(&self.core)).map_err(|stop| self.ended(stop))?;
        if let Some(start) = app(&self.core).start {
            let message = start_message(&self.core, app(&self.core), start, args)?;
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
                Some(Sent::Click(on)) => on.value.copy().ok_or_else(|| too_deep(&self.core)),
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
                Some(state) => Some(state.copy().ok_or_else(|| too_deep(&self.core))?),
                None => None,
            };
            m.update(app(&self.core), state, message)
        });
        match handled {
            Ok((effects, state)) => {
                self.state = state;
                self.perform(effects)?;
                Ok(true)
            }
            Err(Stop::Trap(trap)) => {
                let name = &app(&self.core).name;
                self.traps.push(report(&self.core, name, &trap));
                Ok(false)
            }
            Err(stop) => Err(self.ended(stop)),
        }
    }

    fn perform(&mut self, mut effects: Value) -> Result<(), End> {
        if !matches!(effects, Value::List(_)) {
            return Ok(());
        }
        let app = app(&self.core);
        match perform(app, &mut effects, &mut self.world, &mut self.queue) {
            Some(end) => Err(end),
            None => Ok(()),
        }
    }

    /// Runs the view on a copy of the state, since a trap there keeps both state and tree; one
    /// error node when the tree is `fresh`: the first, or the first after a swap.
    fn redraw(&mut self, m: &mut Machine, fresh: bool) -> Result<(), End> {
        let view = app(&self.core).view.expect("a visual app");
        let state = self.state.as_ref().and_then(Value::copy);
        let drawn = match state {
            Some(state) => m.draw(view, vec![state]),
            None => Err(too_deep(&self.core)),
        };
        match drawn {
            Ok(mut tree) => {
                tree::keep(&self.tree, &mut tree);
                self.tree = tree;
            }
            Err(Stop::Trap(trap)) => {
                if fresh {
                    let at = place(&self.core, trap.site);
                    self.tree = Node::Error(format!("trap {} at {at}", trap.kind.name()));
                }
                self.traps
                    .push(report(&self.core, &app(&self.core).name, &trap));
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
        ended(&self.core, app(&self.core), stop)
    }
}

fn app(core: &Program) -> &App {
    core.app.as_ref().expect("a visual app")
}

/// A copy that nests too deep, a `stack` trap reported at the handler.
fn too_deep(core: &Program) -> Stop {
    Trap::bare(Kind::Stack, core.fns[app(core).update].site).into()
}
