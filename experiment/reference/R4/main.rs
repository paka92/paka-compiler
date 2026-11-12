use std::collections::BTreeMap;
use std::{env, fs, process};

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

struct Service<'a> {
    deps: Vec<&'a str>,
    threshold: usize,
}

enum Line<'a> {
    Service(&'a str, Vec<&'a str>, Option<usize>),
    Down(&'a str),
}

fn parse_line<'a>(words: &[&'a str]) -> Option<Line<'a>> {
    if let ["down", name] = words {
        return is_name(name).then_some(Line::Down(name));
    }
    let name = words[0].strip_suffix(':').filter(|n| is_name(n))?;
    let mut deps = &words[1..];
    let mut threshold = None;
    if let [rest @ .., "/", k] = deps {
        if !(1..=3).contains(&k.len()) || !k.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        threshold = Some(k.parse().unwrap());
        deps = rest;
    }
    deps.iter()
        .all(|d| is_name(d))
        .then(|| Line::Service(name, deps.to_vec(), threshold))
}

fn run(text: &str) -> Result<String, String> {
    let mut services: BTreeMap<&str, Service> = BTreeMap::new();
    // Every name to check, with its line: dependencies and down names in file order.
    let mut used: Vec<(usize, &str)> = vec![];
    let mut down: Vec<&str> = vec![];
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        if words.is_empty() {
            continue;
        }
        match parse_line(&words).ok_or(format!("line {n}: bad line"))? {
            Line::Down(name) => {
                used.push((n, name));
                down.push(name);
            }
            Line::Service(name, deps, k) => {
                if services.contains_key(name) {
                    return Err(format!("line {n}: duplicate service {name}"));
                }
                if let Some(j) = (1..deps.len()).find(|&j| deps[..j].contains(&deps[j])) {
                    return Err(format!("line {n}: duplicate dependency {}", deps[j]));
                }
                let threshold = k.unwrap_or(deps.len());
                if k.is_some() && (threshold == 0 || threshold > deps.len()) {
                    return Err(format!("line {n}: bad threshold"));
                }
                used.extend(deps.iter().map(|&d| (n, d)));
                services.insert(name, Service { deps, threshold });
            }
        }
    }
    if let Some((n, name)) = used.iter().find(|(_, name)| !services.contains_key(name)) {
        return Err(format!("line {n}: unknown service {name}"));
    }
    if down.is_empty() {
        return Err("nothing down".into());
    }
    cascade(&services, &down)
}

fn cascade(services: &BTreeMap<&str, Service>, first: &[&str]) -> Result<String, String> {
    let names: Vec<&str> = services.keys().copied().collect();
    let index = |name: &str| names.binary_search(&name).unwrap();
    let deps: Vec<Vec<usize>> = services
        .values()
        .map(|s| s.deps.iter().map(|d| index(d)).collect())
        .collect();
    let mut is_down = vec![false; names.len()];
    let mut round: Vec<usize> = first.iter().map(|d| index(d)).collect();
    round.sort();
    round.dedup();
    let mut out = String::new();
    let mut r = 0;
    while !round.is_empty() {
        let listed: Vec<&str> = round.iter().map(|&i| names[i]).collect();
        out += &format!("round {r}: {}\n", listed.join(" "));
        for &i in &round {
            is_down[i] = true;
        }
        r += 1;
        round = (0..names.len())
            .filter(|&i| {
                let k = services[names[i]].threshold;
                let count = deps[i].iter().filter(|&&d| is_down[d]).count();
                !is_down[i] && k > 0 && count >= k
            })
            .collect();
    }
    let up: Vec<&str> = (0..names.len())
        .filter(|&i| !is_down[i])
        .map(|i| names[i])
        .collect();
    let up = if up.is_empty() {
        "none".to_string()
    } else {
        up.join(" ")
    };
    out += &format!("up: {up}\n");
    Ok(out)
}

fn main() {
    let path = env::args().nth(1).expect("usage: main FILE");
    let text = fs::read_to_string(path).expect("cannot read the input file");
    match run(&text) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
