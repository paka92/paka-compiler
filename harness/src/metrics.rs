//! The metrics tool: the raw logs under a runs root and a rulings file to the three metrics of
//! M3-02 section 1.9 and the verdict of section 1.7, pooled over runs. All comparisons use whole
//! numbers, so no float tie decides a verdict.

use crate::Error;
use crate::json::Value;
use crate::log;
use crate::prompt::SUBMISSIONS;
use crate::runner::Lang;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const LANGS: [Lang; 3] = [Lang::Paka, Lang::Rust, Lang::TypeScript];
/// Practice tasks never enter the verdict (M3-04).
const PRACTICE: [&str; 2] = ["X1", "X2"];
/// More dropped tasks than this make the verdict inconclusive (M3-03 section 1.2).
const MAX_DROPPED: usize = 2;

fn malformed<T>(what: String) -> Result<T, Error> {
    Err(Error::Malformed(what))
}

/// What an `end` line says.
struct End {
    passed: bool,
    first_try: bool,
    submissions: i64,
    edits: i64,
    input: i64,
    output: i64,
    cut_off: i64,
}

impl End {
    /// Reads the line and checks it against the counting rules of M3-01: 8 edits for a run that
    /// never passed, else one less than its submissions.
    fn new(id: &str, line: &Value) -> Result<End, Error> {
        let bad = || {
            Error::Malformed(format!(
                "run {id}: the end line lacks a field or adds up wrong"
            ))
        };
        let flag = |key: &str| line.get(key).and_then(Value::as_bool).ok_or_else(bad);
        let number = |key: &str| {
            let n = line.get(key).and_then(Value::as_u64);
            n.and_then(|n| i64::try_from(n).ok()).ok_or_else(bad)
        };
        let end = End {
            passed: flag("passed")?,
            first_try: flag("first_try")?,
            submissions: number("submissions")?,
            edits: number("edits")?,
            input: number("input")?,
            output: number("output")?,
            cut_off: number("cut_off")?,
        };
        let edits = if end.passed {
            end.submissions - 1
        } else {
            i64::from(SUBMISSIONS)
        };
        let first_try = end.passed && end.submissions == 1;
        if end.edits == edits && end.first_try == first_try {
            Ok(end)
        } else {
            Err(bad())
        }
    }
}

/// A run folder `<task>-<language>-<rep>`: the end of its last attempt that ended, if any.
struct Run {
    id: String,
    task: String,
    lang: Lang,
    refused: i64,
    end: Option<End>,
}

impl Run {
    fn read(root: &Path, id: String) -> Result<Run, Error> {
        let mut parts = id.rsplitn(3, '-');
        let rep = parts.next().and_then(|rep| rep.parse::<u32>().ok());
        let lang = parts
            .next()
            .and_then(|n| LANGS.into_iter().find(|l| l.name() == n));
        let (Some(_), Some(lang), Some(task)) = (rep, lang, parts.next()) else {
            return malformed(format!("{id} is not a run folder, <task>-<language>-<rep>"));
        };
        if PRACTICE.contains(&task) {
            return malformed(format!(
                "{id} is a practice run, which never enters the verdict"
            ));
        }
        let lines = last_lines(&root.join(&id))?;
        let event = |line: &Value| line.get("event").and_then(Value::as_str).map(str::to_owned);
        let refusal = Error::Refused.to_string();
        let refused = (lines.iter())
            .filter(|l| event(l).as_deref() == Some("void"))
            .filter(|l| l.get("reason").and_then(Value::as_str) == Some(&refusal))
            .count();
        let end = (lines.iter().rev())
            .find(|l| event(l).as_deref() == Some("end"))
            .map(|line| End::new(&id, line))
            .transpose()?;
        let (task, refused) = (task.to_string(), refused as i64);
        Ok(Run {
            id,
            task,
            lang,
            refused,
            end,
        })
    }
}

/// The last complete line of each attempt of a run, in order.
fn last_lines(run: &Path) -> Result<Vec<Value>, Error> {
    let mut lines = vec![];
    for n in 1.. {
        let attempt = run.join(n.to_string());
        if !attempt.exists() {
            break;
        }
        lines.extend(log::last_line(&attempt.join("log"))?);
    }
    Ok(lines)
}

