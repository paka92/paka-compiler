use std::collections::BTreeMap;
use std::{env, fs, process};

const COINS: [i64; 6] = [5, 10, 20, 50, 100, 200];

enum Command<'a> {
    Load(&'a str, i64, i64),
    Coin(i64),
    Select(&'a str),
    Cancel,
}

fn digits(s: &str, min: usize, max: usize) -> Option<i64> {
    let ok = (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| s.parse().unwrap())
}

fn is_slot(s: &str) -> bool {
    matches!(s.as_bytes(), [b'A'..=b'Z', b'1'..=b'9'])
}

fn price(s: &str) -> Option<i64> {
    let (units, cents) = s.split_once('.')?;
    let value = digits(units, 1, 2)? * 100 + digits(cents, 2, 2)?;
    (value >= 5 && value % 5 == 0).then_some(value)
}

fn parse_box(line: &str) -> Option<[i64; 6]> {
    let words: Vec<&str> = line.split(' ').collect();
    if words.len() != 7 || words[0] != "box" {
        return None;
    }
    let mut counts = [0; 6];
    for (count, word) in counts.iter_mut().zip(&words[1..]) {
        *count = digits(word, 1, 3)?;
    }
    Some(counts)
}

fn parse_command(line: &str) -> Option<Command<'_>> {
    let words: Vec<&str> = line.split(' ').collect();
    match words[..] {
        ["load", slot, p, n] if is_slot(slot) => {
            let count = digits(n, 1, 2).filter(|&c| c >= 1)?;
            Some(Command::Load(slot, price(p)?, count))
        }
        ["coin", v] => Some(Command::Coin(digits(v, 1, 3)?)),
        ["select", slot] if is_slot(slot) => Some(Command::Select(slot)),
        ["cancel"] => Some(Command::Cancel),
        _ => None,
    }
}

fn money(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

fn money_list(coins: &[i64]) -> String {
    let parts: Vec<String> = coins.iter().map(|&c| money(c)).collect();
    parts.join(" ")
}

struct Slot {
    price: i64,
    items: i64,
}

#[derive(Default)]
struct Machine<'a> {
    coin_box: [i64; 6],
    credit: i64,
    session: Vec<i64>,
    slots: BTreeMap<&'a str, Slot>,
}

impl<'a> Machine<'a> {
    fn take_out(&mut self, coins: &[i64]) {
        for coin in coins {
            let i = COINS.iter().position(|c| c == coin).unwrap();
            self.coin_box[i] -= 1;
        }
    }

    /// The coins paid for `change`, largest first, if the greedy rule can pay it.
    fn change_coins(&self, mut change: i64) -> Option<Vec<i64>> {
        let mut paid = vec![];
        for i in (0..6).rev() {
            let n = (change / COINS[i]).min(self.coin_box[i]);
            paid.extend(std::iter::repeat_n(COINS[i], n as usize));
            change -= n * COINS[i];
        }
        (change == 0).then_some(paid)
    }

    fn run(&mut self, command: Command<'a>) -> Option<String> {
        match command {
            Command::Load(slot, price, count) => {
                if self.credit != 0 {
                    return Some("busy".into());
                }
                let items = self.slots.get(slot).map_or(0, |s| s.items) + count;
                if items > 20 {
                    return Some(format!("slot full {slot}"));
                }
                self.slots.insert(slot, Slot { price, items });
                None
            }
            Command::Coin(value) => {
                let fits = self.credit + value <= 500;
                let Some(i) = COINS.iter().position(|&c| c == value).filter(|_| fits) else {
                    return Some(format!("rejected {}", money(value)));
                };
                self.coin_box[i] += 1;
                self.credit += value;
                self.session.push(value);
                None
            }
            Command::Select(slot) => {
                let Some(s) = self.slots.get(slot) else {
                    return Some(format!("no slot {slot}"));
                };
                if s.items == 0 {
                    return Some(format!("sold out {slot}"));
                }
                if self.credit < s.price {
                    return Some(format!("insert {} more", money(s.price - self.credit)));
                }
                let change = self.credit - s.price;
                let Some(paid) = self.change_coins(change) else {
                    return Some("no change".into());
                };
                self.slots.get_mut(slot).unwrap().items -= 1;
                self.take_out(&paid);
                self.credit = 0;
                self.session.clear();
                Some(match change {
                    0 => format!("sold {slot}"),
                    _ => format!(
                        "sold {slot}, change {}: {}",
                        money(change),
                        money_list(&paid)
                    ),
                })
            }
            Command::Cancel => {
                if self.credit == 0 {
                    return Some("nothing to return".into());
                }
                let session = std::mem::take(&mut self.session);
                self.take_out(&session);
                let returned = money_list(&session);
                Some(format!(
                    "returned {}: {returned}",
                    money(std::mem::take(&mut self.credit))
                ))
            }
        }
    }
}

fn run(text: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.lines().collect();
    let coin_box = lines.first().and_then(|l| parse_box(l)).ok_or("bad box")?;
    let mut commands = vec![];
    for (i, line) in lines.iter().enumerate().skip(1) {
        commands.push(parse_command(line).ok_or(format!("line {}: bad command", i + 1))?);
    }
    let mut machine = Machine {
        coin_box,
        ..Default::default()
    };
    let mut out = String::new();
    for command in commands {
        if let Some(line) = machine.run(command) {
            out += &line;
            out.push('\n');
        }
    }
    out += &format!("credit {}\n", money(machine.credit));
    let counts: Vec<String> = machine.coin_box.iter().map(|n| n.to_string()).collect();
    out += &format!("box {}\n", counts.join(" "));
    for (name, slot) in &machine.slots {
        out += &format!("{name}: {} at {}\n", slot.items, money(slot.price));
    }
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
