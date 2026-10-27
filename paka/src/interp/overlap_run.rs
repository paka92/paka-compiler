//! The inout overlap run (M6-22): generated programs whose calls often read a part of an `&`
//! argument's local beside it, run with the overlap detector on. A program the checker refuses
//! must be refused for an overlap; in one it accepts, the detector flags no call. The full run is
//! ignored; docs/results/M6-22.md gives its command and numbers.

use super::effects::Host;
use super::run_watched;
use crate::body::RULE_A_OFF;
use crate::source::Source;
use std::io::sink;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// What a run over seeds `0..n` found.
#[derive(Default, Debug)]
struct Sweep {
    programs: u64,
    refused: u64,
    /// Calls run in accepted programs that took an inout and another argument from its local.
    shared: u64,
    /// Each accepted program with a flagged call, and refusals for anything but an overlap.
    flagged: Vec<u64>,
    wrong: Vec<String>,
}

impl Sweep {
    fn add(&mut self, seed: u64, rule_a: bool) {
        self.programs += 1;
        let text = paka_gen::program_with_overlaps(seed);
        RULE_A_OFF.set(!rule_a);
        let checked = crate::check("main.paka", &Source::file("main.paka", text.into()));
        match checked {
            Err(d) if d.code == "inout-overlap" => self.refused += 1,
            Err(d) => self.wrong.push(format!("seed {seed}: {d}")),
            Ok((core, ..)) => {
                let (_, detector) = run_watched(&core, &[], &mut Host(sink()), &mut sink());
                self.shared += detector.shared;
                if !detector.flagged.is_empty() {
                    self.flagged.push(seed);
                }
            }
        }
    }

    fn join(&mut self, other: Sweep) {
        self.programs += other.programs;
        self.refused += other.refused;
        self.shared += other.shared;
        self.flagged.extend(other.flagged);
        self.wrong.extend(other.wrong);
    }
}

/// Seeds `0..n` on every core, with Rule A of the checker on or off.
fn sweep(n: u64, rule_a: bool) -> Sweep {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = Arc::new(AtomicU64::new(0));
    let found = Arc::new(Mutex::new(Sweep::default()));
    let workers: Vec<_> = (0..threads)
        .map(|_| {
            let (next, found) = (next.clone(), found.clone());
            // The main thread's stack, which the checker runs on.
            let worker = std::thread::Builder::new().stack_size(8 << 20);
            worker.spawn(move || {
                let mut mine = Sweep::default();
                loop {
                    let seed = next.fetch_add(1, Ordering::Relaxed);
                    if seed >= n {
                        break;
                    }
                    mine.add(seed, rule_a);
                }
                found.lock().unwrap().join(mine);
            })
        })
        .collect();
    workers.into_iter().for_each(|w| w.unwrap().join().unwrap());
    let mut found = Arc::try_unwrap(found).unwrap().into_inner().unwrap();
    found.flagged.sort();
    found
}

/// The run of seeds `0..n` with every rule on; prints its report.
fn clean(n: u64) -> Sweep {
    let start = Instant::now();
    let found = sweep(n, true);
    println!(
        "{} programs, {} refused for an overlap, {} accepted; {} accepted calls took an inout \
         and another argument from its local; {} flagged; {:.0?}",
        found.programs,
        found.refused,
        found.programs - found.refused,
        found.shared,
        found.flagged.len(),
        start.elapsed()
    );
    assert!(found.wrong.is_empty(), "{}", found.wrong.join("\n"));
    assert_eq!(found.flagged, [] as [u64; 0], "flagged seeds");
    found
}

const SMALL: u64 = 2_000;

#[test]
fn accepted_calls_never_overlap_in_a_small_run() {
    let found = clean(SMALL);
    assert!(found.refused > 0 && found.shared > 0, "{found:?}");
}

/// `PAKA_OVERLAP_SEEDS` sets how many (default one million).
#[test]
#[ignore = "the full run; see docs/results/M6-22.md"]
fn accepted_calls_never_overlap_in_a_million_programs() {
    let n = std::env::var("PAKA_OVERLAP_SEEDS").map_or(1_000_000, |n| n.parse().unwrap());
    assert!(clean(n).shared >= 100_000);
}

#[test]
fn with_rule_a_off_the_small_run_flags_an_overlap() {
    let found = sweep(SMALL, false);
    println!("flagged seeds: {:?}", found.flagged);
    assert_eq!(found.refused, 0);
    assert!(!found.flagged.is_empty());
}
