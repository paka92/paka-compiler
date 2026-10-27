//! Incremental state equals a clean rebuild (M4-14): seeded edit sequences over generated
//! programs, each step compared with a workspace built from the same text: names, index, errors,
//! reads and hashes. Views and an app naming one are added and edited too (M5-14).
//! `PAKA_EDIT_SEEDS` sets how many sequences (default 10,000).

use paka::parser::split;
use paka::workspace::Workspace;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const STEPS: usize = 8;
const KINDS: [&str; 7] = [
    "body",
    "signature",
    "layout",
    "add",
    "delete",
    "rename",
    "view",
];

#[test]
fn edit_sequences_match_a_clean_rebuild() {
    let n = std::env::var("PAKA_EDIT_SEEDS").map_or(10_000, |n| n.parse().unwrap());
    let next = AtomicU64::new(0);
    let found = Mutex::new(Found::default());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            // The main thread's stack, which `paka` checks on.
            let worker = std::thread::Builder::new().stack_size(8 << 20);
            worker
                .spawn_scoped(s, || {
                    loop {
                        let seed = next.fetch_add(1, Ordering::Relaxed);
                        if seed >= n {
                            break;
                        }
                        sequence(seed, &found);
                    }
                })
                .unwrap();
        }
    });
    let found = found.into_inner().unwrap();
    for kind in KINDS {
        let (made, applied) = found.kinds.get(kind).copied().unwrap_or_default();
        println!("{kind:>9}: {made} made, {applied} applied");
        assert!(applied > 0, "no {kind} edit applied");
    }
    assert!(found.failures.is_empty(), "{}", found.failures.join("\n"));
}

#[derive(Default)]
struct Found {
    /// Edits made and applied of each kind; an edit is refused on a program-wide error.
    kinds: BTreeMap<&'static str, (u64, u64)>,
    failures: Vec<String>,
}

fn sequence(seed: u64, found: &Mutex<Found>) {
    let mut ws = Workspace::new(paka_gen::program(seed).as_bytes()).unwrap();
    let mut script = Edits {
        rng: Rng(seed),
        deleted: vec![],
        fresh: 0,
    };
    for step in 0..STEPS {
        let defs = defs(&ws.text());
        let (kind, edits) = script.edit(&defs);
        let applied = edits.iter().fold(false, |any, (name, text)| {
            ws.edit(name, text.as_deref()).is_ok() || any
        });
        let mismatch = same_as_rebuilt(&ws).err();
        let mut found = found.lock().unwrap();
        let counts = found.kinds.entry(kind).or_default();
        counts.0 += 1;
        counts.1 += u64::from(applied);
        if let Some(why) = mismatch {
            let edits = format!("{edits:?}");
            (found.failures).push(format!("seed {seed} step {step} {kind} {edits}: {why}"));
            return;
        }
    }
}

fn same_as_rebuilt(ws: &Workspace) -> Result<(), String> {
    let clean = Workspace::new(ws.text().as_bytes()).map_err(|d| d.to_string())?;
    if ws.index() != clean.index() {
        return Err(format!(
            "index {:?}, rebuilt {:?}",
            ws.index(),
            clean.index()
        ));
    }
    let (kept, rebuilt) = (show(ws), show(&clean));
    match kept.iter().zip(&rebuilt).find(|(k, r)| k != r) {
        Some((k, r)) => Err(format!("kept {k}, rebuilt {r}")),
        None if kept.len() != rebuilt.len() => Err("definitions differ".into()),
        None => Ok(()),
    }
}

fn show(ws: &Workspace) -> Vec<String> {
    let show = |c: paka::DefCheck| {
        let error = c.error.map(|d| d.to_string());
        let (body, interface) = (c.body, c.interface);
        format!("{} {error:?} {:?} {body:?} {interface:?}", c.name, c.reads)
    };
    ws.checks().into_iter().map(show).collect()
}

/// Each definition's name and text.
fn defs(text: &str) -> Vec<(String, String)> {
    let defs = split("main.paka", text.as_bytes()).unwrap().into_iter();
    defs.map(|(name, t)| (name.into(), String::from_utf8_lossy(t).into()))
        .collect()
}

type Edit = (String, Option<String>);

struct Edits {
    rng: Rng,
    /// Deleted definitions, which an add may restore.
    deleted: Vec<String>,
    /// Names made so far, to keep new names apart.
    fresh: usize,
}

