use super::{Int, Trap};

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

    fn at(&self, i: Int) -> Result<char, Trap> {
        usize::try_from(i.get())
            .ok()
            .and_then(|i| self.0.chars().nth(i))
            .ok_or(Trap::OutOfBounds)
    }
}
