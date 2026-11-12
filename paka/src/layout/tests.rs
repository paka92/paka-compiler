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
type Flags { on: List<Bool>, grid: List<List<Bool>> }
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
        Type::Ui => "Ui".into(),
    }
}

/// `size [tag] [field@offset ...] ... owns [entry@offset ...] ...`, a boxed field marked `^`
/// and one in its variant's block `+`.
fn show(p: &Program, l: &Layout) -> String {
    let list = |items: Vec<String>| format!("[{}]", items.join(" "));
    let place = |at: &Place| {
        let mark = match (at.boxed, at.in_block) {
            (true, _) => "^",
            (_, true) => "+",
            _ => "",
        };
        format!("{mark}{}@{}", name(p, at.ty), at.offset)
    };
    let entry = |e: &Entry| {
        let kind = match e.kind {
            Owned::String => "String".into(),
            Owned::List(t) => format!("List({})", name(p, t)),
            Owned::Map(k, v) => format!("Map({}, {})", name(p, k), name(p, v)),
            Owned::Box(t) => format!("Box({})", name(p, t)),
            Owned::Block(t, alt) => format!("Block({}, {alt}, {})", name(p, t), l.blocks[alt]),
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
            "16 tag [Int@8] [+Expr@0 +Expr@16] [+Option<Expr>@0] \
             owns [] [Block(Expr, 1, 32)@8] [Block(Expr, 2, 24)@8]",
        ),
        ("Option<Expr>", "24 tag [Expr@8] [] owns [Value(Expr)@8] []"),
        ("A", "16 [^B@0 Int@8] owns [Box(B)@0]"),
        (
            "B",
            "40 [^Option<A>@0 String@8] owns [Box(Option<A>)@0 String@8]",
        ),
    ]);
}

#[test]
fn runtime_headers_and_alignment_match_the_hand_layout() {
    let words = |h: &Header| [h.size, h.ptr, h.len, h.cap];
    assert_eq!((words(&STRING), CHARS), ([32, 0, 8, 24], 16));
    assert_eq!(words(&LIST), [24, 0, 8, 16]);
    assert_eq!(CONSTANT, 0, "a cap of 0 owns no block");
    let source = Source::file("main.paka", FIXTURE.into());
    let (mut p, ..) = crate::check("main.paka", &source).unwrap();
    let layouts = Layouts::new(&mut p);
    for t in 0..p.types.len() {
        let l = layouts.get(t);
        let places = l.alternatives.iter().flatten().map(|f| f.offset);
        assert!(
            places.chain([l.size]).all(|n| n % ALIGN == 0),
            "{}",
            name(&p, t)
        );
    }
}

#[test]
fn a_bool_list_item_is_one_byte() {
    let source = Source::file("main.paka", FIXTURE.into());
    let (mut p, ..) = crate::check("main.paka", &source).unwrap();
    let layouts = Layouts::new(&mut p);
    let specs = layouts.specs();
    let blocks =
        (0..p.types.len()).map(|t| layouts.get(t).blocks.iter().filter(|&&b| b != 0).count());
    assert_eq!(specs.len(), p.types.len() + blocks.sum::<usize>());
    for (t, spec) in specs.iter().take(p.types.len()).enumerate() {
        let l = layouts.get(t);
        let want = if p.types[t] == Type::Bool { 1 } else { l.size };
        assert_eq!(l.item, want, "{}", name(&p, t));
        if let Type::List(item) = p.types[t] {
            let part = &spec.as_ref().unwrap().alternatives[0][0].1;
            let size = layouts.get(item).item as u64;
            let ok = matches!(part, Part::List { size: s, .. } if *s == size);
            assert!(ok, "{}", name(&p, t));
        }
    }
    shown(&[(
        "Flags",
        "48 [List<Bool>@0 List<List<Bool>>@24] owns [List(Bool)@0 List(List<Bool>)@24]",
    )]);
}

#[test]
fn a_variant_block_has_its_record_after_the_types() {
    let source = Source::file("main.paka", FIXTURE.into());
    let (mut p, ..) = crate::check("main.paka", &source).unwrap();
    let layouts = Layouts::new(&mut p);
    let specs = layouts.specs();
    let expr = (0..p.types.len()).find(|&t| name(&p, t) == "Expr").unwrap();
    let alt = &specs[expr].as_ref().unwrap().alternatives[1];
    let &[(8, Part::Box { item, size: 32 })] = &alt[..] else {
        panic!("Add's block")
    };
    let block = specs[item].as_ref().unwrap();
    assert!(item >= p.types.len() && !block.tagged && block.size == 32);
    let fields: Vec<_> = (block.alternatives[0].iter())
        .map(|(at, part)| (*at, matches!(part, Part::Value(t) if *t == expr)))
        .collect();
    assert_eq!(fields, [(0, true), (16, true)]);
}