impl Edits {
    /// One edit of a random kind, as the definition edits that make it.
    fn edit(&mut self, defs: &[(String, String)]) -> (&'static str, Vec<Edit>) {
        if defs.is_empty() {
            return (
                "add",
                vec![("g0".into(), Some("fn g0() -> Int {\n  0\n}".into()))],
            );
        }
        loop {
            let kind = KINDS[self.rng.below(KINDS.len())];
            if let Some(edits) = self.of(kind, defs) {
                return (kind, edits);
            }
        }
    }

    fn of(&mut self, kind: &str, defs: &[(String, String)]) -> Option<Vec<Edit>> {
        self.fresh += 1;
        let n = self.fresh;
        let (name, text) = &defs[self.rng.below(defs.len())];
        let (header, rest) = text.split_once('\n').unwrap_or((text, ""));
        let replace = |text: String| Some(vec![(name.clone(), Some(text))]);
        match kind {
            "body" if header.starts_with("fn ") || header.starts_with("view ") => {
                let read = self.read(defs);
                replace(format!("{header}\n  let z{n} = {read}\n{rest}"))
            }
            "body" if header.starts_with("app ") => {
                let view = self.pick(defs, "view ")?;
                let line = |l: &str| {
                    let field = l.trim_start().starts_with("view:");
                    if field {
                        format!("  view: {view}")
                    } else {
                        l.into()
                    }
                };
                replace(text.lines().map(line).collect::<Vec<_>>().join("\n"))
            }
            "signature" if header.starts_with("fn ") || header.starts_with("view ") => {
                let param = if header.contains("()") { "" } else { ", " };
                replace(header.replacen('(', &format!("(q{n}: Int{param}"), 1) + "\n" + rest)
            }
            "layout" if header.starts_with("type ") => {
                let default = [" = 0", ""][self.rng.below(2)];
                replace(text.replacen("{ ", &format!("{{ z{n}: Int{default}, "), 1))
            }
            "layout" if header.starts_with("enum ") => {
                replace(text.replacen("{ ", &format!("{{ Z{n}, "), 1))
            }
            "add" if !self.deleted.is_empty() && self.rng.below(2) == 0 => {
                let text = self.deleted.remove(self.rng.below(self.deleted.len()));
                let name = split("main.paka", text.as_bytes()).unwrap()[0].0.to_owned();
                Some(vec![(name, Some(text))])
            }
            "add" => {
                let read = self.read(defs);
                let text = format!("fn g{n}() -> Int {{\n  let a = {read}\n  {n}\n}}");
                Some(vec![(format!("g{n}"), Some(text))])
            }
            "view" if !defs.iter().any(|d| d.1.starts_with("app ")) && self.rng.below(2) == 0 => {
                let view = self.pick(defs, "view ")?;
                let state = self.pick(defs, "type ").unwrap_or("Int".into());
                let update = self.pick(defs, "fn ").unwrap_or("update".into());
                let text = format!(
                    "app a{n} {{\n  state: {state}\n  update: {update}\n  view: {view}\n  grants: []\n}}"
                );
                Some(vec![(format!("a{n}"), Some(text))])
            }
            "view" => {
                let read = self.read(defs);
                let msg = self.pick(defs, "enum ").unwrap_or("Msg".into());
                let call = match self.pick(defs, "view ") {
                    Some(v) => format!("\n    {v}(s)"),
                    None => String::new(),
                };
                let text = format!(
                    "view v{n}(s: Int) -> View<{msg}> {{\n  Col() {{\n    Text(\"{{{read}}}\"){call}\n  }}\n}}"
                );
                Some(vec![(format!("v{n}"), Some(text))])
            }
            "delete" => {
                self.deleted.push(text.clone());
                Some(vec![(name.clone(), None)])
            }
            "rename" => {
                self.deleted.push(text.clone());
                let new = format!("{name}r");
                let text = text.replacen(&format!(" {name}"), &format!(" {new}"), 1);
                Some(vec![(new, Some(text)), (name.clone(), None)])
            }
            _ => None,
        }
    }

    /// The name of a definition whose text starts with `start`, if one does.
    fn pick(&mut self, defs: &[(String, String)], start: &str) -> Option<String> {
        let some: Vec<_> = defs.iter().filter(|d| d.1.starts_with(start)).collect();
        (!some.is_empty()).then(|| some[self.rng.below(some.len())].0.clone())
    }

    /// A name to read: one defined or once defined, sometimes misspelled by one letter.
    fn read(&mut self, defs: &[(String, String)]) -> String {
        let name = &defs[self.rng.below(defs.len())].0;
        match self.rng.below(4) {
            0 => format!("{name}x"),
            _ => name.clone(),
        }
    }
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}
