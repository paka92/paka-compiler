//! A visual app the server runs with no window (M5-20), on the interpreter, in a thread that owns
//! its program. Its effects are performed; what it prints is dropped.

use crate::core_form::Program;
use crate::interp::Windowless;
use paka_rt::effects::Host;
use paka_rt::route::Event;
use std::ffi::OsString;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct Visual {
    events: Sender<Vec<Event>>,
    shown: Receiver<Shown>,
}

/// The tree text after a turn, and whether the app has ended.
pub type Shown = (String, bool);

impl Visual {
    /// Launches the visual app of `core` with `args`; and what it first shows.
    pub fn start(core: Program, args: Vec<OsString>) -> (Visual, Shown) {
        let (events, inbox) = mpsc::channel::<Vec<Event>>();
        let (outbox, shown) = mpsc::channel();
        thread::spawn(move || {
            let mut ui = Windowless::start(&core, &args, Host(io::sink()));
            let show = |ui: &Windowless<_>| outbox.send((ui.tree.to_string(), ui.end.is_some()));
            if show(&ui).is_err() {
                return;
            }
            for events in inbox {
                ui.send(events);
                if show(&ui).is_err() {
                    return;
                }
            }
        });
        let visual = Visual { events, shown };
        let first = visual.next();
        (visual, first)
    }

    /// Handles `events`; what the app shows after them.
    pub fn send(&self, events: Vec<Event>) -> Shown {
        match self.events.send(events) {
            Ok(()) => self.next(),
            Err(_) => (String::new(), true),
        }
    }

    fn next(&self) -> Shown {
        self.shown.recv().unwrap_or((String::new(), true))
    }
}
