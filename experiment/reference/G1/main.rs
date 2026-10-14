use std::{env, fs, process};

enum Entry<'a> {
    Keep(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

impl Entry<'_> {
    fn uses_old(&self) -> bool {
        !matches!(self, Entry::Insert(_))
    }

    fn uses_new(&self) -> bool {
        !matches!(self, Entry::Delete(_))
    }

    fn is_change(&self) -> bool {
        !matches!(self, Entry::Keep(_))
    }

    fn text(&self) -> String {
        match self {
            Entry::Keep(line) => format!(" {line}"),
            Entry::Delete(line) => format!("-{line}"),
            Entry::Insert(line) => format!("+{line}"),
        }
    }
}

fn context_size(arg: &str) -> Option<usize> {
    if !arg.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    arg.parse().ok().filter(|&c| c <= 10)
}

fn walk<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Entry<'a>> {
    let (n, m) = (old.len(), new.len());
    let mut common = vec![vec![0; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            common[i][j] = if old[i] == new[j] {
                common[i + 1][j + 1] + 1
            } else {
                common[i + 1][j].max(common[i][j + 1])
            };
        }
    }

    let (mut i, mut j) = (0, 0);
    let mut list = vec![];
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            list.push(Entry::Keep(old[i]));
            i += 1;
            j += 1;
        } else if j == m || (i < n && common[i + 1][j] >= common[i][j + 1]) {
            list.push(Entry::Delete(old[i]));
            i += 1;
        } else {
            list.push(Entry::Insert(new[j]));
            j += 1;
        }
    }
    list
}

/// The index ranges of the hunks: each group of changes with its context.
fn hunks(list: &[Entry], context: usize) -> Vec<(usize, usize)> {
    let changes: Vec<usize> = (0..list.len()).filter(|&k| list[k].is_change()).collect();
    let mut groups: Vec<(usize, usize)> = vec![];
    for &k in &changes {
        match groups.last_mut() {
            Some((_, last)) if k - *last - 1 <= 2 * context => *last = k,
            _ => groups.push((k, k)),
        }
    }
    groups
        .into_iter()
        .map(|(first, last)| {
            let start = first.saturating_sub(context);
            let end = (last + context + 1).min(list.len());
            (start, end)
        })
        .collect()
}

fn old_lines(entries: &[Entry]) -> usize {
    entries.iter().filter(|e| e.uses_old()).count()
}

fn new_lines(entries: &[Entry]) -> usize {
    entries.iter().filter(|e| e.uses_new()).count()
}

/// The header part `-A,B` or `+D,E`: `before` lines are used ahead of the hunk, `count` inside it.
fn range(sign: char, before: usize, count: usize) -> String {
    let first = if count == 0 { before } else { before + 1 };
    format!("{sign}{first},{count}")
}

fn render(list: &[Entry], context: usize) -> String {
    if list.iter().all(|e| !e.is_change()) {
        return "no differences\n".to_string();
    }
    let mut out = String::new();
    for (start, end) in hunks(list, context) {
        let (ahead, hunk) = (&list[..start], &list[start..end]);
        let old = range('-', old_lines(ahead), old_lines(hunk));
        let new = range('+', new_lines(ahead), new_lines(hunk));
        out += &format!("@@ {old} {new} @@\n");
        for entry in hunk {
            out += &entry.text();
            out.push('\n');
        }
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [old_name, new_name, context] = &args[..] else {
        panic!("usage: main OLD NEW CONTEXT");
    };
    let Some(context) = context_size(context) else {
        println!("error: bad context");
        process::exit(20);
    };
    let old_text = fs::read_to_string(old_name).expect("old file is readable");
    let new_text = fs::read_to_string(new_name).expect("new file is readable");
    let old: Vec<&str> = old_text.lines().collect();
    let new: Vec<&str> = new_text.lines().collect();
    print!("{}", render(&walk(&old, &new), context));
}
