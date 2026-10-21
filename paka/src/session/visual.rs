//! A visual app the server runs with no window (M5-20), on the interpreter, in a thread that owns
//! its program and each version installed into it. Its effects are performed; what it prints is
//! dropped.

use crate::core_form::{FnId, Program};
use crate::interp::Windowless;
use paka_rt::effects::Host;
use paka_rt::route::Event;
use std::ffi::OsString;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct Visual {
    inbox: Sender<Input>,
    shown: Receiver<(Shown, Option<String>)>,
}

/// The tree text after a turn, and whether the app has ended.
pub type Shown = (String, bool);

enum Input {
    Events(Vec<Event>),
    /// The next version, and whether it comes as a reset.
    Swap(Arc<Program>, bool),
    /// The next version, with a migrate as its function.
    Migrate(Arc<Program>, FnId),
}

impl Visual {
    /// Launches the visual app of `core` with `args`; and what it first shows.
    pub fn start(core: Program, args: Vec<OsString>) -> (Visual, Shown) {
        let (inbox, inputs) = mpsc::channel();
        let (outbox, shown) = mpsc::channel();
        thread::spawn(move || {
            let mut ui = Windowless::start(Arc::new(core), &args, Host(io::sink()));
            let show = |ui: &Windowless<_>, trap| {
                outbox.send(((ui.tree.to_string(), ui.end.is_some()), trap))
            };
            if show(&ui, None).is_err() {
                return;
            }
            for input in inputs {
                let mut trap = None;
                match input {
                    Input::Events(events) => ui.send(events),
                    Input::Swap(core, false) => ui.swap(core),
                    Input::Swap(core, true) => ui.reset(core, &args),
                    Input::Migrate(core, f) => {
                        let before = ui.traps.len();
                        if !ui.migrate(core, f) {
                            trap = ui.traps.get(before).cloned();
                        }
                    }
                }
                if show(&ui, trap).is_err() {
                    return;
                }
            }
        });
        let visual = Visual { inbox, shown };
        let first = visual.next().0;
        (visual, first)
    }

    /// Handles `events`; what the app shows after them.
    pub fn send(&self, events: Vec<Event>) -> Shown {
        self.put(Input::Events(events)).0
    }

    /// Installs `core` between turns, keeping the state or, on a `reset`, launching again; what
    /// the app shows after.
    pub fn swap(&self, core: Program, reset: bool) -> Shown {
        self.put(Input::Swap(Arc::new(core), reset)).0
    }

    /// Installs `core` between turns, its function `f` migrating the state; what the app shows
    /// after, and the report of the trap that kept the old version running, if one did.
    pub fn migrate(&self, core: Program, f: FnId) -> (Shown, Option<String>) {
        self.put(Input::Migrate(Arc::new(core), f))
    }

    fn put(&self, input: Input) -> (Shown, Option<String>) {
        match self.inbox.send(input) {
            Ok(()) => self.next(),
            Err(_) => ((String::new(), true), None),
        }
    }

    fn next(&self) -> (Shown, Option<String>) {
        self.shown.recv().unwrap_or(((String::new(), true), None))
    }
}
