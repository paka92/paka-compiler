//! Programs lowered and compared with core written by hand (docs/core.md).

use super::*;
use crate::core_form::{
    Arg, Arith, Arm, Bin, Block, Callee, Cmp, Compound, FloatArith, FnRef, Place, Root, Step, Stmt,
    TrapSite, Un, Walk,
};
use crate::{body, builtins, parser::parse, resolve::resolve};
use paka_rt::ops::{Float, Int};
use std::collections::BTreeSet;

const LIGHT: &[&str] = &[
    "enum Msg { Start }",
    "",
    "fn update(m: Msg) -> List<Effect<Msg>> {",
    "  [Print(\"let there be light\"), Exit]",
    "}",
    "",
    "app light {",
    "  start:  Start",
    "  update: update",
    "  grants: [Console]",
    "}",
];

fn lowered(lines: &[&str]) -> Program {
    let source = lines.join("\n") + "\n";
    let program = parse("main.paka", source.as_bytes()).unwrap();
    let resolved = resolve("main.paka", &program).unwrap();
    let mut table = Table::build("main.paka", &program, &resolved);
    let bodies = body::check("main.paka", &program, &resolved, &mut table);
    assert!(
        bodies.defs.iter().all(Option::is_none),
        "the program checks"
    );
    let source = Source::file("main.paka", source.as_bytes().to_vec());
    lower(&source, &program, &resolved, &table, &bodies).0
}

/// A program of the function `f` and the definitions after it.
fn with(lines: &[&str], rest: &[&str]) -> Program {
    lowered(&[lines, rest].concat())
}

fn ex(ty: Ty, kind: ExprKind) -> Expr {
    Expr { ty: Some(ty), kind }
}

fn int(ty: Ty, text: &str) -> Expr {
    ex(ty, ExprKind::Int(Int::parse(text).unwrap()))
}

fn float(ty: Ty, text: &str) -> Expr {
    ex(ty, ExprKind::Float(Float::parse(text).unwrap()))
}

fn text(ty: Ty, s: &str) -> Expr {
    ex(ty, ExprKind::Str(s.into()))
}

fn flag(ty: Ty, b: bool) -> Expr {
    ex(ty, ExprKind::Bool(b))
}

fn place(local: Local) -> Place {
    Place {
        root: Root::Local(local),
        steps: vec![],
    }
}

fn read(ty: Ty, local: Local) -> Expr {
    ex(ty, ExprKind::Place(place(local)))
}

fn binary(ty: Ty, op: Bin, left: Expr, right: Expr) -> Expr {
    ex(ty, ExprKind::Binary(op, Box::new(left), Box::new(right)))
}

fn call(ty: Option<Ty>, callee: Callee, args: Vec<Expr>, site: Site) -> Expr {
    let args = args.into_iter().map(Arg::Value).collect();
    Expr {
        ty,
        kind: ExprKind::Call(callee, args, site),
    }
}

fn variant(ty: Ty, tag: Tag, fields: Vec<Expr>) -> Expr {
    ex(ty, ExprKind::Variant(tag, fields))
}

fn stmts(stmts: Vec<Stmt>) -> Block {
    Block { stmts, value: None }
}

fn valued(stmts: Vec<Stmt>, value: Expr) -> Block {
    Block {
        stmts,
        value: Some(Box::new(value)),
    }
}

fn param(ty: Ty) -> Param {
    Param { ty, inout: false }
}

/// The function `f` of a program, with its parameters not `inout`.
fn func(site: Site, params: &[Ty], locals: &[Ty], result: Option<Ty>, body: Block) -> Fn {
    Fn {
        name: "f".into(),
        site,
        params: params.iter().copied().map(param).collect(),
        locals: locals.into(),
        result,
        body,
        claims: vec![],
    }
}

/// The built-in whose row is written `signature`.
fn row(signature: &str) -> Callee {
    let at = builtins::ROWS.iter().position(|r| r.1 == signature);
    Callee::Builtin(at.unwrap())
}

fn site(line: usize, col: usize) -> TrapSite {
    TrapSite {
        line,
        col,
        hole: None,
    }
}

/// The first function, with the types and sites the lowering gave.
fn first(core: &Program) -> (&Fn, &[Type], &[TrapSite]) {
    (&core.fns[0], &core.types, &core.sites)
}

