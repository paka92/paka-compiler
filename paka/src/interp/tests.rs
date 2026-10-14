//! The interpreter on core programs written by hand (docs/core.md): every node, each trap
//! kind, the call-depth limit, and the effects the handler returns.

use super::value::{Entries, Items, Key, Value, equals};
use super::{on_big_stack, run};
use crate::builtins::{self, ROWS};
use crate::core_form::{
    App, Arg, Arith, Arm, Bin, Block, Callee, Cmp, Compound, Effect, Enum, Expr, ExprKind, Field,
    FloatArith, Fn as Func, FnRef, Grant, Param, Place, Program, Root, Step, Stmt, Tag, TrapSite,
    Type, Un, Variant, Walk,
};
use paka_rt::ops::{Float, Int};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{self, ErrorKind, Write};

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;

/// Trap sites: each has its own line and column, so a report names which one it means.
const OP: usize = 40;
const CALL: usize = 41;
const NEG: usize = 42;
const STEP: usize = 43;
const EQ: usize = 44;
const JOIN: usize = 45;
const INTERP: usize = 46;
/// Holes, with the names `todo` and `later`.
const TODO: usize = 60;
const LATER: usize = 61;
/// The name of every function.
const NAME: usize = 62;

type Out = (String, String, u8);

// Expressions

fn e(kind: ExprKind) -> Expr {
    Expr { ty: None, kind }
}

fn int(n: i64) -> Expr {
    e(ExprKind::Int(Int::new(n)))
}

fn float(text: &str) -> Expr {
    e(ExprKind::Float(Float::parse(text).unwrap()))
}

fn text(s: &str) -> Expr {
    e(ExprKind::Str(s.into()))
}

fn flag(b: bool) -> Expr {
    e(ExprKind::Bool(b))
}

fn hole(site: usize) -> Expr {
    e(ExprKind::Hole(site))
}

fn place(local: usize, steps: Vec<Step>) -> Place {
    Place {
        root: Root::Local(local),
        steps,
    }
}

fn at(local: usize, steps: Vec<Step>) -> Expr {
    e(ExprKind::Place(place(local, steps)))
}

fn var(local: usize) -> Expr {
    at(local, vec![])
}

fn item(i: i64) -> Step {
    Step::Item(int(i), STEP)
}

fn key(k: Expr) -> Step {
    Step::Key(k, STEP)
}

fn cut(from: i64, to: i64) -> Step {
    Step::Slice(int(from), int(to), STEP)
}

fn list(items: Vec<Expr>) -> Expr {
    e(ExprKind::List(items))
}

fn record(fields: Vec<Expr>) -> Expr {
    e(ExprKind::Record(0, fields))
}

fn map(pairs: Vec<(Expr, Expr)>) -> Expr {
    e(ExprKind::Map(pairs))
}

fn variant(tag: Tag, fields: Vec<Expr>) -> Expr {
    e(ExprKind::Variant(tag, fields))
}

fn some(x: Expr) -> Expr {
    variant(Tag::Some, vec![x])
}

fn none() -> Expr {
    variant(Tag::None, vec![])
}

fn unary(op: Un, x: Expr) -> Expr {
    e(ExprKind::Unary(op, Box::new(x)))
}

fn bin(op: Bin, a: Expr, b: Expr) -> Expr {
    e(ExprKind::Binary(op, Box::new(a), Box::new(b)))
}

fn arith(op: Arith, a: Expr, b: Expr) -> Expr {
    bin(Bin::Int(op, OP), a, b)
}

fn sum(a: Expr, b: Expr) -> Expr {
    arith(Arith::Add, a, b)
}

fn eq(a: Expr, b: Expr) -> Expr {
    bin(Bin::Eq(EQ), a, b)
}

fn lt(a: Expr, b: Expr) -> Expr {
    bin(Bin::Order(Cmp::Lt), a, b)
}

fn nan() -> Expr {
    bin(Bin::Float(FloatArith::Div), float("0.0"), float("0.0"))
}

fn call_io(id: usize, args: Vec<Arg>) -> Expr {
    e(ExprKind::Call(Callee::Fn(id), args, CALL))
}

fn call(id: usize, args: Vec<Expr>) -> Expr {
    call_io(id, args.into_iter().map(Arg::Value).collect())
}

fn call_value(f: Expr, args: Vec<Expr>) -> Expr {
    let args = args.into_iter().map(Arg::Value).collect();
    e(ExprKind::Call(Callee::Value(Box::new(f)), args, CALL))
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    let row = ROWS.iter().position(|(_, r)| builtins::name(r) == name);
    let args = args.into_iter().map(Arg::Value).collect();
    e(ExprKind::Call(Callee::Builtin(row.unwrap()), args, CALL))
}

fn block(stmts: Vec<Stmt>, value: Option<Expr>) -> Block {
    Block {
        stmts,
        value: value.map(Box::new),
    }
}

fn branch(cond: Expr, then: Expr, other: Option<Expr>) -> Expr {
    let other = other.map(|x| block(vec![], Some(x)));
    e(ExprKind::If(
        Box::new(cond),
        block(vec![], Some(then)),
        other,
    ))
}

fn arm(tags: Vec<Tag>, binds: Vec<Option<usize>>, value: Expr) -> Arm {
    Arm {
        tags,
        binds,
        body: block(vec![], Some(value)),
    }
}

fn matching(subject: Expr, arms: Vec<Arm>) -> Expr {
    e(ExprKind::Match(Box::new(subject), arms))
}

/// The parts joined by commas, as a String.
fn csv(parts: Vec<Expr>) -> Expr {
    let mut all = vec![];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            all.push(text(","));
        }
        all.push(part);
    }
    e(ExprKind::Interp(all, 0))
}

fn show(x: Expr) -> Expr {
    e(ExprKind::Interp(vec![x], 0))
}

// Statements

fn set(local: usize, steps: Vec<Step>, value: Expr) -> Stmt {
    Stmt::Assign(place(local, steps), None, value)
}

fn put(local: usize, steps: Vec<Step>, op: Compound, value: Expr) -> Stmt {
    Stmt::Assign(place(local, steps), Some(op), value)
}

fn add_to(local: usize, value: Expr) -> Stmt {
    put(local, vec![], Compound::Int(Arith::Add, OP), value)
}

