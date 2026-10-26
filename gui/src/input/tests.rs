use super::*;

fn at(cursor: usize) -> Edit {
    Edit {
        cursor,
        ..Edit::default()
    }
}

fn selected(cursor: usize, sel: Range<usize>) -> Edit {
    Edit {
        cursor,
        sel,
        ..Edit::default()
    }
}

fn change(text: &str, cursor: usize) -> Option<Change> {
    Some(Change {
        text: text.into(),
        cursor,
    })
}

/// Runs `keys` from `edit` over `text`, returning the edit after and the last key's change.
fn run(mut edit: Edit, text: &str, keys: &[Key]) -> (Edit, Option<Change>) {
    let mut sent = None;
    for &key in keys {
        sent = edit.key(text, key);
    }
    (edit, sent)
}

#[test]
fn typing_sends_the_whole_new_text_and_leaves_the_edit() {
    let mut edit = at(3);
    assert_eq!(edit.key("mil", Key::Type('k')), change("milk", 4));
    assert_eq!(edit, at(3));
    assert_eq!(at(0).key("ilk", Key::Type('m')), change("milk", 1));
    assert_eq!(at(1).key("mlk", Key::Type('i')), change("milk", 2));
}

#[test]
fn typing_counts_code_points_in_non_ascii_text() {
    assert_eq!(at(2).key("çö", Key::Type('ğ')), change("çöğ", 3));
    assert_eq!(at(1).key("日本", Key::Type('語')), change("日語本", 2));
    assert_eq!(at(1).key("a😀", Key::Type('é')), change("aé😀", 2));
}

#[test]
fn typing_replaces_the_selection() {
    assert_eq!(
        selected(3, 1..3).key("abcd", Key::Type('x')),
        change("axd", 2)
    );
    assert_eq!(
        selected(0, 0..2).key("ğüş", Key::Type('s')),
        change("sş", 1)
    );
}

#[test]
fn line_breaks_and_tabs_send_nothing() {
    for c in ['\n', '\r', '\t'] {
        assert_eq!(at(1).key("ab", Key::Type(c)), None);
    }
}

#[test]
fn back_removes_the_code_point_before_the_cursor() {
    assert_eq!(at(4).key("milk", Key::Back), change("mil", 3));
    assert_eq!(at(2).key("aö😀", Key::Back), change("a😀", 1));
    assert_eq!(at(3).key("aö😀", Key::Back), change("aö", 2));
}

#[test]
fn delete_removes_the_code_point_after_the_cursor() {
    assert_eq!(at(0).key("milk", Key::Delete), change("ilk", 0));
    assert_eq!(at(1).key("a😀b", Key::Delete), change("ab", 1));
}

#[test]
fn back_and_delete_remove_the_selection() {
    assert_eq!(selected(1, 1..3).key("aöüb", Key::Back), change("ab", 1));
    assert_eq!(selected(3, 1..3).key("aöüb", Key::Delete), change("ab", 1));
}

#[test]
fn a_key_that_would_not_change_the_text_sends_nothing() {
    assert_eq!(at(0).key("ab", Key::Back), None);
    assert_eq!(at(2).key("ab", Key::Delete), None);
    assert_eq!(at(0).key("", Key::Back), None);
    assert_eq!(at(0).key("", Key::Delete), None);
}

#[test]
fn arrows_step_one_code_point_and_send_nothing() {
    let text = "aö😀";
    assert_eq!(run(at(0), text, &[Key::Right(false)]), (at(1), None));
    assert_eq!(run(at(1), text, &[Key::Right(false); 2]), (at(3), None));
    assert_eq!(run(at(3), text, &[Key::Right(false)]), (at(3), None));
    assert_eq!(run(at(3), text, &[Key::Left(false)]), (at(2), None));
    assert_eq!(run(at(0), text, &[Key::Left(false)]), (at(0), None));
}

