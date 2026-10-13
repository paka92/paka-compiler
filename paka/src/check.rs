//! Name resolution and type checking (spec/m1.md sections 6 to 8). The first problem in the
//! spec's fixed order is the one reported.

use crate::ast::{Def, Entry, Expr, ExprKind, Name, Pos, Program, Type};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

type Check<T> = Result<T, Diagnostic>;

/// A program that passed, reduced to what running it needs.
#[derive(Debug)]
pub struct Checked<'a> {
    pub app: &'a str,
    pub start: &'a str,
    /// The body of the `update` function.
    pub update: &'a Expr<'a>,
    pub grants: Vec<&'a str>,
}

/// What a name in the pool stands for.
enum Item<'a> {
    /// A built-in type and its number of type arguments.
    Type(usize),
    Enum,
    /// A variant and the enum it belongs to.
    Variant(&'a str),
    /// An effect and its number of arguments.
    Effect(usize),
    Grant,
    Fn(Func<'a>),
    App,
}

#[derive(Clone, Copy)]
struct Func<'a> {
    param_type: &'a Type<'a>,
    result: &'a Type<'a>,
    body: &'a Expr<'a>,
}

struct Keys<'a> {
    start: &'a Name<'a>,
    update: &'a Name<'a>,
    grants: &'a [Name<'a>],
}

const STRING: Type = Type {
    name: Name {
        text: "String",
        at: (0, 0),
    },
    arg: None,
};

pub fn check<'a>(path: &'a str, program: &'a Program<'a>) -> Check<Checked<'a>> {
    let mut checker = Checker {
        path,
        pool: builtins(),
    };
    for def in &program.defs {
        checker.declare_def(def)?;
    }
    checker.single_app(&program.defs)?;
    let mut checked = None;
    for def in &program.defs {
        match def {
            Def::Enum { .. } => {}
            Def::Fn {
                param_type,
                result,
                body,
                ..
            } => checker.check_fn(Func {
                param_type,
                result,
                body,
            })?,
            Def::App { name, entries } => checked = Some(checker.check_app(name, entries)?),
        }
    }
    checked.ok_or_else(|| Diagnostic {
        path: path.into(),
        position: None,
        code: "no-app",
        message: "the program has no app".into(),
    })
}

fn builtins<'a>() -> BTreeMap<&'a str, Item<'a>> {
    BTreeMap::from([
        ("String", Item::Type(0)),
        ("List", Item::Type(1)),
        ("Effect", Item::Type(1)),
        ("Print", Item::Effect(1)),
        ("Exit", Item::Effect(0)),
        ("Console", Item::Grant),
    ])
}

/// The argument of `t` when it is the type `name` with one argument.
fn inner<'t, 'a>(t: &'t Type<'a>, name: &str) -> Option<&'t Type<'a>> {
    t.arg.as_deref().filter(|_| t.name.text == name)
}

fn same(a: &Type, b: &Type) -> bool {
    a.name.text == b.name.text
        && match (&a.arg, &b.arg) {
            (None, None) => true,
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        }
}

fn key(entry: &Entry) -> (Pos, &'static str) {
    match entry {
        Entry::Start(at, _) => (*at, "start"),
        Entry::Update(at, _) => (*at, "update"),
        Entry::Grants(at, _) => (*at, "grants"),
    }
}

struct Checker<'a> {
    path: &'a str,
    pool: BTreeMap<&'a str, Item<'a>>,
}

impl<'a> Checker<'a> {
    fn fail(&self, at: Pos, code: &'static str, message: String) -> Diagnostic {
        Diagnostic {
            path: self.path.into(),
            position: Some(at),
            code,
            message,
        }
    }

    fn declare(&mut self, name: &Name<'a>, item: Item<'a>) -> Check<()> {
        match self.pool.insert(name.text, item) {
            None => Ok(()),
            Some(_) => Err(self.fail(
                name.at,
                "duplicate-name",
                format!("`{}` is already declared", name.text),
            )),
        }
    }

