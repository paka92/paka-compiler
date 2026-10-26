// A 64-bit mixing generator folded into a running hash. Wrapping arithmetic.

fn hash(n: i64) -> i64 {
    let mut s: i64 = 1;
    let mut h: i64 = 0;
    for _ in 0..n {
        s = s.wrapping_add(-7046029254386353131);
        let mut z = (s ^ (s >> 30)).wrapping_mul(-4658895280553007687);
        z = (z ^ (z >> 27)).wrapping_mul(-7723592293110705685);
        z ^= z >> 31;
        h = (h ^ z).wrapping_mul(1099511628211);
    }
    h
}

fn main() {
    println!("{}", hash(1000000000));
}
