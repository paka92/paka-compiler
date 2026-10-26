//! Text shaping and measuring through Parley (M5-05): the one shipped font, 14 units on 20-unit
//! lines, at scale factor 1. Layout and drawing both measure through here, so they agree.

use std::cell::RefCell;
use std::sync::Arc;

use paka_rt::layout::Measure;
use parley::fontique::{Blob, Collection, CollectionOptions};
use parley::{
    FontContext, FontFamily, FontFamilyName, Layout, LayoutContext, LineHeight, StyleProperty,
};

const FAMILY: &str = "Noto Sans";
const SIZE: f32 = 14.0;
const LINE: f32 = 20.0;

/// The font and Parley's reusable state, loaded once.
pub struct Text {
    contexts: RefCell<(FontContext, LayoutContext<()>)>,
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
        self.layout(text, None).full_width()
    }
}

impl Measure for Text {
    fn width(&self, text: &str) -> u64 {
        self.advance(text).ceil() as u64
    }

    fn lines(&self, text: &str, width: u64) -> u64 {
        (self.layout(text, Some(width as f32)).len() as u64).max(1)
    }
}
