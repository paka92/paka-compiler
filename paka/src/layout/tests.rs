//! Fixture types covering every type form of the reference, with layouts worked out by hand from
//! docs/decisions/M4-05.md section 8.

use super::*;
use crate::source::Source;

const FIXTURE: &str = "\
type Empty {}
type Point { x: Int, y: Float }
type Named { name: String, tags: List<String>, seen: Map<String, Int>, on: Bool, f: fn(Int) -> Int }
enum Shape { Dot, Circle(center: Point, r: Float), Label(text: String, at: Option<Point>) }
enum Light { Red, Amber, Green }
type Tree { value: Int, kids: List<Tree> }
enum Expr { Num(n: Int), Add(a: Expr, b: Expr), Neg(e: Option<Expr>) }
type A { b: B, n: Int }
type B { a: Option<A>, s: String }
type Wrap { inner: Named, r: Result<String, Int> }
enum Msg { Start(path: String), Done }
fn effects(e: List<Effect<Msg>>) {}
";

/// Each type spelled in `want`, laid out and shown.
fn shown(want: &[(&str, &str)]) {
    let source = Source::file("main.paka", FIXTURE.into());
    let (mut p, ..) = crate::check("main.paka", &source).unwrap();
    let layouts = Layouts::new(&mut p);
    let by_name: BTreeMap<String, Ty> = (0..p.types.len()).map(|t| (name(&p, t), t)).collect();
    for (spelled, want) in want {
        let layout = layouts.get(by_name[*spelled]);
        assert_eq!(show(&p, layout), *want, "{spelled}");
    }
}

/// The spelling of a type: `fn(A, B) -> R`, `Map<K, V>`.
fn name(p: &Program, ty: Ty) -> String {
    let list = |tys: &[Ty]| {
        tys.iter()
            .map(|&t| name(p, t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match &p.types[ty] {
        Type::Int => "Int".into(),
        Type::Float => "Float".into(),
        Type::Bool => "Bool".into(),
        Type::String => "String".into(),
        Type::List(t) => format!("List<{}>", list(&[*t])),
        Type::Map(k, v) => format!("Map<{}>", list(&[*k, *v])),
        Type::Option(t) => format!("Option<{}>", list(&[*t])),
        Type::Result(t, e) => format!("Result<{}>", list(&[*t, *e])),
        Type::Effect(m) => format!("Effect<{}>", list(&[*m])),
        Type::View(m) => format!("View<{}>", list(&[*m])),
        Type::Fn(params, None) => format!("fn({})", list(params)),
        Type::Fn(params, Some(r)) => format!("fn({}) -> {}", list(params), name(p, *r)),
        Type::Record(i) => p.records[*i].name.clone(),
        Type::Enum(i) => p.enums[*i].name.clone(),
    }
}

/// `size [tag] [field@offset ...] ... owns [entry@offset ...] ...`, a boxed field marked `^`.
fn show(p: &Program, l: &Layout) -> String {
    let list = |items: Vec<String>| format!("[{}]", items.join(" "));
    let place = |at: &Place| {
        let mark = if at.boxed { "^" } else { "" };
        format!("{mark}{}@{}", name(p, at.ty), at.offset)
    };
    let entry = |e: &Entry| {
        let kind = match e.kind {
            Owned::String => "String".into(),
            Owned::List(t) => format!("List({})", name(p, t)),
            Owned::Map(k, v) => format!("Map({}, {})", name(p, k), name(p, v)),
            Owned::Box(t) => format!("Box({})", name(p, t)),
            Owned::Value(t) => format!("Value({})", name(p, t)),
        };
        format!("{kind}@{}", e.offset)
    };
    let alts: Vec<_> = (l.alternatives.iter())
        .map(|a| list(a.iter().map(place).collect()))
        .collect();
    let owned: Vec<_> = (l.owned.iter())
        .map(|o| list(o.iter().map(entry).collect()))
        .collect();
    let tag = if l.tagged { "tag " } else { "" };
    format!(
        "{} {tag}{} owns {}",
        l.size,
        alts.join(" "),
        owned.join(" ")
    )
}

#[test]
fn scalars_and_built_in_containers() {
    shown(&[
        ("Int", "8  owns []"),
        ("Float", "8  owns []"),
        ("Bool", "8  owns []"),
        ("fn(Int) -> Int", "8  owns []"),
        ("String", "32  owns [String@0]"),
        ("List<String>", "24  owns [List(String)@0]"),
        ("Map<String, Int>", "8  owns [Map(String, Int)@0]"),
    ]);
}

#[test]
fn records_lay_fields_in_declared_order() {
    shown(&[
        ("Empty", "0 [] owns []"),
        ("Point", "16 [Int@0 Float@8] owns []"),
        (
            "Named",
            "80 [String@0 List<String>@32 Map<String, Int>@56 Bool@64 fn(Int) -> Int@72] \
             owns [String@0 List(String)@32 Map(String, Int)@56]",
        ),
        (
            "Wrap",
            "120 [Named@0 Result<String, Int>@80] \
             owns [String@0 List(String)@32 Map(String, Int)@56 Value(Result<String, Int>)@80]",
        ),
    ]);
}

#[test]
fn enums_take_a_tag_and_their_largest_payload() {
    shown(&[
        ("Light", "8 tag [] [] [] owns [] [] []"),
        (
            "Shape",
            "64 tag [] [Point@8 Float@24] [String@8 Option<Point>@40] owns [] [] [String@8]",
        ),
        ("Option<Point>", "24 tag [Point@8] [] owns [] []"),
        (
            "Result<String, Int>",
            "40 tag [String@8] [Int@8] owns [String@8] []",
        ),
        ("Msg", "40 tag [String@8] [] owns [String@8] []"),
        (
            "Effect<Msg>",
            "88 tag [String@8] [String@8 fn(String) -> Msg@40 fn(String) -> Msg@48] \
             [String@8 String@40 ^Msg@72 fn(String) -> Msg@80] [] [Int@8] \
             owns [String@8] [String@8] [String@8 String@40 Box(Msg)@72] [] []",
        ),
        ("List<Effect<Msg>>", "24  owns [List(Effect<Msg>)@0]"),
    ]);
}

#[test]
fn a_field_that_reaches_its_own_type_by_value_is_boxed() {
    shown(&[
        ("Tree", "32 [Int@0 List<Tree>@8] owns [List(Tree)@8]"),
        (
            "Expr",
            "24 tag [Int@8] [^Expr@8 ^Expr@16] [^Option<Expr>@8] \
             owns [] [Box(Expr)@8 Box(Expr)@16] [Box(Option<Expr>)@8]",
        ),
        ("Option<Expr>", "32 tag [Expr@8] [] owns [Value(Expr)@8] []"),
        ("A", "16 [^B@0 Int@8] owns [Box(B)@0]"),
        (
            "B",
            "40 [^Option<A>@0 String@8] owns [Box(Option<A>)@0 String@8]",
        ),
    ]);
}
