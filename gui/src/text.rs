//! Text shaping and measuring through Parley (M5-05): the one shipped font, 14 units on 20-unit
//! lines, at scale factor 1. Layout and drawing both measure through here, so they agree.
//! Measures are kept, so a tree laid out again shapes only text it has not measured (M5-43).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::layout::Measure;
use parley::fontique::{Blob, Collection, CollectionOptions};
use parley::{
    FontContext, FontFamily, FontFamilyName, Layout, LayoutContext, LineHeight, StyleProperty,
};

const FAMILY: &str = "Noto Sans";
const SIZE: f32 = 14.0;
const LINE: f32 = 20.0;
/// How many measures are kept before all are dropped, which bounds the memory they take.
const KEPT: usize = 1 << 16;

/// The width and line count of each text measured, by its wrap width.
type Measures = BTreeMap<(String, Option<u64>), (f32, usize)>;

/// The font and Parley's reusable state, loaded once.
pub struct Text {
    contexts: RefCell<(FontContext, LayoutContext<()>)>,
    measures: RefCell<Measures>,
}

impl Default for Text {
    /// Registers `crate::FONT` with no system font lookup.
    fn default() -> Text {
        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: false,
        });
        collection.register_fonts(Blob::new(Arc::new(crate::FONT)), None);
        let fonts = FontContext {
            collection,
            ..FontContext::default()
        };
        Text {
            contexts: RefCell::new((fonts, LayoutContext::new())),
            measures: RefCell::default(),
        }
    }
}

impl Text {
    /// `text` shaped and broken into lines no wider than `wrap`, or one line without it.
    pub fn layout(&self, text: &str, wrap: Option<f32>) -> Layout<()> {
        let (fonts, layouts) = &mut *self.contexts.borrow_mut();
        let mut builder = layouts.ranged_builder(fonts, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
            FontFamilyName::Named(FAMILY.into()),
        )));
        builder.push_default(StyleProperty::FontSize(SIZE));
        builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(LINE)));
        let mut layout = builder.build(text);
        layout.break_all_lines(wrap);
        layout
    }

    /// The exact width of `text` on one line. Parley gives an empty text the width of a space.
    pub fn advance(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        self.measure(text, None).0
    }

    /// The width and line count of `text` wrapped to `wrap`, shaped only the first time.
    fn measure(&self, text: &str, wrap: Option<u64>) -> (f32, usize) {
        let key = (text.to_owned(), wrap);
        if let Some(&kept) = self.measures.borrow().get(&key) {
            return kept;
        }
        let layout = self.layout(text, wrap.map(|w| w as f32));
        let measured = (layout.full_width(), layout.len());
        let mut measures = self.measures.borrow_mut();
        if measures.len() >= KEPT {
            measures.clear();
        }
        measures.insert(key, measured);
        measured
    }
}

impl Measure for Text {
    fn width(&self, text: &str) -> u64 {
        self.advance(text).ceil() as u64
    }

    fn lines(&self, text: &str, width: u64) -> u64 {
        (self.measure(text, Some(width)).1 as u64).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_measures_kept_stay_bounded_and_right() {
        let text = Text::default();
        let wide = text.width("wide");
        for n in 0..=KEPT {
            text.width(&n.to_string());
        }
        assert!(text.measures.borrow().len() <= KEPT);
        assert_eq!(text.width("wide"), wide);
    }
}