#[test]
fn light_paka_lowers() {
    let core = lowered(LIGHT);
    let print = variant(
        1,
        Tag::Effect(core_form::Effect::Print),
        vec![text(3, "let there be light")],
    );
    let exit = variant(1, Tag::Effect(core_form::Effect::Exit), vec![]);
    let list = ex(2, ExprKind::List(vec![print, exit]));
    let update = Fn {
        name: "update".into(),
        ..func(0, &[0], &[0], Some(2), valued(vec![], list))
    };
    let by_hand = Program {
        files: vec![("main.paka".into(), 1)],
        types: vec![Type::Enum(0), Type::Effect(0), Type::List(1), Type::String],
        records: vec![],
        enums: vec![Enum {
            name: "Msg".into(),
            variants: vec![Variant {
                name: "Start".into(),
                fields: vec![],
            }],
        }],
        fns: vec![update],
        examples: vec![],
        ui_examples: vec![],
        foreign: vec![],
        libraries: vec![],
        app: Some(App {
            name: "light".into(),
            state: None,
            start: Some(0),
            view: None,
            update: 0,
            grants: BTreeSet::from([Grant::Console]),
            foreign: BTreeSet::new(),
        }),
        sites: vec![site(3, 4)],
    };
    assert_eq!(lowered(LIGHT), by_hand);
    assert_eq!(core, by_hand);
}

#[test]
fn statements() {
    let core = lowered(&[
        "fn f(xs: List<Int>) {",
        "  var a = 1",
        "  a = 2",
        "  a += 3",
        "  for x in xs { a = x }",
        "  for i in 0..<3 { continue }",
        "  while a < 9 { break }",
        "  _ = a",
        "}",
    ]);
    let set = |value| Stmt::Assign(place(1), None, value);
    let body = stmts(vec![
        Stmt::Let(1, int(0, "1")),
        set(int(0, "2")),
        Stmt::Assign(place(1), Some(Compound::Int(Arith::Add, 1)), int(0, "3")),
        Stmt::For(2, Walk::List(read(1, 0)), stmts(vec![set(read(0, 2))])),
        Stmt::For(
            3,
            Walk::Range(int(0, "0"), int(0, "3")),
            stmts(vec![Stmt::Continue]),
        ),
        Stmt::While(
            binary(2, Bin::Order(Cmp::Lt), read(0, 1), int(0, "9")),
            stmts(vec![Stmt::Break]),
        ),
        Stmt::Expr(read(0, 1)),
    ]);
    let by_hand = func(0, &[1], &[1, 0, 0, 0], None, body);
    let types = [Type::Int, Type::List(0), Type::Bool];
    assert_eq!(
        first(&core),
        (&by_hand, &types[..], &[site(1, 4), site(4, 5)][..])
    );
}

#[test]
fn return_and_calls_as_statements() {
    let core = with(
        &[
            "fn f(n: Int) -> Int {",
            "  shout(\"a\")",
            "  if n > 0 { return n }",
            "  n + 1",
            "}",
            "",
        ],
        &["fn shout(text: String) {", "  _ = text", "}"],
    );
    let shout = call(None, Callee::Fn(1), vec![text(1, "a")], 1);
    let early = Stmt::Return(Some(read(0, 0)));
    let cond = binary(2, Bin::Order(Cmp::Gt), read(0, 0), int(0, "0"));
    let guard = Expr {
        ty: None,
        kind: ExprKind::If(Box::new(cond), stmts(vec![early]), None),
    };
    let add = Bin::Int(Arith::Add, 2);
    let sum = binary(0, add, read(0, 0), int(0, "1"));
    let body = valued(vec![Stmt::Expr(shout), Stmt::Expr(guard)], sum);
    let (f, types, sites) = first(&core);
    assert_eq!(f, &func(0, &[0], &[0], Some(0), body));
    assert_eq!(types, [Type::Int, Type::String, Type::Bool]);
    assert_eq!(sites[..3], [site(1, 4), site(2, 3), site(4, 5)]);
}

fn discard(e: Expr) -> Stmt {
    Stmt::Expr(e)
}

#[test]
fn literals_interpolation_and_holes() {
    let core = lowered(&[
        "fn f(n: Int) {",
        "  _ = 1.5",
        "  _ = true",
        "  _ = \"a{n}b\"",
        "  ??todo",
        "}",
    ]);
    let float = float(1, "1.5");
    let parts = vec![text(3, "a"), read(0, 0), text(3, "b")];
    let body = stmts(vec![
        discard(float),
        discard(flag(2, true)),
        discard(ex(3, ExprKind::Interp(parts, 1))),
        discard(Expr {
            ty: None,
            kind: ExprKind::Hole(2),
        }),
    ]);
    let hole = TrapSite {
        line: 5,
        col: 3,
        hole: Some("todo".into()),
    };
    let types = [Type::Int, Type::Float, Type::Bool, Type::String];
    let (f, found, sites) = first(&core);
    assert_eq!(f, &func(0, &[0], &[0], None, body));
    assert_eq!(found, types);
    assert_eq!(sites, [site(1, 4), site(4, 7), hole]);
}

