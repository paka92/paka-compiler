use std::{env, fs, process};

type Grid = Vec<Vec<u8>>;

fn step_limit(arg: &str) -> Option<usize> {
    if !arg.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    arg.parse().ok().filter(|&s| s <= 1000)
}

fn parse_grid(text: &str) -> Result<Grid, String> {
    let lines: Vec<&str> = text.lines().collect();
    let width = lines.first().map_or(0, |l| l.len());
    if width == 0 {
        return Err("empty grid".to_string());
    }
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        if let Some(col) = line.bytes().position(|b| !b".#o".contains(&b)) {
            return Err(format!("line {n} col {}: bad cell", col + 1));
        }
        if line.len() != width {
            return Err(format!("line {n}: ragged row"));
        }
    }
    Ok(lines.iter().map(|l| l.bytes().collect()).collect())
}

/// Where the grain at (r, c) goes, if it moves.
fn target(grid: &Grid, r: usize, c: usize) -> Option<(usize, usize)> {
    if r + 1 == grid.len() {
        return None;
    }
    let free = |r: usize, c: usize| grid[r][c] == b'.';
    if free(r + 1, c) {
        Some((r + 1, c))
    } else if c > 0 && free(r, c - 1) && free(r + 1, c - 1) {
        Some((r + 1, c - 1))
    } else if c + 1 < grid[r].len() && free(r, c + 1) && free(r + 1, c + 1) {
        Some((r + 1, c + 1))
    } else {
        None
    }
}

/// Runs one step and tells whether any grain moved.
fn step(grid: &mut Grid) -> bool {
    let mut moved = false;
    for r in (0..grid.len()).rev() {
        for c in 0..grid[r].len() {
            if grid[r][c] != b'o' {
                continue;
            }
            if let Some((r2, c2)) = target(grid, r, c) {
                grid[r][c] = b'.';
                grid[r2][c2] = b'o';
                moved = true;
            }
        }
    }
    moved
}

fn run(file: &str, steps: &str) -> Result<String, String> {
    let limit = step_limit(steps).ok_or("bad steps")?;
    let text = fs::read_to_string(file).expect("grid file is readable");
    let mut grid = parse_grid(&text)?;

    let mut done = 0;
    let settled = loop {
        if done == limit {
            break false;
        }
        if !step(&mut grid) {
            break true;
        }
        done += 1;
    };

    let mut out: String = grid
        .iter()
        .map(|row| String::from_utf8_lossy(row) + "\n")
        .collect();
    if settled {
        out += &format!("steps: {done}, settled\n");
    } else {
        out += &format!("steps: {limit}, limit\n");
    }
    Ok(out)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [file, steps] = &args[..] else {
        panic!("usage: main GRID STEPS");
    };
    match run(file, steps) {
        Ok(out) => print!("{out}"),
        Err(message) => {
            println!("error: {message}");
            process::exit(20);
        }
    }
}
