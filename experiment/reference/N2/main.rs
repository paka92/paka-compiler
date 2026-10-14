use std::collections::BTreeMap;
use std::{env, fs, process};

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_value(word: &str) -> Option<i64> {
    let digits = word.strip_prefix('-').unwrap_or(word);
    if !(1..=5).contains(&digits.len()) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    word.parse().ok()
}

/// Two decimals; a value that rounds to zero has no sign.
fn two_decimals(x: f64) -> String {
    let text = format!("{x:.2}");
    if text == "-0.00" { "0.00".to_string() } else { text }
}

fn or_dash(stat: Option<f64>) -> String {
    stat.map_or("-".to_string(), two_decimals)
}

fn statistics(values: &mut [i64]) -> String {
    let n = values.len();
    values.sort();
    let mean = (n > 0).then(|| values.iter().sum::<i64>() as f64 / n as f64);
    let median = (n > 0).then(|| (values[(n - 1) / 2] + values[n / 2]) as f64 / 2.0);
    let sd = mean.filter(|_| n > 1).map(|mean| {
        let squares: f64 = values.iter().map(|&v| (v as f64 - mean).powi(2)).sum();
        (squares / (n - 1) as f64).sqrt()
    });
    format!(
        "n={n} mean={} median={} sd={}",
        or_dash(mean),
        or_dash(median),
        or_dash(sd)
    )
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut groups: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let mut words = line.split(' ').filter(|w| !w.is_empty());
        let Some(first) = words.next() else {
            continue;
        };
        let Some(name) = first.strip_suffix(':').filter(|name| is_name(name)) else {
            return Err(format!("line {n}: bad line"));
        };
        let mut values = vec![];
        for word in words {
            values.push(parse_value(word).ok_or(format!("line {n}: bad value {word}"))?);
        }
        groups.entry(name).or_default().extend(values);
    }
    if groups.is_empty() {
        return Err("no groups".to_string());
    }
    let out = groups
        .iter_mut()
        .map(|(name, values)| format!("{name}: {}\n", statistics(values)))
        .collect();
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
