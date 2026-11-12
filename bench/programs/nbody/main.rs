// Five bodies under gravity, many time steps, then the total energy.

struct Body {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    mass: f64,
}

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

fn bodies() -> Vec<Body> {
    randoms(35, 1)
        .chunks(7)
        .map(|r| Body {
            x: 10.0 * r[0] - 5.0,
            y: 10.0 * r[1] - 5.0,
            z: 10.0 * r[2] - 5.0,
            vx: r[3] - 0.5,
            vy: r[4] - 0.5,
            vz: r[5] - 0.5,
            mass: r[6] + 0.5,
        })
        .collect()
}

fn advance(bs: &mut [Body], dt: f64) {
    for i in 0..bs.len() {
        let (left, right) = bs.split_at_mut(i + 1);
        let a = &mut left[i];
        for b in right {
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dz = a.z - b.z;
            let d2 = dx * dx + dy * dy + dz * dz;
            let mag = dt / (d2 * d2.sqrt());
            let mi = a.mass * mag;
            let mj = b.mass * mag;
            a.vx -= dx * mj;
            a.vy -= dy * mj;
            a.vz -= dz * mj;
            b.vx += dx * mi;
            b.vy += dy * mi;
            b.vz += dz * mi;
        }
    }
    for b in bs {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}

fn energy(bs: &[Body]) -> f64 {
    let mut e = 0.0;
    for (i, a) in bs.iter().enumerate() {
        e += 0.5 * a.mass * (a.vx * a.vx + a.vy * a.vy + a.vz * a.vz);
        for b in &bs[i + 1..] {
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dz = a.z - b.z;
            e -= a.mass * b.mass / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    e
}

fn main() {
    let mut bs = bodies();
    for _ in 0..30000000 {
        advance(&mut bs, 0.001);
    }
    println!("{}", (energy(&bs) * 1000000000000.0) as i64);
}
