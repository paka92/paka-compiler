//! What a runtime built with the `measure` feature adds (M6-30, M6-31): a banner on standard error
//! as a run starts, and as it ends the time the copy routine took, the state copy of each handler
//! and the frames of a visual app. Without the feature it adds nothing.
//!
//! Each interval loses the clock's own cost, once for itself and once for each interval begun
//! inside it, so the timing stays out of the figures.

use paka_gui::layout::layout;
use paka_gui::text::Text;
use paka_gui::tree::Node;
use std::hint::black_box;
use std::io::{self, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

pub const ON: bool = cfg!(feature = "measure");

/// A sum of intervals and their count.
struct Total {
    nanos: AtomicU64,
    calls: AtomicU64,
}

impl Total {
    const fn new() -> Total {
        Total {
            nanos: AtomicU64::new(0),
            calls: AtomicU64::new(0),
        }
    }

    fn add(&self, nanos: u64) {
        self.nanos.fetch_add(nanos, Relaxed);
        self.calls.fetch_add(1, Relaxed);
    }

    fn get(&self) -> (u64, u64) {
        (self.nanos.load(Relaxed), self.calls.load(Relaxed))
    }
}

static COPY: Total = Total::new();
/// Calls of compiled code.
static CODE: Total = Total::new();
static FRAMES: Total = Total::new();
/// The time in compiled code during frames.
static FRAME_CODE: AtomicU64 = AtomicU64::new(0);
/// Each handler's name and state copies.
static HANDLERS: Mutex<Vec<(String, u64, u64)>> = Mutex::new(vec![]);
/// The intervals begun so far.
static BEGUN: AtomicU64 = AtomicU64::new(0);
/// The nanoseconds one reading of the clock adds to an interval.
static CLOCK: AtomicU64 = AtomicU64::new(0);

/// Runs `run`, one whole entry, between the banner and the figures.
pub fn around(run: impl FnOnce() -> i32) -> i32 {
    if !ON {
        return run();
    }
    let _ = writeln!(io::stderr(), "MEASUREMENT BUILD: not a Paka program");
    CLOCK.store(clock(), Relaxed);
    let status = run();
    let _ = io::stderr().write_all(figures().as_bytes());
    status
}

/// The median cost of a start and a stop.
fn clock() -> u64 {
    let mut costs: Vec<_> = (0..1001)
        .map(|_| Instant::now().elapsed().as_nanos())
        .collect();
    costs.sort_unstable();
    u64::try_from(costs[costs.len() / 2]).unwrap_or(0)
}

fn figures() -> String {
    let (nanos, calls) = COPY.get();
    let mut text = format!("MEASUREMENT BUILD: copy routine {nanos} ns in {calls} calls\n");
    for (name, nanos, calls) in HANDLERS.lock().unwrap_or_else(|e| e.into_inner()).iter() {
        text += &format!("MEASUREMENT BUILD: state copy {name} {nanos} ns in {calls} handlers\n");
    }
    let (whole, frames) = FRAMES.get();
    if frames > 0 {
        let code = FRAME_CODE.load(Relaxed);
        text +=
            &format!("MEASUREMENT BUILD: frames {frames} paka code {code} ns whole {whole} ns\n");
    }
    text
}

/// The number `handler` is timed by, the variant name its message text starts with.
pub fn handler(message: &str) -> usize {
    let name = message.split('(').next().unwrap_or_default();
    let mut all = HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
    all.iter().position(|h| h.0 == name).unwrap_or_else(|| {
        all.push((name.into(), 0, 0));
        all.len() - 1
    })
}

/// What an interval is added to.
#[derive(Clone, Copy)]
enum Sink {
    Copy,
    Code,
    Handler(usize),
}

/// An interval begun: its start and the intervals begun before it.
struct Begun(Instant, u64);

impl Begun {
    fn now() -> Option<Begun> {
        ON.then(|| Begun(Instant::now(), BEGUN.fetch_add(1, Relaxed) + 1))
    }

    /// The time since, less the clock readings in it.
    fn nanos(&self) -> u64 {
        let elapsed = u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let readings = 1 + BEGUN.load(Relaxed) - self.1;
        elapsed.saturating_sub(readings * CLOCK.load(Relaxed))
    }
}

/// Adds the time from its making to its drop to its sink.
pub struct Timed(Option<(Begun, Sink)>);

impl Timed {
    fn start(sink: Sink) -> Timed {
        Timed(Begun::now().map(|b| (b, sink)))
    }

    /// The copy routine.
    pub fn copy() -> Timed {
        Timed::start(Sink::Copy)
    }

    /// A call of compiled code.
    pub fn code() -> Timed {
        Timed::start(Sink::Code)
    }

    /// The state copy for the handler numbered `handler`.
    pub fn state(handler: usize) -> Timed {
        Timed::start(Sink::Handler(handler))
    }
}

impl Drop for Timed {
    fn drop(&mut self) {
        if let Some((begun, sink)) = &self.0 {
            let nanos = begun.nanos();
            match *sink {
                Sink::Copy => COPY.add(nanos),
                Sink::Code => CODE.add(nanos),
                Sink::Handler(i) => {
                    let mut all = HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
                    all[i].1 += nanos;
                    all[i].2 += 1;
                }
            }
        }
    }
}

/// A frame: a handled message, its view and the layout of its tree; counted only once ended.
pub struct Frame(Option<(Begun, u64)>);

impl Frame {
    pub fn start() -> Frame {
        Frame(Begun::now().map(|b| (b, CODE.get().0)))
    }

    pub fn end(self) {
        if let Some((begun, code)) = self.0 {
            FRAMES.add(begun.nanos());
            FRAME_CODE.fetch_add(CODE.get().0 - code, Relaxed);
        }
    }
}

/// Lays `root` out as a window of the default size does each frame.
pub fn lay_out<M>(root: &Node<M>) {
    thread_local! {
        static TEXT: Text = Text::default();
    }
    TEXT.with(|text| black_box(layout(root, paka_gui::SIZE.0, text)));
}
