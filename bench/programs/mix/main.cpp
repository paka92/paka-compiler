// A 64-bit mixing generator folded into a running hash. Wrapping arithmetic.

#include <cstdint>
#include <cstdio>

// Sign-copying right shift, as Paka's shift_right.
static uint64_t sar(uint64_t z, int k) { return uint64_t(int64_t(z) >> k); }

static uint64_t hash(int64_t n) {
    uint64_t s = 1, h = 0;
    for (int64_t i = 0; i < n; i++) {
        s += 0x9E3779B97F4A7C15u;
        uint64_t z = (s ^ sar(s, 30)) * 0xBF58476D1CE4E5B9u;
        z = (z ^ sar(z, 27)) * 0x94D049BB133111EBu;
        z ^= sar(z, 31);
        h = (h ^ z) * 1099511628211u;
    }
    return h;
}

int main() {
    std::printf("%lld\n", static_cast<long long>(hash(1000000000)));
}