#[test]
fn places_with_every_step() {
    let core = lowered(&[
        "type Pt { x: Int = 0, y: Int }",
        "",
        "fn f(xs: List<Int>, m: Map<String, Int>, s: String, p: Pt) {",
        "  _ = p.y",
        "  _ = xs[0]",
        "  _ = m[\"k\"]",
        "  _ = xs[1..<2]",
        "  _ = s[0]",
        "  _ = list()[0]",
        "}",
        "",
        "fn list() -> List<Int> {",
        "  [1]",
        "}",
    ]);
    let step = |root, step| {
        let place = Place {
            root,
            steps: vec![step],
        };
        ExprKind::Place(place)
    };
    let local = |l| Root::Local(l);
    let body = stmts(vec![
        discard(ex(0, step(local(3), Step::Field(1)))),
        discard(ex(0, step(local(0), Step::Item(int(0, "0"), 1)))),
        discard(ex(0, step(local(1), Step::Key(text(2, "k"), 2)))),
        discard(ex(
            1,
            step(local(0), Step::Slice(int(0, "1"), int(0, "2"), 3)),
        )),
        discard(ex(2, step(local(2), Step::Item(int(0, "0"), 4)))),
        discard(ex(
            0,
            step(
                Root::Value(Box::new(call(Some(1), Callee::Fn(1), vec![], 5))),
                Step::Item(int(0, "0"), 6),
            ),
        )),
    ]);
    let (f, types, sites) = first(&core);
    assert_eq!(f, &func(0, &[1, 3, 2, 4], &[1, 3, 2, 4], None, body));
    let want = [Type::Int, Type::List(0), Type::String, Type::Map(2, 0)];
    assert_eq!(types[..4], want);
    let list = [
        site(3, 4),
        site(5, 9),
        site(6, 8),
        site(7, 9),
        site(8, 8),
        site(9, 7),
        site(9, 13),
    ];
    assert_eq!(sites[..7], list);
}

#[test]
fn constructors() {
    let core = lowered(&[
        "type Pt { x: Int = 0, y: Int }",
        "enum Shape { Dot, Circle(r: Float) }",
        "",
        "fn f(n: Int) {",
        "  _ = Pt { y: n }",
        "  _ = [n, 1]",
        "  _ = [\"k\": n]",
        "  _ = Circle(1.5)",
        "  _ = Dot",
        "  _ = inc",
        "  _ = Circle",
        "}",
        "",
        "fn inc(n: Int) -> Int {",
        "  n + 1",
        "}",
    ]);
    let float = float(1, "1.5");
    let body = stmts(vec![
        discard(ex(2, ExprKind::Record(0, vec![int(0, "0"), read(0, 0)]))),
        discard(ex(3, ExprKind::List(vec![read(0, 0), int(0, "1")]))),
        discard(ex(5, ExprKind::Map(vec![(text(4, "k"), read(0, 0))]))),
        discard(variant(6, Tag::User(0, 1), vec![float])),
        discard(variant(6, Tag::User(0, 0), vec![])),
        discard(ex(7, ExprKind::FnValue(FnRef::Fn(1)))),
        discard(ex(8, ExprKind::FnValue(FnRef::Variant(0, 1)))),
    ]);
    let (f, types, _) = first(&core);
    assert_eq!(f, &func(0, &[0], &[0], None, body));
    let want = [
        Type::Int,
        Type::Float,
        Type::Record(0),
        Type::List(0),
        Type::String,
        Type::Map(4, 0),
        Type::Enum(0),
        Type::Fn(vec![0], Some(0)),
        Type::Fn(vec![1], Some(6)),
    ];
    assert_eq!(types, want);
}

