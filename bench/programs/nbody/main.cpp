// Five bodies under gravity, many time steps, then the total energy.

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <vector>

struct Body {
    double x, y, z, vx, vy, vz, mass;
};

static std::vector<double> randoms(std::size_t n, uint64_t s) {
    std::vector<double> xs(n);
    for (auto& x : xs) {
        s = s * 6364136223846793005u + 1442695040888963407u;
        x = double((s >> 11) & 9007199254740991u) / 9007199254740992.0;
    }
    return xs;
}

static std::vector<Body> bodies() {
    auto r = randoms(35, 1);
    std::vector<Body> bs;
    for (std::size_t k = 0; k < 35; k += 7)
        bs.push_back({10.0 * r[k] - 5.0, 10.0 * r[k + 1] - 5.0, 10.0 * r[k + 2] - 5.0,
                      r[k + 3] - 0.5, r[k + 4] - 0.5, r[k + 5] - 0.5, r[k + 6] + 0.5});
    return bs;
}

static void advance(std::vector<Body>& bs, double dt) {
    for (std::size_t i = 0; i < bs.size(); i++)
        for (std::size_t j = i + 1; j < bs.size(); j++) {
            double dx = bs[i].x - bs[j].x;
            double dy = bs[i].y - bs[j].y;
            double dz = bs[i].z - bs[j].z;
            double d2 = dx * dx + dy * dy + dz * dz;
            double mag = dt / (d2 * std::sqrt(d2));
            double mi = bs[i].mass * mag;
            double mj = bs[j].mass * mag;
            bs[i].vx -= dx * mj;
            bs[i].vy -= dy * mj;
            bs[i].vz -= dz * mj;
            bs[j].vx += dx * mi;
            bs[j].vy += dy * mi;
            bs[j].vz += dz * mi;
        }
    for (auto& b : bs) {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}

static double energy(const std::vector<Body>& bs) {
    double e = 0.0;
    for (std::size_t i = 0; i < bs.size(); i++) {
        const Body& a = bs[i];
        e += 0.5 * a.mass * (a.vx * a.vx + a.vy * a.vy + a.vz * a.vz);
        for (std::size_t j = i + 1; j < bs.size(); j++) {
            double dx = a.x - bs[j].x;
            double dy = a.y - bs[j].y;
            double dz = a.z - bs[j].z;
            e -= a.mass * bs[j].mass / std::sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return e;
}

int main() {
    auto bs = bodies();
    for (int s = 0; s < 30000000; s++) advance(bs, 0.001);
    std::printf("%lld\n", static_cast<long long>(energy(bs) * 1000000000000.0));
}