/// Every run folder under `root`; anything else there is an error.
fn load(root: &Path) -> Result<Vec<Run>, Error> {
    let mut ids = vec![];
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_dir() {
            return malformed(format!("{id} is not a run folder"));
        }
        ids.push(id);
    }
    ids.sort();
    ids.into_iter().map(|id| Run::read(root, id)).collect()
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
            let words: Vec<&str> = line.split_whitespace().collect();
            let (set, name) = match words[..] {
                ["run", id, "void", _, ..] => (&mut rulings.void, id),
                ["run", id, "tagged", _, ..] => (&mut rulings.tagged, id),
                ["task", id, "dropped", _, ..] => (&mut rulings.dropped, id),
                _ => return malformed(format!("rulings line {}: {line}", n + 1)),
            };
            set.insert(name.to_string());
        }
        Ok(rulings)
    }

    /// A ruling that names no run or task is a typo, and would change nothing silently.
    fn check(&self, runs: &[Run]) -> Result<(), Error> {
        let known_run = |id: &&String| runs.iter().all(|r| r.id != **id);
        let known_task = |t: &&String| runs.iter().all(|r| r.task != **t);
        let unknown = (self.void.iter().chain(&self.tagged).filter(known_run))
            .chain(self.dropped.iter().filter(known_task))
            .next();
        match unknown {
            Some(name) => malformed(format!("the rulings name {name}, which is no run or task")),
            None => Ok(()),
        }
    }
}

/// Sums over a set of runs.
#[derive(Default, Clone, Copy)]
struct Tally {
    runs: i64,
    first_try: i64,
    solved: i64,
    edits: i64,
    calls: i64,
    input: i64,
    output: i64,
    cut_off: i64,
    refused: i64,
}

impl Tally {
    fn of<'a>(runs: impl IntoIterator<Item = &'a &'a Run>) -> Tally {
        let mut t = Tally::default();
        for run in runs {
            t.refused += run.refused;
            let Some(end) = &run.end else { continue };
            t.runs += 1;
            t.first_try += i64::from(end.first_try);
            t.solved += i64::from(end.passed);
            t.edits += end.edits;
            t.calls += end.submissions;
            t.input += end.input;
            t.output += end.output;
            t.cut_off += end.cut_off;
        }
        t
    }

    fn of_lang(runs: &[&Run], lang: Lang) -> Tally {
        Tally::of(runs.iter().filter(|r| r.lang == lang))
    }

    /// A share of the runs, as a percentage.
    fn share(&self, part: i64) -> String {
        match self.runs {
            0 => "-".into(),
            n => format!("{:.1}%", 100.0 * part as f64 / n as f64),
        }
    }

    fn cells(&self) -> Vec<String> {
        let mean = match self.runs {
            0 => "-".into(),
            n => format!("{:.2}", self.edits as f64 / n as f64),
        };
        vec![
            self.runs.to_string(),
            self.share(self.first_try),
            mean,
            self.output.to_string(),
        ]
    }
}

/// A table under a title, with the first column left-aligned and the rest right-aligned, each
/// as wide as its longest cell.
fn grid(title: &str, head: &[&str], rows: Vec<Vec<String>>) -> String {
    let width = |c: usize| {
        let cells = rows.iter().map(|row| row[c].len());
        (cells.chain([head[c].len()]).max().unwrap_or(0) + 2).max(10)
    };
    let line = |cells: Vec<&str>| {
        let line: String = (cells.iter().enumerate())
            .map(|(c, cell)| match c {
                0 => format!("{cell:<16}"),
                _ => format!("{cell:>w$}", w = width(c)),
            })
            .collect();
        line.trim_end().to_string() + "\n"
    };
    let body: String = rows
        .iter()
        .map(|row| line(row.iter().map(String::as_str).collect()))
        .collect();
    format!("{title}\n{}{body}", line(head.to_vec()))
}

const METRIC_HEAD: [&str; 5] = ["language", "runs", "first-try", "edits", "output"];

