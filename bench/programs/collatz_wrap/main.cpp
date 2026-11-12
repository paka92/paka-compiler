// The start below N of the longest Collatz chain. Wrapping arithmetic.

#include <cstdint>
#include <cstdio>

static uint64_t chain(uint64_t x) {
    uint64_t n = 1;
    while (x != 1) {
        x = x % 2 == 0 ? x / 2 : 3 * x + 1;
        n += 1;
    }
    return n;
}

int main() {
    uint64_t best = 0, best_start = 0;
    for (uint64_t i = 1; i < 9000000; i++) {
        uint64_t n = chain(i);
        if (n > best) {
            best = n;
            best_start = i;
        }
    }
    std::printf("%llu\n", static_cast<unsigned long long>(best_start));
}
