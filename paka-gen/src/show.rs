//! A text function per printed type, `s{i}(x: T) -> String`, written once.

use crate::Gen;
use crate::ty::Ty;

impl Gen {
    /// The name of the text function of `ty`, written on first use.
    pub(crate) fn show(&mut self, ty: &Ty) -> String {
        if let Some(i) = self.shows.get(ty) {
            return format!("s{i}");
        }
        let body = match ty {
            Ty::Int | Ty::Bool => "\"{x}\"".to_string(),
            Ty::Float => "fixed(x, digits: 17)".into(),
            Ty::Str => r#""\"{x}\"""#.into(),
            Ty::Fn(..) => r#""fn""#.into(),
            Ty::List(t) => {
                let s = self.show(t);
                format!(
                    "var out = \"[\"\n  for item in x {{\n    out += {s}(item) + \", \"\n  }}\n  out + \"]\""
                )
            }
            Ty::Map(k, v) => {
                let (sk, sv) = (self.show(k), self.show(v));
                format!(
                    "var out = \"[\"\n  for k in keys(x) {{\n    out += {sk}(k) + \": \" + {sv}(x[k]) + \", \"\n  }}\n  out + \"]\""
                )
            }
            Ty::Opt(t) => {
                let s = self.show(t);
                format!(
                    "match x {{\n    Some(v) => \"Some(\" + {s}(v) + \")\"\n    None => \"None\"\n  }}"
                )
            }
            Ty::Res(t, e) => {
                let (st, se) = (self.show(t), self.show(e));
                format!(
                    "match x {{\n    Ok(v) => \"Ok(\" + {st}(v) + \")\"\n    Err(e) => \"Err(\" + {se}(e) + \")\"\n  }}"
                )
            }
            Ty::Rec(i) => {
                let fields = self.records[*i].fields.clone();
                let parts: Vec<_> = (fields.iter().enumerate())
                    .map(|(j, f)| format!("{}(x.x{j})", self.show(&f.0)))
                    .collect();
                format!("\"R{i}\\{{\" + {} + \"\\}}\"", parts.join(" + \", \" + "))
            }
            Ty::Enum(i) => {
                let mut arms = String::new();
                for (j, fields) in self.enums[*i].variants.clone().iter().enumerate() {
                    if fields.is_empty() {
                        arms += &format!("    V{i}_{j} => \"V{i}_{j}\"\n");
                        continue;
                    }
                    let names: Vec<_> = (0..fields.len()).map(|k| format!("y{k}")).collect();
                    let parts: Vec<_> = (fields.iter().zip(&names))
                        .map(|(t, n)| format!("{}({n})", self.show(t)))
                        .collect();
                    let text = parts.join(" + \", \" + ");
                    arms += &format!(
                        "    V{i}_{j}({}) => \"V{i}_{j}(\" + {text} + \")\"\n",
                        names.join(", ")
                    );
                }
                format!("match x {{\n{arms}  }}")
            }
        };
        let i = self.shows.len();
        self.shows.insert(ty.clone(), i);
        self.show_text += &format!("fn s{i}(x: {ty}) -> String {{\n  {body}\n}}\n\n");
        format!("s{i}")
    }
}