    fn declare_def(&mut self, def: &'a Def<'a>) -> Check<()> {
        match def {
            Def::Enum { name, variants } => {
                self.declare(name, Item::Enum)?;
                variants
                    .iter()
                    .try_for_each(|v| self.declare(v, Item::Variant(name.text)))
            }
            Def::Fn {
                name,
                param_type,
                result,
                body,
                ..
            } => {
                let func = Func {
                    param_type,
                    result,
                    body,
                };
                self.declare(name, Item::Fn(func))
            }
            Def::App { name, .. } => self.declare(name, Item::App),
        }
    }

    fn single_app(&self, defs: &[Def]) -> Check<()> {
        let mut apps = defs.iter().filter_map(|d| match d {
            Def::App { name, .. } => Some(name),
            _ => None,
        });
        match apps.nth(1) {
            Some(second) => {
                Err(self.fail(second.at, "multiple-apps", "a program holds one app".into()))
            }
            None => Ok(()),
        }
    }

    /// Outside in: the name, then the argument count, then the argument.
    fn well_formed(&self, t: &Type) -> Check<()> {
        let arity = match self.pool.get(t.name.text) {
            Some(Item::Type(n)) => *n,
            Some(Item::Enum) => 0,
            _ => {
                let message = format!("`{}` is not a type", t.name.text);
                return Err(self.fail(t.name.at, "unknown-type", message));
            }
        };
        if arity != usize::from(t.arg.is_some()) {
            let message = format!("`{}` takes {arity} type arguments", t.name.text);
            return Err(self.fail(t.name.at, "wrong-type-arguments", message));
        }
        t.arg.as_deref().map_or(Ok(()), |a| self.well_formed(a))
    }

    fn check_fn(&self, f: Func) -> Check<()> {
        self.well_formed(f.param_type)?;
        self.well_formed(f.result)?;
        self.expr(f.body, f.result)
    }

    /// Checks `e` against the well formed type `want`.
    fn expr(&self, e: &Expr, want: &Type) -> Check<()> {
        let mismatch = |found: &str| {
            let message = format!("expected `{}`, found {found}", want.name.text);
            self.fail(e.at, "type-mismatch", message)
        };
        match &e.kind {
            ExprKind::Str(_) if want.name.text == "String" => Ok(()),
            ExprKind::Str(_) => Err(mismatch("a string")),
            ExprKind::List(items) => {
                let elem = inner(want, "List").ok_or_else(|| mismatch("a list"))?;
                items.iter().try_for_each(|i| self.expr(i, elem))
            }
            ExprKind::Apply(name, args) => {
                let (arity, fits) = match self.pool.get(name) {
                    Some(Item::Variant(en)) => (0, want.name.text == *en),
                    Some(Item::Effect(n)) => (*n, inner(want, "Effect").is_some()),
                    _ => {
                        let message = format!("`{name}` is not a variant or an effect");
                        return Err(self.fail(e.at, "unknown-name", message));
                    }
                };
                if args.len() != arity {
                    let message = format!("`{name}` takes {arity} arguments");
                    return Err(self.fail(e.at, "wrong-argument-count", message));
                }
                if !fits {
                    return Err(mismatch(&format!("`{name}`")));
                }
                args.iter().try_for_each(|a| self.expr(a, &STRING))
            }
        }
    }

