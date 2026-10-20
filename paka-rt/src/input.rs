//! Editing an Input (M5-03): keys work out a new text and cursor against the text shown; the
//! element changes only when the view hands a different text back. Positions count code points.

use crate::tree::Edit;
use std::ops::Range;

/// A key that reaches the focused Input. `bool` extends the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Type(char),
    Back,
    Delete,
    Left(bool),
    Right(bool),
    Home(bool),
    End(bool),
}

/// The whole new text to send through `on_change:`, and where the key puts the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub text: String,
    pub cursor: usize,
}

impl Edit {
    /// Applies `key` to `text`. A move changes the edit and sends nothing; an edit leaves it as
    /// is and returns the change to send. A key that would not change the text gives `None`.
    pub fn key(&mut self, text: &str, key: Key) -> Option<Change> {
        let len = text.chars().count();
        self.select(self.sel.start.min(len), self.sel.end.min(len));
        self.cursor = self.cursor.min(len);
        let (sel, at) = (self.sel.clone(), self.cursor);
        let cut = if sel.is_empty() { at..at } else { sel.clone() };
        let (to, extend) = match key {
            Key::Type('\n' | '\r' | '\t') => return None,
            Key::Type(c) => return Some(splice(text, cut, Some(c))),
            Key::Back | Key::Delete if !sel.is_empty() => return Some(splice(text, cut, None)),
            Key::Back if at > 0 => return Some(splice(text, at - 1..at, None)),
            Key::Delete if at < len => return Some(splice(text, at..at + 1, None)),
            Key::Back | Key::Delete => return None,
            Key::Left(false) if !sel.is_empty() => (sel.start, false),
            Key::Right(false) if !sel.is_empty() => (sel.end, false),
            Key::Left(x) => (at.saturating_sub(1), x),
            Key::Right(x) => ((at + 1).min(len), x),
            Key::Home(x) => (0, x),
            Key::End(x) => (len, x),
        };
        let anchor = match sel {
            _ if !extend => to,
            sel if sel.is_empty() => at,
            sel if sel.start == at => sel.end,
            sel => sel.start,
        };
        self.select(anchor, to);
        self.cursor = to;
        None
    }

    /// Follows the text the view handed back (M5-03 section 1.3). The same text as `before` moves
    /// nothing; any other is shown with the cursor at `sent`, or where it was when the app set the
    /// text itself, pulled back to the end, and the selection cleared.
    pub fn follow(&mut self, before: &str, after: &str, sent: Option<usize>) {
        if before != after {
            self.cursor = sent.unwrap_or(self.cursor).min(after.chars().count());
            self.sel = 0..0;
        }
    }

    fn select(&mut self, a: usize, b: usize) {
        self.sel = if a == b { 0..0 } else { a.min(b)..a.max(b) };
    }
}

/// `text` with the code points in `cut` replaced by `put`, the cursor just after.
fn splice(text: &str, cut: Range<usize>, put: Option<char>) -> Change {
    let byte = |i| text.char_indices().nth(i).map_or(text.len(), |(b, _)| b);
    let mut new = text[..byte(cut.start)].to_string();
    new.extend(put);
    new.push_str(&text[byte(cut.end)..]);
    Change {
        text: new,
        cursor: cut.start + usize::from(put.is_some()),
    }
}

#[cfg(test)]
mod tests;
