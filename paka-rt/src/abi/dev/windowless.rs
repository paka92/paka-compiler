//! A visual app with no window on dev code (M5-25), run as the interpreter runs one
//! (paka/src/interp/windowless.rs): one queue of events and effect results, each handled to the
//! end, its view included, before the next. A message a click sends is borrowed from the drawing
//! it lies in, which is released only once the next tree is drawn. The handler works on a copy of
//! the state: in-place writes and frees leave the old one whole, and a trap frees all the turn
//! made since it began.

use super::app::{
    App, Call, Queued, Stopped, Words, addr, answer, close, copy, effects, release, start_message,
    state, status, update, words,
};
use super::view::{self, Drawing};
use super::{Context, Trap, until};
use crate::effects::{End, World};
use crate::route::{Event, Sent, focused, route};
use crate::tree::{self, Node};
use std::collections::VecDeque;
use std::mem;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// How the loop reaches compiled code: its calls, a message's text from its type and address,
/// and a trap's first line without the app's name.
pub struct Code<'a> {
    pub call: &'a mut Call<'a>,
    pub show: &'a mut dyn FnMut(u64, u64) -> String,
    pub at: &'a dyn Fn(&Trap) -> String,
}

/// Why the app ended: an effect, a trap outside a handler or view (in the context) or the
/// interrupt.
type Ended = Result<End, Stopped>;

pub struct Windowless<'a, W> {
    ctx: &'a mut Context,
    code: Code<'a>,
    app: &'a App<'a>,
    world: &'a mut W,
    state: Words,
    queue: VecDeque<Queued>,
    /// The tree last drawn: one error node if the first view run trapped.
    pub drawing: Drawing,
    /// The traps that leave the app running: in a handler or a view.
    pub traps: Vec<Trap>,
    pub end: Option<Ended>,
}

impl<'a, W: World> Windowless<'a, W> {
    /// Launches the visual `app` (M5-01 section 1.6): state from its defaults, the start message
    /// with fields `start` if the app has one, then the first tree.
    pub fn start(
        ctx: &'a mut Context,
        code: Code<'a>,
        app: &'a App<'a>,
        start: &[String],
        world: &'a mut W,
    ) -> Self {
        let mut ui = Windowless {
            ctx,
            code,
            app,
            world,
            state: vec![],
            queue: VecDeque::new(),
            drawing: Drawing::default(),
            traps: vec![],
            end: None,
        };
        let launched = ui.launch(start);
        ui.ended(launched);
        ui
    }

    /// Queues `events` and handles the queue until it is empty or the app ends.
    pub fn send(&mut self, events: impl IntoIterator<Item = Event>) {
        self.queue(events);
        if self.end.is_none() {
            let drained = self.drain();
            self.ended(drained);
        }
    }

    /// Queues `events` without handling them, as input that comes while a turn runs.
    pub fn queue(&mut self, events: impl IntoIterator<Item = Event>) {
        if self.end.is_none() {
            self.queue.extend(events.into_iter().map(Queued::Event));
        }
    }

    /// Moves the run onto `code` and `app`, the next version with the same stored types, between
    /// turns, as `paka::interp::Windowless::swap` does: the state words are kept, the view runs
    /// again, then the queue, which holds only events, is handled against the new tree. A view
    /// that traps here shows one error node; the old drawing's messages are never sent.
    pub fn swap(&mut self, code: Code<'a>, app: &'a App<'a>) {
        if self.end.is_none() {
            (self.code, self.app) = (code, app);
            let drawn = self.redraw(true).and_then(|()| self.drain());
            self.ended(drawn);
        }
    }

    /// Moves the run onto `code` and `app`, any next version, as a launch again with start fields
    /// `start`, as `paka::interp::Windowless::reset` does: the drawing, the state and the queue,
    /// answers to old effects included, are freed by the old version, then the app launches.
    pub fn reset(&mut self, code: Code<'a>, app: &'a App<'a>, start: &[String]) {
        if self.end.is_none() {
            mem::take(&mut self.drawing).release(self.ctx);
            let queue = mem::take(&mut self.queue);
            close(self.ctx, self.app, &mut mem::take(&mut self.state), queue);
            (self.code, self.app) = (code, app);
            let launched = self.launch(start);
            self.ended(launched);
        }
    }

