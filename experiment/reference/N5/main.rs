use std::cmp::Ordering;
use std::{env, fs, process};

/// An exact score: `num / den`, with `den` > 0.
#[derive(Clone, Copy)]
struct Ratio {
    num: i64,
    den: i64,
}

impl Ratio {
    fn cmp(self, other: Ratio) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }

    fn at_least(self, whole: i64) -> bool {
        self.num >= whole * self.den
    }

    /// One decimal, halves up; the value is not negative.
    fn tenths(self) -> String {
        let t = (20 * self.num + self.den) / (2 * self.den);
        format!("{}.{}", t / 10, t % 10)
    }
}

struct Student {
    name: String,
    score: Option<Ratio>,
}

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn digits(s: &str) -> Option<i64> {
    if !(1..=3).contains(&s.len()) || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// A score's value, or None when excused.
fn score(s: &str) -> Option<Option<i64>> {
    if s == "x" {
        return Some(None);
    }
    let (body, late) = match s.strip_suffix('L') {
        Some(body) => (body, 10),
        None => (s, 0),
    };
    let v = digits(body).filter(|&v| v <= 100)?;
    Some(Some((v - late).max(0)))
}

fn parse_parts(words: &[&str]) -> Result<Vec<i64>, String> {
    if words[0] != "parts" {
        return Err("expected parts".into());
    }
    if words.len() == 1 {
        return Err("no parts".into());
    }
    let mut names: Vec<&str> = vec![];
    let mut weights = vec![];
    for word in &words[1..] {
        let parts: Vec<&str> = word.split(':').collect();
        if parts.len() != 2 || !is_name(parts[0]) {
            return Err(format!("bad part {word}"));
        }
        let w = digits(parts[1])
            .filter(|w| (1..=100).contains(w))
            .ok_or(format!("bad weight {word}"))?;
        if names.contains(&parts[0]) {
            return Err(format!("duplicate part {}", parts[0]));
        }
        names.push(parts[0]);
        weights.push(w);
    }
    let sum: i64 = weights.iter().sum();
    if sum != 100 {
        return Err(format!("weights sum to {sum}"));
    }
    Ok(weights)
}

fn parse_student(words: &[&str], weights: &[i64], seen: &[Student]) -> Result<Student, String> {
    let name = words[0];
    if !is_name(name) {
        return Err(format!("bad name {name}"));
    }
    if seen.iter().any(|s| s.name == name) {
        return Err(format!("duplicate student {name}"));
    }
    let k = words.len() - 1;
    if k != weights.len() {
        return Err(format!("expected {} scores, got {k}", weights.len()));
    }
    let (mut t, mut s) = (0, 0);
    for (word, w) in words[1..].iter().zip(weights) {
        if let Some(v) = score(word).ok_or(format!("bad score {word}"))? {
            t += w * v;
            s += w;
        }
    }
    let score = (s > 0).then_some(Ratio { num: t, den: s });
    Ok(Student {
        name: name.to_string(),
        score,
    })
}

fn grade(score: Option<Ratio>) -> char {
    let Some(r) = score else { return 'I' };
    [(90, 'A'), (80, 'B'), (70, 'C'), (60, 'D')]
        .iter()
        .find(|(least, _)| r.at_least(*least))
        .map_or('F', |&(_, g)| g)
}

fn report(mut students: Vec<Student>) -> String {
    students.sort_by(|a, b| match (a.score, b.score) {
        (Some(x), Some(y)) => y.cmp(x).then(a.name.cmp(&b.name)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });
    let scores: Vec<Ratio> = students.iter().filter_map(|s| s.score).collect();
    let mut out = String::new();
    for st in &students {
        out += &match st.score {
            Some(r) => {
                let rank = 1 + scores.iter().filter(|o| o.cmp(r).is_gt()).count();
                format!("{rank} {} {} {}\n", st.name, r.tenths(), grade(st.score))
            }
            None => format!("- {} - I\n", st.name),
        };
    }
    // Sorted highest first; the median is the same either way.
    let n = scores.len();
    let median = match n {
        0 => "-".to_string(),
        _ if n % 2 == 1 => scores[n / 2].tenths(),
        _ => {
            let (a, b) = (scores[n / 2 - 1], scores[n / 2]);
            let num = a.num * b.den + b.num * a.den;
            Ratio {
                num,
                den: 2 * a.den * b.den,
            }
            .tenths()
        }
    };
    out += &format!("median {median}\n");
    let counts: Vec<String> = ['A', 'B', 'C', 'D', 'F', 'I']
        .iter()
        .map(|&g| {
            let k = students.iter().filter(|s| grade(s.score) == g).count();
            format!("{g}={k}")
        })
        .collect();
    out + &format!("grades {}\n", counts.join(" "))
}

fn run(text: &str) -> Result<String, String> {
    let mut weights: Option<Vec<i64>> = None;
    let mut students: Vec<Student> = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        if words.is_empty() {
            continue;
        }
        let at = |e: String| format!("line {}: {e}", i + 1);
        match &weights {
            None => weights = Some(parse_parts(&words).map_err(at)?),
            Some(w) => students.push(parse_student(&words, w, &students).map_err(at)?),
        }
    }
    if weights.is_none() {
        return Err("no parts".into());
    }
    if students.is_empty() {
        return Err("no students".into());
    }
    Ok(report(students))
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
