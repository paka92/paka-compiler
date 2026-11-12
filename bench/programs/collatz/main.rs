// The start below N of the longest Collatz chain.

fn chain(start: i64) -> i64 {
    let mut x = start;
    let mut n: i64 = 1;
    while x != 1 {
        x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
        n += 1;
    }
    n
}

fn main() {
    let (mut best, mut best_start) = (0, 0);
    for i in 1..9000000 {
        let n = chain(i);
        if n > best {
            (best, best_start) = (n, i);
        }
    }
    println!("{best_start}");
}
