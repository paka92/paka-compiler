//! Lowering a checked M1 program into the core form, and the view of it the M1 back ends run.

use crate::ast::{self, Def, StmtKind, StrPart};
use crate::check::Checked;
use crate::core_form::{
    self, App, Block, Enum, Expr, ExprKind, Fn, Grant, Param, Program, Tag, TrapSite, Ty, Type,
    Variant,
};

/// What the M1 back ends run: the effects of an `update` whose body is a constant list.
#[derive(Debug, PartialEq)]
pub struct M1 {
    pub app: String,
    /// The start variant's position in the message enum.
    pub start: usize,
    pub console: bool,
    /// The Paka name of the update function.
    pub update_name: String,
    /// The effects `update` returns, in order.
    pub update: Vec<Effect>,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Print(String),
    Exit,
}

/// Runs on a checked program, where every lookup succeeds and every node is of the M1 subset.
pub fn lower(path: &str, program: &ast::Program, checked: &Checked) -> Program {
    let mut core = Program {
        file: path.into(),
        types: vec![],
        records: vec![],
        enums: vec![],
        fns: vec![],
        examples: vec![],
        app: None,
        sites: vec![],
    };
    for def in &program.defs {
        if let Def::Enum { name, variants } = def {
            let variants = variants.iter().map(|v| Variant {
                name: v.name.text.into(),
                fields: vec![], // M1 variants have none.
            });
            core.enums.push(Enum {
                name: name.text.into(),
                variants: variants.collect(),
            });
        }
    }
    for def in &program.defs {
        if let Def::Fn {
            name,
            params,
            result,
            body,
        } = def
        {
            let f = function(&mut core, name, params, result.as_ref(), body);
            core.fns.push(f);
        }
    }
    core.app = Some(app(&core, checked));
    core
}

/// An M1 function: one parameter, a result and a body of one expression.
fn function(
    core: &mut Program,
    name: &ast::Name,
    params: &[ast::Param],
    result: Option<&ast::Type>,
    body: &ast::Block,
) -> Fn {
    let (line, col) = name.at;
    core.sites.push(TrapSite {
        line,
        col,
        hole: None,
    });
    let site = core.sites.len() - 1;
    let params = params.iter().map(|p| Param {
        ty: ty(core, &p.ty),
        inout: p.inout,
    });
    let params = params.collect();
    let result = ty(core, result.expect("an M1 function has a result"));
    let [ast::Node { kind, .. }] = body.stmts.as_slice() else {
        unreachable!("an M1 body is one statement");
    };
    let StmtKind::Expr(e) = kind else {
        unreachable!("an M1 statement is an expression");
    };
    let value = Some(Box::new(expr(core, e, result)));
    Fn {
        name: name.text.into(),
        site,
        params,
        result: Some(result),
        body: Block {
            stmts: vec![],
            value,
        },
    }
}

fn ty(core: &mut Program, t: &ast::Type) -> Ty {
    let ast::Type::Name { name, args } = t else {
        unreachable!("M1 has no function types");
    };
    let t = match (name.text, args.as_slice()) {
        ("String", []) => Type::String,
        ("List", [item]) => Type::List(ty(core, item)),
        ("Effect", [message]) => Type::Effect(ty(core, message)),
        (name, []) => Type::Enum(enum_id(core, name)),
        _ => unreachable!("M1 has no other types"),
    };
    core.ty(t)
}

/// `e` lowered as a value of type `want`, the type the checker held it to.
fn expr(core: &mut Program, e: &ast::Expr, want: Ty) -> Expr {
    let kind = match &e.kind {
        ast::ExprKind::Str(parts) => ExprKind::Str(text(parts)),
        ast::ExprKind::List(items) => {
            let Type::List(item) = core.types[want] else {
                unreachable!("a list is checked against a List type");
            };
            ExprKind::List(items.iter().map(|i| expr(core, i, item)).collect())
        }
        ast::ExprKind::Name(name) => ExprKind::Variant(tag(core, name, want), vec![]),
        ast::ExprKind::Call(callee, args) => {
            let ast::ExprKind::Name(name) = callee.kind else {
                unreachable!("M1 calls only names");
            };
            let string = core.ty(Type::String);
            let fields = args.iter().map(|a| expr(core, &a.value, string)).collect();
            ExprKind::Variant(tag(core, name, want), fields)
        }
        _ => unreachable!("M1 has no other expressions"),
    };
    Expr {
        ty: Some(want),
        kind,
    }
}

/// The variant `name` of the enum or effect type `want`.
fn tag(core: &Program, name: &str, want: Ty) -> Tag {
    match core.types[want] {
        Type::Effect(_) if name == "Print" => Tag::Effect(core_form::Effect::Print),
        Type::Effect(_) => Tag::Effect(core_form::Effect::Exit),
        Type::Enum(e) => Tag::User(e, variant_id(&core.enums[e], name)),
        _ => unreachable!("M1 variants are of an enum or an effect"),
    }
}

/// The text of a string without interpolations, which is all the M1 checker accepts.
fn text(parts: &[StrPart]) -> String {
    let texts = parts.iter().filter_map(|p| match p {
        StrPart::Text(t) => Some(t.as_str()),
        StrPart::Expr(_) => None,
    });
    texts.collect()
}

fn app(core: &Program, checked: &Checked) -> App {
    let update = core.fns.iter().position(|f| f.name == checked.update_name);
    let update = update.expect("the update function is defined");
    let message = core.fns[update]
        .params
        .last()
        .expect("update has a message");
    let Type::Enum(message) = core.types[message.ty] else {
        unreachable!("the message type is an enum");
    };
    App {
        name: checked.app.into(),
        state: None,
        start: variant_id(&core.enums[message], checked.start),
        update,
        grants: checked.grants.iter().map(|g| grant(g)).collect(),
    }
}