#[test]
fn operators() {
    let core = lowered(&[
        "fn f(a: Int, x: Float, s: String, b: Bool) {",
        "  _ = a + 1",
        "  _ = a - 1",
        "  _ = a * 2",
        "  _ = a / 2",
        "  _ = a % 2",
        "  _ = x + 1.5",
        "  _ = x - 1.5",
        "  _ = x * 1.5",
        "  _ = x / 1.5",
        "  _ = s + \"t\"",
        "  _ = a == 1",
        "  _ = a != 1",
        "  _ = a < 1",
        "  _ = a <= 1",
        "  _ = a > 1",
        "  _ = a >= 1",
        "  _ = b && b",
        "  _ = b || b",
        "  _ = -a",
        "  _ = -x",
        "  _ = !b",
        "}",
    ]);
    let ints = |op, site| binary(0, Bin::Int(op, site), read(0, 0), int(0, "1"));
    let ints2 = |op, site| binary(0, Bin::Int(op, site), read(0, 0), int(0, "2"));
    let floats = |op| binary(1, Bin::Float(op), read(1, 1), float(1, "1.5"));
    let test = |op| binary(3, op, read(0, 0), int(0, "1"));
    let both = |op| binary(3, op, read(3, 3), read(3, 3));
    let body = stmts(
        [
            ints(Arith::Add, 1),
            ints(Arith::Sub, 2),
            ints2(Arith::Mul, 3),
            ints2(Arith::Div, 4),
            ints2(Arith::Rem, 5),
            floats(FloatArith::Add),
            floats(FloatArith::Sub),
            floats(FloatArith::Mul),
            floats(FloatArith::Div),
            binary(2, Bin::Join(6), read(2, 2), text(2, "t")),
            test(Bin::Eq(7)),
            test(Bin::Ne(8)),
            test(Bin::Order(Cmp::Lt)),
            test(Bin::Order(Cmp::Le)),
            test(Bin::Order(Cmp::Gt)),
            test(Bin::Order(Cmp::Ge)),
            both(Bin::And),
            both(Bin::Or),
            ex(0, ExprKind::Unary(Un::Neg(9), Box::new(read(0, 0)))),
            ex(1, ExprKind::Unary(Un::NegFloat, Box::new(read(1, 1)))),
            ex(3, ExprKind::Unary(Un::Not, Box::new(read(3, 3)))),
        ]
        .into_iter()
        .map(discard)
        .collect(),
    );
    let (f, types, sites) = first(&core);
    assert_eq!(f, &func(0, &[0, 1, 2, 3], &[0, 1, 2, 3], None, body));
    assert_eq!(types, [Type::Int, Type::Float, Type::String, Type::Bool]);
    let cols = [
        (1, 4),
        (2, 9),
        (3, 9),
        (4, 9),
        (5, 9),
        (6, 9),
        (11, 9),
        (12, 9),
        (13, 9),
        (20, 7),
    ];
    let found: Vec<_> = sites.iter().map(|s| (s.line, s.col)).collect();
    assert_eq!(found, cols);
}

#[test]
fn calls_of_every_callee() {
    let core = lowered(&[
        "fn f(n: Int, g: fn(Int) -> Int, xs: List<Int>) -> Int {",
        "  var v = n",
        "  bump(&v)",
        "  _ = g(v)",
        "  _ = len(xs)",
        "  _ = max(n, 2)",
        "  _ = Some(n)",
        "  _ = clamp(n, lo: 0, hi: 5)",
        "  v",
        "}",
        "",
        "fn bump(n: inout Int) {",
        "  n += 1",
        "}",
        "",
        "fn clamp(x: Int; lo: Int, hi: Int) -> Int {",
        "  max(lo, min(x, hi))",
        "}",
    ]);
    let bump = Expr {
        ty: None,
        kind: ExprKind::Call(Callee::Fn(1), vec![Arg::Inout(place(3))], 1),
    };
    let by_value = Callee::Value(Box::new(read(1, 1)));
    let body = valued(
        vec![
            Stmt::Let(3, read(0, 0)),
            discard(bump),
            discard(call(Some(0), by_value, vec![read(0, 3)], 2)),
            discard(call(
                Some(0),
                row("len(x: List<T>) -> Int"),
                vec![read(2, 2)],
                3,
            )),
            discard(call(
                Some(0),
                row("max(a: Int, b: Int) -> Int"),
                vec![read(0, 0), int(0, "2")],
                4,
            )),
            discard(variant(3, Tag::Some, vec![read(0, 0)])),
            discard(call(
                Some(0),
                Callee::Fn(2),
                vec![read(0, 0), int(0, "0"), int(0, "5")],
                5,
            )),
        ],
        read(0, 3),
    );
    let (f, types, sites) = first(&core);
    assert_eq!(f, &func(0, &[0, 1, 2], &[0, 1, 2, 0], Some(0), body));
    let want = [
        Type::Int,
        Type::Fn(vec![0], Some(0)),
        Type::List(0),
        Type::Option(0),
    ];
    assert_eq!(types, want);
    let found: Vec<_> = sites.iter().map(|s| (s.line, s.col)).collect();
    assert_eq!(found[..6], [(1, 4), (3, 3), (4, 7), (5, 7), (6, 7), (8, 7)]);
}

