// Count the primes below N with a sieve of flags, for a run of N. Wrapping arithmetic.

fn count_primes(n: usize) -> usize {
    let mut prime = vec![true; n];
    let mut count: usize = 0;
    for i in 2..n {
        if prime[i] {
            count = count.wrapping_add(1);
            let mut j = i.wrapping_mul(i);
            while j < n {
                prime[j] = false;
                j = j.wrapping_add(i);
            }
        }
    }
    count
}

fn main() {
    let total = (0..80).fold(0usize, |t, r| {
        t.wrapping_add(count_primes(10000000usize.wrapping_add(r)))
    });
    println!("{total}");
}
