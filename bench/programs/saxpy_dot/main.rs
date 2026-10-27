// Scale-and-add over two lists, then their dot product, repeated.

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

fn saxpy(a: f64, x: &[f64], y: &mut [f64]) {
    for (yi, xi) in y.iter_mut().zip(x) {
        *yi = a * xi + *yi;
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn main() {
    let x = randoms(1000000, 1);
    let mut y = randoms(1000000, 2);
    let mut total = 0.0;
    for _ in 0..1000 {
        saxpy(0.5, &x, &mut y);
        total += dot(&x, &y);
    }
    println!("{}", (total * 1000.0) as i64);
}
