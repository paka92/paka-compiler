// Plain matrix product of two square matrices held in flat lists.

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

fn multiply(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n * n];
    for (a_row, c_row) in a.chunks(n).zip(c.chunks_mut(n)) {
        for (aik, b_row) in a_row.iter().zip(b.chunks(n)) {
            for (cij, bkj) in c_row.iter_mut().zip(b_row) {
                *cij += aik * bkj;
            }
        }
    }
    c
}

fn main() {
    let n = 1500;
    let c = multiply(&randoms(n * n, 1), &randoms(n * n, 2), n);
    let total: f64 = c.iter().sum();
    println!("{}", (total * 1000000.0) as i64);
}
