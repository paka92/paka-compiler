use super::{CAP, Int, Trap};

/// A row of Unicode scalar values. Positions and lengths count code points (M2-03 1.7); the
/// derived order is by code point value, which is the order of the UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text(String);

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        Text(s.into())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        Text(s)
    }
}

impl Text {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len(&self) -> Int {
        Int::new(self.0.chars().count() as i64)
    }

    /// The code point at `i`, as a one-code-point String.
    pub fn index(&self, i: Int) -> Result<Text, Trap> {
        self.at(i).map(|c| Text(c.into()))
    }

    pub fn code_at(&self, i: Int) -> Result<Int, Trap> {
        self.at(i).map(|c| Int::new(u32::from(c).into()))
    }

    /// The code points from `from` up to `to` excluded.
    pub fn slice(&self, from: Int, to: Int) -> Result<Text, Trap> {
        let (from, to) = (from.get(), to.get());
        if !(0..=to).contains(&from) || to > self.len().get() {
            return Err(Trap::OutOfBounds);
        }
        let byte = |n: i64| {
            self.0
                .char_indices()
                .nth(n as usize)
                .map_or(self.0.len(), |(b, _)| b)
        };
        Ok(Text(self.0[byte(from)..byte(to)].into()))
    }

    pub fn add(&self, other: &Text) -> Text {
        Text(self.0.clone() + &other.0)
    }

    /// Appends `other` in place.
    pub fn push(&mut self, other: &Text) {
        self.0.push_str(&other.0);
    }

    pub fn min(self, other: Text) -> Text {
        Ord::min(self, other)
    }

    pub fn max(self, other: Text) -> Text {
        Ord::max(self, other)
    }

    /// A one-code-point String; surrogates and values past 1114111 trap.
    pub fn from_code(n: Int) -> Result<Text, Trap> {
        u32::try_from(n.get())
            .ok()
            .and_then(char::from_u32)
            .map(|c| Text(c.into()))
            .ok_or(Trap::BadConversion)
    }

    /// The runs between white space, in order.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.0.split(white).filter(|w| !w.is_empty())
    }

    /// Where `part` first starts, in code points.
    pub fn find(&self, part: &Text) -> Option<Int> {
        let at = self.0.find(&part.0)?;
        Some(Int::new(self.0[..at].chars().count() as i64))
    }

    /// The pieces between matches of `sep`; one per code point for an empty `sep`. A `CAP`-th
    /// match would make one piece too many, and only a text of `CAP` copies of `sep` can hold one.
    pub fn split(&self, sep: &Text) -> Result<Vec<&str>, Trap> {
        let (t, sep) = (self.as_str(), sep.as_str());
        match sep {
            "" => Ok(self.chars().collect()),
            _ if t.len() / sep.len() >= CAP && t.matches(sep).nth(CAP - 1).is_some() => {
                Err(Trap::Memory)
            }
            _ => Ok(t.split(sep).collect()),
        }
    }

    /// Each code point as a String.
    pub fn chars(&self) -> impl Iterator<Item = &str> {
        let t = self.as_str();
        t.char_indices().map(move |(b, c)| &t[b..b + c.len_utf8()])
    }

    /// The pieces between line feeds, without an empty last one.
    pub fn lines(&self) -> Vec<&str> {
        let mut parts: Vec<_> = self.0.split('\n').collect();
        if parts.last() == Some(&"") {
            parts.pop();
        }
        parts
    }

    /// `parts` with `self` between each two.
    pub fn join(&self, parts: &[&str]) -> Result<Text, Trap> {
        let bytes: usize = parts.iter().map(|p| p.len()).sum();
        let seps = self.0.len() as u128 * parts.len().saturating_sub(1) as u128;
        room(bytes as u128 + seps)?;
        Ok(Text(parts.join(self.as_str())))
    }

    pub fn repeat(&self, n: Int) -> Result<Text, Trap> {
        let times = usize::try_from(n.get()).map_err(|_| Trap::OutOfBounds)?;
        room(self.0.len() as u128 * times as u128)?;
        Ok(Text(self.0.repeat(times)))
    }

    /// Spaces at the front or the back until the String is `width` code points long.
    pub fn pad(&self, width: Int, front: bool) -> Result<Text, Trap> {
        let width = usize::try_from(width.get()).unwrap_or(0);
        let fill = width.saturating_sub(self.0.chars().count());
        room(self.0.len() as u128 + fill as u128)?;
        let spaces = " ".repeat(fill);
        Ok(Text(match front {
            true => spaces + &self.0,
            false => self.0.clone() + &spaces,
        }))
    }

    /// Every match of `old`, found as `split` finds them, becomes `new`; an empty `old` matches
    /// nothing.
    pub fn replace(&self, old: &Text, new: &Text) -> Result<Text, Trap> {
        let (s, old, new) = (self.as_str(), old.as_str(), new.as_str());
        if old.is_empty() {
            return Ok(self.clone());
        }
        let matches = s.matches(old).count() as i128;
        room((s.len() as i128 + matches * (new.len() as i128 - old.len() as i128)) as u128)?;
        Ok(Text(s.replace(old, new)))
    }

    fn at(&self, i: Int) -> Result<char, Trap> {
        usize::try_from(i.get())
            .ok()
            .and_then(|i| self.0.chars().nth(i))
            .ok_or(Trap::OutOfBounds)
    }
}

/// Checks `CAP` before a String of `bytes` bytes is made.
fn room(bytes: u128) -> Result<(), Trap> {
    match bytes > CAP as u128 {
        true => Err(Trap::Memory),
        false => Ok(()),
    }
}

/// White space: the six characters 9 to 13 and 32 (M2-14 section 1.14).
pub fn white(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_stops_one_past_the_cap() {
        let (a, cap) = (Text::from("a"), CAP as i64);
        assert!(a.repeat(Int::new(cap)).is_ok());
        assert_eq!(a.repeat(Int::new(cap + 1)), Err(Trap::Memory));
        assert!(a.pad(Int::new(cap), true).is_ok());
        assert_eq!(a.pad(Int::new(cap + 1), false), Err(Trap::Memory));
        let ab = Text::from("ab");
        let c = |n| Text::from("c".repeat(n));
        assert!(ab.replace(&Text::from("b"), &c(CAP - 1)).is_ok());
        assert_eq!(ab.replace(&Text::from("b"), &c(CAP)), Err(Trap::Memory));
        assert!(Text::from("").join(&["a", "b"]).is_ok());
        assert_eq!(c(CAP).join(&["a", "b"]), Err(Trap::Memory));
    }

    #[test]
    fn split_stops_one_past_the_cap() {
        let comma = Text::from(",");
        assert_eq!(
            Text::from(",".repeat(CAP - 1))
                .split(&comma)
                .map(|p| p.len()),
            Ok(CAP)
        );
        assert_eq!(Text::from(",".repeat(CAP)).split(&comma), Err(Trap::Memory));
    }
}