    fn check_app(&self, app: &Name<'a>, entries: &'a [Entry<'a>]) -> Check<Checked<'a>> {
        let Keys {
            start,
            update,
            grants,
        } = self.keys(app, entries)?;
        let Some(Item::Fn(f)) = self.pool.get(update.text) else {
            let message = format!("`{}` is not a function", update.text);
            return Err(self.fail(update.at, "unknown-name", message));
        };
        let Some(message) = self.message_enum(f) else {
            let message = format!("`{}` is not `(x: M) -> List<Effect<M>>`", update.text);
            return Err(self.fail(update.at, "bad-update-signature", message));
        };
        self.check_start(start, message)?;
        self.check_grants(grants)?;
        Ok(Checked {
            app: app.text,
            start: start.text,
            update: f.body,
            grants: grants.iter().map(|g| g.text).collect(),
        })
    }

    /// Each key once, then each one present, in the spec's order.
    fn keys(&self, app: &Name, entries: &'a [Entry<'a>]) -> Check<Keys<'a>> {
        let (mut start, mut update, mut grants) = (None, None, None);
        for entry in entries {
            let repeated = match entry {
                Entry::Start(_, n) => start.replace(n).is_some(),
                Entry::Update(_, n) => update.replace(n).is_some(),
                Entry::Grants(_, g) => grants.replace(g.as_slice()).is_some(),
            };
            if repeated {
                let (at, key) = key(entry);
                let message = format!("`{key}` is written twice");
                return Err(self.fail(at, "duplicate-app-entry", message));
            }
        }
        let missing = |code, key| self.fail(app.at, code, format!("the app has no `{key}`"));
        Ok(Keys {
            start: start.ok_or_else(|| missing("missing-start", "start"))?,
            update: update.ok_or_else(|| missing("missing-update", "update"))?,
            grants: grants.ok_or_else(|| missing("missing-grants", "grants"))?,
        })
    }

    /// `M` when `f` is `(x: M) -> List<Effect<M>>` for an enum `M`, judged by the types as written.
    fn message_enum(&self, f: &Func<'a>) -> Option<&'a str> {
        let m = f.param_type;
        let is_enum = m.arg.is_none() && matches!(self.pool.get(m.name.text), Some(Item::Enum));
        let result = inner(f.result, "List").and_then(|t| inner(t, "Effect"))?;
        (is_enum && same(m, result)).then_some(m.name.text)
    }

    fn check_start(&self, start: &Name, message: &str) -> Check<()> {
        match self.pool.get(start.text) {
            Some(Item::Variant(en)) if *en == message => Ok(()),
            Some(Item::Variant(_) | Item::Effect(_)) => {
                let text = format!("`{}` is not a variant of `{message}`", start.text);
                Err(self.fail(start.at, "bad-start", text))
            }
            _ => {
                let text = format!("`{}` is not a variant or an effect", start.text);
                Err(self.fail(start.at, "unknown-name", text))
            }
        }
    }

    fn check_grants(&self, grants: &[Name]) -> Check<()> {
        for (i, grant) in grants.iter().enumerate() {
            if !matches!(self.pool.get(grant.text), Some(Item::Grant)) {
                let message = format!("`{}` is not a grant", grant.text);
                return Err(self.fail(grant.at, "unknown-grant", message));
            }
            if grants[..i].iter().any(|g| g.text == grant.text) {
                let message = format!("`{}` is listed twice", grant.text);
                return Err(self.fail(grant.at, "duplicate-grant", message));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, ExprKind};
    use crate::parser::parse;

    fn checked<'a>(program: &'a Program<'a>) -> Check<Checked<'a>> {
        check("main.paka", program)
    }

    fn expr(at: (usize, usize), kind: ExprKind) -> Expr {
        Expr { at, kind }
    }

    #[test]
    fn light_paka_checks() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/light.paka");
        let src = std::fs::read(path).unwrap();
        let program = parse("main.paka", &src).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!(light.app, "light");
        assert_eq!(light.start, "Start");
        assert_eq!(light.grants, ["Console"]);
        let print = expr(
            (4, 4),
            ExprKind::Apply(
                "Print",
                vec![expr((4, 10), ExprKind::Str("let there be light"))],
            ),
        );
        let exit = expr((4, 33), ExprKind::Apply("Exit", vec![]));
        let body = expr((4, 3), ExprKind::List(vec![print, exit]));
        assert_eq!(light.update, &body);
    }

    #[test]
    fn definitions_and_entries_may_come_in_any_order() {
        let src = "app light { grants: [] update: update start: Go }
            fn update(m: Msg) -> List<Effect<Msg>> { [] }
            enum Msg { Stop, Go }";
        let program = parse("main.paka", src.as_bytes()).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!((light.start, light.grants), ("Go", vec![]));
    }
}
