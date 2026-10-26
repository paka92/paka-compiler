//! Views in release (M6-18a): each node goes to the runtime through `paka_rt1_view` as it is
//! made, in the ops of the dev back end (paka_rt::abi::dev::view), so both write one tree layout.
//! A view's value is a word of no meaning. A placeholder is never lowered.

use super::{Body, Module, Value};
use crate::core_form::{Block, Child, Expr, ExprKind, FnRef, Site, Ty, Type};
use paka::builtins::ELEMENTS;
use paka_rt::abi::dev::view::*;
use paka_rt::abi::table::VIEW;

impl Body<'_, '_> {
    pub(super) fn view(&mut self, e: &Expr) -> Option<Value> {
        match &e.kind {
            ExprKind::Root(lines) => {
                self.view_op(OPEN, &[]);
                self.scoped(&[], lines)?;
                self.view_op(ROOT, &[]);
            }
            ExprKind::Element(row, args, lines, site) => {
                self.element(*row, args, lines.as_ref(), *site)?
            }
            ExprKind::Child(c) => {
                self.child(c)?;
                return Some(None);
            }
            _ => return None,
        }
        Some(Some("0".into()))
    }

    /// Its arguments first; a Row or Col traps `assert` on a negative gap before its children.
    fn element(
        &mut self,
        row: usize,
        args: &[Expr],
        lines: Option<&Block>,
        site: Site,
    ) -> Option<()> {
        let mut x = vec![];
        for a in args {
            x.push(self.expr(a)??);
        }
        match ELEMENTS[row].name {
            "Text" => self.view_op(TEXT, &[("ptr", x[0].clone())]),
            name @ ("Button" | "Input") => {
                let t = args[1].ty?;
                let message = match self.ty(t) {
                    "ptr" => x[1].clone(),
                    ty => {
                        let at = self.name("%m");
                        self.slot(&at, "i64");
                        self.store("i64", "0", &at);
                        self.store(ty, &x[1], &at);
                        at
                    }
                };
                let record = match self.module.specs[t] {
                    Some(_) => self.module.record(t),
                    None => "null".into(),
                };
                let op = if name == "Button" { BUTTON } else { INPUT };
                let (label, size) = (x[0].clone(), self.size(t).to_string());
                let operands = [("ptr", label), ("ptr", message), ("ptr", record)];
                let typed = [("i64", size), ("i64", t.to_string())];
                self.view_op(op, &[&operands[..], &typed[..]].concat());
            }
            name => {
                let bad = self.op(format!("icmp slt i64 {}, 0", x[0]));
                self.check(&bad, "assert", site);
                self.view_op(OPEN, &[]);
                self.scoped(&[], lines?)?;
                let op = if name == "Row" { ROW } else { COL };
                self.view_op(op, &[("i64", x[0].clone())]);
            }
        }
        Some(())
    }

    /// The node, then its key.
    fn child(&mut self, c: &Child) -> Option<()> {
        self.expr(&c.node)?;
        let (kind, key) = match &c.key {
            Some(k) => {
                let string = self.module.core.types[k.ty?] == Type::String;
                let kind = if string { STRING_KEY } else { INT_KEY };
                (kind, (self.ty(k.ty?), self.expr(k)??))
            }
            None => (NO_KEY, ("i64", "0".into())),
        };
        let words = [c.line as u64, u64::from(c.looped), kind].map(|n| ("i64", n.to_string()));
        self.view_op(CHILD, &[&words[..], &[key]].concat());
        Some(())
    }

    /// `paka_rt1_view(op, operands...)`, an address passed as its integer, unused operands 0.
    fn view_op(&mut self, op: u64, operands: &[(&str, String)]) {
        self.declare(VIEW);
        let mut words = vec![format!("i64 {op}")];
        for (ty, v) in operands {
            words.push(match *ty {
                "ptr" => format!("i64 {}", self.op(format!("ptrtoint ptr {v} to i64"))),
                _ => format!("i64 {v}"),
            });
        }
        words.resize(6, "i64 0".into());
        let status = self.op(format!("call i32 @paka_rt1_view({})", words.join(", ")));
        self.pass(&status);
    }
}

impl Module<'_> {
    /// What the runtime shows a message by (ABI section 14): the names of the function values by
    /// id, then a line for each type. Call it once every function is lowered.
    pub(super) fn shapes(&self) -> String {
        let core = self.core;
        let names = self.fn_values.iter().map(|(f, _)| match f {
            FnRef::Fn(id) => core.fns[*id].name.as_str(),
            FnRef::Variant(e, alt) => core.enums[*e].variants[*alt].name.as_str(),
        });
        let mut text = names.collect::<Vec<_>>().join(" ");
        for t in 0..core.types.len() {
            text.push('\n');
            text += &self.shape(t);
        }
        text
    }

    /// Type `t`'s line; empty for one no message holds.
    fn shape(&self, t: Ty) -> String {
        let core = self.core;
        let size = |t: Ty| self.layouts.get(t).size;
        // Each alternative: its name, then each field's place, a record's labelled.
        let alts = |names: &[&str], labels: Option<Vec<&str>>| {
            let alts = names.iter().zip(&self.layouts.get(t).alternatives);
            let alts = alts.map(|(name, places)| {
                let places = places.iter().enumerate().map(|(i, p)| {
                    let label = labels
                        .as_ref()
                        .map_or(String::new(), |l| format!("{}=", l[i]));
                    let boxed = if p.boxed { "*" } else { "" };
                    format!(" {label}{}@{}{boxed}", p.ty, p.offset)
                });
                format!("{name}{}", places.collect::<String>())
            });
            alts.collect::<Vec<_>>().join("|")
        };
        match &core.types[t] {
            Type::Int => "I".into(),
            Type::Float => "F".into(),
            Type::Bool => "B".into(),
            Type::String => "S".into(),
            Type::Fn(..) => "N".into(),
            Type::List(item) => format!("L {item} {}", size(*item)),
            Type::Map(k, v) => format!("M {k} {v} {} {}", size(*k), size(*v)),
            Type::Record(r) => {
                let r = &core.records[*r];
                let labels = r.fields.iter().map(|f| f.name.as_str()).collect();
                format!("R {}", alts(&[&r.name], Some(labels)))
            }
            Type::Enum(e) => {
                let names = core.enums[*e].variants.iter().map(|v| v.name.as_str());
                format!("V {}", alts(&names.collect::<Vec<_>>(), None))
            }
            Type::Option(_) => format!("V {}", alts(&["Some", "None"], None)),
            Type::Result(..) => format!("V {}", alts(&["Ok", "Err"], None)),
            _ => String::new(),
        }
    }
}
