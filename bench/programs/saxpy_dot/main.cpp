// Scale-and-add over two lists, then their dot product, repeated.

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

static void saxpy(double a, const std::vector<double>& x, std::vector<double>& y) {
    for (std::size_t i = 0; i < y.size(); i++) y[i] = a * x[i] + y[i];
}

static double dot(const std::vector<double>& a, const std::vector<double>& b) {
    double sum = 0.0;
    for (std::size_t i = 0; i < a.size(); i++) sum += a[i] * b[i];
    return sum;
}

int main() {
    auto x = randoms(1000000, 1);
    auto y = randoms(1000000, 2);
    double total = 0.0;
    for (int r = 0; r < 1000; r++) {
        saxpy(0.5, x, y);
        total += dot(x, y);
    }
    std::printf("%lld\n", static_cast<long long>(total * 1000.0));
}
