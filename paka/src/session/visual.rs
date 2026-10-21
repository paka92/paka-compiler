//! A visual app the server runs with no window (M5-20), on the interpreter, in a thread that owns
//! its program and each version installed into it. Its effects are performed; what it prints is
//! dropped.

use crate::core_form::Program;
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
    shown: Receiver<Shown>,
}

/// The tree text after a turn, and whether the app has ended.
pub type Shown = (String, bool);

enum Input {
    Events(Vec<Event>),
    /// The next version, and whether it comes as a reset.
    Swap(Arc<Program>, bool),
}

impl Visual {
    /// Launches the visual app of `core` with `args`; and what it first shows.
    pub fn start(core: Program, args: Vec<OsString>) -> (Visual, Shown) {
        let (inbox, inputs) = mpsc::channel();
        let (outbox, shown) = mpsc::channel();
        thread::spawn(move || {
            let mut ui = Windowless::start(Arc::new(core), &args, Host(io::sink()));
            let show = |ui: &Windowless<_>| outbox.send((ui.tree.to_string(), ui.end.is_some()));
            if show(&ui).is_err() {
                return;
            }
            for input in inputs {
                match input {
                    Input::Events(events) => ui.send(events),
                    Input::Swap(core, false) => ui.swap(core),
                    Input::Swap(core, true) => ui.reset(core, &args),
                }
                if show(&ui).is_err() {
                    return;
                }
            }
        });
        let visual = Visual { inbox, shown };
        let first = visual.next();
        (visual, first)
    }

    /// Handles `events`; what the app shows after them.
    pub fn send(&self, events: Vec<Event>) -> Shown {
        self.put(Input::Events(events))
    }

    /// Installs `core` between turns, keeping the state or, on a `reset`, launching again; what
    /// the app shows after.
    pub fn swap(&self, core: Program, reset: bool) -> Shown {
        self.put(Input::Swap(Arc::new(core), reset))
    }

    fn put(&self, input: Input) -> Shown {
        match self.inbox.send(input) {
            Ok(()) => self.next(),
            Err(_) => (String::new(), true),
        }
    }

    fn next(&self) -> Shown {
        self.shown.recv().unwrap_or((String::new(), true))
    }
}
