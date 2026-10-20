//! A visual app with no window on dev code (M5-25), run as the interpreter runs one
//! (paka/src/interp/windowless.rs): one queue of events and effect results, each handled to the
//! end, its view included, before the next. A message a click sends is borrowed from the drawing
//! it lies in, which is released only once the next tree is drawn.

use super::app::{
    App, Call, Queued, Stopped, Words, addr, answer, close, handle, release, start_message, state,
};
use super::view::{self, Drawing};
use super::{Context, Trap};
use crate::effects::{End, World};
use crate::route::{Event, Sent, focused, route};
use crate::tree::{self, Node};
use std::collections::VecDeque;
use std::mem;

/// How the loop reaches compiled code: its calls, a message's text from its type and address,
/// and a trap's first line without the app's name.
pub struct Code<'a> {
    pub call: &'a mut Call<'a>,
    pub show: &'a mut dyn FnMut(u64, u64) -> String,
    pub at: &'a dyn Fn(&Trap) -> String,
}

/// Why the app ended: an effect, a trap in a handler (in the context) or the interrupt.
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
    /// The traps that leave the app running: in the start handler or a view.
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
        if self.end.is_none() {
            self.queue.extend(events.into_iter().map(Queued::Event));
            let drained = self.drain();
            self.ended(drained);
        }
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
            let mut message = start_message(self.ctx, self.app, tag, start).map_err(Err)?;
            let handled = self.handle(addr(&mut message));
            release(self.ctx, self.app.message.0, &mut message);
            if handled == Err(Err(Stopped::Trap)) {
                self.traps.push(mem::take(&mut self.ctx.trap));
                self.release_state();
                self.state = state(self.ctx, self.app, self.code.call).map_err(Err)?;
            } else {
                handled?;
            }
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
        let (mut owned, at) = match next {
            Queued::Message(m) => (Some(m), 0),
            Queued::Answer(f, text) => (Some(self.answer(f, &text)?), 0),
            Queued::Event(event) => match route(&mut self.drawing.root, &event) {
                None => return Ok(()),
                Some(Sent::Click(on)) => (None, on.at),
                Some(Sent::Change(on, change)) => {
                    let at = on.at;
                    let f = self.drawing.word(at);
                    typed = Some((self.shown(), change.cursor));
                    (Some(self.answer(f, &change.text)?), 0)
                }
            },
        };
        let message = match &mut owned {
            Some(m) => addr(m),
            None => self.drawing.address(at),
        };
        let handled = self.handle(message);
        if let Some(mut m) = owned {
            release(self.ctx, self.app.message.0, &mut m);
        }
        handled?;
        self.redraw(false)?;
        if let Some((before, cursor)) = typed
            && let Some(Node::Input { text, edit, .. }) = focused(&mut self.drawing.root)
        {
            edit.follow(&before, text, Some(cursor));
        }
        Ok(())
    }

    fn answer(&mut self, f: u64, text: &str) -> Result<Words, Ended> {
        answer(self.ctx, self.app, self.code.call, f, text).map_err(Err)
    }

    fn handle(&mut self, message: u64) -> Result<(), Ended> {
        let (ctx, app, call) = (&mut *self.ctx, self.app, &mut *self.code.call);
        match handle(
            ctx,
            app,
            call,
            self.world,
            &mut self.state,
            message,
            &mut self.queue,
        ) {
            Ok(None) => Ok(()),
            Ok(Some(end)) => Err(Ok(end)),
            Err(stopped) => Err(Err(stopped)),
        }
    }

    /// Runs the view on the state, which it borrows; a trap there keeps both state and tree.
    fn redraw(&mut self, first: bool) -> Result<(), Ended> {
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
                if first {
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
