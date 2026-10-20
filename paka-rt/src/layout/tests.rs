use super::*;
use crate::tree::Edit;

type Tree = Node<&'static str>;

/// The test measure: 8 units per character, breaking at spaces, at least one character a line.
struct Fixed;

impl Measure for Fixed {
    fn width(&self, text: &str) -> u64 {
        8 * text.chars().count() as u64
    }

    fn lines(&self, text: &str, width: u64) -> u64 {
        let cap = (width / 8).max(1) as usize;
        let (mut lines, mut used) = (1, 0);
        for word in text.split(' ') {
            let n = word.chars().count();
            if used > 0 && used + 1 + n <= cap {
                used += 1 + n;
                continue;
            }
            if used > 0 {
                lines += 1;
            }
            let more = n.saturating_sub(1) / cap;
            lines += more as u64;
            used = n - more * cap;
        }
        lines
    }
}

fn text(t: &str) -> Tree {
    Node::Text(t.into())
}

fn button(label: &str) -> Tree {
    Node::Button {
        label: label.into(),
        on: "Go",
    }
}

fn kids(nodes: Vec<Tree>) -> Vec<(Step, Tree)> {
    nodes
        .into_iter()
        .enumerate()
        .map(|(i, n)| (Step::Place(i), n))
        .collect()
}

fn row(gap: u64, nodes: Vec<Tree>) -> Tree {
    Node::Row {
        gap,
        children: kids(nodes),
    }
}

fn col(gap: u64, nodes: Vec<Tree>) -> Tree {
    Node::Col {
        gap,
        children: kids(nodes),
    }
}

fn r(x: u64, y: u64, w: u64, h: u64) -> Rect {
    Rect { x, y, w, h }
}

fn places(steps: &[usize]) -> Option<Vec<Step>> {
    Some(steps.iter().map(|&i| Step::Place(i)).collect())
}

#[test]
fn hugging_children_take_their_width_and_filling_ones_share_the_rest() {
    let tree = row(8, vec![button("x"), text("buy milk"), button("del")]);
    let want = [
        r(0, 0, 480, 32),
        r(0, 0, 32, 32),
        r(40, 6, 384, 20),
        r(432, 0, 48, 32),
    ];
    assert_eq!(layout(&tree, 480, &Fixed), want);
}

#[test]
fn the_counter_lays_out_as_recorded() {
    let tree = col(
        8,
        vec![
            text("0"),
            row(8, vec![button("-"), button("+")]),
            Node::Hole("reset".into()),
        ],
    );
    let want = [
        r(0, 0, 480, 100),
        r(0, 0, 480, 20),
        r(0, 28, 480, 32),
        r(0, 28, 32, 32),
        r(40, 28, 32, 32),
        r(0, 68, 64, 32),
    ];
    assert_eq!(layout(&tree, 480, &Fixed), want);
}

#[test]
fn text_wraps_word_by_word_and_breaks_a_long_word() {
    let height = |t: &str, w| layout(&text(t), w, &Fixed)[0];
    assert_eq!(height("buy milk and bread", 100), r(0, 0, 100, 40));
    assert_eq!(height("abcdefghijk", 40), r(0, 0, 40, 60));
    assert_eq!(height("ab abcdefghijk", 40), r(0, 0, 40, 80));
    assert_eq!(height("abc", 4), r(0, 0, 4, 60));
    assert_eq!(height("", 100), r(0, 0, 100, 20));
}

#[test]
fn spare_units_go_to_earlier_filling_children() {
    let tree = row(0, vec![text("a"), text("b"), text("c")]);
    let want = [
        r(0, 0, 200, 20),
        r(0, 0, 67, 20),
        r(67, 0, 67, 20),
        r(134, 0, 66, 20),
    ];
    assert_eq!(layout(&tree, 200, &Fixed), want);
    let tree = row(2, vec![text("a"), text("b"), text("c")]);
    let want = [
        r(0, 0, 200, 20),
        r(0, 0, 66, 20),
        r(68, 0, 65, 20),
        r(135, 0, 65, 20),
    ];
    assert_eq!(layout(&tree, 200, &Fixed), want);
}

