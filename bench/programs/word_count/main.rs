// Count the words of the text in a map, repeated; each count is added as it grows.

use std::collections::HashMap;

fn main() {
    let path = std::env::args().nth(1).expect("usage: word_count <text>");
    let text = std::fs::read_to_string(path).expect("cannot read the text");
    let words: Vec<&str> = text.split_ascii_whitespace().collect();
    let mut total: i64 = 0;
    for _ in 0..20 {
        let mut counts: HashMap<&str, i64> = HashMap::new();
        for w in &words {
            let c = counts.entry(w).or_insert(0);
            *c += 1;
            total += *c;
        }
        total += counts.len() as i64;
    }
    println!("{total}");
}
