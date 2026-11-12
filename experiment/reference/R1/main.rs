use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

type Rules = BTreeMap<String, BTreeSet<String>>;

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Reads the rules; every name written gets an entry, with no dependencies unless it is a target.
fn parse(text: &str) -> Result<Rules, String> {
    let mut rules = Rules::new();
    let mut targets = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let mut words = line.split(' ').filter(|w| !w.is_empty());
        let Some(first) = words.next() else {
            continue;
        };
        let target = first.strip_suffix(':').filter(|t| is_name(t));
        let deps: Vec<&str> = words.collect();
        let (Some(target), true) = (target, deps.iter().all(|d| is_name(d))) else {
            return Err(format!("line {n}: bad line"));
        };
        if !targets.insert(target) {
            return Err(format!("line {n}: duplicate target {target}"));
        }
        for dep in &deps {
            rules.entry(dep.to_string()).or_default();
        }
        rules
            .entry(target.to_string())
            .or_default()
            .extend(deps.iter().map(|d| d.to_string()));
    }
    Ok(rules)
}

/// The names in build order, or the names left when none is ready.
fn build_order(rules: &Rules) -> Result<Vec<&String>, BTreeSet<&String>> {
    let mut order: Vec<&String> = vec![];
    let mut built: BTreeSet<&String> = BTreeSet::new();
    while let Some((name, _)) = rules
        .iter()
        .find(|(name, deps)| !built.contains(name) && deps.iter().all(|d| built.contains(d)))
    {
        order.push(name);
        built.insert(name);
    }
    if order.len() == rules.len() {
        Ok(order)
    } else {
        Err(rules.keys().filter(|n| !built.contains(n)).collect())
    }
}

/// The cycle found by always stepping to the first dependency still left, smallest name first.
fn find_cycle<'a>(rules: &'a Rules, left: &BTreeSet<&'a String>) -> Vec<&'a String> {
    let mut path: Vec<&String> = vec![];
    let mut current = *left.first().expect("some name is left");
    while !path.contains(&current) {
        path.push(current);
        current = rules[current]
            .iter()
            .find(|d| left.contains(d))
            .expect("a name left has a dependency left");
    }
    let mut cycle = path.split_off(path.iter().position(|&n| n == current).unwrap());
    let smallest = (0..cycle.len()).min_by_key(|&i| cycle[i]).unwrap();
    cycle.rotate_left(smallest);
    cycle
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let rules = parse(&text).map_err(|e| format!("error: {e}"))?;
    match build_order(&rules) {
        Ok(order) => Ok(order.iter().map(|n| format!("{n}\n")).collect()),
        Err(left) => {
            let mut cycle = find_cycle(&rules, &left);
            cycle.push(cycle[0]);
            let names: Vec<&str> = cycle.iter().map(|n| n.as_str()).collect();
            Err(format!("cycle: {}", names.join(" -> ")))
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    match run(file) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("{message}");
            process::exit(20);
        }
    }
}
