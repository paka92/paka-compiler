//! A drawn message's text in release (ABI section 14): the value text form (M2-09 section 1.6),
//! as dev code shows it, read by the shapes compiled code gives. Line 0 names the function values
//! by id; line `1 + t` is type `t`: `I`, `F`, `B`, `S`, `N` for a function value, `L item size`,
//! `M key value key_size value_size`, `R Name field...` or `V alternative|...`, an alternative
//! being `Name field...`. A field is `[label=]type@offset`, `*` after it when it is boxed.

use super::dev::words::as_str;
use crate::ops::{Float, Int, quoted};
use std::ptr;

pub(crate) struct Shapes<'a> {
    names: Vec<&'a str>,
    types: Vec<&'a str>,
}

impl<'a> Shapes<'a> {
    pub(crate) fn new(text: &'a str) -> Shapes<'a> {
        let mut lines = text.split('\n');
        let names = lines.next().unwrap_or_default().split(' ').collect();
        Shapes {
            names,
            types: lines.collect(),
        }
    }

    /// A drawing's `show`: the text of the message of type `t` whose copy is at `at`. Only
    /// `view::draw` calls it, with the type and address of a message it holds.
    pub(crate) fn show(&self, t: u64, at: u64) -> String {
        // SAFETY: the drawing copied a value of type `t` to `at`, as compiled code laid it out.
        unsafe { self.value(t as usize, at) }
    }

    /// The text of the value of type `t` at `at`.
    ///
    /// # Safety
    ///
    /// `at` holds a value of type `t`, laid out as its shape says.
    unsafe fn value(&self, t: usize, at: u64) -> String {
        let line = self.types.get(t).copied().unwrap_or_default();
        let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
        let n: Vec<usize> = rest.split(' ').map(|n| n.parse().unwrap_or(0)).collect();
        // SAFETY: as the caller vouches; each address below is one the shape gives inside it.
        unsafe {
            match kind {
                "I" => Int::new(word(at) as i64).to_string(),
                "F" => Float::from_bits(word(at)).show(),
                "B" => (word(at) != 0).to_string(),
                "S" => quoted(as_str(at)),
                "N" => self.names[word(at) as usize].into(),
                "L" => {
                    let (items, len) = (word(at), word(at + 8));
                    let shown = (0..len).map(|i| self.value(n[0], items + i * n[1] as u64));
                    format!("[{}]", shown.collect::<Vec<_>>().join(", "))
                }
                "M" => {
                    let block = word(at);
                    let len = if block == 0 { 0 } else { word(block) };
                    if len == 0 {
                        return "[:]".into();
                    }
                    let (key, entry) = (n[2] as u64, (n[2] + n[3]) as u64);
                    let shown = (0..len).map(|i| {
                        let at = block + 16 + i * entry;
                        format!("{}: {}", self.value(n[0], at), self.value(n[1], at + key))
                    });
                    format!("[{}]", shown.collect::<Vec<_>>().join(", "))
                }
                "R" => {
                    let (name, fields) = rest.split_once(' ').unwrap_or((rest, ""));
                    let fields = self.fields(fields, at);
                    let gap = if fields.is_empty() { "" } else { " " };
                    format!("{name} {{{gap}{fields}{gap}}}")
                }
                "V" => {
                    let alt = rest.split('|').nth(word(at) as usize).unwrap_or_default();
                    match alt.split_once(' ') {
                        Some((name, fields)) => format!("{name}({})", self.fields(fields, at)),
                        None => alt.into(),
                    }
                }
                _ => unreachable!("a message's type has a shape"),
            }
        }
    }

    /// The fields of the value at `at`, each `label: text` or its text, joined by `, `.
    ///
    /// # Safety
    ///
    /// As for `value`, `places` being the fields of the value's shape.
    unsafe fn fields(&self, places: &str, at: u64) -> String {
        let shown = places.split(' ').filter(|p| !p.is_empty()).map(|place| {
            let (label, place) = match place.split_once('=') {
                Some((label, place)) => (format!("{label}: "), place),
                None => (String::new(), place),
            };
            let (place, boxed) = match place.strip_suffix('*') {
                Some(place) => (place, true),
                None => (place, false),
            };
            let (t, offset) = place.split_once('@').expect("a field's place");
            let [t, offset] = [t, offset].map(|n| n.parse::<usize>().expect("a number"));
            let at = at + offset as u64;
            // SAFETY: the field lies at its offset, or in the block its word points to.
            unsafe {
                let at = if boxed { word(at) } else { at };
                format!("{label}{}", self.value(t, at))
            }
        });
        shown.collect::<Vec<_>>().join(", ")
    }
}

/// # Safety
///
/// `at` holds a word.
unsafe fn word(at: u64) -> u64 {
    // SAFETY: as the caller vouches.
    unsafe { ptr::with_exposed_provenance::<u64>(at as usize).read() }
}
