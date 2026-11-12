//! The trial's one screen: a text field and a button, laid out, drawn and described from one model.
//! Sizes and colours follow M5-04; units are logical, scaled to pixels only when drawing.

use std::sync::Arc;

use accesskit::{Action, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use parley::fontique::{Blob, Collection, CollectionOptions};
use parley::{
    FontContext, FontFamily, FontFamilyName, Layout, LayoutContext, LineHeight,
    PositionedLayoutItem, StyleProperty,
};
use vello_cpu::color::AlphaColor;
use vello_cpu::kurbo::{Affine, Rect, RoundedRect, Shape};
use vello_cpu::{Glyph, Pixmap, RenderContext, Resources};

pub const FONT_PATH: &str = "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf";
const FAMILY: &str = "Noto Sans";
pub const WIDTH: f64 = 360.0;
pub const HEIGHT: f64 = 100.0;
const PAD: f64 = 16.0;
const GAP: f64 = 8.0;
const FIELD_H: f64 = 32.0;
const LABEL: &str = "Add";

const WINDOW: NodeId = NodeId(1);
const INPUT: NodeId = NodeId(2);
const BUTTON: NodeId = NodeId(3);
const STATUS: NodeId = NodeId(4);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Input,
    Button,
}

/// What the app holds plus the element state the runtime would hold.
#[derive(Default)]
pub struct Ui {
    pub text: String,
    pub preedit: String,
    pub focused: bool,
    pub count: u32,
}

impl Ui {
    pub fn click(&mut self, hit: Hit) {
        match hit {
            Hit::Input => self.focused = true,
            Hit::Button => {
                self.count += 1;
                println!("added {:?} ({})", self.text, self.count);
                self.text.clear();
            }
        }
    }

    pub fn status(&self) -> String {
        format!("Added: {}", self.count)
    }

    /// Applies an AccessKit action request; returns whether anything changed.
    pub fn act(&mut self, target: NodeId, action: Action, value: Option<&str>) -> bool {
        match (target, action) {
            (BUTTON, Action::Click) => self.click(Hit::Button),
            (INPUT, Action::Focus | Action::Click) => self.click(Hit::Input),
            (INPUT, Action::SetValue) => self.text = value.unwrap_or_default().to_string(),
            _ => return false,
        }
        true
    }
}

/// Font and layout state, loaded once.
pub struct Text {
    fonts: FontContext,
    layouts: LayoutContext<()>,
}

impl Text {
    /// Loads the one default font from a file; no system font lookup.
    pub fn load(path: &str) -> Result<Text, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("font {path}: {e}"))?;
        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: false,
        });
        collection.register_fonts(Blob::new(Arc::new(bytes)), None);
        let fonts = FontContext {
            collection,
            ..FontContext::default()
        };
        Ok(Text {
            fonts,
            layouts: LayoutContext::new(),
        })
    }

    /// One line at 14 units on a 20-unit line, as M5-05 measures text.
    pub fn line(&mut self, text: &str) -> Layout<()> {
        let mut builder = self
            .layouts
            .ranged_builder(&mut self.fonts, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
            FontFamilyName::Named(FAMILY.into()),
        )));
        builder.push_default(StyleProperty::FontSize(14.0));
        builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(20.0)));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    }

    pub fn width(&mut self, text: &str) -> f64 {
        let w = f64::from(self.line(text).full_width());
        let whole = w as u32;
        f64::from(whole + u32::from(f64::from(whole) < w))
    }
}

/// Element rectangles in logical units.
pub struct Frame {
    pub input: Rect,
    pub button: Rect,
    pub status: Rect,
}

impl Frame {
    pub fn new(text: &mut Text, width: f64) -> Frame {
        let button_w = text.width(LABEL) + 24.0;
        let right = width - PAD;
        let button = Rect::new(right - button_w, PAD, right, PAD + FIELD_H);
        let input = Rect::new(PAD, PAD, button.x0 - GAP, PAD + FIELD_H);
        let status = Rect::new(PAD, input.y1 + GAP, right, input.y1 + GAP + 20.0);
        Frame {
            input,
            button,
            status,
        }
    }

    pub fn hit(&self, x: f64, y: f64) -> Option<Hit> {
        let inside = |r: &Rect| r.contains((x, y));
        if inside(&self.input) {
            Some(Hit::Input)
        } else if inside(&self.button) {
            Some(Hit::Button)
        } else {
            None
        }
    }