fn when(cond: Expr, stmts: Vec<Stmt>) -> Stmt {
    let then = block(stmts, None);
    Stmt::Expr(e(ExprKind::If(Box::new(cond), then, None)))
}

fn repeat(local: usize, times: i64, body: Vec<Stmt>) -> Stmt {
    Stmt::For(local, Walk::Range(int(0), int(times)), block(body, None))
}

// Programs

fn func(name: &str, inouts: &[bool], locals: usize, body: Block) -> Func {
    Func {
        name: name.into(),
        site: NAME,
        params: inouts.iter().map(|&inout| Param { ty: 0, inout }).collect(),
        locals: vec![0; locals.max(inouts.len())],
        result: None,
        body,
        claims: vec![],
    }
}

/// `update` is the last function; its last parameter is the message, of type 4.
fn program(mut fns: Vec<Func>, mut update: Func) -> Program {
    update.params.last_mut().unwrap().ty = 4;
    fns.push(update);
    let mut sites: Vec<_> = (0..64)
        .map(|i| TrapSite {
            line: i + 1,
            col: 2 * i + 1,
            hole: None,
        })
        .collect();
    sites[TODO].hole = Some("todo".into());
    sites[LATER].hole = Some("later".into());
    Program {
        file: "dir/main.paka".into(),
        types: vec![
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::String,
            Type::Enum(0),
        ],
        records: vec![],
        enums: vec![Enum {
            name: "Msg".into(),
            variants: vec![Variant {
                name: "Start".into(),
                fields: vec![],
            }],
        }],
        app: Some(App {
            name: "light".into(),
            state: None,
            start: 0,
            update: fns.len() - 1,
            grants: BTreeSet::from([Grant::Console]),
        }),
        fns,
        examples: vec![],
        sites,
    }
}

fn print(x: Expr) -> Expr {
    variant(Tag::Effect(Effect::Print), vec![x])
}

fn exit() -> Expr {
    variant(Tag::Effect(Effect::Exit), vec![])
}

fn exit_with(status: i64) -> Expr {
    variant(Tag::Effect(Effect::ExitWith), vec![int(status)])
}

/// A handler that runs `stmts` (locals 1 to 8), then prints `value` and exits.
fn plain(fns: Vec<Func>, stmts: Vec<Stmt>, value: Expr) -> Program {
    let effects = list(vec![print(show(value)), exit()]);
    program(
        fns,
        func("update", &[false], 9, block(stmts, Some(effects))),
    )
}

/// A handler that returns `effects`.
fn light(console: bool, effects: Vec<Expr>) -> Program {
    let update = func("update", &[false], 1, block(vec![], Some(list(effects))));
    let mut p = program(vec![], update);
    if !console {
        p.app.as_mut().unwrap().grants.clear();
    }
    p
}

/// Runs on in-memory streams; returns (stdout, stderr, status).
fn go(p: &Program) -> Out {
    go_with(p, &[])
}

/// The same with the command-line arguments after the file.
fn go_with(p: &Program, args: &[&OsStr]) -> Out {
    let args: Vec<_> = args.iter().map(|a| a.to_os_string()).collect();
    let (mut out, mut err) = (vec![], vec![]);
    let status = run(p, &args, &mut out, &mut err);
    let text = |bytes| String::from_utf8(bytes).unwrap();
    (text(out), text(err), status)
}

fn said_with(fns: Vec<Func>, stmts: Vec<Stmt>, value: Expr) -> String {
    let (out, err, status) = go(&plain(fns, stmts, value));
    assert_eq!((err.as_str(), status), ("", 0));
    out.strip_suffix('\n').expect("one line").into()
}

fn said(value: Expr) -> String {
    said_with(vec![], vec![], value)
}

fn trapped_with(fns: Vec<Func>, stmts: Vec<Stmt>, value: Expr) -> String {
    let (out, err, status) = go(&plain(fns, stmts, value));
    assert_eq!((out.as_str(), status), ("", 3));
    err
}

fn trapped(value: Expr) -> String {
    trapped_with(vec![], vec![], value)
}

/// What a trap with a detail reports.
fn report(kind: &str, site: usize, detail: &str, chain: &str) -> String {
    format!("{}  {detail}\n  in {chain}\n", bare(kind, site))
}

/// What `assert`, `stack` and `memory` report.
fn bare(kind: &str, site: usize) -> String {
    let (line, col) = (site + 1, 2 * site + 1);
    format!("light: trap {kind} at main.paka:{line}:{col}\n")
}

fn truncated(x: Expr) -> String {
    said(builtin("to_int", vec![x]))
}

/// A list, a map, a record and an Int record, in locals 1 to 4.
fn world() -> Vec<Stmt> {
    let deep = list(vec![list(vec![int(7)])]);
    vec![
        Stmt::Let(1, list(vec![int(10), int(20), int(30)])),
        Stmt::Let(2, map(vec![(text("a"), int(1)), (text("b"), int(2))])),
        Stmt::Let(3, record(vec![int(5), text("héllo"), deep])),
        Stmt::Let(4, map(vec![(int(5), int(9))])),
    ]
}

// Literals and operators

#[test]
fn literals_and_interpolation() {
    assert_eq!(said(int(7)), "7");
    assert_eq!(said(flag(true)), "true");
    assert_eq!(said(text("{a}\"")), "{a}\"");
    let parts = vec![text("a"), int(-1), flag(false), text("é")];
    assert_eq!(said(e(ExprKind::Interp(parts, INTERP))), "a-1falseé");
}

#[test]
fn a_hole_traps_with_its_name() {
    assert_eq!(
        trapped(hole(TODO)),
        report("hole", TODO, "??todo", "update")
    );
}

#[test]
fn int_operators() {
    let cases = [
        (Arith::Add, 7, 3, "10"),
        (Arith::Sub, 7, 3, "4"),
        (Arith::Mul, 7, 3, "21"),
        (Arith::Div, 7, 2, "3"),
        (Arith::Div, -7, 2, "-3"),
        (Arith::Rem, 7, 3, "1"),
        (Arith::Rem, -7, 3, "-1"),
        (Arith::Rem, MIN, -1, "0"),
    ];
    for (op, a, b, want) in cases {
        assert_eq!(said(arith(op, int(a), int(b))), want);
    }
    assert_eq!(said(unary(Un::Neg(NEG), int(5))), "-5");
}