    /// Moves the run onto `code` and `app`, the next version with a migrate as its definition
    /// `function`, between turns, as `paka::interp::Windowless::migrate` does: the migrate borrows
    /// the state words and writes the new state, which replaces them once the old version frees
    /// them, as `swap` goes on. If it traps, or the interrupt stops it past `limit`, all it made
    /// is freed, a trap's record is kept and the old version runs on, its state and drawing as
    /// they were, and handles the queue; why it stopped then.
    pub fn migrate(
        &mut self,
        code: Code<'a>,
        app: &'a App<'a>,
        function: usize,
        limit: Duration,
    ) -> Result<(), Stopped> {
        if self.end.is_some() {
            return Err(Stopped::Trap);
        }
        let (_, _, size) = app.state.expect("a migrate gives a state");
        let made = self.ctx.heap.made();
        let mut state = words(size);
        let args = [addr(&mut self.state), addr(&mut state)];
        let (ran, late) = until(self.ctx, limit, |ctx| (code.call)(ctx, function, &args));
        if late {
            self.ctx.interrupt.store(0, Ordering::Relaxed);
        }
        let ran = status(ran);
        match ran {
            Ok(()) => {
                self.release_state();
                (self.state, self.code, self.app) = (state, code, app);
                let drawn = self.redraw(true).and_then(|()| self.drain());
                self.ended(drawn);
            }
            Err(stopped) => {
                self.ctx.heap.sweep(made);
                let drained = match stopped {
                    Stopped::Trap => {
                        self.traps.push(mem::take(&mut self.ctx.trap));
                        self.drain()
                    }
                    Stopped::Time if late => self.drain(),
                    stopped => Err(Err(stopped)),
                };
                self.ended(drained);
            }
        }
        ran
    }

    fn ended(&mut self, done: Result<(), Ended>) {
        if let Err(end) = done {
            self.end = Some(end);
            for queued in mem::take(&mut self.queue) {
                if let Queued::Message(mut m) = queued {
                    release(self.ctx, self.app.message.0, &mut m);
                }
            }
        }
    }

    fn launch(&mut self, start: &[String]) -> Result<(), Ended> {
        self.state = state(self.ctx, self.app, self.code.call).map_err(Err)?;
        if let Some(tag) = self.app.start {
            let message = start_message(self.ctx, self.app, tag, start).map_err(Err)?;
            self.handle(Queued::Message(message), &mut None)?;
        }
        self.redraw(true)?;
        self.drain()
    }

    fn drain(&mut self) -> Result<(), Ended> {
        while let Some(next) = self.queue.pop_front() {
            self.turn(next)?;
        }
        Ok(())
    }

    /// One turn: the message, `update`, its effects, the view; a key's cursor follows last.
    fn turn(&mut self, next: Queued) -> Result<(), Ended> {
        let mut typed = None;
        if !self.handle(next, &mut typed)? {
            return Ok(());
        }
        self.redraw(false)?;
        if let Some((before, cursor)) = typed
            && let Some(Node::Input { text, edit, .. }) = focused(&mut self.drawing.root)
        {
            edit.follow(&before, text, Some(cursor));
        }
        Ok(())
    }

    /// Handles `next` and performs its effects; false when it sends no message or traps. A trap
    /// is recorded and frees all the turn made, leaving state, tree and queue as they were.
    fn handle(&mut self, next: Queued, typed: &mut Option<(String, usize)>) -> Result<bool, Ended> {
        let made = self.ctx.heap.made();
        let list = match self.update(next, typed) {
            Ok(Some(list)) => list,
            Ok(None) => return Ok(false),
            Err(stopped) => {
                self.ctx.heap.sweep(made);
                if stopped != Stopped::Trap {
                    return Err(Err(stopped));
                }
                self.traps.push(mem::take(&mut self.ctx.trap));
                return Ok(false);
            }
        };
        match effects(self.ctx, self.app, list, self.world, &mut self.queue) {
            Ok(None) => Ok(true),
            Ok(Some(end)) => Err(Ok(end)),
            Err(stopped) => Err(Err(stopped)),
        }
    }