fn by_lang(title: &str, runs: &[&Run]) -> String {
    let rows = (LANGS.iter())
        .map(|&lang| {
            [
                vec![lang.name().to_string()],
                Tally::of_lang(runs, lang).cells(),
            ]
            .concat()
        })
        .collect();
    grid(title, &METRIC_HEAD, rows)
}

fn by_task(runs: &[&Run], dropped: &BTreeSet<String>) -> String {
    let tasks: BTreeSet<&str> = runs.iter().map(|r| r.task.as_str()).collect();
    let mut rows = vec![];
    for task in tasks {
        for lang in LANGS {
            let tally = Tally::of(runs.iter().filter(|r| r.task == task && r.lang == lang));
            if tally.runs > 0 {
                let note = if dropped.contains(task) {
                    "dropped"
                } else {
                    ""
                };
                let name = format!("{task} {}", lang.name());
                rows.push([vec![name], tally.cells(), vec![note.into()]].concat());
            }
        }
    }
    let head = [
        "task language",
        "runs",
        "first-try",
        "edits",
        "output",
        "ruling",
    ];
    grid("not the verdict: by task", &head, rows)
}

fn replies(runs: &[&Run]) -> String {
    let rows = (LANGS.iter()).map(|&lang| {
        let t = Tally::of_lang(runs, lang);
        let cells = [t.runs, t.cut_off, t.refused].map(|n| n.to_string());
        [
            vec![lang.name().to_string(), t.share(t.solved)],
            cells[1..].to_vec(),
        ]
        .concat()
    });
    let head = ["language", "solved", "cut-off", "refused"];
    let title = "not the verdict: solve rate, cut-off and refused replies";
    grid(title, &head, rows.collect())
}

