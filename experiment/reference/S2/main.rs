use std::collections::{BTreeMap, HashMap, HashSet};
use std::{env, fs, process};

type Cents = i64;

enum Command<'a> {
    Open(&'a str),
    Deposit(&'a str, Cents),
    Hold(&'a str, &'a str, Cents),
    Capture(&'a str, Cents),
    Release(&'a str),
    Transfer(&'a str, &'a str, Cents),
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase())
}

fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn parse_amount(s: &str) -> Option<Cents> {
    let (units, cents) = s.split_once('.')?;
    let all_digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if !(1..=9).contains(&units.len())
        || cents.len() != 2
        || !all_digits(units)
        || !all_digits(cents)
    {
        return None;
    }
    let total = units.parse::<Cents>().ok()? * 100 + cents.parse::<Cents>().ok()?;
    (total >= 1).then_some(total)
}

fn parse_command(line: &str) -> Option<Command<'_>> {
    let words: Vec<&str> = line.split(' ').collect();
    match words[..] {
        ["open", n] if is_name(n) => Some(Command::Open(n)),
        ["deposit", n, a] if is_name(n) => Some(Command::Deposit(n, parse_amount(a)?)),
        ["hold", n, id, a] if is_name(n) && is_id(id) => {
            Some(Command::Hold(n, id, parse_amount(a)?))
        }
        ["capture", id, a] if is_id(id) => Some(Command::Capture(id, parse_amount(a)?)),
        ["release", id] if is_id(id) => Some(Command::Release(id)),
        ["transfer", f, t, a] if is_name(f) && is_name(t) => {
            Some(Command::Transfer(f, t, parse_amount(a)?))
        }
        _ => None,
    }
}

fn money(cents: Cents) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

struct Hold<'a> {
    account: &'a str,
    amount: Cents,
}

#[derive(Default)]
struct Ledger<'a> {
    balances: BTreeMap<&'a str, Cents>,
    active: HashMap<&'a str, Hold<'a>>,
    used_ids: HashSet<&'a str>,
}

impl<'a> Ledger<'a> {
    fn held(&self, account: &str) -> Cents {
        let holds = self.active.values().filter(|h| h.account == account);
        holds.map(|h| h.amount).sum()
    }

    fn available(&self, account: &str) -> Cents {
        self.balances[account] - self.held(account)
    }

    fn require(&self, account: &str) -> Result<(), String> {
        match self.balances.contains_key(account) {
            true => Ok(()),
            false => Err(format!("no account {account}")),
        }
    }

    fn run(&mut self, command: Command<'a>) -> Result<(), String> {
        match command {
            Command::Open(name) => {
                if self.balances.contains_key(name) {
                    return Err(format!("account exists {name}"));
                }
                self.balances.insert(name, 0);
            }
            Command::Deposit(name, amount) => {
                self.require(name)?;
                *self.balances.get_mut(name).unwrap() += amount;
            }
            Command::Hold(name, id, amount) => {
                self.require(name)?;
                if self.used_ids.contains(id) {
                    return Err(format!("hold exists {id}"));
                }
                if self.available(name) < amount {
                    return Err("insufficient funds".to_string());
                }
                self.used_ids.insert(id);
                let account = name;
                self.active.insert(id, Hold { account, amount });
            }
            Command::Capture(id, amount) => {
                let Some(hold) = self.active.get(id) else {
                    return Err(format!("no hold {id}"));
                };
                if amount > hold.amount {
                    return Err("capture exceeds hold".to_string());
                }
                *self.balances.get_mut(hold.account).unwrap() -= amount;
                self.active.remove(id);
            }
            Command::Release(id) => {
                if self.active.remove(id).is_none() {
                    return Err(format!("no hold {id}"));
                }
            }
            Command::Transfer(from, to, amount) => {
                self.require(from)?;
                self.require(to)?;
                if from == to {
                    return Err("same account".to_string());
                }
                if self.available(from) < amount {
                    return Err("insufficient funds".to_string());
                }
                *self.balances.get_mut(from).unwrap() -= amount;
                *self.balances.get_mut(to).unwrap() += amount;
            }
        }
        Ok(())
    }
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut commands = vec![];
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let command = parse_command(line).ok_or(format!("line {}: bad command", i + 1))?;
        commands.push((i + 1, command));
    }

    let mut ledger = Ledger::default();
    let mut out = String::new();
    for (n, command) in commands {
        if let Err(reason) = ledger.run(command) {
            out += &format!("line {n}: {reason}\n");
        }
    }
    if ledger.balances.is_empty() {
        out += "no accounts\n";
    }
    for (name, balance) in &ledger.balances {
        let held = ledger.held(name);
        out += &format!(
            "{name}: balance {}, held {}\n",
            money(*balance),
            money(held)
        );
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
