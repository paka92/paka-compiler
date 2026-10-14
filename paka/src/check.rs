//! Name resolution and type checking (spec/m1.md sections 6 to 8). The first problem in the
//! spec's fixed order is the one reported.

use crate::ast::{
    Arg, Block, Def, Entry, Expr, ExprKind, Name, Param, Pos, Program, StmtKind, StrPart, Type,
};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

type Check<T> = Result<T, Diagnostic>;

/// A program that passed, reduced to what running it needs.
#[derive(Debug)]
pub struct Checked<'a> {
    pub app: &'a str,
    pub start: &'a str,
    /// The name of the `update` function.
    pub update_name: &'a str,
    /// The body of the `update` function; lowering takes it from the function itself.
    #[allow(dead_code)]
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

struct Func<'a> {
    param_type: &'a Type<'a>,
    result: &'a Type<'a>,
    body: &'a Expr<'a>,
}

struct Keys<'a> {
    start: Name<'a>,
    update: &'a Name<'a>,
    grants: &'a [Name<'a>],
}

const STRING: Type = Type::Name {
    name: Name {
        text: "String",
        at: (0, 0),
    },
    args: Vec::new(),
};

pub fn check<'a>(path: &'a str, program: &'a Program<'a>) -> Check<Checked<'a>> {
    let mut checker = Checker {
        path,
        pool: builtins(),
    };
    for def in &program.defs {
        let declared = checker.declare_def(def);
        declared.map_err(|d| d.in_definition(def.name().text))?;
    }
    checker.single_app(&program.defs)?;
    let mut checked = None;
    for def in &program.defs {
        let result = checker.check_def(def);
        checked = result
            .map_err(|d| d.in_definition(def.name().text))?
            .or(checked);
    }
    checked.ok_or_else(|| Diagnostic::tool(path, "no-app", "the program has no app"))
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
    match t {
        Type::Name { name: n, args } if n.text == name && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}

/// The name of a type that passed `well_formed`.
fn head<'a>(t: &Type<'a>) -> &'a str {
    match t {
        Type::Name { name, .. } => name.text,
        Type::Fn { .. } => "fn",
    }
}

fn same(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Name { name: n, args: x }, Type::Name { name: m, args: y }) => {
            n.text == m.text && x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        _ => false,
    }
}

