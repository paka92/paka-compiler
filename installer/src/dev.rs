//! A dev session (M4-37): the last installed program, its compiled code and its slots; and a run
//! of its app (M4-36) that the interrupt stops once its time is up (M4-39).

use crate::{Loader, Slots};
use paka::alone::{Names, relower};
use paka::core_form::{FnId, Program, Type};
use paka::emit::{Cache, Emitter, show};
use paka::hash::Hash;
use paka::layout::Layouts;
use paka::parser::split;
use paka::source::{Source, in_file};
use paka_rt::abi::dev::app::{App, Stopped, serve};
use paka_rt::abi::dev::records::Records;
use paka_rt::abi::dev::view::{self, Drawing};
use paka_rt::abi::dev::{Context, HOLE, Trap, on_runner};
use paka_rt::effects::{End, World};
use std::ffi::OsString;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

pub struct Dev {
    pub emitter: Emitter,
    pub cache: Cache,
    pub loader: Loader<Hash>,
    pub slots: Slots,
    pub program: Program,
    pub layouts: Layouts,
    /// Each type's record address, 0 when it owns nothing.
    pub records: Vec<u64>,
    pub keys: Vec<Hash>,
    /// The installed program's files, and its definitions by name.
    files: Vec<(OsString, Vec<u8>)>,
    names: Names,
}

impl Default for Dev {
    fn default() -> Dev {
        Dev {
            emitter: Emitter::new().expect("Cranelift supports this host"),
            cache: Cache::default(),
            loader: Loader::default(),
            slots: Slots::new(0),
            program: Program::default(),
            layouts: Layouts::default(),
            records: vec![],
            keys: vec![],
            files: vec![],
            names: Names::default(),
        }
    }
}

/// A run's end, or why it stopped; the context's trap record and the ledger's leaks and faults.
pub struct Ran {
    pub end: Result<End, Stopped>,
    pub trap: Trap,
    pub ledger: (usize, usize),
}

impl Dev {
    /// Checks, compiles and installs `source` in one swap; on an error the last program stays.
    /// When one function is all that changed and it keeps its signature, only it is checked and
    /// compiled (paka::alone).
    pub fn edit(&mut self, source: &Source) -> Result<(), String> {
        if self.edit_one(source).is_some() {
            return Ok(());
        }
        let (mut p, ..) = paka::check("main.paka", source).map_err(|d| d.to_string())?;
        let layouts = Layouts::new(&mut p);
        let keys = self.cache.build(&mut self.emitter, &p, &layouts)?;
        let records = Box::leak(Box::new(Records::new(&layouts.specs())));
        let addresses: Vec<u64> = (0..p.types.len()).map(|t| records.address(t)).collect();
        self.install(&keys, &addresses)?;
        (self.program, self.layouts, self.records, self.keys) = (p, layouts, addresses, keys);
        self.files = files(source);
        self.names = Names::default();
        for (_, text) in &self.files {
            for (_, def) in split("main.paka", text).expect("a checked file splits") {
                self.names.add(&String::from_utf8_lossy(def));
            }
        }
        Ok(())
    }

    /// Fills the slots with the code of `keys` and the record of each type.
    fn install(&mut self, keys: &[Hash], records: &[u64]) -> Result<(), String> {
        let mut group = (self.loader.group(keys, |k| self.cache.code(k)))
            .map_err(|e| format!("cannot install the code: {e}"))?;
        group.extend((records.iter().enumerate()).map(|(t, &a)| (keys.len() + t, a as usize)));
        if group.len() != self.keys.len() + self.program.types.len() {
            self.slots = Slots::new(group.len());
        }
        self.slots
            .swap(&group)
            .expect("a group fills the slot table");
        Ok(())
    }

    /// `edit` of a `source` that differs from the installed one in one function's text alone;
    /// `None` when it does not, leaving the dev session for a whole build.
    fn edit_one(&mut self, source: &Source) -> Option<()> {
        let files = files(source);
        let changed: Vec<usize> = (0..files.len().min(self.files.len()))
            .filter(|&k| self.files[k] != files[k])
            .collect();
        let [k] = changed[..] else { return None };
        let (old, new) = (&self.files[k], &files[k]);
        if self.files.len() != files.len() || old.0 != new.0 {
            return None;
        }
        let (was, now) = (
            split("main.paka", &old.1).ok()?,
            split("main.paka", &new.1).ok()?,
        );
        let mut differ = (was.iter().zip(&now)).filter(|(a, b)| a != b);
        let ((_, before), (_, after)) = differ.next()?;
        if differ.next().is_some() || was.len() != now.len() || !before.starts_with(b"fn ") {
            return None;
        }
        let start = before.as_ptr() as usize - old.1.as_ptr() as usize;
        if start > 0 && old.1[start - 1] != b'\n' {
            return None;
        }
        let line = self.program.files[k].1 + old.1[..start].iter().filter(|&&b| b == b'\n').count();
        let before = String::from_utf8(before.to_vec()).ok()?;
        let after = String::from_utf8(after.to_vec()).ok()?;
        let mut p = std::mem::take(&mut self.program);
        let lowered = relower(
            &mut p,
            &self.names,
            &after,
            line..line + before.lines().count(),
        );
        self.program = p;
        let id = lowered?;
        let key = self
            .cache
            .rebuild(&mut self.emitter, &self.program, &self.layouts, id);
        let mut keys = self.keys.clone();
        keys[id] = key.ok()?;
        let records = self.records.clone();
        if self.install(&keys, &records).is_err() {
            return None;
        }
        self.keys = keys;
        self.names.remove(&before);
        self.names.add(&after);
        self.files = files;
        Some(())
    }

