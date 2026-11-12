//! Native value layouts (M4-22, docs/decisions/M4-05.md sections 8 and 10): every value is whole
//! 8-byte words, fields in declared order after a tag word for a tagged type. One table serves the
//! dev and the release back end. A `Bool` is one byte holding 0 or 1: a List item takes just that
//! byte, any other place a word whose other bytes mean nothing.
//!
//! A field of a program type whose type reaches that type by value, through records, enums,
//! `Option` or `Result`, is a box: a record's field alone, an enum variant's fields all together
//! in one block, the variant's value then the tag and the block's address. Every field on such a
//! cycle reaches its own type, so each cycle holds a box or a block and every size is finite.

use crate::core_form::{Program, Ty, Type, plain};
use paka_rt::abi::dev::map::STRING_KEY;
use paka_rt::abi::dev::records::{Part, Spec};
pub use paka_rt::abi::table::{ALIGN, BLOCK, CHARS, CONSTANT, Header, LIST, STRING};
use std::collections::{BTreeMap, BTreeSet};

/// How a value of one type lies in memory.
#[derive(Debug, PartialEq)]
pub struct Layout {
    /// In bytes.
    pub size: usize,
    /// In bytes, as a List item.
    pub item: usize,
    /// Whether the first word is a tag: the variant's declared position.
    pub tagged: bool,
    /// The fields of each alternative: a record's one list, or one per variant in tag order.
    pub alternatives: Vec<Vec<Place>>,
    /// Per alternative, the bytes of the block its fields lie in; 0 when they lie in the value.
    pub blocks: Vec<usize>,
    /// What copy and release walk: one list, or one per variant when tagged.
    pub owned: Vec<Vec<Entry>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// In bytes from the value's start, or from its block's.
    pub offset: usize,
    pub ty: Ty,
    /// The word points to a heap block holding the value.
    pub boxed: bool,
    /// The field lies in the variant's block, whose address is the word at `BLOCK`.
    pub in_block: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub offset: usize,
    pub kind: Owned,
}

/// An owned part, with the types whose records its walk needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Owned {
    String,
    /// The item type.
    List(Ty),
    Map(Ty, Ty),
    /// The boxed type.
    Box(Ty),
    /// The block of a variant: the enum type and the variant.
    Block(Ty, usize),
    /// A tagged value inline: its own record says what the variant at hand owns.
    Value(Ty),
}

/// The layout of every type of a program.
#[derive(Debug, Default, PartialEq)]
pub struct Layouts {
    layouts: BTreeMap<Ty, Layout>,
    /// For each record or enum type, its boxed fields: (alternative, field) positions.
    boxed: BTreeMap<Ty, BTreeSet<(usize, usize)>>,
}

impl Layouts {
    /// Lays out every type of `p`, adding each record and enum type and the types `Effect`
    /// payloads name.
    pub fn new(p: &mut Program) -> Layouts {
        for i in 0..p.records.len() {
            p.ty(Type::Record(i));
        }
        for i in 0..p.enums.len() {
            p.ty(Type::Enum(i));
        }
        let effects: Vec<Ty> = (p.types.iter())
            .filter_map(|t| match t {
                Type::Effect(m) => Some(*m),
                _ => None,
            })
            .collect();
        let crossing: Vec<Type> = (0..p.foreign.len())
            .flat_map(|k| {
                let f = p.crossing(k);
                f.params
                    .iter()
                    .chain(&f.result)
                    .map(|&t| plain(t))
                    .collect::<Vec<_>>()
            })
            .collect();
        for t in crossing {
            p.ty(t);
        }
        for m in effects {
            let string = p.ty(Type::String);
            p.ty(Type::Int);
            p.ty(Type::Fn(vec![string], Some(m)));
            for k in 0..p.foreign.len() {
                if let Some(r) = p.crossing(k).result {
                    let r = p.ty(plain(r));
                    p.ty(Type::Fn(vec![r], Some(m)));
                }
            }
        }
        let p = &*p;
        let boxed = (0..p.types.len())
            .map(|ty| (ty, boxed_fields(p, ty)))
            .collect();
        let mut layouts = Layouts {
            layouts: BTreeMap::new(),
            boxed,
        };
        for ty in 0..p.types.len() {
            layouts.lay(p, ty);
        }
        for ty in 0..p.types.len() {
            layouts.lay_blocks(p, ty);
        }
        layouts
    }

    pub fn get(&self, ty: Ty) -> &Layout {
        &self.layouts[&ty]
    }

    /// Whether a value of `ty` owns a block, so copying it copies blocks and it must be dropped.
    pub fn owns(&self, ty: Ty) -> bool {
        self.get(ty).owned.iter().any(|alt| !alt.is_empty())
    }

