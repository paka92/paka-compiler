//! What the harness's release mode runs (M6-06): each case's feature tags, from a fixed list, the
//! skip file of tags release does not build yet, and the hand-written exemption list.

use paka::builtins::ROWS;
use paka::core_form::{
    Arg, Bin, Block, Callee, Effect, Expr, ExprKind, Place, Program, Root, Step, Stmt, Tag, Type,
    Walk,
};
use paka_rt::effects::Grant;
use std::collections::BTreeSet;

/// The fixed list of tags. M2-10 has no shared data and no pools, so no case carries those two.
pub const TAGS: [&str; 18] = [
    "integers",
    "control flow",
    "calls",
    "floats",
    "types",
    "enums",
    "match",
    "parameter passing",
    "strings",
    "lists",
    "maps",
    "shared data",
    "pools",
    "generics",
    "function values",
    "error handling",
    "effect values",
    "apps and grants",
];

/// The tags of the features `core` uses beyond M1's light program, an app whose update returns a
/// list of `Print` of a literal and `Exit`. Examples are not part of a release build.
pub fn tags(core: &Program) -> BTreeSet<&'static str> {
    let mut t = Tagger {
        core,
        msg: None,
        tags: BTreeSet::new(),
    };
    if let Some(app) = &core.app {
        let update = &core.fns[app.update];
        if let Some(Type::Enum(msg)) = update.params.last().map(|p| &core.types[p.ty]) {
            t.msg = Some(*msg);
        }
        let start = app.start.map(|s| t.msg.map(|m| &core.enums[m].variants[s]));
        let fields = start.flatten().is_some_and(|v| !v.fields.is_empty());
        let grants = app.grants.iter().any(|g| *g != Grant::Console) || !app.foreign.is_empty();
        if app.state.is_some() || app.view.is_some() || fields || grants {
            t.tags.insert("apps and grants");
        }
        app.state.iter().for_each(|e| t.expr(e));
    }
    for (k, f) in core.fns.iter().enumerate() {
        let update = core.app.as_ref().is_some_and(|a| a.update == k);
        f.params.iter().for_each(|p| {
            t.ty(p.ty);
            t.mark(p.inout, "parameter passing");
        });
        f.locals
            .iter()
            .chain(f.result.iter().filter(|_| !update))
            .for_each(|&ty| t.ty(ty));
        f.body.stmts.iter().for_each(|s| t.stmt(s));
        let value = f.body.value.as_deref();
        match value.map(|e| &e.kind) {
            Some(ExprKind::List(items)) if update => {
                items.iter().filter(|e| !light(e)).for_each(|e| t.item(e))
            }
            _ => value.iter().for_each(|e| t.expr(e)),
        }
    }
    t.tags
}

/// An item M1's light update returns: `Print` of a literal, or `Exit`.
fn light(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Variant(Tag::Effect(Effect::Exit), _) => true,
        ExprKind::Variant(Tag::Effect(Effect::Print), fields) => {
            matches!(&fields[..], [text] if matches!(text.kind, ExprKind::Str(_)))
        }
        _ => false,
    }
}

struct Tagger<'a> {
    core: &'a Program,
    /// The app's message enum, whose values M1 already has.
    msg: Option<usize>,
    tags: BTreeSet<&'static str>,
}