#[test]
fn compound_assignment_of_each_kind() {
    let core = lowered(&[
        "fn f(xs: List<Int>, x: Float, s: String) {",
        "  var y = x",
        "  var t = s",
        "  var zs = xs",
        "  y *= 2.5",
        "  t += \"a\"",
        "  zs[0] += 1",
        "}",
    ]);
    let item = Place {
        root: Root::Local(5),
        steps: vec![Step::Item(int(0, "0"), 2)],
    };
    let body = stmts(vec![
        Stmt::Let(3, read(2, 1)),
        Stmt::Let(4, read(3, 2)),
        Stmt::Let(5, read(1, 0)),
        Stmt::Assign(
            place(3),
            Some(Compound::Float(FloatArith::Mul)),
            float(2, "2.5"),
        ),
        Stmt::Assign(place(4), Some(Compound::Join(1)), text(3, "a")),
        Stmt::Assign(item, Some(Compound::Int(Arith::Add, 3)), int(0, "1")),
    ]);
    let (f, types, sites) = first(&core);
    assert_eq!(f, &func(0, &[1, 2, 3], &[1, 2, 3, 2, 3, 1], None, body));
    assert_eq!(types, [Type::Int, Type::List(0), Type::Float, Type::String]);
    assert_eq!(sites, [site(1, 4), site(6, 5), site(7, 5), site(7, 9)]);
}

#[test]
fn if_with_else_if() {
    let core = lowered(&[
        "fn f(n: Int) -> Int {",
        "  if n > 0 {",
        "    1",
        "  } else if n < 0 {",
        "    2",
        "  } else {",
        "    3",
        "  }",
        "}",
    ]);
    let test = |op| binary(1, Bin::Order(op), read(0, 0), int(0, "0"));
    let inner = ExprKind::If(
        Box::new(test(Cmp::Lt)),
        valued(vec![], int(0, "2")),
        Some(valued(vec![], int(0, "3"))),
    );
    let outer = ExprKind::If(
        Box::new(test(Cmp::Gt)),
        valued(vec![], int(0, "1")),
        Some(valued(vec![], ex(0, inner))),
    );
    let (f, types, _) = first(&core);
    assert_eq!(
        f,
        &func(0, &[0], &[0], Some(0), valued(vec![], ex(0, outer)))
    );
    assert_eq!(types, [Type::Int, Type::Bool]);
}

const SHAPE: &str = "enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }";

fn arm(tags: Vec<Tag>, binds: Vec<Option<Local>>, body: Block) -> Arm {
    Arm { tags, binds, body }
}

#[test]
fn match_on_an_enum_binds_fields() {
    let core = lowered(&[
        SHAPE,
        "",
        "fn f(s: Shape) -> Float {",
        "  match s {",
        "    Dot => 0.5",
        "    Circle(r) => r",
        "    Rect(w, _) => w",
        "  }",
        "}",
    ]);
    let arms = vec![
        arm(
            vec![Tag::User(0, 0)],
            vec![],
            valued(vec![], float(0, "0.5")),
        ),
        arm(
            vec![Tag::User(0, 1)],
            vec![Some(1)],
            valued(vec![], read(0, 1)),
        ),
        arm(
            vec![Tag::User(0, 2)],
            vec![Some(2), None],
            valued(vec![], read(0, 2)),
        ),
    ];
    let subject = Box::new(read(1, 0));
    let by_hand = func(
        0,
        &[1],
        &[1, 0, 0],
        Some(0),
        valued(vec![], ex(0, ExprKind::Match(subject, arms))),
    );
    assert_eq!(first(&core).0, &by_hand);
}

