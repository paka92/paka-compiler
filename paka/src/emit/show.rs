//! The value text form (M2-09 section 1.6) of a value compiled code holds, as the interpreter's
//! `show` gives it: what a drawn message prints (M5-24). The words are read through `read`, which
//! fills its buffer from an address.

use super::constructors;
use crate::core_form::{Program, Ty, Type};
use crate::layout::{Layouts, Place};
use paka_rt::ops::{Float, Int, quoted};

/// The `Effect` variants in declared order, each with the position of its first labelled field.
const EFFECTS: [(&str, usize); 5] = [
    ("Print", usize::MAX),
    ("ReadFile", 1),
    ("WriteFile", 2),
    ("Exit", usize::MAX),
    ("ExitWith", usize::MAX),
];
const LABELS: [&str; 2] = ["then", "or"];

/// The text of the value of type `t` at `at`.
pub fn show(p: &Program, l: &Layouts, t: Ty, at: u64, read: &dyn Fn(u64, &mut [u8])) -> String {
    Shown { p, l, read }.value(t, at)
}

struct Shown<'a> {
    p: &'a Program,
    l: &'a Layouts,
    read: &'a dyn Fn(u64, &mut [u8]),
}

impl Shown<'_> {
    fn word(&self, at: u64) -> u64 {
        let mut word = [0; 8];
        (self.read)(at, &mut word);
        u64::from_le_bytes(word)
    }

    fn value(&self, t: Ty, at: u64) -> String {
        let p = self.p;
        match p.types[t] {
            Type::Int => Int::new(self.word(at) as i64).to_string(),
            Type::Float => Float::from_bits(self.word(at)).show(),
            Type::Bool => (self.word(at) != 0).to_string(),
            Type::String => quoted(&self.text(at)),
            Type::Fn(..) => {
                let id = self.word(at) as usize;
                match p.fns.get(id) {
                    Some(f) => f.name.clone(),
                    None => {
                        let (e, v) = constructors(p)[id - p.fns.len() - p.examples.len()];
                        p.enums[e].variants[v].name.clone()
                    }
                }
            }
            Type::List(item) => {
                let (items, len) = (self.word(at), self.word(at + 8));
                let size = self.l.get(item).size as u64;
                let shown: Vec<_> = (0..len)
                    .map(|i| self.value(item, items + i * size))
                    .collect();
                format!("[{}]", shown.join(", "))
            }
            Type::Map(k, v) => {
                let block = self.word(at);
                let len = if block == 0 { 0 } else { self.word(block) };
                if len == 0 {
                    return "[:]".into();
                }
                let key = self.l.get(k).size as u64;
                let entry = key + self.l.get(v).size as u64;
                let shown: Vec<_> = (0..len)
                    .map(|i| {
                        let at = block + 16 + i * entry;
                        format!("{}: {}", self.value(k, at), self.value(v, at + key))
                    })
                    .collect();
                format!("[{}]", shown.join(", "))
            }
            Type::Record(r) => {
                let record = &p.records[r];
                let places = &self.l.get(t).alternatives[0];
                let fields: Vec<_> = (record.fields.iter().zip(places))
                    .map(|(f, place)| format!("{}: {}", f.name, self.field(place, at)))
                    .collect();
                let gap = if fields.is_empty() { "" } else { " " };
                format!("{} {{{gap}{}{gap}}}", record.name, fields.join(", "))
            }
            Type::Enum(_) | Type::Option(_) | Type::Result(..) | Type::Effect(_) => {
                let tag = self.word(at) as usize;
                let (name, labelled) = match p.types[t] {
                    Type::Enum(e) => (p.enums[e].variants[tag].name.as_str(), usize::MAX),
                    Type::Option(_) => (["None", "Some"][tag], usize::MAX),
                    Type::Result(..) => (["Ok", "Err"][tag], usize::MAX),
                    _ => EFFECTS.get(tag).copied().unwrap_or_else(|| {
                        let fields = self.l.get(t).alternatives[tag].len();
                        let foreign = &p.foreign[tag - EFFECTS.len()];
                        (foreign.name.as_str(), fields - LABELS.len())
                    }),
                };
                let places = &self.l.get(t).alternatives[tag];
                if places.is_empty() {
                    return name.into();
                }
                let parts: Vec<_> = (places.iter().enumerate())
                    .map(|(i, place)| {
                        let label = i.checked_sub(labelled).map_or("", |l| LABELS[l]);
                        let sep = if label.is_empty() { "" } else { ": " };
                        format!("{label}{sep}{}", self.field(place, at))
                    })
                    .collect();
                format!("{name}({})", parts.join(", "))
            }
            _ => unreachable!("a value a message holds"),
        }
    }

    fn field(&self, place: &Place, at: u64) -> String {
        let at = at + place.offset as u64;
        let at = if place.boxed { self.word(at) } else { at };
        self.value(place.ty, at)
    }

    fn text(&self, at: u64) -> String {
        let mut bytes = vec![0; self.word(at + 8) as usize];
        if !bytes.is_empty() {
            (self.read)(self.word(at), &mut bytes);
        }
        String::from_utf8(bytes).expect("a String's bytes are UTF-8")
    }
}
