// Five-point average over a square grid, many sweeps; the border stays fixed.

fn randoms(n: usize, seed: i64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 11) & 9007199254740991) as f64 / 9007199254740992.0
        })
        .collect()
}

fn sweep(src: &[f64], dst: &mut [f64], n: usize) {
    for i in 1..n - 1 {
        for j in 1..n - 1 {
            let p = i * n + j;
            dst[p] = (src[p] + src[p - 1] + src[p + 1] + src[p - n] + src[p + n]) * 0.2;
        }
    }
}

fn main() {
    let n = 1000;
    let mut a = randoms(n * n, 1);
    let mut b = a.clone();
    for _ in 0..1000 {
        sweep(&a, &mut b, n);
        sweep(&b, &mut a, n);
    }
    let total: f64 = a.iter().sum();
    println!("{}", (total * 1000000000.0) as i64);
}
