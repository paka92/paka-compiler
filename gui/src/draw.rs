//! Drawing the laid-out element tree through vello_cpu in M5-04's fixed looks. Units become
//! pixels only here, through the scale factor; the only clip is an Input's own text (M5-05).

use std::slice;

use paka_rt::layout::Rect;
use paka_rt::tree::{Edit, Node};
use parley::{Layout, PositionedLayoutItem};
use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{self, Affine, RoundedRect, Shape};
use vello_cpu::{Glyph, Pixmap, RenderContext, Resources};

use crate::text::Text;

const WHITE: u32 = 0xFFFFFF;
const BLACK: u32 = 0x000000;
const GREY: u32 = 0x808080;
const BUTTON: u32 = 0xE6E6E6;
const FOCUS: u32 = 0x0060DF;
const SELECTION: u32 = 0xB4D5FE;
const ERROR_FILL: u32 = 0xFFE0E0;
const ERROR_EDGE: u32 = 0xC00000;
const DASH: u64 = 4;

/// Draws `root`, whose nodes `rects` places in pre-order as `paka_rt::layout::layout` gives
/// them, on white into a `width` by `height` pixel image at `scale`. Every pixel is opaque.
pub fn draw<M>(
    root: &Node<M>,
    rects: &[Rect],
    text: &Text,
    scale: f64,
    width: u16,
    height: u16,
) -> Pixmap {
    let mut ctx = RenderContext::new(width, height);
    ctx.set_paint(rgb(WHITE));
    ctx.fill_rect(&kurbo::Rect::new(0.0, 0.0, width.into(), height.into()));
    ctx.set_transform(Affine::scale(scale));
    let mut painter = Painter {
        ctx,
        res: Resources::new(),
        text,
    };
    painter.node(root, &mut rects.iter());
    let Painter {
        mut ctx, mut res, ..
    } = painter;
    ctx.flush();
    let mut pixmap = Pixmap::new(width, height);
    ctx.render(&mut pixmap, &mut res);
    pixmap
}

struct Painter<'a> {
    ctx: RenderContext,
    res: Resources,
    text: &'a Text,
}

