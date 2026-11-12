use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::{env, fs, process};

/// Cost and number of roads of a best route.
type Dist = Option<(u64, u64)>;

fn is_name(s: &str) -> bool {
    (1..=20).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn is_cost(s: &str) -> bool {
    (1..=6).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

struct Road<'a> {
    from: &'a str,
    to: &'a str,
    cost: u64,
}

/// Places are indexes into the names in ASCII order; `out[u]` and `into[u]` hold (place, cost).
struct Map<'a> {
    names: Vec<&'a str>,
    out: Vec<Vec<(usize, u64)>>,
    into: Vec<Vec<(usize, u64)>>,
}

impl<'a> Map<'a> {
    fn new(roads: &[Road<'a>]) -> Self {
        let mut names: Vec<&str> = roads.iter().flat_map(|r| [r.from, r.to]).collect();
        names.sort();
        names.dedup();
        let mut map = Map {
            out: vec![vec![]; names.len()],
            into: vec![vec![]; names.len()],
            names,
        };
        for r in roads {
            let (a, b) = (map.place(r.from).unwrap(), map.place(r.to).unwrap());
            map.out[a].push((b, r.cost));
            map.into[b].push((a, r.cost));
        }
        map
    }

    fn place(&self, name: &str) -> Option<usize> {
        self.names.binary_search(&name).ok()
    }

    /// The best cost and roads from every place to `end`.
    fn to_end(&self, end: usize) -> Vec<Dist> {
        let mut dist: Vec<Dist> = vec![None; self.names.len()];
        let mut heap = BinaryHeap::from([Reverse((0, 0, end))]);
        while let Some(Reverse((cost, roads, v))) = heap.pop() {
            if dist[v].is_some() {
                continue;
            }
            dist[v] = Some((cost, roads));
            for &(u, w) in &self.into[v] {
                if dist[u].is_none() {
                    heap.push(Reverse((cost + w, roads + 1, u)));
                }
            }
        }
        dist
    }

    fn trip(&self, from: &str, to: &str) -> String {
        let (Some(start), Some(end)) = (self.place(from), self.place(to)) else {
            let missing = if self.place(from).is_none() { from } else { to };
            return format!("unknown {missing}");
        };
        let dist = self.to_end(end);
        let Some((cost, roads)) = dist[start] else {
            return "no route".into();
        };
        let mut path = vec![self.names[start]];
        let mut u = start;
        while u != end {
            u = self.out[u]
                .iter()
                .filter(|&&(v, w)| dist[v].map(|(c, r)| (c + w, r + 1)) == dist[u])
                .map(|&(v, _)| v)
                .min()
                .unwrap();
            path.push(self.names[u]);
        }
        let word = if roads == 1 { "road" } else { "roads" };
        format!("cost {cost}, {roads} {word}: {}", path.join(" "))
    }
}

fn run(text: &str) -> Result<String, String> {
    let mut roads = vec![];
    let mut trips = vec![];
    for (i, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        let bad = || format!("line {}: bad line", i + 1);
        match words[..] {
            [] => {}
            ["trip", a, b] if is_name(a) && is_name(b) => trips.push((a, b)),
            [kind @ ("one" | "two"), a, b, c] if is_name(a) && is_name(b) && is_cost(c) => {
                if a == b {
                    return Err(format!("line {}: loop road", i + 1));
                }
                let cost = c.parse().unwrap();
                roads.push(Road {
                    from: a,
                    to: b,
                    cost,
                });
                if kind == "two" {
                    roads.push(Road {
                        from: b,
                        to: a,
                        cost,
                    });
                }
            }
            _ => return Err(bad()),
        }
    }
    if trips.is_empty() {
        return Err("no trips".into());
    }
    let map = Map::new(&roads);
    let answers: Vec<String> = trips
        .iter()
        .map(|&(a, b)| format!("{a} to {b}: {}\n", map.trip(a, b)))
        .collect();
    Ok(answers.concat())
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