#[test]
fn int_traps() {
    let cases = [
        (Arith::Add, MAX, 1, "overflow", "9223372036854775807 + 1"),
        (Arith::Sub, MIN, 1, "overflow", "-9223372036854775808 - 1"),
        (Arith::Mul, MAX, 2, "overflow", "9223372036854775807 * 2"),
        (Arith::Div, MIN, -1, "overflow", "-9223372036854775808 / -1"),
        (Arith::Div, 7, 0, "divide_by_zero", "7 / 0"),
        (Arith::Rem, 7, 0, "divide_by_zero", "7 % 0"),
    ];
    for (op, a, b, kind, detail) in cases {
        let want = report(kind, OP, detail, "update");
        assert_eq!(trapped(arith(op, int(a), int(b))), want);
    }
    let want = report("overflow", NEG, "-(-9223372036854775808)", "update");
    assert_eq!(trapped(unary(Un::Neg(NEG), int(MIN))), want);
}

#[test]
fn float_operators() {
    let cases = [
        (FloatArith::Add, "10"),
        (FloatArith::Sub, "5"),
        (FloatArith::Mul, "18"),
        (FloatArith::Div, "3"),
    ];
    for (op, want) in cases {
        let both = bin(Bin::Float(op), float("7.5"), float("2.5"));
        assert_eq!(truncated(both), want);
    }
    assert_eq!(truncated(unary(Un::NegFloat, float("2.5"))), "-2");
    assert_eq!(said(unary(Un::Not, flag(true))), "false");
}

fn cmp(n: usize) -> Cmp {
    [Cmp::Lt, Cmp::Le, Cmp::Gt, Cmp::Ge]
        .into_iter()
        .nth(n)
        .unwrap()
}

#[test]
fn order_follows_the_type() {
    let kinds: [fn(i64) -> Expr; 3] = [int, |n| float(&format!("{n}.5")), |n| text(&n.to_string())];
    let wants = [
        [false, false, true],
        [false, true, true],
        [true, false, false],
        [true, true, false],
    ];
    for (n, want) in wants.iter().enumerate() {
        for kind in kinds {
            for (b, want) in (1..=3).zip(want) {
                let got = said(bin(Bin::Order(cmp(n)), kind(2), kind(b)));
                assert_eq!(got, want.to_string());
            }
        }
    }
    assert_eq!(said(lt(nan(), nan())), "false");
    assert_eq!(said(bin(Bin::Order(Cmp::Ge), nan(), nan())), "false");
}

#[test]
fn and_or_stop_at_the_first_answer() {
    let cases = [
        (Bin::And, false, hole(TODO), "false"),
        (Bin::Or, true, hole(TODO), "true"),
        (Bin::And, true, flag(true), "true"),
        (Bin::Or, false, flag(false), "false"),
    ];
    for (op, a, b, want) in cases {
        assert_eq!(said(bin(op, flag(a), b)), want);
    }
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(trapped(bin(Bin::And, flag(true), hole(TODO))), want);
}

#[test]
fn equality_is_structural() {
    let shape = |x| record(vec![int(1), list(vec![text("a")]), map(vec![(int(1), x)])]);
    let same = eq(shape(some(int(2))), shape(some(int(2))));
    assert_eq!(said(same), "true");
    let cases = [
        eq(shape(some(int(2))), shape(some(int(3)))),
        eq(some(int(1)), none()),
        eq(list(vec![int(1)]), list(vec![int(1), int(1)])),
        eq(map(vec![(int(1), int(1))]), map(vec![(int(2), int(1))])),
        eq(map(vec![]), map(vec![(int(2), int(1))])),
        eq(nan(), nan()),
    ];
    for case in cases {
        assert_eq!(said(case), "false");
    }
    let f = |id| e(ExprKind::FnValue(FnRef::Fn(id)));
    assert_eq!(said(eq(f(0), f(0))), "true");
    assert_eq!(said(eq(f(0), f(1))), "false");
    let ne = bin(Bin::Ne(EQ), some(int(1)), some(int(2)));
    assert_eq!(said(ne), "true");
}

#[test]
fn operands_run_left_to_right() {
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(trapped(sum(hole(TODO), hole(LATER))), want);
    assert_eq!(trapped(sum(int(1), hole(TODO))), want);
}

// Places

#[test]
fn places_read_each_step() {
    let cases = [
        (at(1, vec![item(1)]), "20"),
        (at(1, vec![cut(1, 3), item(1)]), "30"),
        (at(2, vec![key(text("b"))]), "2"),
        (at(4, vec![key(int(5))]), "9"),
        (at(3, vec![Step::Field(0)]), "5"),
        (at(3, vec![Step::Field(1), item(1)]), "é"),
        (at(3, vec![Step::Field(1), cut(1, 3)]), "él"),
        (at(3, vec![Step::Field(2), item(0), item(0)]), "7"),
    ];
    for (value, want) in cases {
        assert_eq!(said_with(vec![], world(), value), want);
    }
}

#[test]
fn a_computed_root_is_a_value_not_a_local() {
    let body = block(
        vec![Stmt::Let(0, list(vec![int(1), int(2), int(3)]))],
        Some(var(0)),
    );
    let root = Place {
        root: Root::Value(Box::new(call(0, vec![]))),
        steps: vec![item(2)],
    };
    let got = said_with(
        vec![func("three", &[], 1, body)],
        vec![],
        e(ExprKind::Place(root)),
    );
    assert_eq!(got, "3");
}

#[test]
fn place_errors() {
    let cases = [
        (at(1, vec![item(5)]), "index 5, length 3"),
        (at(1, vec![item(-1)]), "index -1, length 3"),
        (at(1, vec![cut(2, 9)]), "slice 2..<9, length 3"),
        (at(1, vec![cut(2, 1)]), "slice 2..<1, length 3"),
        (at(1, vec![cut(-1, 2)]), "slice -1..<2, length 3"),
        (at(2, vec![key(text("z"))]), "key \"z\""),
        (at(4, vec![key(int(7))]), "key 7"),
        (at(3, vec![Step::Field(1), item(9)]), "index 9, length 5"),
        (
            at(3, vec![Step::Field(1), cut(1, 9)]),
            "slice 1..<9, length 5",
        ),
    ];
    for (value, detail) in cases {
        let want = report("out_of_bounds", STEP, detail, "update");
        assert_eq!(trapped_with(vec![], world(), value), want);
    }
}

