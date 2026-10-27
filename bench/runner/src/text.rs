//! The one ASCII text expr_parse and word_count read (M6-02 section 2): lines of arithmetic, each
//! token followed by one space or the line end, from the programs' wrapping generator.
//!
//! An expression nests at most `DEPTH` brackets, has at most four terms and multiplies only by a
//! digit, so no value passes 36 * 9,999 * 36^8, about 10^18: evaluating never overflows.

const BYTES: usize = 8_000_000;
const DEPTH: u32 = 8;

struct Gen(u64);

impl Gen {
    fn next(&mut self, below: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) & 9007199254740991) % below
    }
}

pub fn make() -> String {
    let mut g = Gen(1);
    let mut out = String::with_capacity(BYTES + 4096);
    while out.len() < BYTES {
        expr(&mut g, DEPTH, &mut out);
        out.push('\n');
    }
    out
}

fn expr(g: &mut Gen, depth: u32, out: &mut String) {
    for t in 0..1 + g.next(4) {
        if t > 0 {
            out.push_str(if g.next(2) == 0 { " + " } else { " - " });
        }
        factor(g, depth, out);
        if g.next(2) == 0 {
            out.push_str(&format!(" * {}", 1 + g.next(9)));
        }
    }
}

fn factor(g: &mut Gen, depth: u32, out: &mut String) {
    if depth > 0 && g.next(3) == 0 {
        out.push_str("( ");
        expr(g, depth - 1, out);
        out.push_str(" )");
    } else {
        out.push_str(&g.next(10000).to_string());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_text_is_fixed_ascii_lines() {
        let text = super::make();
        assert_eq!(text, super::make());
        assert!(text.len() >= super::BYTES && text.len() < 1 << 24);
        assert!(text.ends_with('\n') && !text.contains("  "));
        let ok = |c: char| c.is_ascii_digit() || " +-*()\n".contains(c);
        assert!(text.chars().all(ok));
    }
}
