// Count the primes below N with a sieve of flags, for a run of N.

fn count_primes(n: usize) -> usize {
    let mut prime = vec![true; n];
    let mut count: usize = 0;
    for i in 2..n {
        if prime[i] {
            count += 1;
            let mut j = i * i;
            while j < n {
                prime[j] = false;
                j += i;
            }
        }
    }
    count
}

fn main() {
    let total: usize = (0..80).map(|r| count_primes(10000000 + r)).sum();
    println!("{total}");
}