#[test]
fn a_place_runs_its_root_then_its_operands_then_the_walk() {
    let boom = func("boom", &[], 0, block(vec![], Some(hole(LATER))));
    let root = Place {
        root: Root::Value(Box::new(call(0, vec![]))),
        steps: vec![Step::Item(hole(TODO), STEP)],
    };
    let want = report("hole", LATER, "??later", "boom, called from update");
    assert_eq!(
        trapped_with(vec![boom], vec![], e(ExprKind::Place(root))),
        want
    );
    let steps = vec![item(9), Step::Item(hole(TODO), STEP)];
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(trapped_with(vec![], world(), at(1, steps)), want);
}

// Constructors, match and function values

#[test]
fn match_binds_the_fields_of_the_variant() {
    let pick = |subject| {
        let arms = vec![
            arm(vec![Tag::Some], vec![Some(5)], sum(var(5), int(1))),
            arm(vec![Tag::None], vec![], int(0)),
        ];
        matching(subject, arms)
    };
    assert_eq!(said(pick(some(int(41)))), "42");
    assert_eq!(said(pick(none())), "0");
    let arms = || {
        vec![
            arm(vec![Tag::Ok], vec![Some(5)], var(5)),
            arm(vec![Tag::Err], vec![Some(5)], sum(var(5), int(100))),
        ]
    };
    assert_eq!(
        said(matching(variant(Tag::Err, vec![int(7)]), arms())),
        "107"
    );
    assert_eq!(said(matching(variant(Tag::Ok, vec![int(7)]), arms())), "7");
}

#[test]
fn match_decides_on_the_variant_number_only() {
    let shape = |n| variant(Tag::User(1, n), vec![int(3), int(4)]);
    let arms = || {
        vec![
            arm(vec![Tag::User(1, 0)], vec![], int(10)),
            arm(vec![Tag::User(1, 1), Tag::User(1, 2)], vec![], int(20)),
        ]
    };
    assert_eq!(said(matching(shape(0), arms())), "10");
    assert_eq!(said(matching(shape(2), arms())), "20");
    let second = vec![arm(vec![Tag::User(1, 1)], vec![None, Some(5)], var(5))];
    assert_eq!(said(matching(shape(1), second)), "4");
}

#[test]
fn match_on_a_place_leaves_it_whole() {
    let pick = || {
        let arms = vec![
            arm(vec![Tag::Some], vec![Some(5)], var(5)),
            arm(vec![Tag::None], vec![], int(0)),
        ];
        matching(var(6), arms)
    };
    let stmts = vec![Stmt::Let(6, some(int(3)))];
    assert_eq!(said_with(vec![], stmts, sum(pick(), pick())), "6");
}

#[test]
fn function_values_call_functions_and_constructors() {
    let double = func(
        "double",
        &[false],
        1,
        block(vec![], Some(arith(Arith::Mul, var(0), int(2)))),
    );
    let stmts = vec![Stmt::Let(6, e(ExprKind::FnValue(FnRef::Fn(0))))];
    assert_eq!(
        said_with(vec![double], stmts, call_value(var(6), vec![int(4)])),
        "8"
    );
    let ctor = e(ExprKind::FnValue(FnRef::Variant(1, 1)));
    let arms = vec![arm(vec![Tag::User(1, 1)], vec![Some(5)], var(5))];
    let made = matching(call_value(var(6), vec![int(3)]), arms);
    assert_eq!(said_with(vec![], vec![Stmt::Let(6, ctor)], made), "3");
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(trapped(call_value(hole(TODO), vec![hole(LATER)])), want);
}

#[test]
fn sort_by_asks_with_the_right_item_first() {
    let fns = || {
        let less = func(
            "less",
            &[false; 2],
            2,
            block(vec![], Some(lt(var(0), var(1)))),
        );
        let always = func("always", &[false; 2], 2, block(vec![], Some(flag(true))));
        vec![less, always]
    };
    let row = ROWS
        .iter()
        .position(|(_, r)| builtins::name(r) == "sort_by");
    let sorted_by = |id| {
        let before = e(ExprKind::FnValue(FnRef::Fn(id)));
        let args = vec![io(6, vec![]), Arg::Value(before)];
        let sort = e(ExprKind::Call(Callee::Builtin(row.unwrap()), args, CALL));
        let list = list(vec![int(3), int(1), int(2)]);
        let items = (0..3).map(|i| at(6, vec![item(i)])).collect();
        said_with(
            fns(),
            vec![Stmt::Let(6, list), Stmt::Expr(sort)],
            csv(items),
        )
    };
    assert_eq!(sorted_by(0), "1,2,3");
    assert_eq!(sorted_by(1), "2,1,3");
}

// Blocks and statements

#[test]
fn if_gives_the_value_of_its_branch() {
    let pick = |c| branch(c, int(1), Some(int(2)));
    assert_eq!(said(pick(flag(true))), "1");
    assert_eq!(said(pick(flag(false))), "2");
    let chain = branch(
        flag(false),
        int(1),
        Some(branch(flag(false), int(2), Some(int(3)))),
    );
    assert_eq!(said(chain), "3");
    let stmts = vec![
        Stmt::Let(1, int(0)),
        when(flag(false), vec![set(1, vec![], int(9))]),
    ];
    assert_eq!(said_with(vec![], stmts, var(1)), "0");
}

#[test]
fn while_runs_until_its_condition_fails_or_it_breaks() {
    let body = block(
        vec![
            add_to(1, int(1)),
            when(eq(var(1), int(3)), vec![Stmt::Continue]),
            when(bin(Bin::Order(Cmp::Gt), var(1), int(6)), vec![Stmt::Break]),
            add_to(2, var(1)),
        ],
        None,
    );
    let stmts = vec![
        Stmt::Let(1, int(0)),
        Stmt::Let(2, int(0)),
        Stmt::While(lt(var(1), int(10)), body),
    ];
    assert_eq!(said_with(vec![], stmts, var(2)), "18");
}

