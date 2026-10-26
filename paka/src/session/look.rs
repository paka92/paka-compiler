//! The Look calls on the running visual app, and a run that launches one
//! (docs/decisions/M5-09.md section 2). Look acts on the element tree, never on a screen.

use super::Session;
use super::visual::{Shown, Visual};
use crate::serve::{Call, Failure, Reply, trap};
use paka_gui::text::Text;
use paka_json::Value;
use paka_rt::input::Key;
use paka_rt::route::{Event, focused};
use paka_rt::tree;
use std::ffi::OsString;

impl Session {
    /// A run of the visual app `app` as `order` asks: launched with no window, its first tree at
    /// once. A run while one runs ends that one first.
    pub(super) fn run_visual(&mut self, app: &str, order: &Value) -> Result<Value, Failure> {
        let args: Vec<OsString> = match order.get("args") {
            Some(Value::Array(args)) => args.iter().filter_map(Value::as_str).map(Into::into),
            _ => unreachable!("an order has its arguments"),
        }
        .collect();
        self.visual = None;
        Ok(self.started(args)?.run(app, false))
    }

    pub(super) fn look(&mut self, call: &Call) -> Reply {
        let visual = self.visual.as_ref().ok_or_else(not_running)?;
        match call.name {
            "look_tree" => {
                let tree = visual.with(|ui, _| ui.tree.to_string());
                let tree = tree.ok_or_else(not_running)?;
                Ok(vec![("tree", tree.into()), ("running", true.into())])
            }
            "look_screenshot" => screenshot(visual, call.int("scale").unwrap_or(1)),
            "look_click" => {
                let path = call.str("path").expect("a click has a path");
                let steps = tree::path(path).ok_or_else(|| {
                    Failure::new("bad_request", format!("`{path}` is not a path"))
                })?;
                let clicked = visual.with(move |ui, _| {
                    let kind = ui.tree.at(&steps)?.kind();
                    let before = ui.traps.len();
                    ui.send([Event::Click(steps, kind)]);
                    Some(super::visual::shown_since(ui, before))
                });
                let shown = clicked
                    .ok_or_else(not_running)?
                    .ok_or_else(|| Failure::new("no_element", format!("no element at `{path}`")))?;
                Ok(self.turned(shown))
            }
            _ => {
                let keys = keys(call.str("text"), call.str("key"))?;
                let typed = call.str("text").is_some();
                let pressed = visual.with(move |ui, _| {
                    if typed && focused(&mut ui.tree).is_none() {
                        return None;
                    }
                    let before = ui.traps.len();
                    ui.send(keys.into_iter().map(Event::Key));
                    Some(super::visual::shown_since(ui, before))
                });
                let shown = pressed
                    .ok_or_else(not_running)?
                    .ok_or_else(|| Failure::new("no_focus", "no Input has focus to type into"))?;
                Ok(self.turned(shown))
            }
        }
    }

    /// The reply to a click or key that left `shown`.
    fn turned(&mut self, shown: Shown) -> Vec<(&'static str, Value)> {
        let app = self
            .visual
            .as_ref()
            .map(|v| v.app.clone())
            .unwrap_or_default();
        let shown = self.after(shown);
        let trap = shown.trap.as_deref().map_or(Value::Null, |t| trap(&app, t));
        vec![
            ("tree", shown.tree.into()),
            ("running", shown.end.is_none().into()),
            ("trap", trap),
        ]
    }
}

/// The current tree drawn at `scale` (1 or 2) as a base64 PNG, at the windowless size.
fn screenshot(visual: &Visual, scale: u64) -> Reply {
    if !(1..=2).contains(&scale) {
        return Err(Failure::new("bad_request", "`scale` must be 1 or 2"));
    }
    let drawn = visual
        .with(move |ui, _| paka_gui::screenshot(&ui.tree, &Text::default(), scale, paka_gui::SIZE));
    let (png, width, height) = drawn.ok_or_else(not_running)?;
    Ok(vec![
        ("png", base64(&png).into()),
        ("width", (width as usize).into()),
        ("height", (height as usize).into()),
    ])
}

/// The keys `look_key` sends: each code point of `text`, or the one named `key`.
fn keys(text: Option<&str>, key: Option<&str>) -> Result<Vec<Key>, Failure> {
    let bad = |message: String| Err(Failure::new("bad_request", message));
    let name = match (text, key) {
        (Some(text), None) => return Ok(text.chars().map(Key::Type).collect()),
        (None, Some(name)) => name,
        _ => return bad("give exactly one of `text` and `key`".into()),
    };
    let (shift, base) = match name.strip_prefix("Shift+") {
        Some(base) => (true, base),
        None => (false, name),
    };
    // Tab, Shift+Tab and Enter move focus or click a Button (M5-08), which M5-36 builds.
    let key = match (shift, base) {
        (false, "Tab" | "Enter") | (true, "Tab") => None,
        (false, "Space") => Some(Key::Type(' ')),
        (false, "Backspace") => Some(Key::Back),
        (false, "Delete") => Some(Key::Delete),
        (_, "Left") => Some(Key::Left(shift)),
        (_, "Right") => Some(Key::Right(shift)),
        (_, "Home") => Some(Key::Home(shift)),
        (_, "End") => Some(Key::End(shift)),
        _ => return bad(format!("no key `{name}`")),
    };
    Ok(key.into_iter().collect())
}

fn not_running() -> Failure {
    Failure::new("not_running", "no visual app is running")
}

/// `bytes` in base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk
            .iter()
            .enumerate()
            .fold(0u32, |w, (i, b)| w | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            out.push(match i <= chunk.len() {
                true => DIGITS[(word >> (18 - 6 * i) & 63) as usize] as char,
                false => '=',
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_pads_to_whole_quads() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
        ];
        for (plain, coded) in cases {
            assert_eq!(super::base64(plain.as_bytes()), coded);
        }
    }
}