#[test]
fn an_arm_with_several_variants_binds_nothing() {
    let core = lowered(&[
        SHAPE,
        "",
        "fn f(s: Shape) -> Int {",
        "  match s {",
        "    Dot | Circle(_) => 1",
        "    Rect(_, _) => 2",
        "  }",
        "}",
    ]);
    let arms = vec![
        arm(
            vec![Tag::User(0, 0), Tag::User(0, 1)],
            vec![],
            valued(vec![], int(2, "1")),
        ),
        arm(
            vec![Tag::User(0, 2)],
            vec![None, None],
            valued(vec![], int(2, "2")),
        ),
    ];
    let subject = Box::new(read(1, 0));
    let by_hand = func(
        0,
        &[1],
        &[1],
        Some(2),
        valued(vec![], ex(2, ExprKind::Match(subject, arms))),
    );
    assert_eq!(first(&core).0, &by_hand);
}

#[test]
fn match_on_an_int_is_a_chain_of_equality_tests() {
    let core = lowered(&[
        "fn f(n: Int) -> String {",
        "  match n {",
        "    0 => \"zero\"",
        "    1 | 2 => \"few\"",
        "    _ => \"many\"",
        "  }",
        "}",
    ]);
    let is = |value: &str, site| binary(2, Bin::Eq(site), read(0, 1), int(0, value));
    let few = binary(2, Bin::Or, is("1", 2), is("2", 3));
    let rest = ExprKind::If(
        Box::new(few),
        valued(vec![], text(1, "few")),
        Some(valued(vec![], text(1, "many"))),
    );
    let chain = ExprKind::If(
        Box::new(is("0", 1)),
        valued(vec![], text(1, "zero")),
        Some(valued(vec![], ex(1, rest))),
    );
    let held = Stmt::Let(1, read(0, 0));
    let whole = ExprKind::If(
        Box::new(flag(2, true)),
        valued(vec![held], ex(1, chain)),
        None,
    );
    let (f, types, sites) = first(&core);
    assert_eq!(
        f,
        &func(0, &[0], &[0, 0], Some(1), valued(vec![], ex(1, whole)))
    );
    assert_eq!(types, [Type::Int, Type::String, Type::Bool]);
    assert_eq!(sites, [site(1, 4), site(3, 5), site(4, 5), site(4, 9)]);
}

#[test]
fn match_on_an_int_tests_a_negative_pattern() {
    let core = lowered(&[
        "fn f(n: Int) -> String {",
        "  match n {",
        "    -1 => \"minus\"",
        "    _ => \"other\"",
        "  }",
        "}",
    ]);
    let test = binary(2, Bin::Eq(1), read(0, 1), int(0, "-1"));
    let chain = ExprKind::If(
        Box::new(test),
        valued(vec![], text(1, "minus")),
        Some(valued(vec![], text(1, "other"))),
    );
    let whole = ExprKind::If(
        Box::new(flag(2, true)),
        valued(vec![Stmt::Let(1, read(0, 0))], ex(1, chain)),
        None,
    );
    let (f, _, sites) = first(&core);
    let body = valued(vec![], ex(1, whole));
    assert_eq!(f, &func(0, &[0], &[0, 0], Some(1), body));
    assert_eq!(sites, [site(1, 4), site(3, 5)]);
}

#[test]
fn a_subject_with_locals_of_its_own_numbers_them_before_the_held_one() {
    let core = lowered(&[
        "fn f(o: Option<Int>) -> Option<String> {",
        "  match o? {",
        "    0 => Some(\"zero\")",
        "    _ => None",
        "  }",
        "}",
    ]);
    let subject = ExprKind::Match(
        Box::new(read(1, 0)),
        vec![
            arm(vec![Tag::Some], vec![Some(1)], valued(vec![], read(0, 1))),
            arm(
                vec![Tag::None],
                vec![],
                stmts(vec![Stmt::Return(Some(variant(3, Tag::None, vec![])))]),
            ),
        ],
    );
    let test = binary(4, Bin::Eq(1), read(0, 2), int(0, "0"));
    let some = variant(3, Tag::Some, vec![text(2, "zero")]);
    let chain = ExprKind::If(
        Box::new(test),
        valued(vec![], some),
        Some(valued(vec![], variant(3, Tag::None, vec![]))),
    );
    let whole = ExprKind::If(
        Box::new(flag(4, true)),
        valued(vec![Stmt::Let(2, ex(0, subject))], ex(3, chain)),
        None,
    );
    let body = valued(vec![], ex(3, whole));
    assert_eq!(core.fns, [func(0, &[1], &[1, 0, 0], Some(3), body)]);
}

