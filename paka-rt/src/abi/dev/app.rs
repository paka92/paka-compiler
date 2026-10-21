//! The run loop of a compiled headless app (M4-36, docs/decisions/M4-05.md section 11). The
//! handler and the answer functions are called through `call`; each effect is decoded from its
//! words and handed to the performer the interpreter shares, `crate::effects`. A visual app's
//! loop (`super::windowless`) handles its messages here too.

use super::records::{rt_copy, rt_drop};
use super::words::{TextWords, as_str, new_text};
use super::{Context, MEMORY, Trap};
use crate::effects::{self, Effect, End, Grant, Next, World};
use crate::ops::Int;
use crate::route::Event;
use std::collections::{BTreeSet, VecDeque};
use std::ptr;

/// An Effect's words: the tag, then the largest payload, `WriteFile`'s (M4-05 section 8).
const EFFECT: u64 = 11 * 8;

/// Calls definition `id` with argument words, `out` last when it has a result; its status.
pub type Call<'a> = dyn FnMut(&mut Context, usize, &[u64]) -> i32 + 'a;

/// What the loop needs of a compiled app. A record is 0 when its type owns nothing; sizes are in
/// bytes.
pub struct App<'a> {
    pub name: &'a str,
    pub grants: &'a BTreeSet<Grant>,
    /// The handler's slot id.
    pub update: usize,
    /// The slot id of the definition that builds the state, the state's record and size.
    pub state: Option<(usize, u64, u64)>,
    /// The start variant's tag, if the app has one.
    pub start: Option<u64>,
    /// The message's record and size.
    pub message: (u64, u64),
    /// The record of `List[Effect[Msg]]`, if the handler returns one.
    pub effects: Option<u64>,
    /// The view's slot id, if the app is visual.
    pub view: Option<usize>,
}

/// Why a run stopped before its end: a trap, recorded in the context, or the interrupt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stopped {
    Trap,
    Time,
}

/// A value's words; the blocks it owns are in the run's heap.
pub(super) type Words = Vec<u64>;

pub(super) enum Queued {
    Message(Words),
    Answer(u64, String),
    Event(Event),
}

/// Runs the app from its start message, whose fields are `start`, until an effect ends it or the
/// queue empties. Every message, effect list and the state are dropped once done with.
pub fn serve(
    ctx: &mut Context,
    app: &App,
    start: &[String],
    call: &mut Call,
    world: &mut impl World,
) -> Result<End, Stopped> {
    let mut state = state(ctx, app, call)?;
    let tag = app.start.expect("a headless app has a start");
    let mut queue = VecDeque::from([Queued::Message(start_message(ctx, app, tag, start)?)]);
    let end = loop {
        let Some(next) = queue.pop_front() else {
            break (3, format!("{}: ended without Exit\n", app.name));
        };
        let mut message = match next {
            Queued::Message(m) => m,
            Queued::Answer(f, text) => answer(ctx, app, call, f, &text)?,
            Queued::Event(_) => unreachable!("a headless app has no events"),
        };
        let at = addr(&mut message);
        let ended = handle(ctx, app, call, world, &mut state, at, &mut queue);
        release(ctx, app.message.0, &mut message);
        if let Some(end) = ended? {
            break end;
        }
    };
    close(ctx, app, &mut state, queue);
    Ok(end)
}

/// Frees the state and the messages still queued.
pub(super) fn close(ctx: &mut Context, app: &App, state: &mut Words, queue: VecDeque<Queued>) {
    for queued in queue {
        if let Queued::Message(mut m) = queued {
            release(ctx, app.message.0, &mut m);
        }
    }
    if let Some((_, record, _)) = app.state {
        release(ctx, record, state);
    }
}

/// Runs the handler on the message at `message`, which it borrows, then performs its effects in
/// order, queueing the answers; the end they bring, if any.
fn handle(
    ctx: &mut Context,
    app: &App,
    call: &mut Call,
    world: &mut impl World,
    state: &mut Words,
    message: u64,
    queue: &mut VecDeque<Queued>,
) -> Result<Option<End>, Stopped> {
    let list = update(ctx, app, call, state, message)?;
    effects(ctx, app, list, world, queue)
}

/// Runs the handler on the message at `message`, which it borrows; the effect list it returns.
pub(super) fn update(
    ctx: &mut Context,
    app: &App,
    call: &mut Call,
    state: &mut Words,
    message: u64,
) -> Result<[u64; 3], Stopped> {
    let mut list = [0u64; 3];
    let mut args = vec![];
    if app.state.is_some() {
        args.push(addr(state));
    }
    args.push(message);
    if app.effects.is_some() {
        args.push(addr(&mut list));
    }
    status(call(ctx, app.update, &args))?;
    Ok(list)
}

/// Performs the effects of `list` in order, queueing the answers, then frees it; the end one
/// brings, if any.
pub(super) fn effects(
    ctx: &mut Context,
    app: &App,
    mut list: [u64; 3],
    world: &mut impl World,
    queue: &mut VecDeque<Queued>,
) -> Result<Option<End>, Stopped> {
    let ended = perform(ctx, app, list, world, queue);
    release(ctx, app.effects.unwrap_or(0), &mut list);
    ended
}