    /// Where the IME should put its candidate window: the text cursor.
    pub fn cursor(&self, ui: &Ui, text: &mut Text) -> Rect {
        let x = self.input.x0 + 8.0 + text.width(&ui.text) + text.width(&ui.preedit);
        Rect::new(x, self.input.y0 + 6.0, x + 1.0, self.input.y1 - 6.0)
    }

    /// The AccessKit tree, bounds in physical pixels.
    pub fn tree(&self, ui: &Ui, scale: f64) -> TreeUpdate {
        let bounds = |r: Rect| {
            let r = r.scale_from_origin(scale);
            accesskit::Rect::new(r.x0, r.y0, r.x1, r.y1)
        };
        let mut window = Node::new(Role::Window);
        window.set_label("Paka trial");
        window.set_children(vec![INPUT, BUTTON, STATUS]);
        let mut input = Node::new(Role::TextInput);
        input.set_label("Draft");
        input.set_value(ui.text.as_str());
        input.set_bounds(bounds(self.input));
        input.add_action(Action::Focus);
        input.add_action(Action::SetValue);
        let mut button = Node::new(Role::Button);
        button.set_label(LABEL);
        button.set_bounds(bounds(self.button));
        button.add_action(Action::Click);
        let mut status = Node::new(Role::Label);
        status.set_value(ui.status());
        status.set_bounds(bounds(self.status));
        TreeUpdate {
            nodes: vec![
                (WINDOW, window),
                (INPUT, input),
                (BUTTON, button),
                (STATUS, status),
            ],
            tree: Some(TreeInfo::new(WINDOW)),
            tree_id: TreeId::ROOT,
            focus: if ui.focused { INPUT } else { WINDOW },
        }
    }
}

fn rgb(hex: u32) -> AlphaColor<vello_cpu::color::Srgb> {
    let [_, r, g, b] = hex.to_be_bytes();
    AlphaColor::from_rgb8(r, g, b)
}

/// A filled box with an inner border of `border` units.
fn boxed(ctx: &mut RenderContext, r: Rect, radius: f64, border: f64, edge: u32, fill: u32) {
    ctx.set_paint(rgb(edge));
    ctx.fill_path(&RoundedRect::from_rect(r, radius).to_path(0.1));
    ctx.set_paint(rgb(fill));
    let inner = r.inset(-border);
    ctx.fill_path(
        &RoundedRect::from_rect(
            inner,
            if radius > border {
                radius - border
            } else {
                0.0
            },
        )
        .to_path(0.1),
    );
}

fn glyphs(ctx: &mut RenderContext, res: &mut Resources, layout: &Layout<()>, x: f64, y: f64) {
    ctx.set_paint(rgb(0x000000));
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
            ctx.glyph_run(res, run.run().font())
                .font_size(run.run().font_size())
                .fill_glyphs(at)
                .expect("glyphs draw");
        }
    }
}

