//! Ownership (M2-05): the `&` targets, the call rules A to D over every form with operands, the
//! frozen walk of a `for` and the samples of the record, accepted and refused.

use super::pattern::say;
use super::*;

/// Definitions the samples call; they follow the function under test.
const OWN: &[&str] = &[
    "type P { left: List<Int> = [], right: List<Int> = [] }",
    "type Rule { name: String = \"\" }",
    "type Item { price: Int = 0 }",
    "type S { rules: List<Rule> = [], count: Int = 0, items: List<Item> = [], total: Int = 0 }",
    "type Account { balance: Int = 0, name: String = \"\" }",
    "type Pair { a: List<Int>, b: List<Int> }",
    "type Graph { adj: List<List<Int>> = [] }",
    "type Cell { shape: Shape = Dot, hits: Int = 0 }",
    "enum Two { Both(a: List<Int>, b: List<Int>) }",
    "fn grow(xs: inout List<Int>, more: List<Int>) {}",
    "fn mend(xs: inout List<Int>, p: P) {}",
    "fn after(p: P, xs: inout List<Int>) {}",
    "fn swap_lists(a: inout List<Int>, b: inout List<Int>) {}",
    "fn take(xs: inout List<Int>) -> List<Int> { xs }",
    "fn drop_n(xs: inout List<Int>) -> Int { 0 }",
    "fn merge(a: List<Int>, b: List<Int>) -> List<Int> { a }",
    "fn keep(xs: List<Int>, n: Int) {}",
    "fn id(xs: List<Int>) -> List<Int> { xs }",
    "fn push(xs: inout List<Int>, x: Int) { append(&xs, x) }",
    "fn apply(s: inout S, r: Rule) {}",
    "fn tally(s: inout S, n: Int) {}",
    "fn cut(s: inout String) -> String { s }",
    "fn tell(t: String, n: Int) {}",
    "fn put(xs: inout List<Int>, n: Int) {}",
    "fn size(rows: inout List<List<Int>>) -> Int { len(rows) }",
    "fn add(n: inout Int, k: Int) { n += k }",
    "fn drop_last(xs: inout List<String>) -> Int { 0 }",
    "fn transfer(a: inout Account, b: inout Account) {}",
    "fn settle(a: inout Account, b: Account) {}",
    "fn check_in(a: inout Account, b: Option<Account>) {}",
    "fn move_n(a: inout Account, n: Int) {}",
    "fn visit(g: inout Graph, v: Int) {}",
    "fn next_of(x: Int) -> Int { x }",
];

/// The parameters every sample may use; each list or map is inout, so it can be passed with `&`.
const ALL: &str = "t(xs: inout List<Int>, names: inout List<String>, rows: inout List<List<Int>>, \
    p: inout P, s: inout S, st: inout List<Int>, m: inout Map<String, Account>, g: inout Graph, \
    doc: inout String, a: String, b: String, i: Int, j: Int, c: Bool)";

/// `fn HEAD { BODY }`, then the definitions of the samples and the prelude.
fn own(head: &str, body: &[&str]) -> String {
    let open = format!("fn {head} {{");
    full(&[&[open.as_str()][..], body, &["}"], OWN].concat())
}

/// A sample in the function that takes every parameter, with its whole message.
fn sample<'a>(code: &'a str, mark: &'a str, message: &'a str, body: &[&str]) -> Say<'a> {
    (code, mark, message, own(ALL, body))
}

const OVERLAP: &str = "inout-overlap";
const READ: &str = "changed-while-read";
const WALKED: &str = "changed-while-walked";

#[test]
fn the_samples_define_their_helpers_cleanly() {
    accepts(&[own("t()", &[])]);
}

#[test]
fn a_copy_is_its_own_value() {
    accepts(&[own(
        "demo() -> Int",
        &["let a = [1, 2, 3]", "var b = a", "append(&b, 4)", "len(a)"],
    )]);
}

