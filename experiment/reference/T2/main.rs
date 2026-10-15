use std::{env, fs, process};

/// The values of one row, or `None` on a quote error.
fn split_row(line: &str) -> Option<Vec<String>> {
    let mut rest = line;
    let mut fields = vec![];
    loop {
        let mut value = String::new();
        if let Some(quoted) = rest.strip_prefix('"') {
            rest = quoted;
            loop {
                let (text, after) = rest.split_once('"')?;
                value += text;
                match after.strip_prefix('"') {
                    Some(doubled) => {
                        value.push('"');
                        rest = doubled;
                    }
                    None => {
                        rest = after;
                        break;
                    }
                }
            }
            if !rest.is_empty() && !rest.starts_with(',') {
                return None;
            }
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            value = rest[..end].to_string();
            if value.contains('"') {
                return None;
            }
            rest = &rest[end..];
        }
        fields.push(value);
        match rest.strip_prefix(',') {
            Some(after) => rest = after,
            None => return Some(fields),
        }
    }
}

fn parse(text: &str) -> Result<Vec<Vec<String>>, String> {
    if text.is_empty() {
        return Err("empty input".to_string());
    }
    let mut rows: Vec<Vec<String>> = vec![];
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let row = split_row(line).ok_or(format!("line {n}: bad quote"))?;
        if let Some(header) = rows.first().filter(|h| h.len() != row.len()) {
            let (h, f) = (header.len(), row.len());
            return Err(format!("line {n}: expected {h} fields, got {f}"));
        }
        rows.push(row);
    }
    Ok(rows)
}

fn as_number(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let shaped = (1..=12).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
    shaped.then(|| value.parse().unwrap())
}

/// The sum of the column when it is a number column.
fn column_sum(data: &[Vec<String>], column: usize) -> Option<i64> {
    let numbers: Option<Vec<i64>> = data.iter().map(|row| as_number(&row[column])).collect();
    numbers.filter(|n| !n.is_empty()).map(|n| n.iter().sum())
}

fn table_line(row: &[String], widths: &[usize], is_number: &[bool]) -> String {
    let padded: Vec<String> = row
        .iter()
        .zip(widths)
        .zip(is_number)
        .map(|((value, &w), &number)| match number {
            true => format!("{value:>w$}"),
            false => format!("{value:<w$}"),
        })
        .collect();
    padded.join(" | ").trim_end_matches(' ').to_string()
}

fn run(file: &str) -> Result<String, String> {
    let text = fs::read_to_string(file).expect("input file is readable");
    let rows = parse(&text)?;
    let (header, data) = rows.split_first().unwrap();
    let sums: Vec<Option<i64>> = (0..header.len()).map(|c| column_sum(data, c)).collect();
    let is_number: Vec<bool> = sums.iter().map(Option::is_some).collect();

    let total: Option<Vec<String>> = is_number.contains(&true).then(|| {
        let cell = |c: usize| match sums[c] {
            Some(sum) => sum.to_string(),
            None if c == 0 => "total".to_string(),
            None => String::new(),
        };
        (0..header.len()).map(cell).collect()
    });
    let shown = rows.iter().chain(total.as_ref());
    let widths: Vec<usize> = (0..header.len())
        .map(|c| shown.clone().map(|row| row[c].len()).max().unwrap().max(1))
        .collect();

    let line = |row: &[String]| table_line(row, &widths, &is_number) + "\n";
    let rule: Vec<String> = widths.iter().map(|&w| "-".repeat(w)).collect();
    let rule = rule.join("-+-") + "\n";
    let mut out = line(header) + &rule;
    out += &data.iter().map(|row| line(row)).collect::<String>();
    if let Some(total) = &total {
        out += &(rule + &line(total));
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
