// Total Collatz steps for 1 to 9,999.

fn steps(start: i64) -> i64 {
    let mut x = start;
    let mut n = 0;
    while x != 1 {
        x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
        n += 1;
    }
    n
}

fn main() {
    let total: i64 = (1..10000).map(steps).sum();
    println!("{total}");
}
