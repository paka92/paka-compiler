//! UI calls (docs/decisions/M5-06.md section 2): `ui_start` takes a visual app, and a literal
//! path is well formed.

use super::{Cx, Res};
use crate::ast::{Arg, Def, Entry, ExprKind as K, StrPart};
use crate::resolve::Binding;
use paka_rt::tree;

impl Cx<'_, '_> {
    /// The checks of a UI call past its signature.
    pub(super) fn ui_call(&self, name: &str, args: &[Arg]) -> Res {
        match (name, args) {
            ("ui_start", [app]) => self.ui_start(app),
            ("ui_click" | "ui_text" | "ui_has", [_, path]) => self.ui_path(path),
            _ => Ok(()),
        }
    }

    fn ui_start(&self, app: &Arg) -> Res {
        let e = &app.value;
        let def = match (&e.kind, self.resolved.get(e.at)) {
            (K::Name(_), Some(Binding::Def(i))) => &self.program.defs[i],
            _ => {
                let message = "ui_start takes the bare name of a visual app";
                return Err(self.fail_at(e, "bad-ui-start", message));
            }
        };
        let Def::App { name, entries } = def else {
            unreachable!("the resolver keeps apps alone");
        };
        if entries.iter().any(|e| matches!(e, Entry::View(..))) {
            return Ok(());
        }
        let message = format!("{} has no view; ui_start takes a visual app", name.text);
        Err(self.fail_at(e, "bad-ui-start", message))
    }

    /// A literal path is well formed (M5-06 point 1.2).
    fn ui_path(&self, path: &Arg) -> Res {
        let K::Str(parts) = &path.value.kind else {
            return Ok(());
        };
        let text: Option<String> = (parts.iter())
            .map(|p| match p {
                StrPart::Text(t) => Some(t.as_str()),
                StrPart::Expr(_) => None,
            })
            .collect();
        match text {
            Some(text) if tree::path(&text).is_none() => {
                let message =
                    format!("{text:?} is not a UI path; join steps with /: 1, 'key', #1 or #1.0");
                Err(self.fail_at(&path.value, "bad-ui-path", message))
            }
            _ => Ok(()),
        }
    }
}