#[test]
fn for_walks_a_list_or_a_range() {
    let total = |walk, extra: Vec<Stmt>| {
        let mut body = extra;
        body.push(add_to(2, var(3)));
        let stmts = vec![Stmt::Let(2, int(0)), Stmt::For(3, walk, block(body, None))];
        said_with(vec![], stmts, var(2))
    };
    let items = || Walk::List(list(vec![int(1), int(2), int(3), int(4)]));
    let range = |a, b| Walk::Range(int(a), int(b));
    assert_eq!(total(items(), vec![]), "10");
    assert_eq!(
        total(items(), vec![when(eq(var(3), int(3)), vec![Stmt::Break])]),
        "3"
    );
    assert_eq!(
        total(
            items(),
            vec![when(eq(var(3), int(2)), vec![Stmt::Continue])]
        ),
        "8"
    );
    assert_eq!(total(range(0, 5), vec![]), "10");
    assert_eq!(total(range(3, 3), vec![]), "0");
    assert_eq!(total(range(4, 1), vec![]), "0");
    let top = Stmt::For(3, range(MAX - 2, MAX), block(vec![add_to(2, int(1))], None));
    assert_eq!(
        said_with(vec![], vec![Stmt::Let(2, int(0)), top], var(2)),
        "2"
    );
}

#[test]
fn break_leaves_the_innermost_loop_only() {
    let inner = Stmt::For(
        4,
        Walk::Range(int(0), int(5)),
        block(vec![Stmt::Break], None),
    );
    let outer = block(vec![inner, add_to(2, int(1))], None);
    let stmts = vec![
        Stmt::Let(2, int(0)),
        Stmt::For(3, Walk::Range(int(0), int(3)), outer),
    ];
    assert_eq!(said_with(vec![], stmts, var(2)), "3");
}

#[test]
fn return_leaves_the_function_from_inside_a_loop() {
    let first_even = || {
        let even = eq(arith(Arith::Rem, var(1), int(2)), int(0));
        let found = when(even, vec![Stmt::Return(Some(var(1)))]);
        let search = Stmt::For(1, Walk::List(var(0)), block(vec![found], None));
        func(
            "first_even",
            &[false],
            2,
            block(vec![search], Some(int(-1))),
        )
    };
    let find = |xs: Vec<i64>| call(0, vec![list(xs.into_iter().map(int).collect())]);
    assert_eq!(
        said_with(vec![first_even()], vec![], find(vec![1, 3, 4, 6])),
        "4"
    );
    assert_eq!(
        said_with(vec![first_even()], vec![], find(vec![1, 3])),
        "-1"
    );
}

// Assignment

#[test]
fn assignments_write_in_place() {
    let mut stmts = world();
    stmts.extend([
        set(3, vec![Step::Field(0)], int(6)),
        set(1, vec![item(1)], int(21)),
        set(2, vec![key(text("a"))], int(11)),
        set(2, vec![key(text("c"))], int(3)),
        set(3, vec![Step::Field(2), item(0), item(0)], int(8)),
        put(1, vec![item(0)], Compound::Int(Arith::Mul, OP), int(3)),
        put(
            2,
            vec![key(text("b"))],
            Compound::Int(Arith::Sub, OP),
            int(5),
        ),
    ]);
    let read = vec![
        at(3, vec![Step::Field(0)]),
        at(1, vec![item(1)]),
        at(1, vec![item(0)]),
        at(2, vec![key(text("a"))]),
        at(2, vec![key(text("c"))]),
        at(2, vec![key(text("b"))]),
        at(3, vec![Step::Field(2), item(0), item(0)]),
    ];
    assert_eq!(said_with(vec![], stmts, csv(read)), "6,21,30,11,3,-3,8");
}

#[test]
fn compound_assignment_covers_each_type() {
    let stmts = vec![
        Stmt::Let(1, float("1.5")),
        put(1, vec![], Compound::Float(FloatArith::Mul), float("3.0")),
        Stmt::Let(2, text("ab")),
        put(2, vec![], Compound::Join(JOIN), text("c")),
    ];
    let value = csv(vec![builtin("to_int", vec![var(1)]), var(2)]);
    assert_eq!(said_with(vec![], stmts, value), "4,abc");
}

#[test]
fn assignment_errors() {
    let cases = [
        (
            set(1, vec![item(9)], int(1)),
            "out_of_bounds",
            STEP,
            "index 9, length 3",
        ),
        (
            set(1, vec![item(-1)], int(1)),
            "out_of_bounds",
            STEP,
            "index -1, length 3",
        ),
        (
            set(5, vec![key(text("zz")), key(text("y"))], int(1)),
            "out_of_bounds",
            STEP,
            "key \"zz\"",
        ),
        (
            put(
                2,
                vec![key(text("z"))],
                Compound::Int(Arith::Add, OP),
                int(1),
            ),
            "out_of_bounds",
            STEP,
            "key \"z\"; write m[k] = unwrap_or(get(m, k), 0) + 1",
        ),
        (
            put(6, vec![], Compound::Int(Arith::Add, OP), int(1)),
            "overflow",
            OP,
            "9223372036854775807 + 1",
        ),
        (
            put(6, vec![], Compound::Int(Arith::Div, OP), int(0)),
            "divide_by_zero",
            OP,
            "9223372036854775807 / 0",
        ),
    ];
    for (stmt, kind, site, detail) in cases {
        let mut stmts = world();
        stmts.push(Stmt::Let(5, map(vec![(text("a"), map(vec![]))])));
        stmts.push(Stmt::Let(6, int(MAX)));
        stmts.push(stmt);
        let want = report(kind, site, detail, "update");
        assert_eq!(trapped_with(vec![], stmts, int(0)), want);
    }
}

#[test]
fn an_assignment_runs_operands_and_value_before_it_walks() {
    let want = report("hole", TODO, "??todo", "update");
    let mut stmts = world();
    stmts.push(set(1, vec![item(9)], hole(TODO)));
    assert_eq!(trapped_with(vec![], stmts, int(0)), want);
    let mut stmts = world();
    stmts.push(set(1, vec![Step::Item(hole(TODO), STEP)], hole(LATER)));
    assert_eq!(trapped_with(vec![], stmts, int(0)), want);
}