impl Painter<'_> {
    fn node<M>(&mut self, node: &Node<M>, rects: &mut slice::Iter<Rect>) {
        let Some(&r) = rects.next() else { return };
        let (x, y) = (r.x as f64, r.y as f64);
        match node {
            Node::Text(s) => self.wrapped(s, x, y, r.w),
            Node::Button { label, .. } => {
                self.boxed(r, 4.0, 1.0, GREY, BUTTON);
                self.line(label, x + 12.0, y + 6.0);
            }
            Node::Input { text, edit, .. } => self.input(r, text, edit),
            Node::Hole(name) => {
                self.dashed(r);
                self.line(name, x + 12.0, y + 6.0);
            }
            Node::Error(message) => {
                self.boxed(r, 0.0, 1.0, ERROR_EDGE, ERROR_FILL);
                self.wrapped(message, x + 6.0, y + 6.0, r.w.saturating_sub(12));
            }
            Node::Row { .. } | Node::Col { .. } => {}
        }
        for (_, child) in node.children() {
            self.node(child, rects);
        }
    }

    /// The Input's box, then its text with the preedit at the cursor, selection behind and
    /// cursor in front when focused, all clipped to the inside of the border.
    fn input(&mut self, r: Rect, text: &str, edit: &Edit) {
        let (border, edge) = if edit.focus {
            (2.0, FOCUS)
        } else {
            (1.0, GREY)
        };
        self.boxed(r, 0.0, border, edge, WHITE);
        let at = text
            .char_indices()
            .nth(edit.cursor)
            .map_or(text.len(), |(i, _)| i);
        let shown = format!("{}{}{}", &text[..at], edit.preedit, &text[at..]);
        let pre = edit.preedit.chars().count();
        let x_at = |n: usize| {
            let end = shown.char_indices().nth(n).map_or(shown.len(), |(i, _)| i);
            f64::from(self.text.advance(&shown[..end]))
        };
        // Text positions after the cursor sit behind the preedit.
        let shift = |n: usize| if n > edit.cursor { n + pre } else { n };
        let (start, cursor) = (x_at(edit.cursor), x_at(edit.cursor + pre));
        let (x, y) = (r.x as f64 + 8.0, r.y as f64 + 6.0);
        self.ctx.push_clip_rect(&inner(r, border));
        if !edit.sel.is_empty() {
            let (from, to) = (x_at(shift(edit.sel.start)), x_at(shift(edit.sel.end)));
            self.fill(x + from, y, x + to, y + 20.0, SELECTION);
        }
        self.line(&shown, x, y);
        if pre > 0 {
            self.fill(x + start, y + 18.0, x + cursor, y + 19.0, BLACK);
        }
        if edit.focus {
            self.fill(x + cursor, y, x + cursor + 1.0, y + 20.0, BLACK);
        }
        self.ctx.pop_clip();
    }

    /// A filled box with a border `border` units wide inside its edge.
    fn boxed(&mut self, r: Rect, radius: f64, border: f64, edge: u32, fill: u32) {
        self.ctx.set_paint(rgb(edge));
        self.ctx
            .fill_path(&RoundedRect::from_rect(outer(r), radius).to_path(0.1));
        self.ctx.set_paint(rgb(fill));
        let radius = if radius > border {
            radius - border
        } else {
            0.0
        };
        let round = RoundedRect::from_rect(inner(r, border), radius);
        self.ctx.fill_path(&round.to_path(0.1));
    }

    /// A one-unit border of dashes, `DASH` units on and off.
    fn dashed(&mut self, r: Rect) {
        let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
        for x in (x0..x1).step_by(2 * DASH as usize) {
            let end = (x + DASH).min(x1);
            self.fill(x as f64, y0 as f64, end as f64, (y0 + 1) as f64, GREY);
            self.fill(x as f64, (y1 - 1) as f64, end as f64, y1 as f64, GREY);
        }
        for y in (y0..y1).step_by(2 * DASH as usize) {
            let end = (y + DASH).min(y1);
            self.fill(x0 as f64, y as f64, (x0 + 1) as f64, end as f64, GREY);
            self.fill((x1 - 1) as f64, y as f64, x1 as f64, end as f64, GREY);
        }
    }

    fn fill(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, colour: u32) {
        self.ctx.set_paint(rgb(colour));
        self.ctx.fill_rect(&kurbo::Rect::new(x0, y0, x1, y1));
    }

    /// One line; a line break is drawn as a space (M5-04).
    fn line(&mut self, s: &str, x: f64, y: f64) {
        let layout = self.text.layout(&s.replace('\n', " "), None);
        self.glyphs(&layout, x, y);
    }

    /// Lines broken to `width`, as layout measured them.
    fn wrapped(&mut self, s: &str, x: f64, y: f64, width: u64) {
        let layout = self.text.layout(s, Some(width as f32));
        self.glyphs(&layout, x, y);
    }

    fn glyphs(&mut self, layout: &Layout<()>, x: f64, y: f64) {
        self.ctx.set_paint(rgb(BLACK));
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                let at = run.positioned_glyphs().map(|g| Glyph {
                    id: g.id,
                    x: x as f32 + g.x,
                    y: y as f32 + g.y,
                });
                // The shipped font always parses; a run that still fails is left blank.
                let _ = self
                    .ctx
                    .glyph_run(&mut self.res, run.run().font())
                    .font_size(run.run().font_size())
                    .fill_glyphs(at);
            }
        }
    }
}

fn outer(r: Rect) -> kurbo::Rect {
    kurbo::Rect::new(
        r.x as f64,
        r.y as f64,
        (r.x + r.w) as f64,
        (r.y + r.h) as f64,
    )
}

fn inner(r: Rect, border: f64) -> kurbo::Rect {
    outer(r).inset(-border)
}

fn rgb(hex: u32) -> AlphaColor<Srgb> {
    let [_, r, g, b] = hex.to_be_bytes();
    AlphaColor::from_rgb8(r, g, b)
}