#[test]
fn accepts_the_call_samples() {
    accepts(&[own(
        ALL,
        &[
            "let old = xs",
            "grow(&xs, old)",
            "append(&xs, xs[0])",
            "append(&xs, len(xs))",
            "grow(&p.left, p.right)",
            "swap_lists(&p.left, &p.right)",
            "tally(&s, s.count)",
            "let same = xs[0] == pop(&xs)",
            "push(&st, pop(&st) + pop(&st))",
            "put(&rows[i], size(&rows))",
            "add(&xs[pop(&xs)], 1)",
            "tell(names[drop_last(&names)], 1)",
            "move_n(&m[a], m[b].balance)",
            "grow(&xs, id(xs))",
            "grow(&xs, merge(xs, xs))",
            "grow(&p.left, take(&p.left))",
            "check_in(&m[a], get(m, b))",
        ],
    )]);
}

#[test]
fn accepts_assignments_whose_sides_overlap() {
    accepts(&[own(
        ALL,
        &[
            "xs[i] = take(&xs)[0]",
            "xs[len(xs) - 1] = pop(&xs)",
            "rows[0] = rows[1]",
            "p.left = p.left",
            "s.rules[i] = s.rules[j]",
        ],
    )]);
}

#[test]
fn accepts_changing_in_place() {
    accepts(&[
        own(
            "touch(c: inout Cell, grid: inout List<List<Int>>, m: inout Map<String, Cell>)",
            &[
                "c.hits += 1",
                "grid[0][1] = 7",
                "m[\"a\"].hits += 1",
                "m[\"b\"] = Cell {}",
                "match c.shape {",
                "  Dot => c.shape = Rect(1.0, 1.0)",
                "  Circle(_) => {}",
                "  Rect(w, h) => c.shape = Rect(w * 2.0, h)",
                "}",
            ],
        ),
        own(
            "send(m: inout Map<String, Account>, a: String, b: String, n: Int)",
            &["m[a].balance -= n", "m[b].balance += n"],
        ),
    ]);
}

#[test]
fn accepts_the_loop_samples() {
    accepts(&[
        own(
            ALL,
            &[
                "for it in s.items {",
                "  s.total += it.price",
                "}",
                "for k in 0..<len(xs) {",
                "  xs[k] += 1",
                "}",
                "for k in keys(m) {",
                "  remove(&m, k)",
                "}",
                "let next = g.adj[i]",
                "for v in next {",
                "  visit(&g, v)",
                "}",
                "for x in take(&xs) {",
                "  append(&xs, x)",
                "}",
                "for row in rows {",
                "  for x in row {",
                "    append(&st, x)",
                "  }",
                "}",
                "var q = [0]",
                "var k = 0",
                "while k < len(q) {",
                "  let x = q[k]",
                "  k += 1",
                "  append(&q, next_of(x))",
                "}",
            ],
        ),
        own(
            "dfs(g: Graph, seen: inout List<Bool>, u: Int)",
            &[
                "seen[u] = true",
                "for v in g.adj[u] {",
                "  if !seen[v] {",
                "    dfs(g, &seen, v)",
                "  }",
                "}",
            ],
        ),
    ]);
}

#[test]
fn match_arms_may_change_the_subject() {
    accepts(&[
        own(
            "widen(s: inout Shape)",
            &[
                "match s {",
                "  Dot => {}",
                "  Circle(_) => {}",
                "  Rect(w, h) => s = Rect(w + 1.0, h)",
                "}",
            ],
        ),
        own(
            "reset(s: inout Shape)",
            &[
                "match s {",
                "  Rect(w, h) => {",
                "    s = Dot",
                "    let a = w",
                "  }",
                "  Dot => {}",
                "  Circle(_) => {}",
                "}",
            ],
        ),
    ]);
}

#[test]
fn accepts_the_counter_examples() {
    accepts(&[
        own("r(xs: inout List<Int>, ys: List<Int>)", &["r(&xs, ys)"]),
        own(
            "copy(a: List<Int>) -> List<Int>",
            &["var b = id(a)", "append(&b, 1)", "b"],
        ),
    ]);
}

