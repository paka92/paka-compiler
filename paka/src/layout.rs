//! Native value layouts (M4-22, docs/decisions/M4-05.md sections 8 and 10): every value is whole
//! 8-byte words, fields in declared order after a tag word for a tagged type. One table serves the
//! dev and the release back end.
//!
//! A field of a program type whose type reaches that type by value, through records, enums,
//! `Option` or `Result`, is a box. Every field on such a cycle reaches its own type, so each cycle
//! holds a box and every size is finite.

use crate::sig::{Item, Node, Ty, Types};
use std::collections::{BTreeMap, BTreeSet};

/// How a value of one type lies in memory.
#[derive(Debug, PartialEq)]
pub struct Layout {
    /// In bytes.
    pub size: usize,
    /// Whether the first word is a tag: the variant's declared position.
    pub tagged: bool,
    /// The fields of each alternative: a record's one list, or one per variant in tag order.
    pub alternatives: Vec<Vec<Place>>,
    /// What copy and release walk: one list, or one per variant when tagged.
    pub owned: Vec<Vec<Entry>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// In bytes from the value's start.
    pub offset: usize,
    pub ty: Ty,
    /// The word points to a heap block holding the value.
    pub boxed: bool,
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
    /// The boxed type. In `WriteFile`'s `then` the word may instead be `u64::MAX`: none given.
    Box(Ty),
    /// A tagged value inline: its own record says what the variant at hand owns.
    Value(Ty),
}

/// The layout of every type a program's checked signatures meet, but those with a type variable.
pub struct Layouts {
    layouts: BTreeMap<Ty, Layout>,
    /// For each program type, its boxed fields: (alternative, field) positions.
    boxed: BTreeMap<usize, BTreeSet<(usize, usize)>>,
}

impl Layouts {
    /// `items` are the program's definitions, in order.
    pub fn new(types: &mut Types, items: &[&Item]) -> Layouts {
        let boxed = (items.iter().enumerate())
            .map(|(i, item)| (i, boxed_fields(types, items, i, item)))
            .collect();
        let mut layouts = Layouts {
            layouts: BTreeMap::new(),
            boxed,
        };
        for ty in types.all() {
            layouts.lay(types, items, ty);
        }
        layouts
    }

    pub fn get(&self, ty: Ty) -> Option<&Layout> {
        self.layouts.get(&ty)
    }

    /// Lays out `ty` and what it holds by value; its size in words.
    fn lay(&mut self, types: &mut Types, items: &[&Item], ty: Ty) -> Option<usize> {
        if let Some(l) = self.layouts.get(&ty) {
            return Some(l.size / 8);
        }
        let leaf = |words, owned: Vec<Entry>| (false, words, vec![], vec![owned]);
        let at0 = |kind| vec![Entry { offset: 0, kind }];
        let (tagged, words, alternatives, owned) = match types.node(ty).clone() {
            Node::Var(_) => return None,
            Node::Fn(..) | Node::Name("Int" | "Float" | "Bool", _) => leaf(1, vec![]),
            Node::Name("String", _) => leaf(4, at0(Owned::String)),
            Node::Name("List", args) => leaf(3, at0(Owned::List(args[0]))),
            Node::Name("Map", args) => leaf(1, at0(Owned::Map(args[0], args[1]))),
            node => {
                let (tagged, alts) = alternatives(types, items, &node);
                let start = if tagged { 8 } else { 0 };
                let mut places = vec![];
                for (a, fields) in alts.iter().enumerate() {
                    let mut offset = start;
                    let mut alt = vec![];
                    for (f, &(ty, boxed)) in fields.iter().enumerate() {
                        let boxed = boxed || self.is_boxed(&node, a, f);
                        alt.push(Place { offset, ty, boxed });
                        offset += if boxed {
                            8
                        } else {
                            self.lay(types, items, ty)? * 8
                        };
                    }
                    places.push(alt);
                }
                let end = |alt: &Vec<Place>| alt.last().map_or(start, |p| p.offset + self.width(p));
                let words = places.iter().map(end).max().unwrap_or(start) / 8;
                let owned = self.owned(tagged, &places);
                (tagged, words, places, owned)
            }
        };
        let layout = Layout {
            size: words * 8,
            tagged,
            alternatives,
            owned,
        };
        self.layouts.insert(ty, layout);
        Some(words)
    }

