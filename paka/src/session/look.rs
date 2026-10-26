//! The Look calls on the running visual app, and a run that launches one
//! (docs/decisions/M5-09.md section 2). Look acts on the element tree, never on a screen. An app
//! in a window runs in the dev runner, which answers its Look calls (M5-38).

use super::Session;
use super::visual::{Shown, Visual, not_running, shown_since};
use crate::serve::{Call, Failure, Reply, trap};
use paka_gui::text::Text;
use paka_json::Value;
use paka_rt::input::Key;
use paka_rt::route::{Event, focused};
use paka_rt::tree::{self, Node};
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

    /// The Look `call` on the windowless app, or else on the runner's app in a window.
    pub(super) fn look(&mut self, call: &Call) -> Reply {
        let Some(visual) = &self.visual else {
            return self.runner.look(call);
        };
        let reply = visual.look(call)?;
        if reply.contains(&("running", false.into())) {
            (self.visual, self.choice) = (None, None);
        }
        Ok(reply)
    }
}

impl Visual {
    /// The reply to the Look `call` on this app.
    pub fn look(&self, call: &Call) -> Reply {
        match call.name {
            "look_tree" => {
                let tree = self.with(|ui, _| ui.tree.to_string());
                let tree = tree.ok_or_else(not_running)?;
                Ok(vec![("tree", tree.into()), ("running", true.into())])
            }
            "look_screenshot" => self.screenshot(call.int("scale").unwrap_or(1)),
            "look_click" => {
                let path = call.str("path").expect("a click has a path");
                let steps = tree::path(path).ok_or_else(|| {
                    Failure::new("bad_request", format!("`{path}` is not a path"))
                })?;
                let clicked = self.with(move |ui, _| {
                    let kind = ui.tree.at(&steps)?.kind();
                    let before = ui.traps.len();
                    ui.send([Event::Click(steps, kind)]);
                    Some(shown_since(ui, before))
                });
                let shown = clicked
                    .ok_or_else(not_running)?
                    .ok_or_else(|| Failure::new("no_element", format!("no element at `{path}`")))?;
                Ok(self.turned(shown))
            }
            _ => {
                let keys = keys(call.str("text"), call.str("key"))?;
                let typed = call.str("text").is_some();
                let pressed = self.with(move |ui, _| {
                    if typed && !matches!(focused(&mut ui.tree), Some(Node::Input { .. })) {
                        return None;
                    }
                    let before = ui.traps.len();
                    ui.send(keys);
                    Some(shown_since(ui, before))
                });
                let shown = pressed
                    .ok_or_else(not_running)?
                    .ok_or_else(|| Failure::new("no_focus", "no Input has focus to type into"))?;
                Ok(self.turned(shown))
            }
        }
    }

    /// The reply to a click or key that left `shown`.
    fn turned(&self, shown: Shown) -> Vec<(&'static str, Value)> {
        let trap = (shown.trap.as_deref()).map_or(Value::Null, |t| trap(&self.app, t));
        vec![
            ("tree", shown.tree.into()),
            ("running", shown.end.is_none().into()),
            ("trap", trap),
        ]
    }

    /// The current tree drawn at `scale` (1 or 2) as a base64 PNG, at the window's size in units.
    fn screenshot(&self, scale: u64) -> Reply {
        if !(1..=2).contains(&scale) {
            return Err(Failure::new("bad_request", "`scale` must be 1 or 2"));
        }
        let size = self.size();
        let drawn =
            self.with(move |ui, _| paka_gui::screenshot(&ui.tree, &Text::default(), scale, size));
        let (png, width, height) = drawn.ok_or_else(not_running)?;
        Ok(vec![
            ("png", base64(&png).into()),
            ("width", (width as usize).into()),
            ("height", (height as usize).into()),
        ])
    }
}

/// The events `look_key` sends: each code point of `text`, or the one named `key`.
fn keys(text: Option<&str>, key: Option<&str>) -> Result<Vec<Event>, Failure> {
    let bad = |message: String| Err(Failure::new("bad_request", message));
    let name = match (text, key) {
        (Some(text), None) => return Ok(text.chars().map(|c| Event::Key(Key::Type(c))).collect()),
        (None, Some(name)) => name,
        _ => return bad("give exactly one of `text` and `key`".into()),
    };
    let (shift, base) = match name.strip_prefix("Shift+") {
        Some(base) => (true, base),
        None => (false, name),
    };
    let key = match (shift, base) {
        (_, "Tab") => return Ok(vec![Event::Tab(shift)]),
        (false, "Enter") => Key::Type('\n'),
        (false, "Space") => Key::Type(' '),
        (false, "Backspace") => Key::Back,
        (false, "Delete") => Key::Delete,
        (_, "Left") => Key::Left(shift),
        (_, "Right") => Key::Right(shift),
        (_, "Home") => Key::Home(shift),
        (_, "End") => Key::End(shift),
        _ => return bad(format!("no key `{name}`")),
    };
    Ok(vec![Event::Key(key)])
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