pub(crate) fn key(entry: &Entry) -> (Pos, &'static str) {
    match entry {
        Entry::State(at, _) => (*at, "state"),
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
    fn fail(&self, at: Pos, len: usize, code: &'static str, message: String) -> Diagnostic {
        Diagnostic::new(self.path, at, len, code, message)
    }

    fn unsupported_name(&self, name: &Name, what: &str) -> Diagnostic {
        self.unsupported(name.at, name.text.len(), what)
    }

    fn fail_at(&self, name: &Name, code: &'static str, message: String) -> Diagnostic {
        self.fail(name.at, name.text.len(), code, message)
    }

    /// A form that only a later milestone's checker handles, reported at `at`.
    fn unsupported(&self, at: Pos, len: usize, what: &str) -> Diagnostic {
        let message = format!("{what} is not supported yet");
        self.fail(at, len, "unsupported-form", message)
    }

    fn check_def(&self, def: &'a Def<'a>) -> Check<Option<Checked<'a>>> {
        match def {
            Def::Fn {
                name,
                params,
                result,
                body,
            } => {
                let func = self.func(name, params, result, body)?;
                self.check_fn(func).map(|()| None)
            }
            Def::App { name, entries } => self.check_app(name, entries).map(Some),
            Def::Enum { .. } | Def::Type { .. } | Def::Example { .. } => Ok(None),
        }
    }

    /// The only function shape of this checker: one plain parameter, a result and one expression.
    fn func(
        &self,
        name: &Name,
        params: &'a [Param<'a>],
        result: &'a Option<Type<'a>>,
        body: &'a Block<'a>,
    ) -> Check<Func<'a>> {
        match (params, result, body.stmts.as_slice()) {
            ([p], Some(result), [stmt]) if !p.inout && !p.labelled => match &stmt.kind {
                StmtKind::Expr(body) => Ok(Func {
                    param_type: &p.ty,
                    result,
                    body,
                }),
                _ => Err(self.unsupported(stmt.at, stmt.len, "a statement")),
            },
            _ => Err(self.unsupported(name.at, name.text.len(), "this function shape")),
        }
    }

    fn declare(&mut self, name: &Name<'a>, item: Item<'a>) -> Check<()> {
        match self.pool.insert(name.text, item) {
            None => Ok(()),
            Some(_) => {
                let message = format!("`{}` is already declared", name.text);
                Err(self.fail_at(name, "duplicate-name", message))
            }
        }
    }

    fn declare_def(&mut self, def: &'a Def<'a>) -> Check<()> {
        match def {
            Def::Enum { name, variants } => {
                self.declare(name, Item::Enum)?;
                variants.iter().try_for_each(|v| {
                    if !v.fields.is_empty() {
                        return Err(self.unsupported_name(&v.name, "a variant with fields"));
                    }
                    self.declare(&v.name, Item::Variant(name.text))
                })
            }
            Def::Fn {
                name,
                params,
                result,
                body,
            } => {
                let func = self.func(name, params, result, body)?;
                self.declare(name, Item::Fn(func))
            }
            Def::App { name, .. } => self.declare(name, Item::App),
            Def::Type { name, .. } => Err(self.unsupported_name(name, "a type definition")),
            Def::Example { name, .. } => Err(self.unsupported_name(name, "an example")),
        }
    }

    fn single_app(&self, defs: &[Def]) -> Check<()> {
        let mut apps = defs.iter().filter_map(|d| match d {
            Def::App { name, .. } => Some(name),
            _ => None,
        });
        match apps.nth(1) {
            Some(second) => {
                let message = "a program holds one app".into();
                Err(self
                    .fail_at(second, "multiple-apps", message)
                    .in_definition(second.text))
            }
            None => Ok(()),
        }
    }

    /// Outside in: the name, then the argument count, then the arguments.
    fn well_formed(&self, t: &Type) -> Check<()> {
        let (name, args) = match t {
            Type::Name { name, args } => (name, args),
            Type::Fn { at, .. } => return Err(self.unsupported(*at, 2, "a function type")),
        };
        let arity = match self.pool.get(name.text) {
            Some(Item::Type(n)) => *n,
            Some(Item::Enum) => 0,
            _ => {
                let message = format!("`{}` is not a type", name.text);
                return Err(self.fail_at(name, "unknown-type", message));
            }
        };
        if arity != args.len() {
            let message = format!("`{}` takes {arity} type arguments", name.text);
            return Err(self.fail_at(name, "wrong-type-arguments", message));
        }
        args.iter().try_for_each(|a| self.well_formed(a))
    }

    fn check_fn(&self, f: Func) -> Check<()> {
        self.well_formed(f.param_type)?;
        self.well_formed(f.result)?;
        self.expr(f.body, f.result)
    }

    /// Checks `e` against the well formed type `want`.
    fn expr(&self, e: &Expr, want: &Type) -> Check<()> {
        let mismatch = |found: &str| {
            let message = format!("expected `{want}`, found `{found}`");
            let d = self.fail(e.at, e.len, "type-mismatch", message);
            d.with_types(want, &found)
        };
        match &e.kind {
            ExprKind::Str(parts) => {
                if parts.iter().any(|p| matches!(p, StrPart::Expr(_))) {
                    return Err(self.unsupported(e.at, e.len, "an interpolated string"));
                }
                if head(want) == "String" {
                    Ok(())
                } else {
                    Err(mismatch("String"))
                }
            }
            ExprKind::List(items) => {
                let elem = inner(want, "List").ok_or_else(|| mismatch("List<_>"))?;
                items.iter().try_for_each(|i| self.expr(i, elem))
            }
            ExprKind::Name(name) => self.apply(e, name, &[], false, want),
            ExprKind::Call(callee, args) => match callee.kind {
                ExprKind::Name(name) => self.apply(e, name, args, true, want),
                _ => Err(self.unsupported(e.at, e.len, "a call of a value")),
            },
            _ => Err(self.unsupported(e.at, e.len, "this expression")),
        }
    }

    /// A variant or an effect `name`, written bare or called with `args`.
    fn apply(&self, e: &Expr, name: &str, args: &[Arg], called: bool, want: &Type) -> Check<()> {
        if let Some(label) = args.iter().find_map(|a| a.label.as_ref()) {
            return Err(self.unsupported_name(label, "a labelled argument"));
        }
        let (arity, fits, found) = match self.pool.get(name) {
            Some(Item::Variant(en)) => (0, head(want) == *en, *en),
            Some(Item::Effect(n)) => (*n, inner(want, "Effect").is_some(), "Effect<_>"),
            _ => {
                let message = format!("`{name}` is not a variant or an effect");
                return Err(self.fail(e.at, name.len(), "unknown-name", message));
            }
        };
        if args.len() != arity || (called && arity == 0) {
            let message = match arity {
                0 => format!("{name} has no fields: write {name}"),
                _ => format!("`{name}` takes {arity} arguments"),
            };
            return Err(self.fail(e.at, name.len(), "wrong-argument-count", message));
        }
        if !fits {
            let message = format!("expected `{want}`, found `{found}`");
            let d = self.fail(e.at, e.len, "type-mismatch", message);
            return Err(d.with_types(want, &found));
        }
        args.iter().try_for_each(|a| self.expr(&a.value, &STRING))
    }

    fn check_app(&self, app: &Name<'a>, entries: &'a [Entry<'a>]) -> Check<Checked<'a>> {
        let Keys {
            start,
            update,
            grants,
        } = self.keys(app, entries)?;
        let start = &start;
        let Some(Item::Fn(f)) = self.pool.get(update.text) else {
            let message = format!("`{}` is not a function", update.text);
            return Err(self.fail_at(update, "unknown-name", message));
        };
        let Some(message) = self.message_enum(f) else {
            let message = format!("`{}` is not `(x: M) -> List<Effect<M>>`", update.text);
            return Err(self.fail_at(update, "bad-update-signature", message));
        };
        self.check_start(start, message)?;
        self.check_grants(grants)?;
        Ok(Checked {
            app: app.text,
            start: start.text,
            update_name: update.text,
            update: f.body,
            grants: grants.iter().map(|g| g.text).collect(),
        })
    }

    /// Each key once, then each one present, in the spec's order.
    fn keys(&self, app: &Name, entries: &'a [Entry<'a>]) -> Check<Keys<'a>> {
        let (mut start, mut update, mut grants) = (None, None, None);
        for entry in entries {
            let repeated = match entry {
                Entry::State(at, _) => {
                    return Err(self.unsupported(*at, 5, "the `state` entry"));
                }
                Entry::Start(_, e) => {
                    let ExprKind::Name(text) = e.kind else {
                        return Err(self.unsupported(e.at, e.len, "a start other than a name"));
                    };
                    start.replace(Name { text, at: e.at }).is_some()
                }
                Entry::Update(_, n) => update.replace(n).is_some(),
                Entry::Grants(_, g) => grants.replace(g.as_slice()).is_some(),
            };
            if repeated {
                let (at, key) = key(entry);
                let message = format!("`{key}` is written twice");
                return Err(self.fail(at, key.len(), "duplicate-app-entry", message));
            }
        }
        let missing = |code, key| self.fail_at(app, code, format!("the app has no `{key}`"));
        Ok(Keys {
            start: start.ok_or_else(|| missing("missing-start", "start"))?,
            update: update.ok_or_else(|| missing("missing-update", "update"))?,
            grants: grants.ok_or_else(|| missing("missing-grants", "grants"))?,
        })
    }

    /// `M` when `f` is `(x: M) -> List<Effect<M>>` for an enum `M`, judged by the types as written.
    fn message_enum(&self, f: &Func<'a>) -> Option<&'a str> {
        let Type::Name { name, args } = f.param_type else {
            return None;
        };
        let is_enum = args.is_empty() && matches!(self.pool.get(name.text), Some(Item::Enum));
        let result = inner(f.result, "List").and_then(|t| inner(t, "Effect"))?;
        (is_enum && same(f.param_type, result)).then_some(name.text)
    }

    fn check_start(&self, start: &Name, message: &str) -> Check<()> {
        match self.pool.get(start.text) {
            Some(Item::Variant(en)) if *en == message => Ok(()),
            Some(Item::Variant(_) | Item::Effect(_)) => {
                let text = format!("`{}` is not a variant of `{message}`", start.text);
                Err(self.fail_at(start, "bad-start", text))
            }
            _ => {
                let text = format!("`{}` is not a variant or an effect", start.text);
                Err(self.fail_at(start, "unknown-name", text))
            }
        }
    }

    fn check_grants(&self, grants: &[Name]) -> Check<()> {
        for (i, grant) in grants.iter().enumerate() {
            if !matches!(self.pool.get(grant.text), Some(Item::Grant)) {
                let message = format!("`{}` is not a grant", grant.text);
                let fix = self.fail_at(grant, "unknown-grant", message);
                return Err(fix.with_fix("Console"));
            }
            if grants[..i].iter().any(|g| g.text == grant.text) {
                let message = format!("`{}` is listed twice", grant.text);
                return Err(self.fail_at(grant, "duplicate-grant", message));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const LIGHT: &str = include_str!("../../examples/light.paka");

    fn checked<'a>(program: &'a Program<'a>) -> Check<Checked<'a>> {
        check("main.paka", program)
    }

    #[test]
    fn light_paka_checks() {
        let program = parse("main.paka", LIGHT.as_bytes()).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!(light.app, "light");
        assert_eq!(light.start, "Start");
        assert_eq!(light.grants, ["Console"]);
    }

    #[test]
    fn definitions_and_entries_may_come_in_any_order() {
        let src = "app light {
              grants: []
              update: update
              start: Go
            }
            fn update(m: Msg) -> List<Effect<Msg>> { [] }
            enum Msg { Stop, Go }";
        let program = parse("main.paka", src.as_bytes()).unwrap();
        let light = checked(&program).unwrap();
        assert_eq!((light.start, light.grants), ("Go", vec![]));
    }

    #[test]
    fn forms_of_later_milestones_are_reported_at_their_text() {
        let cases = [
            (
                "enum Msg { Start }",
                "type Pt {}\nenum Msg { Start }",
                (1, 6),
            ),
            ("{ Start }", "{ Start(n: Int) }", (1, 12)),
            ("m: Msg)", "m: Msg, n: Msg)", (3, 4)),
            ("be light", "be {m}", (4, 10)),
            ("Print(", "Print(text: ", (4, 10)),
            ("  start:", "  state: Msg\n  start:", (8, 3)),
            ("Start\n", "Start()\n", (8, 11)),
        ];
        for (from, to, position) in cases {
            let src = LIGHT.replacen(from, to, 1);
            let program = parse("main.paka", src.as_bytes()).unwrap();
            let d = checked(&program).unwrap_err();
            assert_eq!(
                (d.code, d.position),
                ("unsupported-form", Some(position)),
                "{to}"
            );
        }
    }

    #[test]
    fn a_variant_with_parentheses_has_no_fields() {
        let src = LIGHT.replacen("Exit", "Exit()", 1);
        let program = parse("main.paka", src.as_bytes()).unwrap();
        let d = checked(&program).unwrap_err();
        assert_eq!(
            (d.code, d.position),
            ("wrong-argument-count", Some((4, 33)))
        );
        assert!(d.message.contains("Exit has no fields: write Exit"), "{d}");
    }
}