#[test]
fn match_on_a_string_and_a_bool_as_statements() {
    let core = with(
        &[
            "fn f(s: String, b: Bool) {",
            "  match s {",
            "    \"a\" => shout(\"x\")",
            "    _ => shout(\"y\")",
            "  }",
            "  match b {",
            "    true => shout(\"t\")",
            "    false => shout(\"f\")",
            "  }",
            "}",
            "",
        ],
        &["fn shout(text: String) {", "  _ = text", "}"],
    );
    let said = |word, site| {
        let call = call(None, Callee::Fn(1), vec![text(0, word)], site);
        stmts(vec![discard(call)])
    };
    let chain = |held, right: Expr, site, yes, no| {
        let test = binary(1, Bin::Eq(site), read(right.ty.unwrap(), held), right);
        ExprKind::If(Box::new(test), yes, Some(no))
    };
    let held = |held, value, chain| {
        let block = stmts(vec![Stmt::Let(held, value), discard(unvalued(chain))]);
        discard(unvalued(ExprKind::If(Box::new(flag(1, true)), block, None)))
    };
    let body = stmts(vec![
        held(
            2,
            read(0, 0),
            chain(2, text(0, "a"), 1, said("x", 2), said("y", 3)),
        ),
        held(
            3,
            read(1, 1),
            chain(3, flag(1, true), 4, said("t", 5), said("f", 6)),
        ),
    ]);
    let (f, types, _) = first(&core);
    assert_eq!(f, &func(0, &[0, 1], &[0, 1, 0, 1], None, body));
    assert_eq!(types, [Type::String, Type::Bool]);
}

fn unvalued(kind: ExprKind) -> Expr {
    Expr { ty: None, kind }
}

#[test]
fn question_mark_on_a_result_and_an_option() {
    let core = lowered(&[
        "fn f(r: Result<Int, String>) -> Result<Int, String> {",
        "  let a = r?",
        "  Ok(a)",
        "}",
        "",
        "fn g(o: Option<Int>) -> Option<Int> {",
        "  let a = o?",
        "  Some(a)",
        "}",
    ]);
    let fail = |tag, fields| Stmt::Return(Some(variant(2, tag, fields)));
    let on_result = ExprKind::Match(
        Box::new(read(2, 0)),
        vec![
            arm(vec![Tag::Ok], vec![Some(1)], valued(vec![], read(0, 1))),
            arm(
                vec![Tag::Err],
                vec![Some(2)],
                stmts(vec![fail(Tag::Err, vec![read(1, 2)])]),
            ),
        ],
    );
    let f = func(
        0,
        &[2],
        &[2, 0, 1, 0],
        Some(2),
        valued(
            vec![Stmt::Let(3, ex(0, on_result))],
            variant(2, Tag::Ok, vec![read(0, 3)]),
        ),
    );
    let on_option = ExprKind::Match(
        Box::new(read(3, 0)),
        vec![
            arm(vec![Tag::Some], vec![Some(1)], valued(vec![], read(0, 1))),
            arm(
                vec![Tag::None],
                vec![],
                stmts(vec![Stmt::Return(Some(variant(3, Tag::None, vec![])))]),
            ),
        ],
    );
    let g = Fn {
        name: "g".into(),
        ..func(
            1,
            &[3],
            &[3, 0, 0],
            Some(3),
            valued(
                vec![Stmt::Let(2, ex(0, on_option))],
                variant(3, Tag::Some, vec![read(0, 2)]),
            ),
        )
    };
    assert_eq!(core.fns, [f, g]);
    let want = [Type::Int, Type::String, Type::Result(0, 1), Type::Option(0)];
    assert_eq!(core.types, want);
}

#[test]
fn match_on_an_option_and_a_result() {
    let core = lowered(&[
        "fn f(o: Option<Int>, r: Result<Int, String>) -> Int {",
        "  let a = match o {",
        "    Some(v) => v",
        "    None => 0",
        "  }",
        "  match r {",
        "    Ok(_) => a",
        "    Err(_) => 1",
        "  }",
        "}",
    ]);
    let first_match = ExprKind::Match(
        Box::new(read(1, 0)),
        vec![
            arm(vec![Tag::Some], vec![Some(2)], valued(vec![], read(0, 2))),
            arm(vec![Tag::None], vec![], valued(vec![], int(0, "0"))),
        ],
    );
    let second = ExprKind::Match(
        Box::new(read(3, 1)),
        vec![
            arm(vec![Tag::Ok], vec![None], valued(vec![], read(0, 3))),
            arm(vec![Tag::Err], vec![None], valued(vec![], int(0, "1"))),
        ],
    );
    let body = valued(vec![Stmt::Let(3, ex(0, first_match))], ex(0, second));
    let (f, types, _) = first(&core);
    assert_eq!(f, &func(0, &[1, 3], &[1, 3, 0, 0], Some(0), body));
    assert_eq!(
        types,
        [Type::Int, Type::Option(0), Type::String, Type::Result(0, 2)]
    );
}

