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
