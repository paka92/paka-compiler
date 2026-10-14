use std::{env, fs, process};

struct Member {
    name: String,
    weight: i64,
    total: i64,
}

struct Cost {
    name: String,
    amount: i64,
    /// Positions of the sharers in member order.
    sharers: Vec<usize>,
}

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn digits(s: &str, lengths: std::ops::RangeInclusive<usize>) -> Option<i64> {
    if !lengths.contains(&s.len()) || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn weight(s: &str) -> Option<i64> {
    digits(s, 1..=2).filter(|&w| w >= 1)
}

fn amount(s: &str) -> Option<i64> {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, s),
    };
    let (units, cents) = rest.split_once('.')?;
    Some(sign * (digits(units, 1..=8)? * 100 + digits(cents, 2..=2)?))
}

fn parse_members(words: &[&str]) -> Result<Vec<Member>, String> {
    if words[0] != "members" {
        return Err("expected members".into());
    }
    if words.len() == 1 {
        return Err("no members".into());
    }
    let mut members: Vec<Member> = vec![];
    for word in &words[1..] {
        let parts: Vec<&str> = word.split(':').collect();
        if parts.len() != 2 || !is_name(parts[0]) {
            return Err(format!("bad member {word}"));
        }
        let weight = weight(parts[1]).ok_or(format!("bad weight {word}"))?;
        if members.iter().any(|m| m.name == parts[0]) {
            return Err(format!("duplicate member {}", parts[0]));
        }
        members.push(Member {
            name: parts[0].to_string(),
            weight,
            total: 0,
        });
    }
    Ok(members)
}

fn parse_cost(words: &[&str], members: &[Member]) -> Result<Cost, String> {
    if words[0] != "cost" {
        return Err(format!("unknown command {}", words[0]));
    }
    if words.len() < 3 {
        return Err("incomplete cost".into());
    }
    if !is_name(words[1]) {
        return Err("bad cost name".into());
    }
    let amount = amount(words[2]).ok_or("bad amount")?;
    let mut listed = vec![false; members.len()];
    for word in &words[3..] {
        let at = members
            .iter()
            .position(|m| m.name == *word)
            .ok_or(format!("unknown member {word}"))?;
        if listed[at] {
            return Err(format!("duplicate member {word}"));
        }
        listed[at] = true;
    }
    let all = words.len() == 3;
    let sharers = (0..members.len()).filter(|&i| all || listed[i]).collect();
    Ok(Cost {
        name: words[1].to_string(),
        amount,
        sharers,
    })
}

/// The shares of the sharers, in member order.
fn split(cost: &Cost, members: &[Member]) -> Vec<i64> {
    let size = cost.amount.abs();
    let total_weight: i64 = cost.sharers.iter().map(|&i| members[i].weight).sum();
    let mut shares: Vec<i64> = vec![];
    let mut rests: Vec<(i64, usize)> = vec![];
    for (k, &i) in cost.sharers.iter().enumerate() {
        let product = size * members[i].weight;
        shares.push(product / total_weight);
        rests.push((product % total_weight, k));
    }
    let left = size - shares.iter().sum::<i64>();
    rests.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, k) in rests.iter().take(left as usize) {
        shares[k] += 1;
    }
    shares.iter().map(|s| s * cost.amount.signum()).collect()
}

fn money(c: i64) -> String {
    let units = (c.abs() / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in units.chars().enumerate() {
        if i > 0 && (units.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if c < 0 { "-" } else { "" };
    format!("{sign}{grouped}.{:02}", c.abs() % 100)
}

/// total / grand as a percent with one decimal, halves to even.
fn percent(total: i64, grand: i64) -> String {
    if grand == 0 {
        return "-".into();
    }
    let (n, d) = (total * 1000 * grand.signum(), grand.abs());
    let (q, r) = (n.abs() / d, n.abs() % d);
    let tenths = if 2 * r > d || 2 * r == d && q % 2 == 1 {
        q + 1
    } else {
        q
    };
    let sign = if n < 0 && tenths > 0 { "-" } else { "" };
    format!("{sign}{}.{}%", tenths / 10, tenths % 10)
}

fn run(text: &str) -> Result<String, String> {
    let mut members: Option<Vec<Member>> = None;
    let mut costs: Vec<Cost> = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        if words.is_empty() {
            continue;
        }
        let at = |e: String| format!("line {}: {e}", i + 1);
        match &members {
            None => members = Some(parse_members(&words).map_err(at)?),
            Some(list) => costs.push(parse_cost(&words, list).map_err(at)?),
        }
    }
    let mut members = members.ok_or("no members")?;
    if costs.is_empty() {
        return Err("no costs".into());
    }
    let mut out = String::new();
    for cost in &costs {
        let shares = split(cost, &members);
        let pairs: Vec<String> = cost
            .sharers
            .iter()
            .zip(&shares)
            .map(|(&i, &s)| format!("{} {}", members[i].name, money(s)))
            .collect();
        out += &format!("{}: {}\n", cost.name, pairs.join(", "));
        for (&i, &s) in cost.sharers.iter().zip(&shares) {
            members[i].total += s;
        }
    }
    let grand: i64 = costs.iter().map(|c| c.amount).sum();
    members.sort_by(|a, b| b.total.cmp(&a.total).then(a.name.cmp(&b.name)));
    for m in &members {
        let share = percent(m.total, grand);
        out += &format!("{} {} {share}\n", m.name, money(m.total));
    }
    out += &format!("total {}\n", money(grand));
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file] = &args[..] else {
        panic!("usage: main FILE");
    };
    let text = fs::read_to_string(file).expect("input file is readable");
    match run(&text) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