#[test]
fn an_app_with_state_an_inout_handler_and_an_example() {
    let core = lowered(&[
        "type Tally { count: Int = 0, name: String = \"t\" }",
        "enum Msg { Start }",
        "",
        "fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {",
        "  s.count += 1",
        "  [ExitWith(5)]",
        "}",
        "",
        "example one { 1 + 1 == 2 }",
        "",
        "app tally {",
        "  state:  Tally",
        "  start:  Start",
        "  update: update",
        "  grants: [Console, ReadFiles]",
        "}",
    ]);
    let count = Place {
        root: Root::Local(0),
        steps: vec![Step::Field(0)],
    };
    let add = Some(Compound::Int(Arith::Add, 1));
    let exit = variant(
        4,
        Tag::Effect(core_form::Effect::ExitWith),
        vec![int(0, "5")],
    );
    let update = Fn {
        name: "update".into(),
        site: 0,
        params: vec![Param { ty: 2, inout: true }, param(3)],
        locals: vec![2, 3],
        result: Some(5),
        body: valued(
            vec![Stmt::Assign(count, add, int(0, "1"))],
            ex(5, ExprKind::List(vec![exit])),
        ),
        claims: vec![],
    };
    let sum = binary(0, Bin::Int(Arith::Add, 3), int(0, "1"), int(0, "1"));
    let one = Fn {
        name: "one".into(),
        claims: vec![5],
        ..func(
            2,
            &[],
            &[],
            Some(6),
            valued(vec![], binary(6, Bin::Eq(4), sum, int(0, "2"))),
        )
    };
    let fields = vec![int(0, "0"), text(1, "t")];
    let app = App {
        name: "tally".into(),
        state: Some(ex(2, ExprKind::Record(0, fields))),
        start: Some(0),
        view: None,
        update: 0,
        grants: BTreeSet::from([Grant::Console, Grant::ReadFiles]),
        foreign: BTreeSet::new(),
    };
    let by_hand = Program {
        files: vec![("main.paka".into(), 1)],
        types: vec![
            Type::Int,
            Type::String,
            Type::Record(0),
            Type::Enum(0),
            Type::Effect(3),
            Type::List(4),
            Type::Bool,
        ],
        records: vec![core_form::Record {
            name: "Tally".into(),
            fields: vec![
                Field {
                    name: "count".into(),
                    ty: 0,
                },
                Field {
                    name: "name".into(),
                    ty: 1,
                },
            ],
        }],
        enums: vec![Enum {
            name: "Msg".into(),
            variants: vec![Variant {
                name: "Start".into(),
                fields: vec![],
            }],
        }],
        fns: vec![update],
        examples: vec![one],
        ui_examples: vec![],
        foreign: vec![],
        libraries: vec![],
        app: Some(app),
        sites: vec![
            site(4, 4),
            site(5, 11),
            site(9, 9),
            site(9, 17),
            site(9, 21),
            site(9, 21),
        ],
    };
    assert_eq!(core, by_hand);
}

#[test]
fn a_variant_with_fields_a_function_value_and_a_file_effect() {
    let core = lowered(&[
        "enum Msg { Start(path: String), Loaded(text: String), Failed(reason: String) }",
        "",
        "fn update(m: Msg) -> List<Effect<Msg>> {",
        "  [ReadFile(\"a\", then: Loaded, or: Failed)]",
        "}",
    ]);
    let fns = [FnRef::Variant(0, 1), FnRef::Variant(0, 2)];
    let [then, or] = fns.map(|f| ex(4, ExprKind::FnValue(f)));
    let tag = Tag::Effect(core_form::Effect::ReadFile);
    let read_file = variant(2, tag, vec![text(0, "a"), then, or]);
    let body = valued(vec![], ex(3, ExprKind::List(vec![read_file])));
    let update = Fn {
        name: "update".into(),
        ..func(0, &[1], &[1], Some(3), body)
    };
    let (f, types, _) = first(&core);
    assert_eq!(f, &update);
    let want = [
        Type::String,
        Type::Enum(0),
        Type::Effect(1),
        Type::List(2),
        Type::Fn(vec![0], Some(1)),
    ];
    assert_eq!(types, want);
}