#[test]
fn refuses_the_inout_targets() {
    says(&[
        say(
            ("bad-inout-target", "append(&^a, 2)"),
            "a is a let; declare it with var",
            "f()",
            &["let a = [1]", "append(&a, 2)"],
        ),
        say(
            ("bad-inout-target", "append(&^xs, 1)"),
            "xs is read-only; write xs: inout List<Int> and call f(&ys)",
            "f(xs: List<Int>)",
            &["append(&xs, 1)"],
        ),
        say(
            ("bad-inout-target", "append(&^x, 1)"),
            "x is a for name; it is a copy",
            "f(rows: List<List<Int>>)",
            &["for x in rows {", "  append(&x, 1)", "}"],
        ),
        say(
            ("bad-inout-target", "append(&^xs[0..<2], 1)"),
            "a slice cannot be passed with &; pass the list and the positions",
            "f(xs: inout List<Int>)",
            &["append(&xs[0..<2], 1)"],
        ),
        say(
            ("bad-inout-target", "append(&^text[0], \"a\")"),
            "a String position is not a place",
            "f(text: inout String)",
            &["append(&text[0], \"a\")"],
        ),
        say(
            ("missing-inout-mark", "append(^xs, 1)"),
            "append changes xs: write &xs",
            "f(xs: inout List<Int>)",
            &["append(xs, 1)"],
        ),
    ]);
}

#[test]
fn refuses_an_inout_place_that_overlaps_another_operand() {
    says(&[
        sample(
            OVERLAP,
            "grow(&xs, ^xs)",
            "xs is passed with & and read in one call; bind it first: let old = xs",
            &["grow(&xs, xs)"],
        ),
        sample(
            OVERLAP,
            "extend(&xs, ^xs)",
            "xs is passed with & and read in one call; bind it first: let old = xs",
            &["extend(&xs, xs)"],
        ),
        sample(
            OVERLAP,
            "append(&names, ^names[0])",
            "names is passed with & and read in one call; bind it first: let first = names[0]",
            &["append(&names, names[0])"],
        ),
        sample(
            OVERLAP,
            "grow(&rows[0], ^rows[1])",
            "two items of rows in one call; bind one first",
            &["grow(&rows[0], rows[1])"],
        ),
        sample(
            OVERLAP,
            "grow(&rows[i], ^rows[i - 1])",
            "two items of rows in one call; bind one first",
            &["grow(&rows[i], rows[i - 1])"],
        ),
        sample(
            OVERLAP,
            "swap_lists(&p.left, ^&p.left)",
            "p.left is passed with & twice",
            &["swap_lists(&p.left, &p.left)"],
        ),
        sample(
            OVERLAP,
            "^xs[0..<2])",
            "xs is passed with & and read in one call; bind it first: let part = xs[0..<2]",
            &["grow(&xs, xs[0..<2])"],
        ),
        sample(
            OVERLAP,
            "^s.rules[i])",
            "s is passed with & and read in one call; bind it first: let rule = s.rules[i]",
            &["apply(&s, s.rules[i])"],
        ),
        sample(
            OVERLAP,
            "^&m[b])",
            "two entries of m in one call; write transfer(&m, a, b)",
            &["transfer(&m[a], &m[b])"],
        ),
        sample(
            OVERLAP,
            "^&rows[j])",
            "two items of rows in one call; write swap_lists(&rows, i, j)",
            &["swap_lists(&rows[i], &rows[j])"],
        ),
        sample(
            OVERLAP,
            "^m[b])",
            "two entries of m in one call; bind one first",
            &["settle(&m[a], m[b])"],
        ),
        sample(
            OVERLAP,
            "^g.adj[j])",
            "two items of g.adj in one call; bind one first",
            &["grow(&g.adj[i], g.adj[j])"],
        ),
        sample(
            OVERLAP,
            "^rows[i])",
            "rows[i] is passed with & and read in one call; bind it first: let row = rows[i]",
            &["grow(&rows[i], rows[i])"],
        ),
        sample(
            OVERLAP,
            "^p)",
            "p.left is passed with & and read in one call; bind it first: let old = p",
            &["mend(&p.left, p)"],
        ),
    ]);
}

#[test]
fn the_later_operand_carries_the_overlap() {
    says(&[
        sample(
            OVERLAP,
            "^(xs))",
            "xs is passed with & and read in one call; bind it first: let old = xs",
            &["grow(&xs, (xs))"],
        ),
        sample(
            OVERLAP,
            "^&p.left)",
            "p.left is passed with & and read in one call; bind it first: let old = p",
            &["after(p, &p.left)"],
        ),
    ]);
}

/// The `changed-while-read` text for `xs` read by `f` and changed by `by`.
fn read_by(f: &str, by: &str) -> String {
    format!("xs is read by {f} and changed by {by} in the same call; bind it first: let old = xs")
}

