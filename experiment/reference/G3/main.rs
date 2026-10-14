use std::{env, fs, process};

type Grid = Vec<Vec<u8>>;
/// Cost and number of moves of the best path from a cell to the end.
type Best = Option<(u64, u64)>;

/// The moves in tie-break order, with their row and column steps.
const MOVES: [(char, usize, usize); 3] = [('R', 0, 1), ('D', 1, 0), ('X', 1, 1)];

fn parse_grid(text: &str) -> Result<Grid, String> {
    let lines: Vec<&str> = text.lines().collect();
    let width = lines.first().map_or(0, |l| l.len());
    if width == 0 {
        return Err("empty grid".into());
    }
    for (i, line) in lines.iter().enumerate() {
        if let Some(c) = line.bytes().position(|b| b != b'#' && !b.is_ascii_digit()) {
            return Err(format!("line {} col {}: bad cell", i + 1, c + 1));
        }
    }
    if let Some(i) = lines.iter().position(|l| l.len() != width) {
        return Err(format!("line {}: ragged row", i + 1));
    }
    Ok(lines.iter().map(|l| l.bytes().collect()).collect())
}

fn is_open(g: &Grid, r: usize, c: usize) -> bool {
    r < g.len() && c < g[0].len() && g[r][c] != b'#'
}

/// The cell a move from (r, c) reaches, if the move is allowed.
fn target(
    g: &Grid,
    r: usize,
    c: usize,
    (m, dr, dc): (char, usize, usize),
) -> Option<(usize, usize)> {
    let corner = m != 'X' || (is_open(g, r, c + 1) && is_open(g, r + 1, c));
    (corner && is_open(g, r + dr, c + dc)).then_some((r + dr, c + dc))
}

/// The best path's cost and moves from (r, c) through `mv`, given the best from its target.
fn through(g: &Grid, best: &[Vec<Best>], r: usize, c: usize, mv: (char, usize, usize)) -> Best {
    let (tr, tc) = target(g, r, c, mv)?;
    let (cost, moves) = best[tr][tc]?;
    let extra = if mv.0 == 'X' { 2 } else { 0 };
    Some((cost + u64::from(g[r][c] - b'0') + extra, moves + 1))
}

fn solve(g: &Grid) -> Result<String, String> {
    let (rows, cols) = (g.len(), g[0].len());
    if g[0][0] == b'#' {
        return Err("start is a wall".into());
    }
    if g[rows - 1][cols - 1] == b'#' {
        return Err("end is a wall".into());
    }
    let mut best: Vec<Vec<Best>> = vec![vec![None; cols]; rows];
    best[rows - 1][cols - 1] = Some((u64::from(g[rows - 1][cols - 1] - b'0'), 0));
    for r in (0..rows).rev() {
        for c in (0..cols).rev() {
            if g[r][c] == b'#' || (r, c) == (rows - 1, cols - 1) {
                continue;
            }
            best[r][c] = MOVES
                .iter()
                .filter_map(|&m| through(g, &best, r, c, m))
                .min();
        }
    }
    let (cost, total) = best[0][0].ok_or("no path")?;
    let (mut r, mut c) = (0, 0);
    let mut path = String::new();
    while (r, c) != (rows - 1, cols - 1) {
        let mv = MOVES
            .into_iter()
            .find(|&m| through(g, &best, r, c, m) == best[r][c])
            .unwrap();
        path.push(mv.0);
        (r, c) = (r + mv.1, c + mv.2);
    }
    Ok(format!("cost {cost}\n{}\n", moves_line(total, &path)))
}

fn moves_line(total: u64, path: &str) -> String {
    let mut out = format!("moves {total}");
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let run = bytes[i..].iter().take_while(|&&b| b == bytes[i]).count();
        out.push_str(if i == 0 { ": " } else { " " });
        out.push(bytes[i] as char);
        if run > 1 {
            out += &run.to_string();
        }
        i += run;
    }
    out
}

fn main() {
    let path = env::args().nth(1).expect("usage: main GRID");
    let text = fs::read_to_string(path).expect("cannot read the grid");
    match parse_grid(&text).and_then(|g| solve(&g)) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
