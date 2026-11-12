use std::{env, fs, process};

struct Item {
    name: String,
    gross: i64,
    discount: i64,
    net: i64,
    tax: i64,
    total: i64,
}

/// Divides `n` by `d` (positive) and rounds half away from zero.
fn div_round(n: i64, d: i64) -> i64 {
    let magnitude = (n.abs() * 2 + d) / (2 * d);
    magnitude * n.signum()
}

fn cents(c: i64) -> String {
    let sign = if c < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", c.abs() / 100, c.abs() % 100)
}

/// A run of digits whose length is in `lengths`, as a number.
fn digits(s: &str, lengths: std::ops::RangeInclusive<usize>) -> Option<i64> {
    if !lengths.contains(&s.len()) || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn quantity(s: &str) -> Option<i64> {
    let (sign, rest) = match s.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, s),
    };
    digits(rest, 1..=3).map(|n| n * sign)
}

/// The price in cents.
fn price(s: &str) -> Option<i64> {
    let (units, cents) = s.split_once('.')?;
    Some(digits(units, 1..=5)? * 100 + digits(cents, 2..=2)?)
}

/// The discount in whole percent.
fn discount(s: &str) -> Option<i64> {
    digits(s.strip_suffix('%')?, 1..=3).filter(|&p| p <= 100)
}

/// The tax rate in hundredths of a percent.
fn tax_rate(s: &str) -> Option<i64> {
    let (whole, hundredths) = s.strip_suffix('%')?.split_once('.')?;
    Some(digits(whole, 1..=2)? * 100 + digits(hundredths, 2..=2)?)
}

fn parse_item(words: &[&str]) -> Result<Item, String> {
    let [name, quantity_word, price_word, discount_word, tax_word] = words else {
        return Err(format!("expected 5 fields, got {}", words.len()));
    };
    if !is_name(name) {
        return Err("bad name".to_string());
    }
    let quantity = quantity(quantity_word).ok_or("bad quantity")?;
    let price = price(price_word).ok_or("bad price")?;
    let percent = discount(discount_word).ok_or("bad discount")?;
    let rate = tax_rate(tax_word).ok_or("bad tax")?;

    let gross = quantity * price;
    let discount = div_round(gross * percent, 100);
    let net = gross - discount;
    let tax = div_round(net * rate, 10000);
    Ok(Item {
        name: name.to_string(),
        gross,
        discount,
        net,
        tax,
        total: net + tax,
    })
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut items = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        if !words.is_empty() {
            items.push(parse_item(&words).map_err(|e| format!("line {}: {e}", i + 1))?);
        }
    }
    if items.is_empty() {
        return Err("no items".to_string());
    }

    let mut out = String::new();
    for item in &items {
        let Item {
            name,
            gross,
            discount,
            net,
            tax,
            total,
        } = item;
        out += &format!(
            "{name}: gross {} discount {} net {} tax {} total {}\n",
            cents(*gross),
            cents(*discount),
            cents(*net),
            cents(*tax),
            cents(*total)
        );
    }
    let sum = |amount: fn(&Item) -> i64| items.iter().map(amount).sum::<i64>();
    let total = sum(|i| i.total);
    out += &format!("net {}\n", cents(sum(|i| i.net)));
    out += &format!("tax {}\n", cents(sum(|i| i.tax)));
    if total >= 0 {
        out += &format!("total {}\n", cents(total));
    } else {
        out += &format!("refund {}\n", cents(-total));
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
