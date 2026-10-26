//! The window, drawing and accessibility side of a visual app (docs/decisions/M5-08.md).

/// Noto Sans Regular, the one font every visual app draws with; its notice is `fonts/OFL.txt`.
pub const FONT: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