    /// Runs the app with `args` in `world` on a heap of `cap` bytes; past `limit` the interrupt
    /// stops it.
    pub fn run(
        &self,
        args: &[OsString],
        world: &mut (impl World + Send),
        cap: u64,
        limit: Duration,
    ) -> Ran {
        let p = &self.program;
        let a = p.app.as_ref().expect("an app");
        let update = &p.fns[a.update];
        let (msg, effects) = (update.params.last().unwrap().ty, update.result.unwrap());
        let Type::Enum(e) = p.types[msg] else {
            unreachable!("the message is a user enum");
        };
        let start = a
            .start
            .expect("a headless app: the visual loop is still to come");
        let fields = &p.enums[e].variants[start].fields;
        let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        let typed = |t: usize| (self.records[t], self.layouts.get(t).size as u64);
        let state = paka::emit::state_slot(p).map(|def| {
            let (record, size) = typed(update.params[0].ty);
            (def, record, size)
        });
        let app = App {
            name: &a.name,
            grants: &a.grants,
            update: a.update,
            state,
            start: start as u64,
            message: typed(msg),
            effects: self.records[effects],
        };
        let start = match paka_rt::effects::arguments(&a.name, &names, args) {
            Ok(start) => start,
            Err(end) => {
                let (trap, ledger) = (Trap::default(), (0, 0));
                return Ran {
                    end: Ok(end),
                    trap,
                    ledger,
                };
            }
        };
        on_runner(cap, |ctx| {
            let end = until(ctx, limit, |ctx| {
                // SAFETY: `ctx` is a context that lives through the session.
                let session = unsafe { self.slots.session((&raw mut *ctx).cast()) }.unwrap();
                let mut call = |ctx: &mut Context, id: usize, args: &[u64]| {
                    // SAFETY: `edit` installed every definition as M4-05 section 5 compiles it,
                    // and the loop passes each the argument words its signature takes.
                    unsafe { session.call(id, (&raw mut *ctx).cast(), args) }.unwrap()
                };
                serve(ctx, &app, &start, &mut call, world)
            });
            let ledger = (ctx.heap.leaks().len(), ctx.heap.faults().len());
            let trap = std::mem::take(&mut ctx.trap);
            Ran { end, trap, ledger }
        })
    }

    /// Draws view `id` from its argument words on `ctx` (M5-24); release the drawing on the same
    /// context.
    pub fn draw(&self, ctx: &mut Context, id: FnId, args: &[u64]) -> Result<Drawing, Stopped> {
        // SAFETY: `ctx` is a context that lives through the session.
        let session = unsafe { self.slots.session((&raw mut *ctx).cast()) }.unwrap();
        let mut call = |ctx: &mut Context, id: usize, args: &[u64]| {
            // SAFETY: `edit` installed every definition as M4-05 section 5 compiles it, and
            // `view::draw` passes the view the argument words its signature takes.
            unsafe { session.call(id, (&raw mut *ctx).cast(), args) }.unwrap()
        };
        let read = |at: u64, to: &mut [u8]| {
            let from = ptr::with_exposed_provenance::<u8>(at as usize);
            // SAFETY: `show` reads only the words and blocks of a message the drawing holds.
            unsafe { ptr::copy_nonoverlapping(from, to.as_mut_ptr(), to.len()) };
        };
        let (p, l) = (&self.program, &self.layouts);
        let mut show = |t: u64, at: u64| show(p, l, t as usize, at, &read);
        view::draw(ctx, &mut call, id, args, &mut show)
    }

    /// The first line of the report of `trap` (M2-06 section 1.2), and a hole's detail line; the
    /// other detail lines and the chain are still to come.
    pub fn report(&self, trap: &Trap) -> String {
        let p = &self.program;
        let app = &p.app.as_ref().expect("an app").name;
        let kind = KINDS[trap.kind as usize - 1];
        let site = match trap.site {
            0 => trap.frames.first().copied().unwrap_or(0),
            site => site,
        };
        let (_, at) = p.place(site);
        let ((file, _), line) = in_file(&p.files, |f| f.1, at.line);
        let mut report = format!("{app}: trap {kind} at {file}:{line}:{}\n", at.col);
        if let (HOLE, Some(name)) = (trap.kind, &at.hole) {
            report += &format!("  ??{name}\n");
        }
        report
    }
}

fn files(source: &Source) -> Vec<(OsString, Vec<u8>)> {
    (source.files.iter())
        .map(|f| (f.path.clone(), source.text[f.bytes.clone()].to_vec()))
        .collect()
}

/// The trap kinds by number (M4-05 section 6).
const KINDS: [&str; 8] = [
    "overflow",
    "divide_by_zero",
    "out_of_bounds",
    "bad_conversion",
    "assert",
    "hole",
    "stack",
    "memory",
];

/// Runs `f` on `ctx`, setting its interrupt once `limit` passes (docs/decisions/M4-04.md).
fn until<R>(ctx: &mut Context, limit: Duration, f: impl FnOnce(&mut Context) -> R) -> R {
    let flag = (&raw const ctx.interrupt).expose_provenance();
    let (done, wait) = mpsc::channel::<()>();
    thread::scope(|s| {
        s.spawn(move || {
            if wait.recv_timeout(limit) == Err(RecvTimeoutError::Timeout) {
                // SAFETY: the context outlives this scope, and its interrupt is an atomic that
                // the run only ever loads.
                let interrupt = unsafe { &*ptr::with_exposed_provenance::<AtomicU64>(flag) };
                interrupt.store(1, Ordering::Relaxed);
            }
        });
        let r = f(ctx);
        drop(done);
        r
    })
}
