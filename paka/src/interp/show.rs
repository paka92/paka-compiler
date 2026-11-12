//! The value text form of the tools (M2-09 section 1.6, M2-14 section 1.16): a value as written in
//! source. The type is only needed to name records and variants and to reach their fields' types.

use super::value::{FnKey, Value, position, quoted};
use crate::core_form::{Effect, Program, Ty, Type};

/// Marks a variant whose fields are all bare.
const BARE: usize = usize::MAX;
/// The `Effect` variants in declared order, each with the position of its first labelled field.
const EFFECTS: [(&str, usize); 5] = [
    ("Print", BARE),
    ("ReadFile", 1),
    ("WriteFile", 2),
    ("Exit", BARE),
    ("ExitWith", BARE),
];
const LABELS: [&str; 2] = ["then", "or"];

pub fn show(core: &Program, ty: Ty, v: &Value) -> String {
    let items = |t: Ty, vs: &[Value]| vs.iter().map(|v| show(core, t, v)).collect::<Vec<_>>();
    let variant = |name: &str, tys: &[Ty], vs: &[Value], labelled: usize| {
        if vs.is_empty() {
            return name.to_string();
        }
        let parts = tys.iter().zip(vs).enumerate().map(|(i, (&t, v))| {
            let label = i.checked_sub(labelled).map_or("", |l| LABELS[l]);
            let sep = if label.is_empty() { "" } else { ": " };
            format!("{label}{sep}{}", show(core, t, v))
        });
        format!("{name}({})", parts.collect::<Vec<_>>().join(", "))
    };
    match (&core.types[ty], v) {
        (_, Value::Int(n)) => n.to_string(),
        (_, Value::Float(x)) => x.show(),
        (_, Value::Bool(b)) => b.to_string(),
        (_, Value::Str(t)) => quoted(t.as_str()),
        (_, Value::Fn(FnKey::Fn(f))) => core.fns[*f].name.clone(),
        (_, Value::Fn(FnKey::Ctor(e, n))) => core.enums[*e].variants[*n].name.clone(),
        (Type::List(t), Value::List(vs)) => format!("[{}]", items(*t, &vs.0).join(", ")),
        (Type::Map(..), Value::Map(m)) if m.0.is_empty() => "[:]".to_string(),
        (Type::Map(_, t), Value::Map(m)) => {
            let pairs =
                m.0.iter()
                    .map(|(k, v)| format!("{}: {}", k.show(), show(core, *t, v)));
            format!("[{}]", pairs.collect::<Vec<_>>().join(", "))
        }
        (Type::Record(r), Value::Record(vs)) => {
            let record = &core.records[*r];
            let fields = record.fields.iter().zip(&vs.0);
            let fields = fields.map(|(f, v)| format!("{}: {}", f.name, show(core, f.ty, v)));
            let fields = fields.collect::<Vec<_>>().join(", ");
            let gap = if fields.is_empty() { "" } else { " " };
            format!("{} {{{gap}{fields}{gap}}}", record.name)
        }
        (Type::Enum(e), Value::Variant(n, vs)) => {
            let var = &core.enums[*e].variants[*n];
            let tys: Vec<_> = var.fields.iter().map(|f| f.ty).collect();
            variant(&var.name, &tys, &vs.0, BARE)
        }
        (Type::Option(t), Value::Variant(n, vs)) => {
            variant(["None", "Some"][*n], &[*t], &vs.0, BARE)
        }
        (Type::Result(t, e), Value::Variant(n, vs)) => {
            variant(["Ok", "Err"][*n], &[[*t, *e][*n]], &vs.0, BARE)
        }
        (Type::Effect(m), Value::Variant(n, vs)) if *n == Effect::Foreign as usize => {
            let Some((Value::Int(id), vs)) = vs.0.split_first() else {
                unreachable!("a foreign effect starts with its index");
            };
            let name = &core.foreign[position(*id).expect("an effect index")].name;
            variant(name, &vec![*m; vs.len()], vs, vs.len() - LABELS.len())
        }
        (Type::Effect(m), Value::Variant(n, vs)) => {
            let (name, labelled) = EFFECTS[*n];
            variant(name, &vec![*m; vs.0.len()], &vs.0, labelled)
        }
        _ => unreachable!("a value and its checked type"),
    }
}
