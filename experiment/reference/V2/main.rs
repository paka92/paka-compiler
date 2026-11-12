use std::collections::BTreeMap;
use std::{env, fs, process};

type Cents = i64;

struct Event<'a> {
    enters: bool,
    plate: &'a str,
    minute: i64,
}

fn is_plate(s: &str) -> bool {
    (1..=8).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

fn all_digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit())
}

fn parse_day(s: &str) -> Option<i64> {
    let valid = (1..=3).contains(&s.len()) && all_digits(s);
    valid
        .then(|| s.parse().unwrap())
        .filter(|d| (1..=366).contains(d))
}

fn parse_time(s: &str) -> Option<i64> {
    let (hour, minute) = s.split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 || !all_digits(hour) || !all_digits(minute) {
        return None;
    }
    let (hour, minute): (i64, i64) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour <= 23 && minute <= 59).then_some(hour * 60 + minute)
}

fn parse_event(line: &str) -> Option<Event<'_>> {
    let words: Vec<&str> = line.split(' ').collect();
    let [kind, plate, day, time] = words[..] else {
        return None;
    };
    Some(Event {
        enters: match kind {
            "in" => true,
            "out" => false,
            _ => return None,
        },
        plate: Some(plate).filter(|p| is_plate(p))?,
        minute: (parse_day(day)? - 1) * 1440 + parse_time(time)?,
    })
}

fn fee(minutes: i64) -> Cents {
    if minutes <= 15 {
        return 0;
    }
    let (days, rest) = (minutes / 1440, minutes % 1440);
    let hours = (rest + 59) / 60;
    let charge = match hours {
        0 => 0,
        1 => 200,
        2 | 3 => 200 + 150 * (hours - 1),
        _ => 500 + 100 * (hours - 3),
    };
    days * 1200 + charge.min(1200)
}

fn money(cents: Cents) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

fn parse(text: &str) -> Result<Vec<(usize, Event<'_>)>, String> {
    let mut events = vec![];
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let event = parse_event(line).ok_or(format!("line {}: bad event", i + 1))?;
        events.push((i + 1, event));
    }
    for pair in events.windows(2) {
        if pair[1].1.minute < pair[0].1.minute {
            return Err(format!("line {}: time goes back", pair[1].0));
        }
    }
    Ok(events)
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut inside: BTreeMap<&str, i64> = BTreeMap::new();
    let mut total = 0;
    let mut out = String::new();
    for (n, event) in parse(&text)? {
        let Event {
            enters,
            plate,
            minute,
        } = event;
        match (enters, inside.get(plate).copied()) {
            (true, None) => {
                inside.insert(plate, minute);
            }
            (true, Some(_)) => out += &format!("line {n}: {plate} is already in\n"),
            (false, None) => out += &format!("line {n}: {plate} is not in\n"),
            (false, Some(since)) => {
                inside.remove(plate);
                let stay = fee(minute - since);
                total += stay;
                out += &format!("{plate}: {} min, {}\n", minute - since, money(stay));
            }
        }
    }
    for (plate, since) in inside {
        let (day, time) = (since / 1440 + 1, since % 1440);
        out += &format!(
            "{plate}: still in since day {day} {:02}:{:02}\n",
            time / 60,
            time % 60
        );
    }
    out += &format!("total: {}\n", money(total));
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