    /// The type records' contents for the runtime, by type number, then one per variant block in
    /// type and variant order. A type that owns nothing has none.
    pub fn specs(&self) -> Vec<Option<Spec>> {
        let blocks: Vec<(Ty, usize)> = (self.layouts.iter())
            .flat_map(|(&t, l)| {
                (0..l.blocks.len())
                    .filter(|&a| l.blocks[a] != 0)
                    .map(move |a| (t, a))
            })
            .collect();
        let part = |e: &Entry| match e.kind {
            Owned::String => (e.offset as u64, Part::String),
            Owned::List(item) => {
                let size = self.get(item).item as u64;
                (e.offset as u64, Part::List { item, size })
            }
            Owned::Value(t) => (e.offset as u64, Part::Value(t)),
            Owned::Box(item) => {
                let size = self.get(item).size as u64;
                (e.offset as u64, Part::Box { item, size })
            }
            Owned::Block(t, alt) => {
                let item = self.layouts.len() + blocks.binary_search(&(t, alt)).unwrap_or(0);
                let size = self.get(t).blocks[alt] as u64;
                (e.offset as u64, Part::Box { item, size })
            }
            Owned::Map(k, v) => {
                let string = if self.owns(k) { STRING_KEY } else { 0 };
                let shape = string | self.get(v).size as u64;
                (e.offset as u64, Part::Map { value: v, shape })
            }
        };
        let spec = |(&ty, l): (&Ty, &Layout)| {
            let alternatives = l.owned.iter().map(|alt| alt.iter().map(part).collect());
            let spec = Spec {
                size: l.size as u64,
                tagged: l.tagged,
                alternatives: alternatives.collect(),
            };
            self.owns(ty).then_some(spec)
        };
        let block = |&(t, alt): &(Ty, usize)| {
            let l = self.get(t);
            let owned: Vec<_> = (l.alternatives[alt].iter())
                .flat_map(|p| self.entries(p))
                .collect();
            let spec = Spec {
                size: l.blocks[alt] as u64,
                tagged: false,
                alternatives: vec![owned.iter().map(part).collect()],
            };
            (!owned.is_empty()).then_some(spec)
        };
        let types = self.layouts.iter().map(spec);
        types.chain(blocks.iter().map(block)).collect()
    }

    /// Lays out `ty` and what it holds by value; its size in words.
    fn lay(&mut self, p: &Program, ty: Ty) -> usize {
        if let Some(l) = self.layouts.get(&ty) {
            return l.size / 8;
        }
        let leaf = |words, owned: Vec<Entry>| (false, words, vec![], vec![owned]);
        let at0 = |kind| vec![Entry { offset: 0, kind }];
        let (tagged, words, alternatives, owned) = match p.types[ty] {
            Type::Int | Type::Float | Type::Bool | Type::Fn(..) | Type::View(_) | Type::Ui => {
                leaf(1, vec![])
            }
            Type::String => leaf(STRING.size / 8, at0(Owned::String)),
            Type::List(item) => leaf(LIST.size / 8, at0(Owned::List(item))),
            Type::Map(k, v) => leaf(1, at0(Owned::Map(k, v))),
            _ => {
                let (tagged, alts) = alternatives(p, ty);
                let start = if tagged { 8 } else { 0 };
                let mut places = vec![];
                let mut owned = vec![];
                let blocked: Vec<bool> = (0..alts.len()).map(|a| self.in_block(p, ty, a)).collect();
                for (a, fields) in alts.iter().enumerate() {
                    if blocked[a] {
                        // The fields are laid out once every value size is known.
                        places.push(vec![]);
                        owned.push(vec![Entry {
                            offset: BLOCK,
                            kind: Owned::Block(ty, a),
                        }]);
                        continue;
                    }
                    let mut offset = start;
                    let mut alt = vec![];
                    for (f, &(field, boxed)) in fields.iter().enumerate() {
                        let boxed = boxed || self.boxed[&ty].contains(&(a, f));
                        let place = Place {
                            offset,
                            ty: field,
                            boxed,
                            in_block: false,
                        };
                        offset += if boxed { 8 } else { self.lay(p, field) * 8 };
                        alt.push(place);
                    }
                    owned.push(alt.iter().flat_map(|p| self.entries(p)).collect());
                    places.push(alt);
                }
                let end = |(alt, &blocked): (&Vec<Place>, &bool)| match blocked {
                    true => BLOCK + 8,
                    false => alt.last().map_or(start, |p| p.offset + self.width(p)),
                };
                let words = places.iter().zip(&blocked).map(end).max().unwrap_or(start) / 8;
                let owned = match tagged {
                    true => owned,
                    false => vec![owned.into_iter().flatten().collect()],
                };
                (tagged, words, places, owned)
            }
        };
        let item = if p.types[ty] == Type::Bool {
            1
        } else {
            words * 8
        };
        let layout = Layout {
            size: words * 8,
            item,
            tagged,
            blocks: vec![0; alternatives.len()],
            alternatives,
            owned,
        };
        self.layouts.insert(ty, layout);
        words
    }

    /// The bytes `place` takes; its type is laid out.
    fn width(&self, place: &Place) -> usize {
        if place.boxed {
            8
        } else {
            self.layouts[&place.ty].size
        }
    }

