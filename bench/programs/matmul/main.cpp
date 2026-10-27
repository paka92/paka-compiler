// Plain matrix product of two square matrices held in flat lists.

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

static std::vector<double> multiply(const std::vector<double>& a, const std::vector<double>& b,
                                  std::size_t n) {
    std::vector<double> c(n * n, 0.0);
    for (std::size_t i = 0; i < n; i++)
        for (std::size_t k = 0; k < n; k++) {
            double aik = a[i * n + k];
            for (std::size_t j = 0; j < n; j++) c[i * n + j] += aik * b[k * n + j];
        }
    return c;
}

int main() {
    std::size_t n = 1500;
    auto c = multiply(randoms(n * n, 1), randoms(n * n, 2), n);
    double total = 0.0;
    for (double x : c) total += x;
    std::printf("%lld\n", static_cast<long long>(total * 1000000.0));
}