/// Input tokens; Paka's reference is sent with every call, so its tokens are taken off once per
/// call when the reference's token count is known.
fn input(runs: &[&Run], reference: Option<i64>) -> String {
    let rows = LANGS.iter().map(|&lang| {
        let t = Tally::of_lang(runs, lang);
        let without = match (lang, reference) {
            (Lang::Paka, None) => "needs the reference token count".into(),
            (Lang::Paka, Some(r)) => (t.input - r * t.calls).to_string(),
            _ => t.input.to_string(),
        };
        vec![lang.name().to_string(), t.input.to_string(), without]
    });
    grid(
        "not the verdict: input tokens",
        &["language", "input", "without reference"],
        rows.collect(),
    )
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

/// Three gaps of Paka against another language: name, value as shown, band.
type Gaps = [(&'static str, String, Band); 3];

/// `None` when either language has no run. The bands are decided on exact whole numbers, the
/// values shown are only rounded: points to 2 decimals, edits and ratio to 3.
fn gaps(paka: Tally, other: Tally) -> Option<Gaps> {
    if paka.runs == 0 || other.runs == 0 {
        return None;
    }
    let den = paka.runs * other.runs;
    let first = 100 * (other.first_try * paka.runs - paka.first_try * other.runs);
    let edits = paka.edits * other.runs - other.edits * paka.runs;
    let shown = |n: i64, d: i64| n as f64 / d as f64;
    Some([
        (
            "first-try gap",
            format!("{:.2} points", shown(first, den)),
            Band::of(first, den, (10, 1), (20, 1)),
        ),
        (
            "edits gap",
            format!("{:.3}", shown(edits, den)),
            Band::of(edits, den, (1, 2), (1, 1)),
        ),
        (
            "token ratio",
            format!("{:.3}", shown(paka.output, other.output)),
            Band::of(paka.output, other.output, (17, 10), (2, 1)),
        ),
    ])
}

fn gap_lines(gaps: &Option<Gaps>) -> String {
    let Some(gaps) = gaps else {
        return "no runs of one of the two languages\n".into();
    };
    let line = |(name, shown, band): &(&str, String, Band)| {
        format!("{name:<16}{shown:>16}  {}\n", band.name())
    };
    gaps.iter().map(line).collect()
}

/// The verdict of M3-02 section 1.7; it needs the gaps of Paka against Rust.
fn decide(dropped: usize, gaps: &Option<Gaps>) -> &'static str {
    let Some(gaps) = gaps else {
        return "inconclusive";
    };
    let bands = gaps.iter().map(|g| g.2);
    if dropped > MAX_DROPPED {
        "inconclusive"
    } else if bands.clone().any(|b| b == Band::Lost) {
        "stop"
    } else if bands.into_iter().all(|b| b == Band::Held) {
        "go"
    } else {
        "inconclusive"
    }
}

/// Why the run set cannot give a verdict: a kept task with other counts of ended runs in the
/// three languages, or a run that did not end. `None` when the set is complete.
fn incomplete(kept: &[&Run]) -> Option<String> {
    let mut counts: BTreeMap<&str, [i64; 3]> = BTreeMap::new();
    for run in kept {
        let at = LANGS
            .iter()
            .position(|&l| l == run.lang)
            .expect("a known language");
        counts.entry(&run.task).or_default()[at] += i64::from(run.end.is_some());
    }
    let mut lines = vec![];
    for (task, n) in counts.iter().filter(|(_, n)| n.iter().any(|c| *c != n[0])) {
        let each: Vec<String> = LANGS
            .iter()
            .zip(n)
            .map(|(l, c)| format!("{} {c}", l.name()))
            .collect();
        lines.push(format!("{task}: {}", each.join(", ")));
    }
    let unfinished: Vec<&str> = (kept.iter())
        .filter(|r| r.end.is_none())
        .map(|r| r.id.as_str())
        .collect();
    if !unfinished.is_empty() {
        lines.push(format!("unfinished: {}", unfinished.join(", ")));
    }
    (!lines.is_empty()).then(|| format!("no verdict: incomplete run set\n{}\n", lines.join("\n")))
}

/// The verdict block, or why there is none.
fn verdict(kept: &[&Run], dropped: &BTreeSet<String>) -> (String, bool) {
    if let Some(text) = incomplete(kept) {
        return (text, false);
    }
    let paka = Tally::of_lang(kept, Lang::Paka);
    let gaps = gaps(paka, Tally::of_lang(kept, Lang::Rust));
    let names: Vec<&str> = dropped.iter().map(String::as_str).collect();
    let listed = if names.is_empty() {
        String::new()
    } else {
        format!(" ({})", names.join(", "))
    };
    let head = format!(
        "verdict, Paka against Rust\ndropped tasks: {}{listed}\n",
        dropped.len()
    );
    (
        format!(
            "{head}{}{}\n",
            gap_lines(&gaps),
            decide(dropped.len(), &gaps)
        ),
        true,
    )
}

pub struct Report {
    pub text: String,
    /// False when the run set is incomplete and the text gives no verdict.
    pub complete: bool,
}

/// The whole report for the runs under `root` and the text of a rulings file; `reference` is
/// the token count of the Paka language reference, when known.
pub fn report(root: &Path, rulings: &str, reference: Option<u64>) -> Result<Report, Error> {
    let rulings = Rulings::parse(rulings)?;
    let reference = reference
        .map(i64::try_from)
        .transpose()
        .or_else(|_| malformed("bad reference".into()))?;
    let runs = load(root)?;
    rulings.check(&runs)?;
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
    let unfinished: Vec<&str> = (counted.iter())
        .filter(|r| r.end.is_none())
        .map(|r| r.id.as_str())
        .collect();
    let (verdict, complete) = verdict(&kept, &rulings.dropped);
    let mut sections = vec![
        by_lang("by language, dropped tasks removed", &kept),
        by_task(&counted, &rulings.dropped),
        verdict,
        by_lang("not the verdict: nothing dropped", &counted),
        by_lang("not the verdict: tagged runs removed", &untagged),
        replies(&kept),
        input(&kept, reference),
        format!(
            "not the verdict: Paka against TypeScript\n{}",
            gap_lines(&gaps(
                Tally::of_lang(&kept, Lang::Paka),
                Tally::of_lang(&kept, Lang::TypeScript)
            ))
        ),
    ];
    if !unfinished.is_empty() {
        sections.push(format!(
            "unfinished, not counted: {}\n",
            unfinished.join(", ")
        ));
    }
    Ok(Report {
        text: sections.join("\n"),
        complete,
    })
}
