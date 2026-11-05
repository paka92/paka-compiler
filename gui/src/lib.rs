//! A visual app's element tree, layout and input, and its window, drawing and accessibility side
//! (docs/decisions/M5-08.md).

pub mod access;
pub mod draw;
pub mod events;
pub mod input;
pub mod layout;
pub mod png;
pub mod route;
pub mod text;
pub mod tree;
pub mod window;

use crate::layout::layout;
use crate::tree::Node;
use text::Text;

/// Noto Sans Regular, the one font every visual app draws with; its notice is `fonts/OFL.txt`.
pub const FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// The size in units a window opens at, and a screenshot's with no window (M5-05).
pub const SIZE: (u64, u64) = (480, 360);

/// `root` laid out `size` units wide and tall and drawn at `scale` as a PNG; and its size in
/// pixels.
pub fn screenshot<M>(
    root: &Node<M>,
    text: &Text,
    scale: u64,
    size: (u64, u64),
) -> (Vec<u8>, u32, u32) {
    let rects = layout(root, size.0, text);
    let (w, h) = (size.0 * scale, size.1 * scale);
    let pixmap = draw::draw(root, &rects, text, scale as f64, w as u16, h as u16);
    let (w, h) = (w as u32, h as u32);
    (png::encode(w, h, pixmap.data_as_u8_slice()), w, h)
}