fn grant(name: &str) -> Grant {
    match name {
        "Console" => Grant::Console,
        "ReadFiles" => Grant::ReadFiles,
        "WriteFiles" => Grant::WriteFiles,
        _ => unreachable!("the checker knows no other grant"),
    }
}

fn enum_id(core: &Program, name: &str) -> usize {
    let at = core.enums.iter().position(|e| e.name == name);
    at.expect("a checked type name is an enum")
}

fn variant_id(e: &Enum, name: &str) -> usize {
    let at = e.variants.iter().position(|v| v.name == name);
    at.expect("a checked variant is in its enum")
}

/// The M1 view of `core`, or `None` when it holds what the M1 back ends cannot run.
pub fn m1(core: &Program) -> Option<M1> {
    let app = core.app.as_ref()?;
    let update = &core.fns[app.update];
    let only_console = app.grants.iter().all(|g| *g == Grant::Console);
    if app.state.is_some() || !only_console || !update.body.stmts.is_empty() {
        return None;
    }
    let ExprKind::List(items) = &update.body.value.as_ref()?.kind else {
        return None;
    };
    Some(M1 {
        app: app.name.clone(),
        start: app.start,
        console: app.grants.contains(&Grant::Console),
        update_name: update.name.clone(),
        update: items.iter().map(effect).collect::<Option<_>>()?,
    })
}

fn effect(e: &Expr) -> Option<Effect> {
    match &e.kind {
        ExprKind::Variant(Tag::Effect(core_form::Effect::Exit), _) => Some(Effect::Exit),
        ExprKind::Variant(Tag::Effect(core_form::Effect::Print), fields) => match &fields[..] {
            [
                Expr {
                    kind: ExprKind::Str(text),
                    ..
                },
            ] => Some(Effect::Print(text.clone())),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_form::{
        App, Block, Enum, Expr, ExprKind, Fn, Grant, Param, Stmt, Tag, TrapSite, Type, Variant,
    };
    use crate::{check::check, parser::parse};
    use std::collections::BTreeSet;

    fn light() -> Program {
        let program = parse("main.paka", include_bytes!("../../examples/light.paka")).unwrap();
        let checked = check("main.paka", &program).unwrap();
        lower("main.paka", &program, &checked)
    }

    /// An effect of `Effect<Msg>`, type 1 in light.paka.
    fn effect(effect: core_form::Effect, fields: Vec<Expr>) -> Expr {
        let kind = ExprKind::Variant(Tag::Effect(effect), fields);
        Expr { ty: Some(1), kind }
    }

    #[test]
    fn light_paka_lowers() {
        let text = Expr {
            ty: Some(3),
            kind: ExprKind::Str("let there be light".into()),
        };
        let effects = vec![
            effect(core_form::Effect::Print, vec![text]),
            effect(core_form::Effect::Exit, vec![]),
        ];
        let list = Expr {
            ty: Some(2),
            kind: ExprKind::List(effects),
        };
        let update = Fn {
            name: "update".into(),
            site: 0,
            params: vec![Param {
                ty: 0,
                inout: false,
            }],
            result: Some(2),
            body: Block {
                stmts: vec![],
                value: Some(Box::new(list)),
            },
        };
        let start = Variant {
            name: "Start".into(),
            fields: vec![],
        };
        let by_hand = Program {
            file: "main.paka".into(),
            types: vec![Type::Enum(0), Type::Effect(0), Type::List(1), Type::String],
            records: vec![],
            enums: vec![Enum {
                name: "Msg".into(),
                variants: vec![start],
            }],
            fns: vec![update],
            examples: vec![],
            app: Some(App {
                name: "light".into(),
                state: None,
                start: 0,
                update: 0,
                grants: BTreeSet::from([Grant::Console]),
            }),
            sites: vec![TrapSite {
                line: 3,
                col: 4,
                hole: None,
            }],
        };
        assert_eq!(light(), by_hand);
    }

    #[test]
    fn light_paka_reaches_the_m1_back_ends_as_before() {
        let by_hand = M1 {
            app: "light".into(),
            start: 0,
            console: true,
            update_name: "update".into(),
            update: vec![Effect::Print("let there be light".into()), Effect::Exit],
        };
        assert_eq!(m1(&light()), Some(by_hand));
    }

    #[test]
    fn m1_refuses_what_its_back_ends_cannot_run() {
        let mut state = light();
        state.records.push(core_form::Record {
            name: "State".into(),
            fields: vec![],
        });
        let ty = state.ty(Type::Record(0));
        let literal = Expr {
            ty: Some(ty),
            kind: ExprKind::Record(0, vec![]),
        };
        state.app.as_mut().unwrap().state = Some(literal);
        let mut grant = light();
        grant.app.as_mut().unwrap().grants.insert(Grant::ReadFiles);
        let mut statement = light();
        let text = Expr {
            ty: Some(3),
            kind: ExprKind::Str("unused".into()),
        };
        statement.fns[0].body.stmts.push(Stmt::Expr(text));
        let mut computed = light();
        let hole = Expr {
            ty: Some(2),
            kind: ExprKind::Hole(0),
        };
        computed.fns[0].body.value = Some(Box::new(hole));
        for core in [state, grant, statement, computed] {
            assert_eq!(m1(&core), None);
        }
    }
}
