//! A visual app run on the interpreter (M5-20), in a thread that owns its program and each version
//! installed into it, with no window or with one (M5-35). Its effects are performed; what it
//! prints is dropped. Work, the window's input included, reaches it as jobs, each run between
//! turns; the window is handed each job's tree before the job's answer is sent (M5-38).

use crate::core_form::{FnId, Program};
use crate::interp::Windowless;
use crate::serve::{Failure, trap};
use paka_gui::window::{Size, Window};
use paka_json::Value;
use paka_rt::effects::{End, Host};
use paka_rt::route::Event;
use std::ffi::OsString;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

/// The running app, and the arguments it was launched with.
pub type Ui = Windowless<Host<io::Sink>>;
/// A job, and what it sends once the window has the tree it left.
type Job = Box<dyn FnOnce(&mut Ui, &[OsString]) -> Answer + Send>;
type Answer = Box<dyn FnOnce()>;

pub struct Visual {
    inbox: Sender<Job>,
    /// The app's name, which its trap reports start with.
    pub app: String,
    /// Its window's size, when it shows in one.
    pub window: Option<Size>,
}

/// What the app shows after a job: its tree text, its end once it has ended, and the report of
/// the first trap the job recorded.
pub struct Shown {
    pub tree: String,
    pub end: Option<End>,
    pub trap: Option<String>,
}

impl Visual {
    /// Launches the visual app of `core` with `args`, in a window when `windowed` and a display
    /// lets one open; and what it first shows. Closing the window ends the app with status 0.
    pub fn start(core: Program, args: Vec<OsString>, windowed: bool) -> (Visual, Shown) {
        let app = core.app.as_ref().map_or(String::new(), |a| a.name.clone());
        let (inbox, jobs) = mpsc::channel::<Job>();
        let (first, shown) = mpsc::channel();
        let (title, closer, input) = (app.clone(), inbox.clone(), inbox.clone());
        thread::spawn(move || {
            let mut ui = Windowless::start(Arc::new(core), &args, Host(io::sink()));
            let on_close = move || drop(closer.send(Box::new(close)));
            let on_input = move |event: Event| {
                let _ = input.send(Box::new(move |ui: &mut Ui, _: &[OsString]| -> Answer {
                    ui.send([event]);
                    Box::new(|| ())
                }));
            };
            let window = windowed
                .then(|| Window::open(&title, &ui.tree, on_close, on_input))
                .flatten();
            let _ = first.send((shown_since(&ui, 0), window.as_ref().map(Window::size)));
            while ui.end.is_none() {
                let Ok(job) = jobs.recv() else { break };
                let answer = job(&mut ui, &args);
                window.iter().for_each(|w| w.show(&ui.tree));
                answer();
            }
        });
        let (first, window) = shown.recv().unwrap_or_else(|_| (gone(), None));
        (Visual { inbox, app, window }, first)
    }

    /// What `f` gives on the app between turns; `None` once the app has ended.
    pub fn with<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Ui, &[OsString]) -> R + Send + 'static,
    ) -> Option<R> {
        let (reply, answer) = mpsc::channel();
        let job = move |ui: &mut Ui, args: &[OsString]| -> Answer {
            let r = f(ui, args);
            Box::new(move || drop(reply.send(r)))
        };
        self.inbox.send(Box::new(job)).ok()?;
        answer.recv().ok()
    }

    /// Handles `events`; what the app shows after them.
    pub fn send(&self, events: Vec<Event>) -> Shown {
        self.turn(move |ui, _| ui.send(events))
    }

    /// Installs `core` between turns, keeping the state or, on a `reset`, launching again; what
    /// the app shows after.
    pub fn swap(&self, core: Program, reset: bool) -> Shown {
        let core = Arc::new(core);
        self.turn(move |ui, args| match reset {
            false => ui.swap(core),
            true => ui.reset(core, args),
        })
    }

    /// Installs `core` between turns, its function `f` migrating the state, stopped past `limit`;
    /// what the app shows after, its trap the report of the trap or stop that kept the old version
    /// running, if one did.
    pub fn migrate(&self, core: Program, f: FnId, limit: Duration) -> Shown {
        let core = Arc::new(core);
        let migrated = self.with(move |ui, _| {
            let before = ui.traps.len();
            let migrated = ui.migrate(core, f, limit);
            let mut shown = shown_since(ui, before);
            if migrated {
                shown.trap = None;
            }
            shown
        });
        migrated.unwrap_or_else(gone)
    }

    /// Its window's size in units, or the windowless size (M5-05).
    pub fn size(&self) -> (u64, u64) {
        self.window.as_ref().map_or(paka_gui::SIZE, Size::get)
    }

    /// Runs `f` as one turn; what the app shows after.
    pub fn turn(&self, f: impl FnOnce(&mut Ui, &[OsString]) + Send + 'static) -> Shown {
        let shown = self.with(move |ui, args| {
            let before = ui.traps.len();
            f(ui, args);
            shown_since(ui, before)
        });
        shown.unwrap_or_else(gone)
    }
}

impl Drop for Visual {
    /// Ends the app, which its window's close callback would otherwise keep waiting for jobs.
    fn drop(&mut self) {
        let _ = self.inbox.send(Box::new(close));
    }
}

fn close(ui: &mut Ui, _: &[OsString]) -> Answer {
    ui.end.get_or_insert((0, String::new()));
    Box::new(|| ())
}

/// The refusal of a Look call with no visual app running.
pub fn not_running() -> Failure {
    Failure::new("not_running", "no visual app is running")
}

impl Shown {
    /// The reply to a run that launched `app` and first showed this (docs/decisions/M5-09.md).
    pub fn run(&self, app: &str, window: bool) -> Value {
        let (status, stderr) = match &self.end {
            Some((status, line)) => (Value::from(usize::from(*status)), line.as_str()),
            None => (Value::Null, ""),
        };
        Value::object([
            ("tree", self.tree.as_str().into()),
            ("window", window.into()),
            ("stdout", "".into()),
            ("stderr", stderr.into()),
            ("status", status),
            ("trap", trap(app, self.trap.as_deref().unwrap_or(stderr))),
            ("stopped", Value::Null),
        ])
    }
}

/// What `ui` shows, with the first trap recorded past the first `before`.
pub(super) fn shown_since(ui: &Ui, before: usize) -> Shown {
    Shown {
        tree: ui.tree.to_string(),
        end: ui.end.clone(),
        trap: ui.traps.get(before).cloned(),
    }
}

/// What an app shows once its thread is gone.
fn gone() -> Shown {
    Shown {
        tree: String::new(),
        end: Some((3, String::new())),
        trap: None,
    }
}