#[test]
fn refuses_a_read_that_a_sibling_changes() {
    let take = read_by("merge", "take(&xs)");
    let set = read_by("keep", "xs = []");
    let deep = read_by("keep", "append(&xs, 1)");
    says(&[
        sample(READ, "merge(^xs", &take, &["_ = merge(xs, take(&xs))"]),
        sample(READ, "&xs), ^xs)", &take, &["_ = merge(take(&xs), xs)"]),
        sample(
            READ,
            "keep(^xs",
            &set,
            &["keep(xs, if c {", "  xs = []", "  1", "} else { 0 })"],
        ),
        sample(
            READ,
            "keep(^xs",
            &deep,
            &[
                "keep(xs, inc(if c {",
                "  append(&xs, 1)",
                "  1",
                "} else { 0 }))",
            ],
        ),
        sample(
            READ,
            "keep(^xs",
            &set,
            &[
                "keep(xs, match c {",
                "  true => {",
                "    xs = []",
                "    1",
                "  }",
                "  false => 0",
                "})",
            ],
        ),
        sample(
            READ,
            ", ^xs)",
            &read_by("grow", "drop_n(&xs)"),
            &["grow(&rows[drop_n(&xs)], xs)"],
        ),
        sample(
            READ,
            "^xs[0..<2]",
            "xs[0..<2] is read by merge and changed by take(&xs) in the same call; bind it first: \
             let part = xs[0..<2]",
            &["_ = merge(xs[0..<2], take(&xs))"],
        ),
        sample(
            READ,
            "tell(^doc",
            "doc is read by tell and changed by cut(&doc) in the same call; bind it first: \
             let old = doc",
            &["tell(doc, len(cut(&doc)))"],
        ),
    ]);
}

#[test]
fn every_form_with_operands_is_a_call() {
    let take = |f: &str| read_by(f, "take(&xs)");
    says(&[
        sample(
            READ,
            "= ^xs ==",
            &take("=="),
            &["let same = xs == take(&xs)"],
        ),
        sample(
            READ,
            "[^xs,",
            &take("this list"),
            &["let both = [xs, take(&xs)]"],
        ),
        sample(
            READ,
            ": ^xs, \"y\"",
            &take("this map"),
            &["let both = [\"x\": xs, \"y\": take(&xs)]"],
        ),
        sample(
            READ,
            "a: ^xs",
            &take("Pair"),
            &["let pair = Pair { a: xs, b: take(&xs) }"],
        ),
        sample(
            READ,
            "Both(^xs",
            &take("Both"),
            &["let two = Both(xs, take(&xs))"],
        ),
        sample(
            READ,
            "\"{^doc}",
            "doc is read by this text and changed by cut(&doc) in the same call; bind it first: \
             let old = doc",
            &["let line = \"{doc} {cut(&doc)}\""],
        ),
    ]);
    let head = "t(h: fn(List<Int>, List<Int>) -> Int, xs: inout List<Int>) -> Int";
    says(&[(READ, "h(^xs", &take("h"), own(head, &["h(xs, take(&xs))"]))]);
}

#[test]
fn rules_a_and_b_spare_what_is_read_first() {
    accepts(&[own(
        ALL,
        &[
            "_ = merge(id(xs), take(&xs))",
            "_ = merge(take(&xs), take(&xs))",
            "let same = xs[0] == take(&xs)[0]",
            "let line = \"{len(doc)} {cut(&doc)}\"",
            "let both = [len(xs), drop_n(&xs)]",
            "keep(take(&xs), if c {",
            "  xs = []",
            "  1",
            "} else { 0 })",
            "let flags = [c, c]",
            "let n = i + i",
        ],
    )]);
}

/// The `changed-while-walked` text for the walk of `place` by a loop named `name`.
fn walked(place: &str, name: &str) -> String {
    format!(
        "{place} is walked by this for; walk a copy (let items = {place}, then for {name} in items) \
         or by position (for i in 0..<len({place}))"
    )
}