#[test]
fn strings_stop_growing_at_the_cap() {
    let doubling = |times, step| vec![Stmt::Let(1, text("x")), repeat(2, times, vec![step])];
    let join = || put(1, vec![], Compound::Join(JOIN), var(1));
    let plus = || set(1, vec![], bin(Bin::Join(JOIN), var(1), var(1)));
    for step in [join, plus] {
        assert_eq!(
            said_with(vec![], doubling(24, step()), text("done")),
            "done"
        );
        let stmts = doubling(25, step());
        assert_eq!(
            trapped_with(vec![], stmts, text("done")),
            bare("memory", JOIN)
        );
    }
    let glued = |tail| e(ExprKind::Interp(vec![var(1), text(tail)], INTERP));
    assert_eq!(
        said_with(vec![], doubling(24, join()), eq(glued(""), var(1))),
        "true"
    );
    let stmts = doubling(24, join());
    assert_eq!(
        trapped_with(vec![], stmts, glued("y")),
        bare("memory", INTERP)
    );
}

// Calls

/// `down(n)` calls itself n times, through a few levels of expression.
fn down() -> Func {
    let next = call(0, vec![arith(Arith::Sub, var(0), int(1))]);
    let body = branch(eq(var(0), int(0)), int(0), Some(sum(int(0), next)));
    func("down", &[false], 1, block(vec![], Some(body)))
}

#[test]
fn the_call_depth_limit_is_10000() {
    assert_eq!(
        said_with(vec![down()], vec![], call(0, vec![int(9998)])),
        "0"
    );
    let over = trapped_with(vec![down()], vec![], call(0, vec![int(9999)]));
    assert_eq!(over, bare("stack", CALL));
}

#[test]
fn expressions_nest_past_one_stack_segment() {
    let sum = (0..3_000).fold(int(0), |x, _| arith(Arith::Add, int(1), x));
    assert_eq!(said(sum), "3000");
}

#[test]
fn a_trap_names_the_frames_it_leaves() {
    let step = |name, next| func(name, &[], 0, block(vec![], Some(call(next, vec![]))));
    let fault = func(
        "c",
        &[],
        0,
        block(vec![], Some(arith(Arith::Div, int(7), int(0)))),
    );
    let fns = vec![step("a", 1), step("b", 2), fault];
    let chain = "c, called from b, called from a, called from update";
    let want = report("divide_by_zero", OP, "7 / 0", chain);
    assert_eq!(trapped_with(fns, vec![], call(0, vec![])), want);
}

/// `deep(n)` calls itself n times, then divides by zero.
fn deep() -> Func {
    let fault = arith(Arith::Div, int(1), int(0));
    let next = call(0, vec![arith(Arith::Sub, var(0), int(1))]);
    let body = branch(eq(var(0), int(0)), fault, Some(next));
    func("deep", &[false], 1, block(vec![], Some(body)))
}

#[test]
fn a_report_names_at_most_eight_frames() {
    let names = |n: usize| vec!["deep"; n].join(", called from ");
    let eight = format!("{}, called from update", names(7));
    let want = report("divide_by_zero", OP, "1 / 0", &eight);
    assert_eq!(
        trapped_with(vec![deep()], vec![], call(0, vec![int(6)])),
        want
    );
    let more = format!("{}, ...", names(8));
    let want = report("divide_by_zero", OP, "1 / 0", &more);
    assert_eq!(
        trapped_with(vec![deep()], vec![], call(0, vec![int(7)])),
        want
    );
}

#[test]
fn a_detail_is_cut_at_256_bytes_on_a_character() {
    let mut p = plain(vec![], vec![], hole(TODO));
    p.sites[TODO].hole = Some("€".repeat(100));
    let detail = format!("??{}", "€".repeat(84));
    assert_eq!(go(&p).1, report("hole", TODO, &detail, "update"));
}

#[test]
fn assert_and_unreachable_report_one_line() {
    for name in ["assert", "unreachable"] {
        let args = if name == "assert" {
            vec![flag(false)]
        } else {
            vec![]
        };
        let stmts = vec![Stmt::Expr(builtin(name, args))];
        assert_eq!(trapped_with(vec![], stmts, int(0)), bare("assert", CALL));
    }
    let stmts = vec![Stmt::Expr(builtin("assert", vec![flag(true)]))];
    assert_eq!(said_with(vec![], stmts, int(0)), "0");
}

#[test]
fn abs_to_int_and_from_code() {
    assert_eq!(said(builtin("abs", vec![int(-5)])), "5");
    assert_eq!(truncated(builtin("abs", vec![float("-2.5")])), "2");
    assert_eq!(said(builtin("from_code", vec![int(233)])), "é");
    let huge = float("100000000000000000000.0");
    let edge = float("9223372036854775807.0");
    let cases = [
        (
            builtin("abs", vec![int(MIN)]),
            "overflow",
            "abs(-9223372036854775808)",
        ),
        (
            builtin("to_int", vec![nan()]),
            "bad_conversion",
            "to_int(NaN)",
        ),
        (
            builtin("to_int", vec![huge]),
            "bad_conversion",
            "to_int(100000000000000000000.0)",
        ),
        (
            builtin("to_int", vec![edge]),
            "bad_conversion",
            "to_int(9223372036854776000.0)",
        ),
        (
            builtin("from_code", vec![int(1_114_112)]),
            "bad_conversion",
            "from_code(1114112)",
        ),
        (
            builtin("from_code", vec![int(55_296)]),
            "bad_conversion",
            "from_code(55296)",
        ),
        (
            builtin("from_code", vec![int(-1)]),
            "bad_conversion",
            "from_code(-1)",
        ),
    ];
    for (value, kind, detail) in cases {
        assert_eq!(trapped(value), report(kind, CALL, detail, "update"));
    }
}

#[test]
fn an_unimplemented_built_in_is_an_internal_error() {
    let (out, err, status) = go(&plain(vec![], vec![], builtin("upper", vec![text("a")])));
    assert_eq!(out, "");
    assert_eq!(
        err,
        "light: internal error: built-in upper is not implemented yet\n"
    );
    assert_eq!(status, 4);
}

