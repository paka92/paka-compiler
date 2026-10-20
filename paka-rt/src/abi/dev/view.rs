//! Views in dev code (M5-24). A compiled view writes its tree into the context's `tree` buffer
//! through `rt.view`, one op and its operands at a time; `draw` decodes the buffer into a
//! `tree::Node`. A String is its byte length, then its bytes packed into words. A message's words
//! follow its op, copied with every block they own, so the drawing owns them until it is released.

use super::Context;
use super::app::{Call, Stopped, status};
use super::records::{rt_copy, rt_drop};
use super::words::as_str;
use crate::tree::{self, Node, Step};
use std::fmt::{self, Display, Formatter};
use std::{mem, ptr};

/// Starts a child list.
pub const OPEN: u64 = 0;
/// Closes the list into the view's root.
pub const ROOT: u64 = 1;
/// Closes the list into a Row or a Col: the gap.
pub const ROW: u64 = 2;
pub const COL: u64 = 3;
/// A String's address.
pub const TEXT: u64 = 4;
pub const HOLE: u64 = 5;
/// The label's address, the message's address, its type's record, its size in bytes and its type.
pub const BUTTON: u64 = 6;
pub const INPUT: u64 = 7;
/// Adds the last node to the list: its line, whether a loop made it, the key's kind and the key.
pub const CHILD: u64 = 8;

/// Key kinds of `CHILD`: none, an Int, a String's address.
pub const NO_KEY: u64 = 0;
pub const INT_KEY: u64 = 1;
pub const STRING_KEY: u64 = 2;

/// `rt.view`: appends op `op` and its operands to the tree; a message past the cap is a `memory`
/// trap, and the op is not appended.
///
/// # Safety
///
/// Each operand is what `op` says: a String's address is live, and a message's address holds a
/// value of its record and size.
pub unsafe extern "sysv64" fn rt_view(
    ctx: &mut Context,
    op: u64,
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
) -> i32 {
    let mut tree = mem::take(&mut ctx.tree);
    let start = tree.len();
    tree.push(op);
    // SAFETY: the caller vouches each address holds what `op` says.
    let copied = unsafe {
        match op {
            TEXT | HOLE | BUTTON | INPUT => put_text(&mut tree, a),
            ROW | COL => tree.push(a),
            CHILD => {
                tree.extend([a, b, c]);
                match c {
                    INT_KEY => tree.push(d),
                    STRING_KEY => put_text(&mut tree, d),
                    _ => {}
                }
            }
            _ => {}
        }
        !matches!(op, BUTTON | INPUT) || {
            tree.extend([e, c, d]);
            put_value(ctx, &mut tree, b, c, d)
        }
    };
    if !copied {
        tree.truncate(start);
    }
    ctx.tree = tree;
    i32::from(!copied)
}

/// # Safety
///
/// `at` holds a String's words.
unsafe fn put_text(tree: &mut Vec<u64>, at: u64) {
    // SAFETY: the caller vouches for the String.
    let s = unsafe { as_str(at) };
    tree.push(s.len() as u64);
    tree.extend(s.as_bytes().chunks(8).map(|c| {
        let mut word = [0; 8];
        word[..c.len()].copy_from_slice(c);
        u64::from_le_bytes(word)
    }));
}

/// Appends a copy of the value at `from`; false on a `memory` trap, what was copied by then
/// being freed when the run ends.
///
/// # Safety
///
/// `from` holds a value of `record` and `bytes`.
unsafe fn put_value(
    ctx: &mut Context,
    tree: &mut Vec<u64>,
    from: u64,
    record: u64,
    bytes: u64,
) -> bool {
    let at = tree.len();
    tree.resize(at + (bytes as usize).div_ceil(8).max(1), 0);
    let to = tree[at..].as_mut_ptr();
    if record == 0 {
        let from = ptr::with_exposed_provenance::<u8>(from as usize);
        // SAFETY: `from` holds `bytes`, and `to` has room for them in another block.
        unsafe { ptr::copy_nonoverlapping(from, to.cast(), bytes as usize) };
        return true;
    }
    let to = to.expose_provenance() as u64;
    // SAFETY: `record` is the value's, and `to` has room for it apart from `from`.
    unsafe { rt_copy(ctx, 0, record, from, to) == 0 }
}

/// A message a node carries: where its value lies in the drawing, and its value text form.
#[derive(Debug, PartialEq)]
pub struct Message {
    pub at: usize,
    pub text: String,
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// A drawn tree and the words its messages lie in, which own blocks of the run's heap until
/// `release`.
pub struct Drawing {
    pub root: Node<Message>,
    words: Vec<u64>,
    /// Each message's record and position, 0 when it owns nothing.
    owned: Vec<(u64, usize)>,
}

impl Drawing {
    /// The address of the message's value.
    pub fn address(&self, m: &Message) -> u64 {
        self.words[m.at..].as_ptr().expose_provenance() as u64
    }

