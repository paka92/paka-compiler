use std::collections::HashSet;
use std::{env, fs, process};

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Days since 1900-01-01, which is day 0 and a Monday.
type Day = i64;

#[derive(Clone, Copy)]
struct Date {
    year: i64,
    month: i64,
    day: i64,
}

enum Command {
    Days(Date, Date),
    Weekday(Date),
    Add(Date, i64),
    Holiday(Date),
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn month_length(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn year_length(year: i64) -> i64 {
    if is_leap(year) { 366 } else { 365 }
}

impl Date {
    fn is_valid(&self) -> bool {
        (1900..=2100).contains(&self.year)
            && (1..=12).contains(&self.month)
            && (1..=month_length(self.year, self.month)).contains(&self.day)
    }

    fn day_number(&self) -> Day {
        let years: i64 = (1900..self.year).map(year_length).sum();
        let months: i64 = (1..self.month).map(|m| month_length(self.year, m)).sum();
        years + months + self.day - 1
    }

    fn from_day_number(mut n: Day) -> Date {
        let mut year = 1900;
        while n >= year_length(year) {
            n -= year_length(year);
            year += 1;
        }
        let mut month = 1;
        while n >= month_length(year, month) {
            n -= month_length(year, month);
            month += 1;
        }
        Date {
            year,
            month,
            day: n + 1,
        }
    }

    fn text(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The date if `s` is written `YYYY-MM-DD`; it may still be invalid.
fn parse_date(s: &str) -> Option<Date> {
    let b = s.as_bytes();
    let shaped = b.len() == 10
        && b.iter()
            .enumerate()
            .all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
    shaped.then(|| Date {
        year: s[0..4].parse().unwrap(),
        month: s[5..7].parse().unwrap(),
        day: s[8..10].parse().unwrap(),
    })
}

fn parse_count(s: &str) -> Option<i64> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if !(1..=3).contains(&digits.len()) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn parse_command(line: &str) -> Option<Command> {
    let words: Vec<&str> = line.split(' ').collect();
    match words[..] {
        ["days", a, b] => Some(Command::Days(parse_date(a)?, parse_date(b)?)),
        ["weekday", a] => Some(Command::Weekday(parse_date(a)?)),
        ["add", a, count] => Some(Command::Add(parse_date(a)?, parse_count(count)?)),
        ["holiday", a] => Some(Command::Holiday(parse_date(a)?)),
        _ => None,
    }
}

fn dates(command: &Command) -> Vec<Date> {
    match *command {
        Command::Days(a, b) => vec![a, b],
        Command::Weekday(a) | Command::Add(a, _) | Command::Holiday(a) => vec![a],
    }
}

struct Calendar {
    holidays: HashSet<Day>,
}

impl Calendar {
    fn is_working(&self, day: Day) -> bool {
        day.rem_euclid(7) < 5 && !self.holidays.contains(&day)
    }

    /// Moves `count` working days from `start`; `None` when that leaves 1900 to 2100.
    fn add(&self, start: Day, count: i64) -> Option<Day> {
        let last = Date {
            year: 2100,
            month: 12,
            day: 31,
        }
        .day_number();
        let direction = count.signum();
        let mut day = start;
        for _ in 0..count.abs() {
            day += direction;
            while (0..=last).contains(&day) && !self.is_working(day) {
                day += direction;
            }
        }
        (0..=last).contains(&day).then_some(day)
    }
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let mut commands = vec![];
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
        let n = i + 1;
        let command = parse_command(line).ok_or(format!("line {n}: bad command"))?;
        if !dates(&command).iter().all(Date::is_valid) {
            return Err(format!("line {n}: bad date"));
        }
        commands.push((n, command));
    }

    let holidays = commands
        .iter()
        .filter_map(|(_, c)| match c {
            Command::Holiday(date) => Some(date.day_number()),
            _ => None,
        })
        .collect();
    let calendar = Calendar { holidays };

    let mut out = String::new();
    for (n, command) in &commands {
        match command {
            Command::Days(a, b) => out += &format!("{}\n", b.day_number() - a.day_number()),
            Command::Weekday(a) => {
                out += &format!("{}\n", WEEKDAYS[a.day_number().rem_euclid(7) as usize])
            }
            Command::Add(a, count) => match calendar.add(a.day_number(), *count) {
                Some(day) => out += &format!("{}\n", Date::from_day_number(day).text()),
                None => out += &format!("line {n}: out of range\n"),
            },
            Command::Holiday(_) => {}
        }
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
