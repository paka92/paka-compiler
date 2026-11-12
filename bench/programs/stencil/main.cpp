// Five-point average over a square grid, many sweeps; the border stays fixed.

#include <cstdint>
#include <cstdio>
#include <vector>

static std::vector<double> randoms(std::size_t n, uint64_t s) {
    std::vector<double> xs(n);
    for (auto& x : xs) {
        s = s * 6364136223846793005u + 1442695040888963407u;
        x = double((s >> 11) & 9007199254740991u) / 9007199254740992.0;
    }
    return xs;
}

static void sweep(const std::vector<double>& src, std::vector<double>& dst, std::size_t n) {
    for (std::size_t i = 1; i < n - 1; i++)
        for (std::size_t j = 1; j < n - 1; j++) {
            std::size_t p = i * n + j;
            dst[p] = (src[p] + src[p - 1] + src[p + 1] + src[p - n] + src[p + n]) * 0.2;
        }
}

int main() {
    std::size_t n = 1000;
    auto a = randoms(n * n, 1);
    auto b = a;
    for (int s = 0; s < 1000; s++) {
        sweep(a, b, n);
        sweep(b, a, n);
    }
    double total = 0.0;
    for (double x : a) total += x;
    std::printf("%lld\n", static_cast<long long>(total * 1000000000.0));
}
