use paka_gui::layout::Measure;
use paka_gui::text::Text;

#[test]
fn a_string_measures_the_same_twice() {
    let text = Text::default();
    let w = text.width("Buy milk and eggs");
    assert!(w > 0);
    assert_eq!(text.width("Buy milk and eggs"), w);
    assert_eq!(text.lines("Buy milk and eggs", w), 1);
    assert_eq!(text.lines("Buy milk and eggs", w), 1);
}

#[test]
fn half_the_width_wraps_into_more_lines() {
    let text = Text::default();
    let s = "one two three four five six";
    let w = text.width(s);
    assert_eq!(text.lines(s, w), 1);
    assert!(text.lines(s, w / 2) > 1);
}

#[test]
fn a_missing_glyph_falls_back_to_the_fonts_box() {
    let text = Text::default();
    let layout = text.layout("a\u{6F22}", None);
    let ids: Vec<u32> = layout
        .lines()
        .flat_map(|line| line.items().collect::<Vec<_>>())
        .filter_map(|item| match item {
            parley::PositionedLayoutItem::GlyphRun(run) => Some(run),
            _ => None,
        })
        .flat_map(|run| run.positioned_glyphs().map(|g| g.id).collect::<Vec<_>>())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], 0);
    assert_eq!(ids[1], 0, "the CJK character is not in Noto Sans");
    assert!(text.width("a\u{6F22}") > text.width("a"));
}

#[test]
fn nothing_measures_nothing() {
    assert_eq!(Text::default().width(""), 0);
}