    fn is_boxed(&self, node: &Node, alternative: usize, field: usize) -> bool {
        let Node::User(i) = node else { return false };
        self.boxed[i].contains(&(alternative, field))
    }

    /// The bytes `place` takes; its type is laid out.
    fn width(&self, place: &Place) -> usize {
        if place.boxed {
            8
        } else {
            self.layouts[&place.ty].size
        }
    }

    fn owned(&self, tagged: bool, places: &[Vec<Place>]) -> Vec<Vec<Entry>> {
        let entries = |alt: &Vec<Place>| alt.iter().flat_map(|p| self.entries(p)).collect();
        let owned: Vec<Vec<Entry>> = places.iter().map(entries).collect();
        match tagged {
            true => owned,
            false => vec![owned.into_iter().flatten().collect()],
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

fn alternatives(types: &mut Types, items: &[&Item], node: &Node) -> Alternatives {
    let plain = |tys: &[Ty]| tys.iter().map(|&t| (t, false)).collect();
    match node {
        Node::User(i) => match items[*i] {
            Item::Record(fields) => (false, vec![fields.iter().map(|f| (f.ty, false)).collect()]),
            Item::Enum(variants) => {
                let fields =
                    |v: &crate::sig::Variant| v.fields.iter().map(|f| (f.ty, false)).collect();
                (true, variants.iter().map(fields).collect())
            }
            _ => unreachable!("a type definition"),
        },
        Node::Name("Option", args) => (true, vec![plain(args), vec![]]),
        Node::Name("Result", args) => (true, vec![plain(&args[..1]), plain(&args[1..])]),
        Node::Name("Effect", args) => {
            let msg = args[0];
            let string = types.named("String", &[]);
            let int = types.named("Int", &[]);
            let answer = types.intern(Node::Fn([string].into(), Some(msg)));
            let s = (string, false);
            let f = (answer, false);
            (
                true,
                vec![
                    vec![s],
                    vec![s, f, f],
                    vec![s, s, (msg, true), f],
                    vec![],
                    vec![(int, false)],
                ],
            )
        }
        node => unreachable!("no alternatives in {node:?}"),
    }
}

/// The fields of program type `i` whose types reach it by value.
fn boxed_fields(types: &Types, items: &[&Item], i: usize, item: &Item) -> BTreeSet<(usize, usize)> {
    let alts: Vec<Vec<Ty>> = match item {
        Item::Record(fields) => vec![fields.iter().map(|f| f.ty).collect()],
        Item::Enum(variants) => (variants.iter())
            .map(|v| v.fields.iter().map(|f| f.ty).collect())
            .collect(),
        _ => return BTreeSet::new(),
    };
    let mut boxed = BTreeSet::new();
    for (a, fields) in alts.iter().enumerate() {
        for (f, &ty) in fields.iter().enumerate() {
            if reaches(types, items, ty, i, &mut BTreeSet::new()) {
                boxed.insert((a, f));
            }
        }
    }
    boxed
}

/// Whether a value of `ty` holds program type `target` by value.
fn reaches(types: &Types, items: &[&Item], ty: Ty, target: usize, seen: &mut BTreeSet<Ty>) -> bool {
    if !seen.insert(ty) {
        return false;
    }
    match types.node(ty) {
        Node::User(i) if *i == target => true,
        Node::User(i) => {
            (items[*i].each_field()).any(|f| reaches(types, items, f.ty, target, seen))
        }
        Node::Name("Option" | "Result", args) => {
            (args.iter()).any(|&a| reaches(types, items, a, target, seen))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
