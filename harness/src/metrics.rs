//! The metrics tool: the raw logs under a runs root and a rulings file to the three metrics of
//! M3-02 section 1.9 and the verdict of section 1.7, pooled over runs. All comparisons use whole
//! numbers, so no float tie decides a verdict.

use crate::Error;
use crate::json::Value;
use crate::log;
use crate::runner::Lang;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const LANGS: [Lang; 3] = [Lang::Paka, Lang::Rust, Lang::TypeScript];
/// More dropped tasks than this make the verdict inconclusive (M3-03 section 1.2).
const MAX_DROPPED: usize = 2;

/// One run that ended: what its last ended attempt logged.
struct Run {
    id: String,
    task: String,
    lang: Lang,
    first_try: bool,
    edits: i64,
    output: i64,
}

impl Run {
    /// A run from its folder name `<task>-<language>-<rep>` and its `end` line.
    fn new(id: String, end: &Value) -> Result<Run, Error> {
        let bad = || Error::Malformed(format!("run {id}"));
        let mut parts = id.rsplitn(3, '-').skip(1);
        let (Some(lang), Some(task)) = (parts.next(), parts.next()) else {
            return Err(bad());
        };
        let lang = LANGS
            .into_iter()
            .find(|l| l.name() == lang)
            .ok_or_else(bad)?;
        let number = |key| {
            end.get(key)
                .and_then(Value::as_u64)
                .and_then(|n| i64::try_from(n).ok())
        };
        let first_try = end
            .get("first_try")
            .and_then(Value::as_bool)
            .ok_or_else(bad)?;
        let edits = number("edits").ok_or_else(bad)?;
        let output = number("output").ok_or_else(bad)?;
        Ok(Run {
            task: task.to_string(),
            id,
            lang,
            first_try,
            edits,
            output,
        })
    }
}

/// The `end` line of the last attempt of a run that ended.
fn ended(run: &Path) -> Result<Option<Value>, Error> {
    let mut found = None;
    for n in 1.. {
        let attempt = run.join(n.to_string());
        if !attempt.exists() {
            break;
        }
        if let Some(line) = log::last_line(&attempt.join("log"))?
            && line.get("event").and_then(Value::as_str) == Some("end")
        {
            found = Some(line);
        }
    }
    Ok(found)
}

/// The runs that ended, and the folders of those with no ended attempt.
fn load(root: &Path) -> Result<(Vec<Run>, Vec<String>), Error> {
    let mut ids = vec![];
    for entry in fs::read_dir(root)? {
        ids.push(entry?.file_name().to_string_lossy().into_owned());
    }
    ids.sort();
    let (mut runs, mut unfinished) = (vec![], vec![]);
    for id in ids {
        match ended(&root.join(&id))? {
            Some(end) => runs.push(Run::new(id, &end)?),
            None => unfinished.push(id),
        }
    }
    Ok((runs, unfinished))
}

/// The rulings file; see the README.
#[derive(Default)]
struct Rulings {
    void: BTreeSet<String>,
    tagged: BTreeSet<String>,
    dropped: BTreeSet<String>,
}

impl Rulings {
    fn parse(text: &str) -> Result<Rulings, Error> {
        let mut rulings = Rulings::default();
        for (n, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let mut words = line.split_whitespace();
            let (set, name) = match (words.next(), words.next(), words.next()) {
                (Some("run"), Some(id), Some("void")) => (&mut rulings.void, id),
                (Some("run"), Some(id), Some("tagged")) => (&mut rulings.tagged, id),
                (Some("task"), Some(id), Some("dropped")) => (&mut rulings.dropped, id),
                _ => return Err(Error::Malformed(format!("rulings line {}: {line}", n + 1))),
            };
            set.insert(name.to_string());
        }
        Ok(rulings)
    }

    /// A ruling that names no run or task is a typo, and would change nothing silently.
    fn check(&self, folders: &[&str]) -> Result<(), Error> {
        let task = |t: &String| folders.iter().any(|f| f.starts_with(&format!("{t}-")));
        let unknown = (self.void.iter().chain(&self.tagged))
            .filter(|id| !folders.contains(&id.as_str()))
            .chain(self.dropped.iter().filter(|t| !task(t)))
            .next();
        match unknown {
            Some(name) => Err(Error::Malformed(format!(
                "the rulings name {name}, which is no run or task"
            ))),
            None => Ok(()),
        }
    }
}

/// Sums over a set of runs.
#[derive(Default, Clone, Copy)]
struct Tally {
    runs: i64,
    first_try: i64,
    edits: i64,
    output: i64,
}

impl Tally {
    fn of<'a>(runs: impl IntoIterator<Item = &'a &'a Run>) -> Tally {
        runs.into_iter().fold(Tally::default(), |t, r| Tally {
            runs: t.runs + 1,
            first_try: t.first_try + i64::from(r.first_try),
            edits: t.edits + r.edits,
            output: t.output + r.output,
        })
    }

    fn cells(&self) -> String {
        if self.runs == 0 {
            return format!("{:>6}{:>11}{:>8}{:>9}", 0, "-", "-", 0);
        }
        let share = 100.0 * self.first_try as f64 / self.runs as f64;
        let mean = self.edits as f64 / self.runs as f64;
        format!(
            "{:>6}{share:>10.1}%{mean:>8.2}{:>9}",
            self.runs, self.output
        )
    }
}