#[test]
fn home_and_end_go_to_the_ends() {
    assert_eq!(run(at(1), "çay", &[Key::End(false)]).0, at(3));
    assert_eq!(run(at(2), "çay", &[Key::Home(false)]).0, at(0));
}

#[test]
fn shift_moves_select_from_the_anchor() {
    let text = "şeker";
    assert_eq!(
        run(at(1), text, &[Key::Right(true); 2]).0,
        selected(3, 1..3)
    );
    assert_eq!(run(at(3), text, &[Key::Left(true); 2]).0, selected(1, 1..3));
    assert_eq!(run(at(2), text, &[Key::End(true)]).0, selected(5, 2..5));
    assert_eq!(run(at(2), text, &[Key::Home(true)]).0, selected(0, 0..2));
}

#[test]
fn a_selection_can_cross_its_anchor_and_close() {
    let text = "abcde";
    let left = run(
        at(2),
        text,
        &[Key::Right(true), Key::Left(true), Key::Left(true)],
    );
    assert_eq!(left.0, selected(1, 1..2));
    assert_eq!(
        run(at(2), text, &[Key::Right(true), Key::Left(true)]).0,
        at(2)
    );
    let flipped = run(
        at(1),
        text,
        &[Key::Right(true), Key::Right(true), Key::Home(true)],
    );
    assert_eq!(flipped.0, selected(0, 0..1));
}

#[test]
fn a_plain_arrow_closes_the_selection_at_its_side() {
    assert_eq!(run(selected(3, 1..3), "abcd", &[Key::Left(false)]).0, at(1));
    assert_eq!(
        run(selected(1, 1..3), "abcd", &[Key::Right(false)]).0,
        at(3)
    );
    assert_eq!(run(selected(3, 1..3), "abcd", &[Key::Home(false)]).0, at(0));
}

#[test]
fn a_cursor_past_a_shorter_text_is_clipped_first() {
    assert_eq!(at(9).key("ab", Key::Type('c')), change("abc", 3));
    assert_eq!(selected(9, 1..9).key("ab", Key::Back), change("a", 1));
    assert_eq!(run(at(9), "ab", &[Key::Left(false)]).0, at(1));
    assert_eq!(run(selected(5, 2..5), "ab", &[Key::Left(false)]).0, at(1));
}

#[test]
fn the_app_wins_and_the_cursor_follows() {
    let before = "mil";
    let mut edit = at(3);
    let sent = edit.key(before, Key::Type('k')).unwrap();
    for (shown, cursor) in [("mil", 3), ("milk", 4), ("MILK", 4), ("mi", 2)] {
        let mut after = edit.clone();
        after.follow(before, shown, Some(sent.cursor));
        assert_eq!(after, at(cursor), "{shown}");
    }
}

#[test]
fn a_refused_edit_keeps_the_selection() {
    let mut edit = selected(3, 1..3);
    let sent = edit.key("abcd", Key::Back).unwrap();
    edit.follow("abcd", "abcd", Some(sent.cursor));
    assert_eq!(edit, selected(3, 1..3));
}

#[test]
fn a_text_the_app_sets_pulls_the_cursor_back_and_clears_the_selection() {
    let mut edit = selected(4, 1..4);
    edit.follow("milk", "", None);
    assert_eq!(edit, at(0));
    let mut edit = selected(4, 1..4);
    edit.follow("milk", "bread", None);
    assert_eq!(edit, at(4));
    let mut edit = selected(4, 1..4);
    edit.follow("milk", "milk", None);
    assert_eq!(edit, selected(4, 1..4));
}

#[test]
fn preedit_and_focus_are_left_alone() {
    let mut edit = Edit {
        cursor: 1,
        preedit: "に".into(),
        focus: true,
        ..Edit::default()
    };
    assert_eq!(edit.key("ab", Key::Type('x')), change("axb", 2));
    edit.key("ab", Key::End(false));
    edit.follow("ab", "abc", Some(3));
    assert_eq!(
        (edit.preedit.as_str(), edit.focus, edit.cursor),
        ("に", true, 3)
    );
}