impl Tagger<'_> {
    fn mark(&mut self, on: bool, tag: &'static str) {
        if on {
            self.tags.insert(tag);
        }
    }

    fn ty(&mut self, ty: usize) {
        let tag = match &self.core.types[ty] {
            Type::Int | Type::Bool => "integers",
            Type::Float => "floats",
            Type::String => "strings",
            Type::List(t) => return self.within("lists", &[*t]),
            Type::Map(k, v) => return self.within("maps", &[*k, *v]),
            Type::Option(t) => return self.within("enums", &[*t]),
            Type::Result(t, e) => return self.within("error handling", &[*t, *e]),
            Type::Effect(m) => return self.within("effect values", &[*m]),
            Type::Fn(params, result) => {
                let all: Vec<_> = params.iter().chain(result).copied().collect();
                return self.within("function values", &all);
            }
            Type::Record(_) => "types",
            Type::Enum(e) if Some(*e) == self.msg => return,
            Type::Enum(_) => "enums",
            Type::View(_) | Type::Ui => "apps and grants",
        };
        self.tags.insert(tag);
    }

    fn within(&mut self, tag: &'static str, inner: &[usize]) {
        self.tags.insert(tag);
        inner.iter().for_each(|&t| self.ty(t));
    }

    fn block(&mut self, block: &Block) {
        block.stmts.iter().for_each(|s| self.stmt(s));
        block.value.iter().for_each(|e| self.expr(e));
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(_, e) | Stmt::Expr(e) => self.expr(e),
            Stmt::Assign(place, _, e) => {
                self.place(place);
                self.expr(e);
            }
            Stmt::Return(e) => {
                self.tags.insert("control flow");
                e.iter().for_each(|e| self.expr(e));
            }
            Stmt::Break | Stmt::Continue => drop(self.tags.insert("control flow")),
            Stmt::For(_, walk, body) => {
                self.tags.insert("control flow");
                match walk {
                    Walk::List(e) => self.expr(e),
                    Walk::Range(a, b) => [a, b].iter().for_each(|e| self.expr(e)),
                }
                self.block(body);
            }
            Stmt::While(cond, body) => {
                self.tags.insert("control flow");
                self.expr(cond);
                self.block(body);
            }
        }
    }

    fn place(&mut self, place: &Place) {
        if let Root::Value(e) = &place.root {
            self.expr(e);
        }
        for step in &place.steps {
            match step {
                Step::Field(_) => {}
                Step::Item(e, _) | Step::Key(e, _) => self.expr(e),
                Step::Slice(a, b, _) => [a, b].iter().for_each(|e| self.expr(e)),
            }
        }
    }

    /// An item of the list update returns, whose effect type M1 already has.
    fn item(&mut self, item: &Expr) {
        match &item.kind {
            ExprKind::Variant(tag @ Tag::Effect(_), args) => self.variant(tag, args),
            _ => self.expr(item),
        }
    }

    fn variant(&mut self, tag: &Tag, args: &[Expr]) {
        let app = match tag {
            Tag::User(e, _) => Some(*e) == self.msg,
            Tag::Effect(e) => matches!(e, Effect::ExitWith | Effect::ReadFile | Effect::WriteFile),
            _ => false,
        };
        self.mark(app, "apps and grants");
        args.iter().for_each(|e| self.expr(e));
    }

    fn expr(&mut self, expr: &Expr) {
        expr.ty.iter().for_each(|&t| self.ty(t));
        match &expr.kind {
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Hole(_)
            | ExprKind::Placeholder(_) => {}
            ExprKind::Interp(parts, _) | ExprKind::List(parts) | ExprKind::Record(_, parts) => {
                parts.iter().for_each(|e| self.expr(e))
            }
            ExprKind::Place(place) => self.place(place),
            ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            ExprKind::Variant(tag, args) => self.variant(tag, args),
            ExprKind::FnValue(_) => drop(self.tags.insert("function values")),
            ExprKind::Unary(_, e) => self.expr(e),
            ExprKind::Binary(op, a, b) => {
                self.mark(matches!(op, Bin::And | Bin::Or), "control flow");
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Call(callee, args, _) => {
                self.tags.insert("calls");
                match callee {
                    Callee::Fn(_) => {}
                    Callee::Builtin(row) => match ROWS.get(*row) {
                        Some((_, row)) => self.mark(generic(row), "generics"),
                        None => drop(self.tags.insert("apps and grants")),
                    },
                    Callee::Value(e) => {
                        self.tags.insert("function values");
                        self.expr(e);
                    }
                }
                for arg in args {
                    match arg {
                        Arg::Value(e) => self.expr(e),
                        Arg::Inout(p) => {
                            self.tags.insert("parameter passing");
                            self.place(p);
                        }
                    }
                }
            }
            ExprKind::If(cond, then, otherwise) => {
                self.tags.insert("control flow");
                self.expr(cond);
                self.block(then);
                otherwise.iter().for_each(|b| self.block(b));
            }
            ExprKind::Match(subject, arms) => {
                self.tags.insert("match");
                self.expr(subject);
                arms.iter().for_each(|arm| self.block(&arm.body));
            }
            ExprKind::Root(lines) => self.block(lines),
            ExprKind::Element(_, args, children, _) => {
                self.tags.insert("apps and grants");
                args.iter().for_each(|e| self.expr(e));
                children.iter().for_each(|b| self.block(b));
            }
            ExprKind::Child(c) => {
                self.expr(&c.node);
                c.key.iter().for_each(|k| self.expr(k));
            }
        }
    }
}

