use super::*;

type Tree = Node<&'static str>;

fn text(t: &str) -> Tree {
    Node::Text(t.into())
}

fn button(label: &str, on: &'static str) -> Tree {
    Node::Button {
        label: label.into(),
        on,
    }
}

fn input(text: &str, on_change: &'static str, edit: Edit) -> Tree {
    Node::Input {
        text: text.into(),
        on_change,
        edit,
    }
}

fn key(s: &str) -> Step {
    Step::Str(s.into())
}

#[test]
fn the_counter_at_zero_prints_with_its_placeholder() {
    let row = Node::Row {
        gap: 8,
        children: vec![
            (Step::Place(0), button("-", "Dec")),
            (Step::Place(1), button("+", "Inc")),
        ],
    };
    let tree = Node::Col {
        gap: 8,
        children: vec![
            (Step::Place(0), text("0")),
            (Step::Place(1), row),
            (Step::Place(2), Node::Hole("reset".into())),
        ],
    };
    let want = "\
Col gap=8
  Text #0 \"0\"
  Row #1 gap=8
    Button #0 \"-\" on=Dec
    Button #1 \"+\" on=Inc
  Hole #2 reset
";
    assert_eq!(tree.to_string(), want);
}

#[test]
fn the_records_sample_prints_with_an_error_node_and_focus() {
    let row = Node::Row {
        gap: 8,
        children: vec![
            (Step::Place(0), button("[ ]", "Toggle(7)")),
            (Step::Place(1), text("buy milk")),
        ],
    };
    let draft = Edit {
        cursor: 3,
        focus: true,
        ..Edit::default()
    };
    let tree = Node::Col {
        gap: 8,
        children: vec![
            (key("new"), input("mil", "Draft", draft)),
            (Step::Place(1), button("Add", "Add")),
            (Step::Int(7), row),
            (
                Step::Int(8),
                Node::Error("duplicate key 8: 2 children of Col".into()),
            ),
            (Step::Place(4), Node::Hole("footer".into())),
        ],
    };
    let want = "\
Col gap=8
  Input 'new' \"mil\" cursor=3 on_change=Draft focus
  Button #1 \"Add\" on=Add
  Row 7 gap=8
    Button #0 \"[ ]\" on=Toggle(7)
    Text #1 \"buy milk\"
  Error 8 \"duplicate key 8: 2 children of Col\"
  Hole #4 footer
";
    assert_eq!(tree.to_string(), want);
}

#[test]
fn steps_texts_and_input_state_print_escaped_and_defaults_left_out() {
    let edit = Edit {
        cursor: 4,
        sel: 1..4,
        preedit: "gyuu".into(),
        focus: false,
    };
    let rows = Node::Col {
        gap: 0,
        children: vec![
            (Step::Int(-3), input("milk", "Rename", edit)),
            (key("it's \\ here"), text("two\nlines \"quoted\" {x}")),
            (Step::LoopHole(2, 0), Node::Hole("row".into())),
            (Step::LoopHole(2, 1), Node::Hole("row".into())),
            (
                Step::Int(5),
                Node::Error("duplicate key 5: 3 children of Row".into()),
            ),
        ],
    };
    let tree = Node::Row {
        gap: 0,
        children: vec![
            (key(""), rows),
            (key("3"), input("", "Draft", Edit::default())),
        ],
    };
    let want = "\
Row
  Col ''
    Input -3 \"milk\" cursor=4 sel=1..4 preedit=\"gyuu\" on_change=Rename
    Text 'it\\'s \\\\ here' \"two\\nlines \\\"quoted\\\" \\{x\\}\"
    Hole #2.0 row
    Hole #2.1 row
    Error 5 \"duplicate key 5: 3 children of Row\"
  Input '3' \"\" cursor=0 on_change=Draft
";
    assert_eq!(tree.to_string(), want);
}

#[test]
fn a_lone_root_is_one_line() {
    assert_eq!(text("hi").to_string(), "Text \"hi\"\n");
    assert_eq!(Tree::Hole("main".into()).to_string(), "Hole main\n");
}

#[test]
fn siblings_sharing_a_key_become_one_error_node_at_the_first() {
    let children = vec![
        (Step::Int(8), text("a")),
        (key("x"), text("b")),
        (Step::Int(8), text("c")),
        (Step::Place(3), text("d")),
        (key("x"), text("e")),
        (Step::Int(8), text("f")),
    ];
    let tree = Node::Col {
        gap: 0,
        children: unique("Col", children),
    };
    let want = "\
Col
  Error 8 \"duplicate key 8: 3 children of Col\"
  Error 'x' \"duplicate key \\\"x\\\": 2 children of Col\"
  Text #3 \"d\"
";
    assert_eq!(tree.to_string(), want);
}

fn col(children: Vec<(Step, Tree)>) -> Tree {
    Node::Col {
        gap: 0,
        children: unique("Col", children),
    }
}

fn row(children: Vec<(Step, Tree)>) -> Tree {
    Node::Row {
        gap: 0,
        children: unique("Row", children),
    }
}