    pub fn release(self, ctx: &mut Context) {
        release(ctx, self.words, &self.owned);
    }
}

/// Draws view `id` from argument words `args`; `show` gives a message's text from its type and
/// address. On a trap or a stop, what the view drew so far is freed.
pub fn draw(
    ctx: &mut Context,
    call: &mut Call,
    id: usize,
    args: &[u64],
    show: &mut dyn FnMut(u64, u64) -> String,
) -> Result<Drawing, Stopped> {
    ctx.tree.clear();
    let mut out = 0u64;
    let mut given = args.to_vec();
    given.push((&raw mut out).expose_provenance() as u64);
    let ran = status(call(ctx, id, &given));
    let words = mem::take(&mut ctx.tree);
    if let Err(stopped) = ran {
        let (_, owned) = decode(&words, &mut |_, _| String::new());
        release(ctx, words, &owned);
        return Err(stopped);
    }
    let (root, owned) = decode(&words, show);
    let root = root.expect("a view that ran wrote its root");
    Ok(Drawing { root, words, owned })
}

fn release(ctx: &mut Context, mut words: Vec<u64>, owned: &[(u64, usize)]) {
    for &(record, at) in owned.iter().filter(|(record, _)| *record != 0) {
        let at = words[at..].as_mut_ptr().expose_provenance() as u64;
        // SAFETY: `rt.view` copied a value of `record` there, which nothing uses after.
        unsafe { rt_drop(ctx, record, at) };
    }
}

/// The tree in `words`, as the interpreter builds it (M5-02), and each message's record and
/// position; no tree when the view wrote no root.
fn decode(
    words: &[u64],
    show: &mut dyn FnMut(u64, u64) -> String,
) -> (Option<Node<Message>>, Vec<(u64, usize)>) {
    let mut r = Reader { words, at: 0 };
    let (mut lists, mut last, mut owned) = (vec![], None, vec![]);
    while r.at < words.len() {
        match r.next() {
            OPEN => lists.push(vec![]),
            ROOT => last = pop(&mut lists).into_iter().next().map(|(_, n)| n),
            op @ (ROW | COL) => {
                let gap = r.next();
                let name = if op == ROW { "Row" } else { "Col" };
                let children = tree::unique(name, pop(&mut lists));
                last = Some(match op {
                    ROW => Node::Row { gap, children },
                    _ => Node::Col { gap, children },
                });
            }
            TEXT => last = Some(Node::Text(r.text())),
            HOLE => last = Some(Node::Hole(r.text())),
            op @ (BUTTON | INPUT) => {
                let label = r.text();
                let [ty, record, bytes] = [r.next(), r.next(), r.next()];
                let at = r.at;
                r.at += (bytes as usize).div_ceil(8).max(1);
                owned.push((record, at));
                let address = words[at..].as_ptr().expose_provenance() as u64;
                let message = Message {
                    at,
                    text: show(ty, address),
                };
                last = Some(match op {
                    BUTTON => Node::Button { label, on: message },
                    _ => Node::Input {
                        text: label,
                        on_change: message,
                        edit: tree::Edit::default(),
                    },
                });
            }
            CHILD => {
                let [line, looped, kind] = [r.next(), r.next(), r.next()].map(|w| w as usize);
                let key = match kind as u64 {
                    INT_KEY => Some(Step::Int(r.next() as i64)),
                    STRING_KEY => Some(Step::Str(r.text())),
                    _ => None,
                };
                let node = last.take().expect("a node before its child line");
                let kids = lists.last_mut().expect("a child list being built");
                let step = match key {
                    Some(step) => step,
                    None if looped == 1 && matches!(node, Node::Hole(_)) => {
                        let made = kids
                            .iter()
                            .filter(|(s, _)| matches!(s, Step::LoopHole(l, _) if *l == line));
                        Step::LoopHole(line, made.count())
                    }
                    None => Step::Place(line),
                };
                kids.push((step, node));
            }
            op => unreachable!("a view op is {op}"),
        }
    }
    (last.filter(|_| lists.is_empty()), owned)
}

fn pop<M>(lists: &mut Vec<Vec<(Step, Node<M>)>>) -> Vec<(Step, Node<M>)> {
    lists.pop().expect("a child list being built")
}

struct Reader<'a> {
    words: &'a [u64],
    at: usize,
}

impl Reader<'_> {
    fn next(&mut self) -> u64 {
        self.at += 1;
        self.words[self.at - 1]
    }

    fn text(&mut self) -> String {
        let bytes = self.next() as usize;
        let words = &self.words[self.at..self.at + bytes.div_ceil(8)];
        self.at += words.len();
        let mut s: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        s.truncate(bytes);
        String::from_utf8(s).expect("a String's bytes are UTF-8")
    }
}
