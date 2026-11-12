use std::process::exit;

struct Canvas {
    w: i64,
    h: i64,
    cells: Vec<Vec<u8>>,
}

impl Canvas {
    fn on(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    fn put(&mut self, x: i64, y: i64, c: u8) {
        if self.on(x, y) {
            self.cells[y as usize][x as usize] = c;
        }
    }

    fn mark(&mut self, x: i64, y: i64, m: u8) {
        if !self.on(x, y) {
            return;
        }
        let old = self.cells[y as usize][x as usize];
        let cross = m == b'+'
            || (m == b'-' && (old == b'|' || old == b'+'))
            || (m == b'|' && (old == b'-' || old == b'+'));
        self.cells[y as usize][x as usize] = if cross { b'+' } else { m };
    }

    // The columns from..from + n that lie on a side of length `side`.
    fn span(from: i64, n: i64, side: i64) -> std::ops::Range<i64> {
        from.max(0)..(from + n).min(side).max(from.max(0))
    }

    fn run(&mut self, words: &[&str], text: &str) {
        let n: Vec<i64> = words[1..].iter().map(|w| number(w).unwrap_or(0)).collect();
        match words[0] {
            "hline" => Self::span(n[0], n[2], self.w).for_each(|x| self.mark(x, n[1], b'-')),
            "vline" => Self::span(n[1], n[2], self.h).for_each(|y| self.mark(n[0], y, b'|')),
            "text" => {
                for (i, c) in text.bytes().enumerate() {
                    self.put(n[0] + i as i64, n[1], c);
                }
            }
            "fill" => {
                for y in Self::span(n[1], n[3], self.h) {
                    for x in Self::span(n[0], n[2], self.w) {
                        self.put(x, y, words[5].as_bytes()[0]);
                    }
                }
            }
            "box" => {
                let (x0, y0, x1, y1) = (n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1);
                for y in Self::span(y0, n[3], self.h) {
                    for x in Self::span(x0, n[2], self.w) {
                        let (row, col) = (y == y0 || y == y1, x == x0 || x == x1);
                        match (row, col) {
                            (true, true) => self.mark(x, y, b'+'),
                            (true, false) => self.mark(x, y, b'-'),
                            (false, true) => self.mark(x, y, b'|'),
                            (false, false) => {}
                        }
                    }
                }
            }
            _ => {
                let old = self.cells.clone();
                let (dx, dy) = (n[4] - n[0], n[5] - n[1]);
                for y in Self::span(n[1], n[3], self.h) {
                    for x in Self::span(n[0], n[2], self.w) {
                        self.put(x + dx, y + dy, old[y as usize][x as usize]);
                    }
                }
            }
        }
    }
}

fn number(w: &str) -> Option<i64> {
    let digits = w.strip_prefix('-').unwrap_or(w);
    let good = !digits.is_empty()
        && digits.len() <= 4
        && digits.bytes().all(|c| c.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && w != "-0";
    if good { w.parse().ok() } else { None }
}

fn header(line: Option<&&str>) -> Option<(i64, i64)> {
    let words: Vec<&str> = line?.split(' ').collect();
    let size = |w: &str| number(w).filter(|n| (1..=80).contains(n));
    if words.len() != 2 {
        return None;
    }
    Some((size(words[0])?, size(words[1])?))
}

// The error in one command line, if any.
fn check(words: &[&str]) -> Option<&'static str> {
    let count = match words[0] {
        "hline" | "vline" | "text" => 4,
        "box" => 5,
        "fill" => 6,
        "copy" => 7,
        _ => return Some("unknown command"),
    };
    let wrong = if words[0] == "text" {
        words.len() < 4
    } else {
        words.len() != count
    };
    if wrong {
        return Some("wrong count");
    }
    let numbers = if words[0] == "text" {
        2
    } else {
        count - 1 - (words[0] == "fill") as usize
    };
    if words[1..=numbers].iter().any(|w| number(w).is_none()) {
        return Some("bad number");
    }
    if words[0] == "fill" && words[5].len() != 1 {
        return Some("bad char");
    }
    let sizes: &[usize] = match words[0] {
        "hline" | "vline" => &[3],
        "box" | "fill" | "copy" => &[3, 4],
        _ => &[],
    };
    if sizes.iter().any(|&i| number(words[i]).unwrap_or(0) < 1) {
        return Some("bad size");
    }
    None
}

fn fail(message: String) -> ! {
    println!("{message}");
    exit(20);
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let Some((w, h)) = header(lines.first()) else {
        fail("error: line 1: bad header".to_string());
    };
    let mut commands = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split(' ').collect();
        if let Some(what) = check(&words) {
            fail(format!("error: line {}: {what}", i + 1));
        }
        commands.push(words);
    }
    let mut canvas = Canvas {
        w,
        h,
        cells: vec![vec![b' '; w as usize]; h as usize],
    };
    for words in &commands {
        let rest = if words[0] == "text" {
            words[3..].join(" ")
        } else {
            String::new()
        };
        canvas.run(words, &rest);
    }
    for row in &canvas.cells {
        println!("{}", String::from_utf8_lossy(row).trim_end_matches(' '));
    }
}