/// Whether a built-in row is generic: it names one of the type variables T, K, V or E.
fn generic(row: &str) -> bool {
    (row.split(|c: char| !c.is_alphanumeric())).any(|w| matches!(w, "T" | "K" | "V" | "E"))
}

/// The tags the skip file `text` lists, one per line, `#` lines and blank lines aside; an error
/// names a line that is not on the fixed list.
pub fn skipped(text: &str) -> Result<BTreeSet<&'static str>, String> {
    let lines = text.lines().map(str::trim);
    let lines = lines.filter(|l| !l.is_empty() && !l.starts_with('#'));
    (lines.map(|l| {
        TAGS.into_iter()
            .find(|t| *t == l)
            .ok_or(format!("unknown tag {l:?}"))
    }))
    .collect()
}

/// The exemption list `text`: the cases under each kind, one `KIND CASE` per line, `#` lines and
/// blank lines aside. The kinds are `hole`, a case release must refuse by its holes' names, and
/// `examples`, a case compared in interpreter and dev only.
pub fn exempt(text: &str) -> Result<[BTreeSet<String>; 2], String> {
    let mut sets = [BTreeSet::new(), BTreeSet::new()];
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (kind, case) = line.split_once(' ').unwrap_or((line, ""));
        let k = (["hole", "examples"].iter().position(|w| *w == kind))
            .filter(|_| !case.trim().is_empty())
            .ok_or(format!("not `hole CASE` or `examples CASE`: {line:?}"))?;
        sets[k].insert(case.trim().to_owned());
    }
    Ok(sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paka::source::Source;

    const LIGHT: &str = "enum Msg { Start }\n\
        fn update(m: Msg) -> List<Effect<Msg>> {\n  [Print(\"hi\"), Exit]\n}\n\
        app light {\n  start: Start\n  update: update\n  grants: [Console]\n}\n";

    fn tagged(text: &str) -> Vec<&'static str> {
        let source = Source::file("main.paka", text.into());
        let (core, ..) = paka::check("main.paka", &source).expect("the program checks");
        tags(&core).into_iter().collect()
    }

    #[test]
    fn the_light_program_carries_no_tag() {
        assert!(tagged(LIGHT).is_empty());
        let examples = LIGHT.to_owned() + "example e { 1 + 1 == 2 }\n";
        assert!(tagged(&examples).is_empty(), "examples are not built");
    }

    #[test]
    fn each_feature_carries_its_tags() {
        let with = |item: &str| tagged(&LIGHT.replace("Print(\"hi\")", item));
        assert_eq!(with("Print(\"a\" + \"b\")"), ["strings"]);
        assert_eq!(with("ExitWith(7)"), ["apps and grants", "integers"]);
        assert_eq!(
            with("Print(\"{len([1])}\")"),
            ["calls", "generics", "integers", "lists", "strings"]
        );
        assert_eq!(
            with("Print(fixed(1.5, digits: 2))"),
            ["calls", "floats", "integers", "strings"]
        );
        let fields = LIGHT.replace("Start }", "Start(src: String) }");
        assert!(tagged(&fields).contains(&"apps and grants"));
    }

    #[test]
    fn the_skip_file_lists_tags_only() {
        let listed = skipped("# a comment\n\nstrings\n  calls\n").unwrap();
        assert_eq!(listed.into_iter().collect::<Vec<_>>(), ["calls", "strings"]);
        assert!(skipped("apps/app_light\n").is_err());
        skipped(include_str!("../../tests/conformance/release-skip")).unwrap();
    }

    #[test]
    fn the_exemption_list_has_two_kinds() {
        let [holes, examples] = exempt("# x\nhole a/b\nexamples c/d\n").unwrap();
        assert_eq!((holes.len(), examples.len()), (1, 1));
        assert!(exempt("skip a/b\n").is_err());
        assert!(exempt("hole\n").is_err());
        exempt(include_str!("../../tests/conformance/release-exempt")).unwrap();
    }
}