    /// Whether variant `a` of `ty` lies in a block: a variant of an enum with a field reaching it.
    fn in_block(&self, p: &Program, ty: Ty, a: usize) -> bool {
        matches!(p.types[ty], Type::Enum(_)) && self.boxed[&ty].iter().any(|&(x, _)| x == a)
    }

    /// Lays out the fields of each variant of `ty` that lies in a block, from the block's start.
    fn lay_blocks(&mut self, p: &Program, ty: Ty) {
        let Type::Enum(e) = p.types[ty] else { return };
        for (a, v) in p.enums[e].variants.iter().enumerate() {
            if !self.in_block(p, ty, a) {
                continue;
            }
            let mut offset = 0;
            let places: Vec<Place> = (v.fields.iter())
                .map(|f| {
                    let place = Place {
                        offset,
                        ty: f.ty,
                        boxed: false,
                        in_block: true,
                    };
                    offset += self.layouts[&f.ty].size;
                    place
                })
                .collect();
            let l = self.layouts.get_mut(&ty).expect("a laid out type");
            (l.alternatives[a], l.blocks[a]) = (places, offset);
        }
    }

    /// What the value at `place` owns, by offset in the enclosing value.
    fn entries(&self, place: &Place) -> Vec<Entry> {
        let at = |kind| {
            vec![Entry {
                offset: place.offset,
                kind,
            }]
        };
        if place.boxed {
            return at(Owned::Box(place.ty));
        }
        let inner = &self.layouts[&place.ty];
        match inner.tagged {
            true if inner.owned.iter().all(Vec::is_empty) => vec![],
            true => at(Owned::Value(place.ty)),
            false => (inner.owned[0].iter())
                .map(|e| Entry {
                    offset: place.offset + e.offset,
                    ..*e
                })
                .collect(),
        }
    }
}

/// A tagged or untagged list of alternatives, each a list of (type, always boxed) fields.
type Alternatives = (bool, Vec<Vec<(Ty, bool)>>);

fn alternatives(p: &Program, ty: Ty) -> Alternatives {
    let fields = |fs: &[crate::core_form::Field]| fs.iter().map(|f| (f.ty, false)).collect();
    match p.types[ty] {
        Type::Record(i) => (false, vec![fields(&p.records[i].fields)]),
        Type::Enum(i) => (
            true,
            p.enums[i]
                .variants
                .iter()
                .map(|v| fields(&v.fields))
                .collect(),
        ),
        Type::Option(t) => (true, vec![vec![(t, false)], vec![]]),
        Type::Result(t, e) => (true, vec![vec![(t, false)], vec![(e, false)]]),
        Type::Effect(msg) => {
            let find = |t: Type| p.types.iter().position(|known| *known == t).unwrap();
            let string = find(Type::String);
            let int = find(Type::Int);
            let answer = find(Type::Fn(vec![string], Some(msg)));
            let s = (string, false);
            let f = (answer, false);
            let mut alts = vec![
                vec![s],
                vec![s, f, f],
                vec![s, s, (msg, true), f],
                vec![],
                vec![(int, false)],
            ];
            // A foreign effect's arguments, then `then`: a function of its result, or the message
            // boxed when it has none.
            alts.extend((0..p.foreign.len()).map(|k| {
                let c = p.crossing(k);
                let mut fields: Vec<_> =
                    c.params.iter().map(|&t| (find(plain(t)), false)).collect();
                fields.push(match c.result {
                    Some(r) => (find(Type::Fn(vec![find(plain(r))], Some(msg))), false),
                    None => (msg, true),
                });
                fields.push(f);
                fields
            }));
            (true, alts)
        }
        ref t => unreachable!("no alternatives in {t:?}"),
    }
}

/// The fields of record or enum type `ty` whose types reach it by value.
fn boxed_fields(p: &Program, ty: Ty) -> BTreeSet<(usize, usize)> {
    if !matches!(p.types[ty], Type::Record(_) | Type::Enum(_)) {
        return BTreeSet::new();
    }
    let mut boxed = BTreeSet::new();
    for (a, fields) in alternatives(p, ty).1.iter().enumerate() {
        for (f, &(field, _)) in fields.iter().enumerate() {
            if reaches(p, field, ty, &mut BTreeSet::new()) {
                boxed.insert((a, f));
            }
        }
    }
    boxed
}

/// Whether a value of `ty` holds type `target` by value.
fn reaches(p: &Program, ty: Ty, target: Ty, seen: &mut BTreeSet<Ty>) -> bool {
    if ty == target {
        return true;
    }
    if !seen.insert(ty) {
        return false;
    }
    match p.types[ty] {
        Type::Record(_) | Type::Enum(_) | Type::Option(_) | Type::Result(..) => {
            let (_, alts) = alternatives(p, ty);
            (alts.iter().flatten()).any(|&(t, _)| reaches(p, t, target, seen))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
