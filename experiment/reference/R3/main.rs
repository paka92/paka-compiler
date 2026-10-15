use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse(text: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut links = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        match words[..] {
            [] => {}
            [a, b] if is_name(a) && is_name(b) => links.push((a, b)),
            _ => return Err(format!("line {}: bad link", i + 1)),
        }
    }
    Ok(links)
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// The group of every name, as the index of one name in it, without the link `skip`.
fn groups_of(count: usize, links: &[(usize, usize)], skip: Option<usize>) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..count).collect();
    for (i, &(a, b)) in links.iter().enumerate() {
        if Some(i) != skip {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            parent[ra] = rb;
        }
    }
    (0..count).map(|x| find(&mut parent, x)).collect()
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let written = parse(&text)?;
    let names: Vec<&str> = written
        .iter()
        .flat_map(|&(a, b)| [a, b])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let index = |name: &str| names.binary_search(&name).unwrap();
    let links: Vec<(usize, usize)> = written.iter().map(|&(a, b)| (index(a), index(b))).collect();

    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (name, root) in groups_of(names.len(), &links, None).into_iter().enumerate() {
        by_root.entry(root).or_default().push(name);
    }
    let mut groups: Vec<Vec<usize>> = by_root.into_values().collect();
    groups.sort();

    let mut bridges: Vec<(usize, usize)> = links
        .iter()
        .enumerate()
        .filter(|&(i, &(a, b))| {
            let without = groups_of(names.len(), &links, Some(i));
            without[a] != without[b]
        })
        .map(|(_, &(a, b))| (a.min(b), a.max(b)))
        .collect();
    bridges.sort();

    let mut out = format!("groups: {}\n", groups.len());
    for group in &groups {
        let members: Vec<&str> = group.iter().map(|&n| names[n]).collect();
        out += &format!("{}\n", members.join(" "));
    }
    out += &format!("bridges: {}\n", bridges.len());
    for (a, b) in bridges {
        out += &format!("{} {}\n", names[a], names[b]);
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    match run(file) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