#[test]
fn refuses_changing_the_walked_place() {
    let (xs, adj, rows) = (
        walked("xs", "x"),
        walked("g.adj[i]", "v"),
        walked("rows", "row"),
    );
    let items = walked("s.items", "it");
    let in_xs = |line| ["for x in xs {", line, "}"];
    says(&[
        sample(WALKED, "^&xs, x)", &xs, &in_xs("  append(&xs, x)")),
        sample(WALKED, "^xs[0] = x", &xs, &in_xs("  xs[0] = x")),
        sample(WALKED, "^&xs)", &xs, &in_xs("  _ = pop(&xs)")),
        sample(
            WALKED,
            "^&xs))",
            &xs,
            &in_xs("  keep(id(xs), inc(drop_n(&xs)))"),
        ),
        sample(
            WALKED,
            "^xs[0] = x",
            &xs,
            &["for x in xs {", "  if x > 0 {", "    xs[0] = x", "  }", "}"],
        ),
        sample(
            WALKED,
            "^&g, v)",
            &adj,
            &["for v in g.adj[i] {", "  visit(&g, v)", "}"],
        ),
        sample(
            WALKED,
            "^&g.adj[j]",
            &adj,
            &["for v in g.adj[i] {", "  append(&g.adj[j], v)", "}"],
        ),
        sample(
            WALKED,
            "^s = S {}",
            &items,
            &["for it in s.items {", "  s = S {}", "}"],
        ),
        sample(
            WALKED,
            "^s.items[0] = it",
            &items,
            &["for it in s.items {", "  s.items[0] = it", "}"],
        ),
        sample(
            WALKED,
            "^&rows, row)",
            &rows,
            &["for row in rows {", "  append(&rows, row)", "}"],
        ),
        sample(
            WALKED,
            "^&rows[0], x)",
            &rows,
            &[
                "for row in rows {",
                "  for x in row {",
                "    append(&rows[0], x)",
                "  }",
                "}",
            ],
        ),
    ]);
}

#[test]
fn a_for_name_over_a_place_is_changed_by_position() {
    says(&[
        sample(
            "assign-to-let",
            "^row[0] = 1",
            "row is a for name; write rows[i][0] = 1 walking by position",
            &["for row in rows {", "  row[0] = 1", "}"],
        ),
        sample(
            "assign-to-let",
            "^x += 1",
            "x is a for name; write xs[i] += 1 walking by position",
            &["for x in xs {", "  x += 1", "}"],
        ),
        sample(
            "assign-to-let",
            "^it.price = 0",
            "it is a for name; write s.items[i].price = 0 walking by position",
            &["for it in s.items {", "  it.price = 0", "}"],
        ),
        sample(
            "assign-to-let",
            "^x = 1",
            "x is a for name; declare it with var",
            &["for x in take(&xs) {", "  x = 1", "}"],
        ),
    ]);
}

#[test]
fn a_variant_field_names_its_subject() {
    says(&[(
        "bad-assignment-target",
        "^c.shape.w = 3.0",
        "w is a field of the variant Rect; build Rect(..) and assign c.shape",
        own("touch(c: inout Cell)", &["c.shape.w = 3.0"]),
    )]);
}

#[test]
fn the_earliest_error_wins_across_both_passes() {
    says(&[
        sample(
            OVERLAP,
            "grow(&xs, ^xs)",
            "xs is passed with & and read in one call; bind it first: let old = xs",
            &["grow(&xs, xs)", "let n: Int = \"a\""],
        ),
        sample(
            "type-mismatch",
            "n: Int = ^\"a\"",
            "expected `Int`, found `String`",
            &["let n: Int = \"a\"", "grow(&xs, xs)"],
        ),
        sample(
            OVERLAP,
            "grow(&rows[0], ^rows[1])",
            "two items of rows in one call; bind one first",
            &["grow(&rows[0], rows[1])", "grow(&xs, xs)"],
        ),
    ]);
}

#[test]
fn a_deep_chain_with_one_change_fits_the_stack() {
    for head in [false, true] {
        deep(|| {
            let links = " + doc".repeat(998);
            let wrap = |inner: String| match head {
                true => format!("({inner}{links} + doc)"),
                false => format!("(doc{links} + {inner})"),
            };
            let expr = (0..62).fold("cut(&doc) + doc".to_string(), |inner, _| wrap(inner));
            let src = own("t(doc: inout String) -> String", &[&expr]);
            let found = errors(&src);
            assert_eq!(found.iter().map(|d| d.code).collect::<Vec<_>>(), [READ]);
        });
    }
}