    /// The effect list the handler returns for `next`, `None` when it sends no message. The
    /// handler runs on a copy of the state, which replaces the state once it returns.
    fn update(
        &mut self,
        next: Queued,
        typed: &mut Option<(String, usize)>,
    ) -> Result<Option<[u64; 3]>, Stopped> {
        let (mut owned, at) = match next {
            Queued::Message(m) => (Some(m), 0),
            Queued::Answer(f, text) => (Some(self.answer(f, &text)?), 0),
            Queued::Event(event) => match route(&mut self.drawing.root, &event) {
                None => return Ok(None),
                Some(Sent::Click(on)) => (None, on.at),
                Some(Sent::Change(on, change)) => {
                    let at = on.at;
                    let f = self.drawing.word(at);
                    *typed = Some((self.shown(), change.cursor));
                    (Some(self.answer(f, &change.text)?), 0)
                }
            },
        };
        let message = match &mut owned {
            Some(m) => addr(m),
            None => self.drawing.address(at),
        };
        let ran = self.copy_state().and_then(|mut state| {
            let list = update(self.ctx, self.app, self.code.call, &mut state, message)?;
            Ok((state, list))
        });
        if let Some(mut m) = owned {
            release(self.ctx, self.app.message.0, &mut m);
        }
        let (state, list) = ran?;
        self.release_state();
        self.state = state;
        Ok(Some(list))
    }

    fn answer(&mut self, f: u64, text: &str) -> Result<Words, Stopped> {
        answer(self.ctx, self.app, self.code.call, f, text)
    }

    fn copy_state(&mut self) -> Result<Words, Stopped> {
        match self.app.state {
            Some((_, record, size)) => copy(self.ctx, (record, size), addr(&mut self.state)),
            None => Ok(vec![]),
        }
    }

    /// Runs the view on the state, which it borrows; a trap there keeps both state and tree, or
    /// shows one error node when the tree is `fresh`: the first, or the first after a swap.
    fn redraw(&mut self, fresh: bool) -> Result<(), Ended> {
        let view = self.app.view.expect("a visual app");
        let args = match self.app.state {
            Some(_) => vec![addr(&mut self.state)],
            None => vec![],
        };
        let code = &mut self.code;
        match view::draw(self.ctx, code.call, view, &args, code.show) {
            Ok(mut drawing) => {
                tree::keep(&self.drawing.root, &mut drawing.root);
                mem::replace(&mut self.drawing, drawing).release(self.ctx);
            }
            Err(Stopped::Trap) => {
                let trap = mem::take(&mut self.ctx.trap);
                if fresh {
                    self.drawing.root = Node::Error((self.code.at)(&trap));
                }
                self.traps.push(trap);
            }
            Err(stopped) => return Err(Err(stopped)),
        }
        Ok(())
    }

    /// The text of the focused Input.
    fn shown(&mut self) -> String {
        match focused(&mut self.drawing.root) {
            Some(Node::Input { text, .. }) => text.clone(),
            _ => unreachable!("a key reached the focused Input"),
        }
    }

    fn release_state(&mut self) {
        if let Some((_, record, _)) = self.app.state {
            release(self.ctx, record, &mut self.state);
        }
    }
}

/// Frees the drawing, the state and what is still queued.
impl<W> Drop for Windowless<'_, W> {
    fn drop(&mut self) {
        mem::take(&mut self.drawing).release(self.ctx);
        let queue = mem::take(&mut self.queue);
        close(self.ctx, self.app, &mut self.state, queue);
    }
}
