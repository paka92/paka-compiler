use paka_gui::draw::draw;
use paka_gui::png;
use paka_gui::text::Text;
use paka_rt::layout::{Measure, Rect, layout};
use paka_rt::tree::{Node, Step};

const WIDTH: u64 = 480;
const HEIGHT: u64 = 360;

/// The counter sample's first tree: a count, a Row of "-" and "+", and `??reset`.
fn counter() -> Node<()> {
    let button = |label: &str| Node::Button {
        label: label.into(),
        on: (),
    };
    Node::Col {
        gap: 8,
        children: vec![
            (Step::Place(0), Node::Text("0".into())),
            (
                Step::Place(1),
                Node::Row {
                    gap: 8,
                    children: vec![(Step::Place(0), button("-")), (Step::Place(1), button("+"))],
                },
            ),
            (Step::Place(2), Node::Hole("reset".into())),
        ],
    }
}

/// The colour at `x`,`y` of a PNG holding stored deflate blocks, as `png::encode` writes.
fn pixel(png: &[u8], x: u64, y: u64) -> [u8; 3] {
    let word = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
    let (width, idat) = (word(16), word(33));
    let mut blocks = &png[43..41 + idat - 4];
    let mut raw = Vec::new();
    while !blocks.is_empty() {
        let len = usize::from(u16::from_le_bytes([blocks[1], blocks[2]]));
        raw.extend_from_slice(&blocks[5..5 + len]);
        blocks = &blocks[5 + len..];
    }
    let at = y as usize * (1 + 4 * width) + 1 + 4 * x as usize;
    raw[at..at + 3].try_into().unwrap()
}

#[test]
fn the_counter_draws_to_a_png_with_no_display() {
    const INNER: &str = "PAKA_GUI_NO_DISPLAY";
    if std::env::var_os(INNER).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "the_counter_draws_to_a_png_with_no_display"])
            .env(INNER, "1")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DISPLAY")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let text = Text::default();
    let tree = counter();
    let rects = layout(&tree, WIDTH, &text);
    let minus = text.width("-") + 24;
    let row = Rect {
        x: 0,
        y: 28,
        w: 480,
        h: 32,
    };
    let buttons = [
        Rect { w: minus, ..row },
        Rect {
            x: minus + 8,
            w: text.width("+") + 24,
            ..row
        },
    ];
    assert_eq!(&rects[2..5], &[row, buttons[0], buttons[1]]);

    for scale in [1, 2] {
        let pixmap = draw(
            &tree,
            &rects,
            &text,
            scale as f64,
            (WIDTH * scale) as u16,
            (HEIGHT * scale) as u16,
        );
        let image = png::encode(
            (WIDTH * scale) as u32,
            (HEIGHT * scale) as u32,
            pixmap.data_as_u8_slice(),
        );
        let at = |x: u64, y: u64| pixel(&image, x * scale, y * scale);
        for b in buttons {
            let (left, right, mid) = (b.x, b.x + b.w - 1, b.y + 16);
            assert_eq!(at(left, mid), [0x80; 3], "border at scale {scale}");
            assert_eq!(at(right, mid), [0x80; 3], "border at scale {scale}");
            assert_eq!(at(left + 2, mid), [0xE6; 3], "fill at scale {scale}");
            assert_eq!(at(right - 2, mid), [0xE6; 3], "fill at scale {scale}");
            assert_eq!(at(left + 2, b.y - 1), [0xFF; 3], "above at scale {scale}");
            assert_eq!(at(left + 2, b.y + b.h), [0xFF; 3], "below at scale {scale}");
            assert_eq!(at(right + 1, mid), [0xFF; 3], "right at scale {scale}");
        }
    }
}