#[test]
fn a_filling_child_keeps_the_floor_and_the_row_overflows() {
    let tree = row(0, vec![button("abcdefghij"), text("a")]);
    let want = [r(0, 0, 100, 32), r(0, 0, 104, 32), r(104, 6, 40, 20)];
    assert_eq!(layout(&tree, 100, &Fixed), want);
}

#[test]
fn row_children_are_centred_with_an_odd_unit_below() {
    let tree = row(0, vec![col(1, vec![text("a"), text("b")]), button("x")]);
    let want = [
        r(0, 0, 200, 41),
        r(0, 0, 168, 41),
        r(0, 0, 168, 20),
        r(0, 21, 168, 20),
        r(168, 4, 32, 32),
    ];
    assert_eq!(layout(&tree, 200, &Fixed), want);
}

#[test]
fn gaps_go_between_present_children_and_empty_containers_are_flat() {
    let tree = col(8, vec![row(8, vec![]), text("a")]);
    let want = [r(0, 0, 480, 28), r(0, 0, 480, 0), r(0, 8, 480, 20)];
    assert_eq!(layout(&tree, 480, &Fixed), want);
    let tree = col(8, vec![text("a")]);
    assert_eq!(
        layout(&tree, 480, &Fixed),
        [r(0, 0, 480, 20), r(0, 0, 480, 20)]
    );
}

#[test]
fn hugging_nodes_keep_their_width_in_a_col_and_at_the_root() {
    let tree = col(0, vec![button("ok")]);
    assert_eq!(
        layout(&tree, 480, &Fixed),
        [r(0, 0, 480, 32), r(0, 0, 40, 32)]
    );
    assert_eq!(layout(&button("ok"), 480, &Fixed), [r(0, 0, 40, 32)]);
}

#[test]
fn inputs_fill_and_error_nodes_wrap_inside_their_padding() {
    let input = Node::Input {
        text: "a".into(),
        on_change: "Set",
        edit: Edit::default(),
    };
    let tree = row(0, vec![input, button("x")]);
    let want = [r(0, 0, 100, 32), r(0, 0, 68, 32), r(68, 0, 32, 32)];
    assert_eq!(layout(&tree, 100, &Fixed), want);
    let error: Tree = Node::Error("bad thing here".into());
    assert_eq!(layout(&error, 100, &Fixed), [r(0, 0, 100, 52)]);
}

#[test]
fn the_last_drawn_element_is_hit_even_outside_its_parent() {
    let inner = row(0, vec![text("a"), text("b"), text("c")]);
    let tree = row(0, vec![col(0, vec![inner]), button("x")]);
    let rects = layout(&tree, 100, &Fixed);
    let want = [
        r(0, 0, 100, 32),
        r(0, 6, 68, 20),
        r(0, 6, 68, 20),
        r(0, 6, 40, 20),
        r(40, 6, 40, 20),
        r(80, 6, 40, 20),
        r(68, 0, 32, 32),
    ];
    assert_eq!(rects, want);
    assert_eq!(hit(&tree, &rects, 10, 10), places(&[0, 0, 0]));
    assert_eq!(hit(&tree, &rects, 50, 6), places(&[0, 0, 1]));
    assert_eq!(hit(&tree, &rects, 90, 10), places(&[1]));
    assert_eq!(hit(&tree, &rects, 100, 10), places(&[0, 0, 2]));
    assert_eq!(hit(&tree, &rects, 10, 30), None);
    assert_eq!(hit(&tree, &rects, 130, 10), None);
    let root = text("a");
    assert_eq!(hit(&root, &layout(&root, 480, &Fixed), 5, 5), places(&[]));
}