#[test]
fn an_ill_typed_program_is_one_internal_error_line() {
    let (out, err, status) = go(&plain(vec![], vec![], arith(Arith::Add, text("a"), int(1))));
    assert_eq!((out.as_str(), status), ("", 4));
    let reason = err.strip_prefix("light: internal error: ").expect(&err);
    let reason = reason.strip_suffix('\n').expect(&err);
    assert!(!reason.is_empty() && !reason.contains('\n'), "{err:?}");
}

// Argument passing

fn io(local: usize, steps: Vec<Step>) -> Arg {
    Arg::Inout(place(local, steps))
}

#[test]
fn inout_arguments_are_written_back() {
    let bump = func("bump", &[true], 1, block(vec![add_to(0, int(10))], None));
    let first = vec![
        set(0, vec![item(0)], int(100)),
        Stmt::Return(None),
        set(0, vec![item(0)], int(-1)),
    ];
    let first = func("first", &[true], 1, block(first, None));
    let mut stmts = world();
    stmts.extend([
        Stmt::Expr(call_io(0, vec![io(1, vec![item(1)])])),
        Stmt::Expr(call_io(1, vec![io(1, vec![])])),
        Stmt::Expr(call_io(0, vec![io(3, vec![Step::Field(0)])])),
        Stmt::Expr(call_io(0, vec![io(2, vec![key(text("a"))])])),
    ]);
    let read = vec![
        at(1, vec![item(0)]),
        at(1, vec![item(1)]),
        at(3, vec![Step::Field(0)]),
        at(2, vec![key(text("a"))]),
    ];
    assert_eq!(
        said_with(vec![bump, first], stmts, csv(read)),
        "100,30,15,11"
    );
}

#[test]
fn an_inout_place_is_walked_before_the_body_runs() {
    let touch = func("touch", &[true], 1, block(vec![], Some(hole(TODO))));
    let stmts = vec![Stmt::Let(2, map(vec![]))];
    let call = call_io(0, vec![io(2, vec![key(text("q"))])]);
    let want = report("out_of_bounds", STEP, "key \"q\"", "update");
    assert_eq!(trapped_with(vec![touch], stmts, call), want);
}

#[test]
fn arguments_run_left_to_right_before_inout_places_are_walked() {
    let f = || func("f", &[true, false], 2, block(vec![], None));
    let hole_first = vec![
        io(1, vec![Step::Item(hole(TODO), STEP)]),
        Arg::Value(hole(LATER)),
    ];
    let walk_first = vec![io(1, vec![item(9)]), Arg::Value(hole(LATER))];
    let stmts = || vec![Stmt::Let(1, list(vec![int(1)]))];
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(
        trapped_with(vec![f()], stmts(), call_io(0, hole_first)),
        want
    );
    let want = report("hole", LATER, "??later", "update");
    assert_eq!(
        trapped_with(vec![f()], stmts(), call_io(0, walk_first)),
        want
    );
}

#[test]
fn a_list_passed_by_name_stays_whole_for_the_caller() {
    let ends = arith(Arith::Add, at(0, vec![item(0)]), at(1, vec![item(2)]));
    let ends = func("ends", &[false, false], 2, block(vec![], Some(ends)));
    let keep = func("keep", &[false], 1, block(vec![], Some(var(0))));
    let stmts = vec![
        Stmt::Let(1, list(vec![int(1), int(2), int(3)])),
        Stmt::Let(2, call(1, vec![var(1)])),
    ];
    let read = vec![
        call(0, vec![var(1), var(1)]),
        at(1, vec![item(1)]),
        at(2, vec![item(2)]),
    ];
    assert_eq!(said_with(vec![ends, keep], stmts, csv(read)), "4,2,3");
}

// Values

fn nested(depth: usize, wrap: fn(Value) -> Value) -> Value {
    (0..depth).fold(Value::Int(Int::new(1)), |v, _| wrap(v))
}

const WRAPS: [fn(Value) -> Value; 4] = [
    |v| Value::List(Items(vec![v])),
    |v| Value::Record(Items(vec![v])),
    |v| Value::Variant(1, Items(vec![v])),
    |v| Value::Map(Entries([(Key::Int(Int::new(1)), v)].into())),
];

#[test]
fn a_deep_value_drops_without_recursing() {
    for wrap in WRAPS {
        drop(nested(1_000_000, wrap));
    }
}

#[test]
fn copying_or_comparing_past_10000_levels_gives_up() {
    on_big_stack(|| {
        for wrap in WRAPS {
            assert!(nested(10_000, wrap).copy().is_some());
            assert!(nested(10_001, wrap).copy().is_none());
            let (a, b) = (nested(10_000, wrap), nested(10_000, wrap));
            assert_eq!(equals(&a, &b), Some(true));
            let (a, b) = (nested(10_001, wrap), nested(10_001, wrap));
            assert_eq!(equals(&a, &b), None);
        }
    })
    .unwrap();
}

// The app

#[test]
fn print_then_exit() {
    let p = light(true, vec![print(text("let there be light")), exit()]);
    assert_eq!(go(&p), ("let there be light\n".into(), "".into(), 0));
}

#[test]
fn empty_print_writes_one_line_feed() {
    let p = light(true, vec![print(text("")), exit()]);
    assert_eq!(go(&p), ("\n".into(), "".into(), 0));
}

#[test]
fn prints_in_order() {
    let p = light(true, vec![print(text("a")), print(text("b")), exit()]);
    assert_eq!(go(&p), ("a\nb\n".into(), "".into(), 0));
}

#[test]
fn exit_ends_the_list_untested() {
    let p = light(false, vec![exit(), print(text("x"))]);
    assert_eq!(go(&p), ("".into(), "".into(), 0));
}

#[test]
fn a_list_without_exit_stops_but_keeps_its_output() {
    let ended = "light: ended without Exit\n";
    assert_eq!(go(&light(true, vec![])), ("".into(), ended.into(), 3));
    let p = light(true, vec![print(text("a"))]);
    assert_eq!(go(&p), ("a\n".into(), ended.into(), 3));
}

#[test]
fn a_missing_grant_refuses_and_drops_the_rest() {
    let p = light(false, vec![print(text("x")), print(text("y")), exit()]);
    let want = "light: effect Print refused: grant Console is missing\n";
    assert_eq!(go(&p), ("".into(), want.into(), 3));
}