fn typed(cursor: usize) -> Tree {
    let edit = Edit {
        cursor,
        sel: 0..cursor,
        preedit: String::new(),
        focus: cursor == 2,
    };
    input("abc", "Draft", edit)
}

fn fresh() -> Tree {
    input("abc", "Draft", Edit::default())
}

fn kept(old: &Tree, mut new: Tree) -> String {
    keep(old, &mut new);
    new.to_string()
}

#[test]
fn reordered_keyed_inputs_keep_their_own_state() {
    let old = col(vec![
        (key("a"), typed(1)),
        (key("b"), typed(2)),
        (Step::Int(7), row(vec![(key("in"), typed(3))])),
    ]);
    let new = col(vec![
        (Step::Int(7), row(vec![(key("in"), fresh())])),
        (key("b"), fresh()),
        (key("a"), fresh()),
    ]);
    let want = "\
Col
  Row 7
    Input 'in' \"abc\" cursor=3 sel=0..3 on_change=Draft
  Input 'b' \"abc\" cursor=2 sel=0..2 on_change=Draft focus
  Input 'a' \"abc\" cursor=1 sel=0..1 on_change=Draft
";
    assert_eq!(kept(&old, new), want);
}

#[test]
fn an_inserted_input_starts_fresh_and_the_others_keep_theirs() {
    let old = col(vec![(key("a"), typed(1)), (key("b"), typed(2))]);
    let new = col(vec![
        (key("a"), fresh()),
        (key("new"), fresh()),
        (key("b"), fresh()),
    ]);
    let want = "\
Col
  Input 'a' \"abc\" cursor=1 sel=0..1 on_change=Draft
  Input 'new' \"abc\" cursor=0 on_change=Draft
  Input 'b' \"abc\" cursor=2 sel=0..2 on_change=Draft focus
";
    assert_eq!(kept(&old, new), want);
}

#[test]
fn a_removed_input_loses_its_state_for_good() {
    let old = col(vec![(key("a"), typed(1)), (key("b"), typed(2))]);
    let mut gone = col(vec![(key("a"), fresh())]);
    keep(&old, &mut gone);
    let back = col(vec![(key("a"), fresh()), (key("b"), fresh())]);
    let want = "\
Col
  Input 'a' \"abc\" cursor=1 sel=0..1 on_change=Draft
  Input 'b' \"abc\" cursor=0 on_change=Draft
";
    assert_eq!(kept(&gone, back), want);
}

#[test]
fn a_kind_change_drops_the_state_inside() {
    let old = col(vec![
        (key("a"), typed(1)),
        (Step::Int(7), row(vec![(key("in"), typed(2))])),
    ]);
    let mut swapped = col(vec![
        (key("a"), text("abc")),
        (Step::Int(7), col(vec![(key("in"), fresh())])),
    ]);
    let want = "\
Col
  Text 'a' \"abc\"
  Col 7
    Input 'in' \"abc\" cursor=0 on_change=Draft
";
    keep(&old, &mut swapped);
    assert_eq!(swapped.to_string(), want);
    let back = col(vec![
        (key("a"), fresh()),
        (Step::Int(7), row(vec![(key("in"), fresh())])),
    ]);
    let want = "\
Col
  Input 'a' \"abc\" cursor=0 on_change=Draft
  Row 7
    Input 'in' \"abc\" cursor=0 on_change=Draft
";
    assert_eq!(kept(&swapped, back), want);
}

#[test]
fn an_element_replacing_a_placeholder_starts_fresh() {
    let old = col(vec![
        (Step::Place(0), Node::Hole("draft".into())),
        (Step::LoopHole(1, 0), Node::Hole("row".into())),
    ]);
    let new = col(vec![(Step::Place(0), fresh()), (Step::Int(0), fresh())]);
    let want = "\
Col
  Input #0 \"abc\" cursor=0 on_change=Draft
  Input 0 \"abc\" cursor=0 on_change=Draft
";
    assert_eq!(kept(&old, new), want);
}

#[test]
fn siblings_sharing_a_key_drop_its_state_and_show_one_error_node() {
    let old = col(vec![(Step::Int(3), typed(2)), (key("b"), typed(1))]);
    let mut dup = col(vec![
        (Step::Int(3), fresh()),
        (key("b"), fresh()),
        (Step::Int(3), fresh()),
    ]);
    keep(&old, &mut dup);
    let want = "\
Col
  Error 3 \"duplicate key 3: 2 children of Col\"
  Input 'b' \"abc\" cursor=1 sel=0..1 on_change=Draft
";
    assert_eq!(dup.to_string(), want);
    let fixed = col(vec![(Step::Int(3), fresh()), (key("b"), fresh())]);
    let want = "\
Col
  Input 3 \"abc\" cursor=0 on_change=Draft
  Input 'b' \"abc\" cursor=1 sel=0..1 on_change=Draft
";
    assert_eq!(kept(&dup, fixed), want);
}