/// Draws the screen at `scale` into a new pixmap of `w` by `h` pixels.
pub fn draw(ui: &Ui, frame: &Frame, text: &mut Text, scale: f64, w: u16, h: u16) -> Pixmap {
    let mut ctx = RenderContext::new(w, h);
    let mut res = Resources::new();
    ctx.set_transform(Affine::scale(scale));
    ctx.set_paint(rgb(0xFFFFFF));
    ctx.fill_rect(&Rect::new(0.0, 0.0, f64::from(w), f64::from(h)));

    let (border, edge) = if ui.focused {
        (2.0, 0x0060DF)
    } else {
        (1.0, 0x808080)
    };
    boxed(&mut ctx, frame.input, 0.0, border, edge, 0xFFFFFF);
    let tx = frame.input.x0 + 8.0;
    let ty = frame.input.y0 + 6.0;
    glyphs(&mut ctx, &mut res, &text.line(&ui.text), tx, ty);
    if !ui.preedit.is_empty() {
        let px = tx + text.width(&ui.text);
        glyphs(&mut ctx, &mut res, &text.line(&ui.preedit), px, ty);
        let pw = text.width(&ui.preedit);
        ctx.fill_rect(&Rect::new(px, ty + 18.0, px + pw, ty + 19.0));
    }
    if ui.focused {
        ctx.fill_rect(&frame.cursor(ui, text));
    }

    boxed(&mut ctx, frame.button, 4.0, 1.0, 0x808080, 0xE6E6E6);
    let label = text.line(LABEL);
    glyphs(
        &mut ctx,
        &mut res,
        &label,
        frame.button.x0 + 12.0,
        frame.button.y0 + 6.0,
    );
    glyphs(
        &mut ctx,
        &mut res,
        &text.line(&ui.status()),
        frame.status.x0,
        frame.status.y0,
    );

    ctx.flush();
    let mut pixmap = Pixmap::new(w, h);
    ctx.render(&mut pixmap, &mut res);
    pixmap
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(p: &Pixmap, x: u16, y: u16) -> [u8; 3] {
        let c = p.sample(x, y);
        [c.r, c.g, c.b]
    }

    fn screen(ui: &Ui, scale: f64) -> (Pixmap, Frame) {
        let mut text = Text::load(FONT_PATH).unwrap();
        let frame = Frame::new(&mut text, WIDTH);
        let (w, h) = ((WIDTH * scale) as u16, (HEIGHT * scale) as u16);
        (draw(ui, &frame, &mut text, scale, w, h), frame)
    }

    #[test]
    fn hand_chosen_pixels_have_the_m5_04_colours() {
        let (p, f) = screen(&Ui::default(), 1.0);
        let b = f.button;
        assert_eq!(at(&p, 2, 2), [255, 255, 255]);
        assert_eq!(at(&p, b.x0 as u16 + 3, b.y0 as u16 + 3), [0xE6; 3]);
        assert_eq!(at(&p, b.x0 as u16 + 8, b.y0 as u16), [0x80; 3]);
        assert_eq!(at(&p, 16, 30), [0x80; 3]);
        assert_eq!(at(&p, 18, 30), [255, 255, 255]);
    }

    #[test]
    fn focus_draws_a_two_unit_blue_border_at_scale_two() {
        let ui = Ui {
            focused: true,
            ..Ui::default()
        };
        let (p, _) = screen(&ui, 2.0);
        for x in [32, 35] {
            assert_eq!(at(&p, x, 60), [0x00, 0x60, 0xDF]);
        }
        assert_eq!(at(&p, 36, 40), [255, 255, 255]);
    }

    #[test]
    fn typed_text_draws_ink_in_the_field() {
        let ink = |ui: &Ui| {
            let (p, f) = screen(ui, 1.0);
            let r = f.input.inset(-4.0);
            (r.x0 as u16..r.x1 as u16)
                .flat_map(|x| (r.y0 as u16..r.y1 as u16).map(move |y| (x, y)))
                .filter(|&(x, y)| at(&p, x, y)[0] < 128)
                .count()
        };
        let typed = Ui {
            text: "milk".into(),
            ..Ui::default()
        };
        assert_eq!(ink(&Ui::default()), 0);
        assert!(ink(&typed) > 20);
    }

    #[test]
    fn layout_is_hand_computable() {
        let mut text = Text::load(FONT_PATH).unwrap();
        let f = Frame::new(&mut text, WIDTH);
        assert_eq!((f.button.x1, f.button.y0, f.button.y1), (344.0, 16.0, 48.0));
        assert_eq!(f.input.x1, f.button.x0 - 8.0);
        assert_eq!(f.hit(20.0, 20.0), Some(Hit::Input));
        assert_eq!(f.hit(343.0, 47.0), Some(Hit::Button));
        assert_eq!(f.hit(5.0, 5.0), None);
    }

    #[test]
    fn the_tree_names_both_elements_and_a_click_adds() {
        let mut ui = Ui {
            text: "milk".into(),
            ..Ui::default()
        };
        let mut text = Text::load(FONT_PATH).unwrap();
        let f = Frame::new(&mut text, WIDTH);
        let tree = f.tree(&ui, 1.0);
        let roles: Vec<_> = tree.nodes.iter().map(|(_, n)| n.role()).collect();
        assert_eq!(
            roles,
            [Role::Window, Role::TextInput, Role::Button, Role::Label]
        );
        assert_eq!(tree.nodes[1].1.value(), Some("milk"));
        assert!(ui.act(BUTTON, Action::Click, None));
        assert_eq!((ui.count, ui.text.as_str()), (1, ""));
        assert!(ui.act(INPUT, Action::SetValue, Some("eggs")));
        assert_eq!(f.tree(&ui, 1.0).nodes[1].1.value(), Some("eggs"));
    }
}