#[test]
fn the_state_is_built_once_and_passed_first() {
    let state = at(0, vec![Step::Field(0)]);
    let effects = list(vec![print(show(sum(state, int(1)))), exit()]);
    let update = func("update", &[false, false], 2, block(vec![], Some(effects)));
    let mut p = program(vec![], update);
    p.app.as_mut().unwrap().state = Some(record(vec![int(41)]));
    assert_eq!(go(&p), ("42\n".into(), "".into(), 0));
}

/// A handler that exits, for an app whose start variant has fields called `names`.
fn needing(names: &[&str], args: &[&OsStr]) -> Out {
    let mut p = light(true, vec![exit()]);
    let field = |name: &&str| Field {
        name: name.to_string(),
        ty: 3,
    };
    p.enums[0].variants[0].fields = names.iter().map(field).collect();
    go_with(&p, args)
}

#[test]
fn a_wrong_argument_count_gets_the_usage_line() {
    let none: [&OsStr; 0] = [];
    let one = "light: expected 1 argument (text), got 0\n";
    assert_eq!(needing(&["text"], &none), ("".into(), one.into(), 2));
    let two = "light: expected 2 arguments (a, b), got 1\n";
    assert_eq!(
        needing(&["a", "b"], &["x".as_ref()]),
        ("".into(), two.into(), 2)
    );
    let extra = "light: expected 0 arguments, got 2\n";
    assert_eq!(
        needing(&[], &["x".as_ref(), "y".as_ref()]),
        ("".into(), extra.into(), 2)
    );
}

#[test]
fn arguments_fill_the_start_fields_by_position() {
    assert_eq!(
        needing(&["a", "b"], &["x".as_ref(), "y".as_ref()]),
        ("".into(), "".into(), 0)
    );
}

#[test]
fn an_argument_that_is_not_text_names_its_position() {
    use std::os::unix::ffi::OsStrExt;
    let bad = OsStr::from_bytes(&[0xff, 0x61]);
    let want = "light: argument 2 is not text\n";
    assert_eq!(
        needing(&["a", "b"], &["x".as_ref(), bad]),
        ("".into(), want.into(), 2)
    );
}

#[test]
fn exit_with_ends_with_its_status() {
    for status in [0, 5, 7, 255] {
        let p = light(false, vec![exit_with(status), print(text("x"))]);
        assert_eq!(go(&p), ("".into(), "".into(), status as u8), "{status}");
    }
}

#[test]
fn exit_with_outside_its_range_is_a_stop_after_the_earlier_effects() {
    for status in [1, 2, 3, 4, 256, -1, i64::MAX] {
        let p = light(true, vec![print(text("p")), exit_with(status), exit()]);
        let want = format!("light: effect ExitWith failed: status {status} is not 0 or 5 to 255\n");
        assert_eq!(go(&p), ("p\n".into(), want, 3), "{status}");
    }
}

#[test]
fn an_empty_queue_without_exit_is_the_fourth_stop() {
    let p = light(true, vec![print(text("p"))]);
    assert_eq!(
        go(&p),
        ("p\n".into(), "light: ended without Exit\n".into(), 3)
    );
}

#[test]
fn a_trap_in_the_handler_performs_no_effect() {
    let effects = list(vec![print(text("x")), exit()]);
    let body = block(vec![Stmt::Expr(hole(TODO))], Some(effects));
    let p = program(vec![], func("update", &[false], 1, body));
    let want = report("hole", TODO, "??todo", "update");
    assert_eq!(go(&p), ("".into(), want, 3));
}

/// Accepts `room` bytes one call at a time, then fails with `kind`; logs its calls.
struct Faulty {
    room: usize,
    kind: ErrorKind,
    log: Vec<String>,
}

impl Write for Faulty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.room == 0 {
            return Err(self.kind.into());
        }
        self.room -= 1;
        self.log.push(String::from_utf8_lossy(&buf[..1]).into());
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log.push("flush".into());
        Ok(())
    }
}

fn faulty(room: usize, kind: ErrorKind, effects: Vec<Expr>) -> (Faulty, String, u8) {
    let mut out = Faulty {
        room,
        kind,
        log: vec![],
    };
    let mut err = vec![];
    let status = run(&light(true, effects), &[], &mut out, &mut err);
    (out, String::from_utf8(err).unwrap(), status)
}

#[test]
fn short_writes_are_continued_and_nothing_is_flushed() {
    let effects = vec![print(text("ab")), print(text("c")), exit()];
    let (out, err, status) = faulty(9, ErrorKind::Other, effects);
    assert_eq!(out.log, ["a", "b", "\n", "c", "\n"]);
    assert_eq!((err.as_str(), status), ("", 0));
}

#[test]
fn failed_write_reasons() {
    let reasons = [
        (ErrorKind::StorageFull, "no space left on device"),
        (ErrorKind::QuotaExceeded, "no space left on device"),
        (ErrorKind::BrokenPipe, "broken pipe"),
        (ErrorKind::PermissionDenied, "write error"),
    ];
    for (kind, reason) in reasons {
        let (_, err, status) = faulty(0, kind, vec![print(text("x")), exit()]);
        assert_eq!(err, format!("light: effect Print failed: {reason}\n"));
        assert_eq!(status, 3);
    }
}

#[test]
fn a_failed_write_drops_the_rest_and_keeps_the_bytes_written() {
    let effects = vec![print(text("a")), print(text("b")), exit()];
    let (out, err, status) = faulty(2, ErrorKind::BrokenPipe, effects);
    assert_eq!(out.log, ["a", "\n"]);
    assert_eq!(err, "light: effect Print failed: broken pipe\n");
    assert_eq!(status, 3);
}

#[test]
fn a_write_that_accepts_nothing_is_a_failure() {
    struct Zero;
    impl Write for Zero {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut err = vec![];
    let status = run(
        &light(true, vec![print(text("x")), exit()]),
        &[],
        &mut Zero,
        &mut err,
    );
    assert_eq!(err, b"light: effect Print failed: write error\n");
    assert_eq!(status, 3);
}

#[test]
fn a_failed_standard_error_changes_no_status() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(run(&light(true, vec![]), &[], &mut vec![], &mut Broken), 3);
}
