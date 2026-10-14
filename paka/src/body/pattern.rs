//! Patterns: what each one matches, the type it needs and the names it binds.

use super::call::Bound;
use super::{Cx, How, Local, Res, span};
use crate::ast::{Pat, PatKind};
use crate::diagnostic::Diagnostic;
use crate::resolve::Binding;
use crate::sig::{Item, Node, Ty};
use paka_rt::ops::Int;
use std::collections::BTreeSet;

/// The values a `match` subject is matched against.
#[derive(Clone, Copy)]
pub(super) enum Space {
    /// A type not yet known: nothing is checked against it.
    Unknown,
    /// Int or String: literals, and `_` for the rest.
    Open(&'static str),
    Bool,
    /// An enum of the program, by definition.
    User(usize),
    /// Option or Result, by variant name.
    Builtin(&'static [&'static str]),
}

impl Space {
    fn is_enum(self) -> bool {
        matches!(self, Space::User(_) | Space::Builtin(_))
    }
}

/// What a pattern matches; a Bool is the variant `false` or `true`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key<'p> {
    Wild,
    Variant(usize),
    Int(Int),
    Str(&'p str),
}

pub(super) type Seen<'p> = BTreeSet<Key<'p>>;

impl<'a> Cx<'_, 'a> {
    /// The space of a subject of type `ty`; none when it cannot be matched.
    pub(super) fn space(&self, ty: Ty) -> Option<Space> {
        Some(match self.types.node(ty) {
            Node::Var(_) => Space::Unknown,
            Node::User(i) if matches!(self.defs[*i].item, Item::Enum(_)) => Space::User(*i),
            Node::Name("Int", _) => Space::Open("Int"),
            Node::Name("String", _) => Space::Open("String"),
            Node::Name("Bool", _) => Space::Bool,
            Node::Name("Option", _) => Space::Builtin(&["Some", "None"]),
            Node::Name("Result", _) => Space::Builtin(&["Ok", "Err"]),
            _ => return None,
        })
    }

    /// The variants of the space in declaration order, each with its number of fields.
    pub(super) fn names(&self, sp: Space) -> Vec<(&'a str, usize)> {
        match sp {
            Space::Bool => vec![("false", 0), ("true", 0)],
            Space::Builtin(names) => (names.iter())
                .map(|n| (*n, self.variants[n].params.len()))
                .collect(),
            Space::User(i) => match &self.defs[i].item {
                Item::Enum(variants) => (variants.iter())
                    .map(|v| (v.name, v.fields.len()))
                    .collect(),
                _ => vec![],
            },
            _ => vec![],
        }
    }

    /// What `p` matches in a subject of type `ty`; none when that is not known.
    pub(super) fn pat_key<'p>(&self, sp: Space, ty: Ty, p: &'p Pat) -> Res<Option<Key<'p>>> {
        let wrong = |found: &str| Err(self.mismatch(span(p), &self.types.show(ty), found, ""));
        match (&p.kind, sp) {
            (_, Space::Unknown) | (PatKind::Bind(_), _) => Ok(None),
            (PatKind::Wild, _) => Ok(Some(Key::Wild)),
            (PatKind::Int(text), Space::Open("Int")) => match Int::parse(text) {
                Some(n) => Ok(Some(Key::Int(n))),
                None => Err(self.out_of_range(span(p), "Int", text)),
            },
            (PatKind::Str(text), Space::Open("String")) => Ok(Some(Key::Str(text))),
            (PatKind::Bool(b), Space::Bool) => Ok(Some(Key::Variant(usize::from(*b)))),
            (PatKind::Variant(..), _) => self.variant_key(sp, ty, p),
            (PatKind::Int(_), _) => wrong("Int"),
            (PatKind::Str(_), _) => wrong("String"),
            (PatKind::Bool(_), _) => wrong("Bool"),
        }
    }

    fn variant_key<'p>(&self, sp: Space, ty: Ty, p: &Pat) -> Res<Option<Key<'p>>> {
        let Some(binding) = self.resolved.get(p.at) else {
            return Ok(None);
        };
        let index = match (binding, sp) {
            (Binding::Variant(d, j), Space::User(i)) if d == i => Some(j),
            (Binding::Builtin(b), Space::Builtin(names)) => names.iter().position(|n| *n == b),
            _ => None,
        };
        if let Some(j) = index {
            return Ok(Some(Key::Variant(j)));
        }
        let owner = match binding {
            Binding::Variant(d, _) => self.program.defs[d].name().text,
            Binding::Builtin(b) if self.variants.contains_key(b) => {
                match self.variants[b].result.map(|t| self.types.node(t)) {
                    Some(Node::Name(n, _)) => n,
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };
        Err(self.mismatch(span(p), &self.types.show(ty), owner, ""))
    }

    /// Checks `p`, whose key is `key`, against the subject and the patterns before it, and binds
    /// its names.
    pub(super) fn pattern<'p>(
        &mut self,
        sp: Space,
        ty: Ty,
        p: &'p Pat,
        key: Option<Key<'p>>,
        seen: &mut Seen<'p>,
    ) -> Res {
        if matches!(key, Some(Key::Wild)) && sp.is_enum() {
            let shown = self.types.show(ty);
            let head = shown.split('<').next().unwrap_or_default();
            let message = format!("_ is for Int, String and Bool; name each variant of {head}");
            return Err(self.fail(span(p), "wildcard-on-enum", message));
        }
        let mut fields = None;
        if let (PatKind::Variant(name, parts), Some(Key::Variant(j))) = (&p.kind, key) {
            let f = self.fields(sp, ty, name, j);
            if f.len() != parts.len() {
                return Err(self.arity(p, name, &f));
            }
            fields = Some(f);
        }
        if let Some(k) = key {
            if redundant(sp, &k, seen) {
                let message = format!("{} is already matched above; drop it", spelled(p));
                return Err(self.fail(span(p), "duplicate-arm", message));
            }
            seen.insert(k);
        }
        if let PatKind::Variant(_, parts) = &p.kind {
            for (i, part) in parts.iter().enumerate() {
                if let PatKind::Bind(_) = part.kind {
                    let ty = fields.as_ref().map_or_else(|| self.unknown(), |f| f[i].1);
                    let how = How::Pattern;
                    self.locals.insert(part.at, Local { ty, how });
                }
            }
        }
        Ok(())
    }

    /// The names and types of the fields of variant `j` of the subject's type.
    fn fields(&mut self, sp: Space, ty: Ty, name: &str, j: usize) -> Vec<(&'a str, Ty)> {
        let sig = match sp {
            Space::User(i) => self.variant_sig(i, j),
            _ => self.variants[name].clone(),
        };
        let mut bound = Bound::new();
        if let Some(result) = sig.result {
            self.bind(result, ty, &mut bound);
        }
        let mut fields = Vec::new();
        for p in &sig.params {
            let t = self.subst(p.ty, &bound);
            fields.push((p.name, if self.free(t) { self.unknown() } else { t }));
        }
        fields
    }

    fn arity(&self, p: &Pat, name: &str, fields: &[(&str, Ty)]) -> Diagnostic {
        let n = fields.len();
        let names = fields.iter().map(|f| f.0).collect::<Vec<_>>().join(", ");
        let blanks = vec!["_"; n].join(", ");
        let message = match n {
            0 => format!("{name} has no fields: write {name}"),
            1 => format!("{name} has 1 field ({names}): write {name}({blanks})"),
            _ => format!("{name} has {n} fields ({names}): write {name}({blanks})"),
        };
        self.fail((p.at, name.len()), "wrong-argument-count", message)
    }
}

/// Whether an arm above has matched everything `k` matches.
fn redundant(sp: Space, k: &Key, seen: &Seen) -> bool {
    let both = || seen.contains(&Key::Variant(0)) && seen.contains(&Key::Variant(1));
    seen.contains(&Key::Wild)
        || seen.contains(k)
        || (matches!(sp, Space::Bool) && matches!(k, Key::Wild) && both())
}

/// `p` as it is written.
fn spelled(p: &Pat) -> String {
    match &p.kind {
        PatKind::Wild => "_".to_string(),
        PatKind::Int(text) => text.to_string(),
        PatKind::Str(text) => format!("{text:?}"),
        PatKind::Bool(b) => b.to_string(),
        PatKind::Bind(name) | PatKind::Variant(name, _) => name.to_string(),
    }
}