fn table(title: &str, key: &str, rows: Vec<(String, Tally)>) -> String {
    let head = format!(
        "{key:<16}{:>6}{:>11}{:>8}{:>9}\n",
        "runs", "first-try", "edits", "output"
    );
    let body: String = rows
        .iter()
        .map(|(name, t)| format!("{name:<16}{}\n", t.cells()))
        .collect();
    format!("{title}\n{head}{body}")
}

fn by_lang(title: &str, runs: &[&Run]) -> String {
    let rows = LANGS.map(|lang| {
        let tally = Tally::of(runs.iter().filter(|r| r.lang == lang));
        (lang.name().to_string(), tally)
    });
    table(title, "language", rows.into())
}

fn by_task(title: &str, runs: &[&Run]) -> String {
    let tasks: BTreeSet<&str> = runs.iter().map(|r| r.task.as_str()).collect();
    let mut rows = vec![];
    for task in tasks {
        for lang in LANGS {
            let of = runs.iter().filter(|r| r.task == task && r.lang == lang);
            let tally = Tally::of(of);
            if tally.runs > 0 {
                rows.push((format!("{task} {}", lang.name()), tally));
            }
        }
    }
    table(title, "task language", rows)
}

#[derive(PartialEq, Clone, Copy)]
enum Band {
    Held,
    Close,
    Lost,
}

impl Band {
    /// Where `num / den` falls against two limits, each given as a fraction (M3-02 section 1.6).
    fn of(num: i64, den: i64, held: (i64, i64), close: (i64, i64)) -> Band {
        let within = |(n, d): (i64, i64)| num * d <= n * den;
        if within(held) {
            Band::Held
        } else if within(close) {
            Band::Close
        } else {
            Band::Lost
        }
    }

    fn name(self) -> &'static str {
        match self {
            Band::Held => "held",
            Band::Close => "close",
            Band::Lost => "lost",
        }
    }
}

/// Paka against Rust: each gap as shown and its band.
fn gaps(paka: Tally, rust: Tally) -> [(&'static str, String, Band); 3] {
    let den = paka.runs * rust.runs;
    let first = 100 * (rust.first_try * paka.runs - paka.first_try * rust.runs);
    let edits = paka.edits * rust.runs - rust.edits * paka.runs;
    let shown = |n: i64, d: i64| n as f64 / d as f64;
    [
        (
            "first-try gap",
            format!("{:.1} points", shown(first, den)),
            Band::of(first, den, (10, 1), (20, 1)),
        ),
        (
            "edits gap",
            format!("{:.2}", shown(edits, den)),
            Band::of(edits, den, (1, 2), (1, 1)),
        ),
        (
            "token ratio",
            format!("{:.2}", shown(paka.output, rust.output)),
            Band::of(paka.output, rust.output, (17, 10), (2, 1)),
        ),
    ]
}

fn verdict(runs: &[&Run], dropped: &BTreeSet<String>) -> String {
    let tally = |lang| Tally::of(runs.iter().filter(|r| r.lang == lang));
    let (paka, rust) = (tally(Lang::Paka), tally(Lang::Rust));
    let names: Vec<&str> = dropped.iter().map(String::as_str).collect();
    let mut out = format!(
        "verdict, Paka against Rust\ndropped tasks: {}",
        dropped.len()
    );
    if !names.is_empty() {
        out += &format!(" ({})", names.join(", "));
    }
    out += "\n";
    let bands = if paka.runs > 0 && rust.runs > 0 {
        let gaps = gaps(paka, rust);
        for (name, shown, band) in &gaps {
            out += &format!("{name:<16}{shown:>12}  {}\n", band.name());
        }
        gaps.map(|(_, _, band)| band)
    } else {
        out += "no runs of Paka or Rust\n";
        [Band::Close; 3]
    };
    let word = if dropped.len() > MAX_DROPPED || paka.runs == 0 || rust.runs == 0 {
        "inconclusive"
    } else if bands.contains(&Band::Lost) {
        "stop"
    } else if bands.iter().all(|&b| b == Band::Held) {
        "go"
    } else {
        "inconclusive"
    };
    out + word + "\n"
}

/// The whole report for the runs under `root` and the text of a rulings file.
pub fn report(root: &Path, rulings: &str) -> Result<String, Error> {
    let rulings = Rulings::parse(rulings)?;
    let (runs, unfinished) = load(root)?;
    let folders: Vec<&str> = (runs.iter().map(|r| r.id.as_str()))
        .chain(unfinished.iter().map(String::as_str))
        .collect();
    rulings.check(&folders)?;
    let counted: Vec<&Run> = runs
        .iter()
        .filter(|r| !rulings.void.contains(&r.id))
        .collect();
    let kept: Vec<&Run> = (counted.iter().copied())
        .filter(|r| !rulings.dropped.contains(&r.task))
        .collect();
    let untagged: Vec<&Run> = (kept.iter().copied())
        .filter(|r| !rulings.tagged.contains(&r.id))
        .collect();
    let mut sections = vec![
        by_lang("by language, dropped tasks removed", &kept),
        by_task("by task", &counted),
        verdict(&kept, &rulings.dropped),
        by_lang("not the verdict: nothing dropped", &counted),
        by_lang("not the verdict: tagged runs removed", &untagged),
    ];
    if !unfinished.is_empty() {
        sections.push(format!(
            "unfinished, not counted: {}\n",
            unfinished.join(", ")
        ));
    }
    Ok(sections.join("\n"))
}
