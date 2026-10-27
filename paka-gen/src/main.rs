//! `paka-gen SEED` prints the program of that seed.

fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let Some(seed) = seed else {
        eprintln!("usage: paka-gen SEED");
        std::process::exit(2);
    };
    print!("{}", paka_gen::program(seed));
}