/// The state its definition builds, or no words when the app has none.
pub(super) fn state(ctx: &mut Context, app: &App, call: &mut Call) -> Result<Words, Stopped> {
    let Some((def, _, size)) = app.state else {
        return Ok(vec![]);
    };
    let mut state = words(size);
    status(call(ctx, def, &[addr(&mut state)]))?;
    Ok(state)
}

/// The start variant `tag`, its fields Strings holding `start`.
pub(super) fn start_message(
    ctx: &mut Context,
    app: &App,
    tag: u64,
    start: &[String],
) -> Result<Words, Stopped> {
    let mut message = words(app.message.1);
    message[0] = tag;
    for (i, field) in start.iter().enumerate() {
        let t = text(ctx, field)?;
        message[1 + 4 * i..5 + 4 * i].copy_from_slice(&[
            t.ptr.addr() as u64,
            t.bytes,
            t.chars,
            t.cap,
        ]);
    }
    Ok(message)
}

/// The message answer function `f` makes of `text`.
pub(super) fn answer(
    ctx: &mut Context,
    app: &App,
    call: &mut Call,
    f: u64,
    text: &str,
) -> Result<Words, Stopped> {
    let mut given = self::text(ctx, text)?;
    let mut message = words(app.message.1);
    let at = (&raw mut given).expose_provenance() as u64;
    let ran = call(ctx, f as usize, &[at, addr(&mut message)]);
    if given.cap != 0 {
        ctx.heap.release(given.ptr, given.cap);
    }
    status(ran)?;
    Ok(message)
}

/// Performs the effects of `list` in order, queueing the answers; the end one brings, if any.
fn perform(
    ctx: &mut Context,
    app: &App,
    [items, len, _]: [u64; 3],
    world: &mut impl World,
    queue: &mut VecDeque<Queued>,
) -> Result<Option<End>, Stopped> {
    for i in 0..len {
        // SAFETY: item `i` of the `List[Effect[Msg]]` the handler wrote, which lives until the
        // loop drops it after this call.
        let effect = unsafe { decode(items + i * EFFECT) };
        match effects::perform(app.name, app.grants, effect, world) {
            Ok(Some(Next::Message(boxed))) => {
                queue.push_back(Queued::Message(copy(ctx, app.message, boxed)?))
            }
            Ok(Some(Next::Answer(f, text))) => queue.push_back(Queued::Answer(f, text)),
            Ok(None) => {}
            Err(end) => return Ok(Some(end)),
        }
    }
    Ok(None)
}

/// The effect at `at`: answer functions as slot ids, a message as its box's address.
///
/// # Safety
///
/// `at` holds an Effect's words, whose Strings stay live and unchanged while it is used.
unsafe fn decode<'a>(at: u64) -> Effect<'a, u64, u64> {
    // SAFETY: the caller vouches for the words; each field sits where M4-05 section 8 puts it.
    let word = |i: u64| unsafe { *ptr::with_exposed_provenance::<u64>((at + 8 * i) as usize) };
    // SAFETY: as above; the word holds a String's words.
    let text = |i: u64| unsafe { as_str(at + 8 * i) };
    match word(0) {
        0 => Effect::Print(text(1)),
        1 => Effect::ReadFile(text(1), word(5), word(6)),
        2 => Effect::WriteFile(text(1), text(5), word(9), word(10)),
        3 => Effect::Exit,
        4 => Effect::ExitWith(Int::new(word(1) as i64)),
        tag => unreachable!("an Effect's tag is {tag}"),
    }
}

/// A copy of the value at `from`, owning new blocks.
pub(super) fn copy(
    ctx: &mut Context,
    (record, size): (u64, u64),
    from: u64,
) -> Result<Words, Stopped> {
    let mut to = words(size);
    let at = addr(&mut to);
    if record == 0 {
        let from = ptr::with_exposed_provenance::<u8>(from as usize);
        // SAFETY: `from` is a value of `size` bytes, apart from the new `to`.
        unsafe { ptr::copy_nonoverlapping(from, to.as_mut_ptr().cast(), size as usize) };
    // SAFETY: `record` is the value's, and `to` has room for one apart from `from`.
    } else if unsafe { rt_copy(ctx, 0, record, from, at) } != 0 {
        return Err(Stopped::Trap);
    }
    Ok(to)
}

/// A new String holding `s`, or a `memory` trap.
fn text(ctx: &mut Context, s: &str) -> Result<TextWords, Stopped> {
    new_text(ctx, s).ok_or_else(|| {
        ctx.trap = Trap {
            kind: MEMORY,
            ..Trap::default()
        };
        Stopped::Trap
    })
}

/// Frees what the value in `words` owns, by its type's record.
pub(super) fn release(ctx: &mut Context, record: u64, words: &mut [u64]) {
    if record != 0 {
        // SAFETY: the caller vouches the record is the value's type's, and no one uses it after.
        unsafe { rt_drop(ctx, record, addr(words)) };
    }
}

/// Room for a value of `bytes`, zeroed; one word at least.
pub(super) fn words(bytes: u64) -> Words {
    vec![0; (bytes as usize).div_ceil(8).max(1)]
}

pub(super) fn addr(words: &mut [u64]) -> u64 {
    words.as_mut_ptr().expose_provenance() as u64
}

/// A call's status as the loop's outcome.
pub(super) fn status(status: i32) -> Result<(), Stopped> {
    match status {
        0 => Ok(()),
        2 => Err(Stopped::Time),
        _ => Err(Stopped::Trap),
    }
}
